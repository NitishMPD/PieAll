//! Invoke the browser's own screenshot controls, without clipboard borrowing or page scripts.
use crate::{platform, shortcuts::Shortcut};
use std::time::{Duration, Instant};
use windows::{
    core::BSTR,
    Win32::{Foundation::HWND, System::Com::*, UI::Accessibility::*},
};

pub(crate) fn finish(source: usize, pid: u32, action: &Shortcut) -> Result<(), String> {
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let result = capture(source, pid, action);
        if initialized {
            CoUninitialize();
        }
        result
    }
}

fn validate(source: usize, pid: u32, action: &Shortcut) -> Result<(), String> {
    if unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() } as usize
        != source
        || platform::window_profile(source, pid)?.is_none_or(|p| p.id != action.profile)
    {
        return Err("Browser focus changed. Invoke the screenshot action again.".into());
    }
    Ok(())
}

unsafe fn capture(source: usize, pid: u32, action: &Shortcut) -> Result<(), String> {
    let automation: IUIAutomation =
        CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).map_err(|e| e.to_string())?;
    let root = automation
        .ElementFromHandle(HWND(source as _))
        .map_err(|e| e.to_string())?;
    let full = action.id.ends_with(":capture-full-page");
    let firefox = action.keys == "Ctrl+Shift+S";
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        validate(source, pid, action)?;
        if firefox {
            // Firefox presents its privileged screenshot overlay. Invoking the choice
            // opens the native preview, where the user can Copy or Download.
            let name = if full {
                "Save full page"
            } else {
                "Save visible"
            };
            let condition = automation
                .CreatePropertyCondition(UIA_NamePropertyId, &name.into())
                .map_err(|e| e.to_string())?;
            if let Ok(button) = root.FindFirst(TreeScope_Descendants, &condition) {
                if button.CurrentControlType().ok() == Some(UIA_ButtonControlTypeId) {
                    if let Ok(invoke) = button
                        .GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
                    {
                        validate(source, pid, action)?;
                        return invoke.Invoke().map_err(|e| e.to_string());
                    }
                }
            }
        } else if let Ok(input) = automation.GetFocusedElement() {
            // Only write into the DevTools command menu. Never type into a page,
            // console or arbitrary focused field if the shortcut failed to open it.
            let name = input.CurrentName().unwrap_or_default().to_string();
            if input.CurrentControlType().ok() == Some(UIA_EditControlTypeId)
                && matches!(name.as_str(), "Run" | "Run command" | "Run Command")
            {
                if let Ok(value) =
                    input.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
                {
                    validate(source, pid, action)?;
                    let command = if full {
                        "Capture full size screenshot"
                    } else {
                        "Capture screenshot"
                    };
                    value
                        .SetValue(&BSTR::from(command))
                        .map_err(|e| e.to_string())?;
                    // Let the command menu filter its results before accepting one.
                    std::thread::sleep(Duration::from_millis(250));
                    validate(source, pid, action)?;
                    if !automation
                        .CompareElements(
                            &input,
                            &automation.GetFocusedElement().map_err(|e| e.to_string())?,
                        )
                        .map_err(|e| e.to_string())?
                        .as_bool()
                    {
                        return Err("Screenshot command lost focus. Nothing was submitted.".into());
                    }
                    let enter = Shortcut {
                        id: String::new(),
                        profile: action.profile.clone(),
                        title: action.title.clone(),
                        keys: "Enter".into(),
                        category: "Navigation".into(),
                        priority: None,
                    };
                    return platform::send_app_shortcut(source, pid, &enter);
                }
            }
        }
        if Instant::now() >= deadline {
            return Err("Browser screenshot controls were unavailable. This action requires English browser controls and enabled, docked developer tools in Chromium. Firefox opens its screenshot preview. Finish or close any browser dialog and try again.".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
