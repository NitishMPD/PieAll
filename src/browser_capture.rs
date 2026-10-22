//! Browser screenshots without DevTools, page scripts or clipboard borrowing.
mod scrolling;

use crate::{platform, shortcuts::Shortcut};
use std::time::{Duration, Instant};
use windows::Win32::{Foundation::HWND, System::Com::*, UI::Accessibility::*};

pub(crate) fn run(source: usize, pid: u32, action: &Shortcut) -> Result<Option<String>, String> {
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

unsafe fn capture(source: usize, pid: u32, action: &Shortcut) -> Result<Option<String>, String> {
    validate(source, pid, action)?;
    let full = action.id.ends_with(":capture-full-page");
    let automation: IUIAutomation =
        CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).map_err(|e| e.to_string())?;
    let root = automation
        .ElementFromHandle(HWND(source as _))
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(4);
    if !full {
        // Read the visible document's physical bounds before opening any browser UI.
        loop {
            validate(source, pid, action)?;
            if let Some(rect) = page_rect(&automation, &root, source)? {
                return copy_viewport(source, pid, action, rect)
                    .map(|()| Some("Current page copied to clipboard.".into()));
            }
            if Instant::now() >= deadline {
                return Err("The visible page area is unavailable. Focus a normal browser tab and try again. No DevTools were opened.".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    if !crate::shortcuts::native_full_page_supported(&action.profile) {
        return scrolling::capture(&automation, &root, source, pid, action).map(Some);
    }
    // Edge and Firefox both provide a built-in screenshot overlay on Ctrl+Shift+S.
    // Never fall back to DevTools or another shortcut on unsupported browsers.
    let open = Shortcut {
        id: String::new(),
        profile: action.profile.clone(),
        title: action.title.clone(),
        keys: "Ctrl+Shift+S".into(),
        category: "Navigation".into(),
        priority: None,
    };
    platform::send_app_shortcut(source, pid, &open)?;
    let name = if action.profile == "edge" {
        "Capture full page"
    } else {
        "Save full page"
    };
    let condition = automation
        .CreatePropertyCondition(UIA_NamePropertyId, &name.into())
        .map_err(|e| e.to_string())?;
    loop {
        validate(source, pid, action)?;
        if let Ok(button) = root.FindFirst(TreeScope_Descendants, &condition) {
            if button.CurrentControlType().ok() == Some(UIA_ButtonControlTypeId)
                && button
                    .CurrentIsOffscreen()
                    .is_ok_and(|value| !value.as_bool())
            {
                if let Ok(invoke) =
                    button.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
                {
                    validate(source, pid, action)?;
                    return invoke
                        .Invoke()
                        .map(|()| {
                            Some(
                                "Full-page capture requested. Finish saving in your browser."
                                    .into(),
                            )
                        })
                        .map_err(|e| e.to_string());
                }
            }
        }
        if Instant::now() >= deadline {
            return Err("The browser's full-page screenshot control was unavailable. Use Capture full page in Edge's Screenshot tool, or Save full page in Firefox. No DevTools were opened.".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
impl Rect {
    fn intersect(self, other: Self) -> Option<Self> {
        let rect = Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        };
        (rect.right > rect.left && rect.bottom > rect.top).then_some(rect)
    }
    fn size(self) -> Result<(i32, i32, usize), String> {
        let width = i64::from(self.right) - i64::from(self.left);
        let height = i64::from(self.bottom) - i64::from(self.top);
        if width <= 0
            || height <= 0
            || width > 16384
            || height > 16384
            || width * height > 16 * 1024 * 1024
        {
            return Err("The visible page exceeds the screenshot size limit.".into());
        }
        Ok((width as i32, height as i32, (width * height * 4) as usize))
    }
    fn area(self) -> i64 {
        (i64::from(self.right) - i64::from(self.left))
            * (i64::from(self.bottom) - i64::from(self.top))
    }
}

unsafe fn page_document(
    automation: &IUIAutomation,
    root: &IUIAutomationElement,
    source: usize,
) -> Result<Option<(Rect, IUIAutomationElement)>, String> {
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        Graphics::Gdi::ClientToScreen,
        UI::WindowsAndMessaging::GetClientRect,
    };
    let mut client: RECT = std::mem::zeroed();
    if GetClientRect(source as _, &mut client) == 0 {
        return Err("Cannot read the browser bounds.".into());
    }
    let mut origin = POINT { x: 0, y: 0 };
    if ClientToScreen(source as _, &mut origin) == 0 {
        return Err("Cannot locate the browser on screen.".into());
    }
    let client = Rect {
        left: origin.x,
        top: origin.y,
        right: origin.x + client.right,
        bottom: origin.y + client.bottom,
    };
    let condition = automation
        .CreatePropertyCondition(
            UIA_ControlTypePropertyId,
            &UIA_DocumentControlTypeId.0.into(),
        )
        .map_err(|e| e.to_string())?;
    let documents = root
        .FindAll(TreeScope_Descendants, &condition)
        .map_err(|e| e.to_string())?;
    let mut best: Option<(Rect, IUIAutomationElement)> = None;
    for index in 0..documents.Length().map_err(|e| e.to_string())?.min(512) {
        let Ok(document) = documents.GetElement(index) else {
            continue;
        };
        if !document
            .CurrentIsOffscreen()
            .is_ok_and(|value| !value.as_bool())
        {
            continue;
        }
        let name = document.CurrentName().unwrap_or_default().to_string();
        if name.starts_with("DevTools") || name.starts_with("Developer Tools") {
            continue;
        }
        let Ok(bounds) = document.CurrentBoundingRectangle() else {
            continue;
        };
        let rect = Rect {
            left: bounds.left,
            top: bounds.top,
            right: bounds.right,
            bottom: bounds.bottom,
        };
        if let Some(rect) = rect.intersect(client) {
            if best
                .as_ref()
                .is_none_or(|(current, _)| rect.area() > current.area())
            {
                best = Some((rect, document));
            }
        }
    }
    Ok(best)
}

unsafe fn page_rect(
    automation: &IUIAutomation,
    root: &IUIAutomationElement,
    source: usize,
) -> Result<Option<Rect>, String> {
    Ok(page_document(automation, root, source)?.map(|(rect, _)| rect))
}

unsafe fn copy_viewport(
    source: usize,
    pid: u32,
    action: &Shortcut,
    rect: Rect,
) -> Result<(), String> {
    std::thread::sleep(Duration::from_millis(150));
    let data = read_viewport(source, pid, action, rect)?;
    let (width, height, _) = rect.size()?;
    validate(source, pid, action)?;
    platform::set_clipboard_image(width as u32, height as u32, &data)
}

unsafe fn read_viewport(
    source: usize,
    pid: u32,
    action: &Shortcut,
    rect: Rect,
) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Graphics::Gdi::*;
    let (width, height, bytes) = rect.size()?;
    validate(source, pid, action)?;
    let screen = GetDC(std::ptr::null_mut());
    if screen.is_null() {
        return Err("Windows could not capture the visible page.".into());
    }
    let memory = CreateCompatibleDC(screen);
    if memory.is_null() {
        ReleaseDC(std::ptr::null_mut(), screen);
        return Err("Windows could not allocate screenshot memory.".into());
    }
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = width;
    info.bmiHeader.biHeight = -height;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    let mut pixels = std::ptr::null_mut();
    let bitmap = CreateDIBSection(
        screen,
        &info,
        DIB_RGB_COLORS,
        &mut pixels,
        std::ptr::null_mut(),
        0,
    );
    let result = if bitmap.is_null() || pixels.is_null() {
        Err("Windows could not allocate screenshot pixels.".into())
    } else {
        let previous = SelectObject(memory, bitmap);
        let copied = BitBlt(
            memory,
            0,
            0,
            width,
            height,
            screen,
            rect.left,
            rect.top,
            SRCCOPY | CAPTUREBLT,
        );
        SelectObject(memory, previous);
        if copied == 0 || GdiFlush() == 0 {
            Err("Windows could not capture the visible page.".into())
        } else {
            let mut data = std::slice::from_raw_parts(pixels as *const u8, bytes).to_vec();
            for pixel in data.chunks_exact_mut(4) {
                pixel[3] = 255;
            }
            validate(source, pid, action).map(|()| data)
        }
    };
    if !bitmap.is_null() {
        DeleteObject(bitmap);
    }
    DeleteDC(memory);
    ReleaseDC(std::ptr::null_mut(), screen);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn viewport_capture_clips_to_browser_client_and_rejects_empty_regions() {
        let page = Rect {
            left: -40,
            top: 120,
            right: 1200,
            bottom: 900,
        };
        let client = Rect {
            left: 0,
            top: 0,
            right: 1000,
            bottom: 800,
        };
        let clipped = page.intersect(client).unwrap();
        assert_eq!(
            clipped,
            Rect {
                left: 0,
                top: 120,
                right: 1000,
                bottom: 800
            }
        );
        assert_eq!(clipped.size().unwrap(), (1000, 680, 2_720_000));
        assert!(page
            .intersect(Rect {
                left: 2000,
                top: 0,
                right: 3000,
                bottom: 100
            })
            .is_none());
    }
    #[test]
    fn viewport_capture_bounds_allocations_and_handles_negative_monitor_coordinates() {
        assert_eq!(
            Rect {
                left: -1920,
                top: -100,
                right: 0,
                bottom: 980
            }
            .size()
            .unwrap(),
            (1920, 1080, 8_294_400)
        );
        for rect in [
            Rect {
                left: 0,
                top: 0,
                right: 16385,
                bottom: 1,
            },
            Rect {
                left: 0,
                top: 0,
                right: 16384,
                bottom: 16384,
            },
            Rect {
                left: i32::MIN,
                top: 0,
                right: i32::MAX,
                bottom: 1,
            },
        ] {
            assert!(rect.size().is_err());
        }
    }
}
