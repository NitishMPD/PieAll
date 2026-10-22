use std::{path::PathBuf, ptr};
use windows_sys::Win32::{
    System::{DataExchange::*, Memory::*},
    UI::{Controls::Dialogs::*, WindowsAndMessaging::*},
};
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn show_error(s: &str) {
    unsafe {
        MessageBoxW(
            ptr::null_mut(),
            wide(s).as_ptr(),
            wide("PieAll").as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
pub fn clipboard_text() -> Result<String, String> {
    unsafe {
        if !open_clipboard(0) {
            return Err("Clipboard is busy.".into());
        }
        let h = GetClipboardData(13);
        if h.is_null() {
            CloseClipboard();
            return Err("Clipboard does not contain Unicode text.".into());
        }
        let size = GlobalSize(h);
        if size > 2 * 1024 * 1024 {
            CloseClipboard();
            return Err("Clipboard text exceeds 2 MiB.".into());
        }
        let p = GlobalLock(h) as *const u16;
        if p.is_null() {
            CloseClipboard();
            return Err("Cannot read clipboard.".into());
        }
        let slice = std::slice::from_raw_parts(p, size / 2);
        let n = slice.iter().position(|&x| x == 0).unwrap_or(slice.len());
        let s = String::from_utf16_lossy(&slice[..n]);
        GlobalUnlock(h);
        CloseClipboard();
        Ok(s)
    }
}
pub fn set_clipboard_text(s: &str) -> Result<(), String> {
    set_text(s, false)
}
fn set_text(s: &str, temporary: bool) -> Result<(), String> {
    unsafe {
        let data = wide(s);
        let h = GlobalAlloc(GMEM_MOVEABLE, data.len() * 2);
        if h.is_null() {
            return Err("Clipboard allocation failed.".into());
        }
        let p = GlobalLock(h) as *mut u16;
        if p.is_null() {
            windows_sys::Win32::Foundation::GlobalFree(h);
            return Err("Clipboard allocation failed.".into());
        }
        ptr::copy_nonoverlapping(data.as_ptr(), p, data.len());
        GlobalUnlock(h);
        if !open_clipboard(0) {
            windows_sys::Win32::Foundation::GlobalFree(h);
            return Err("Clipboard is busy.".into());
        }
        EmptyClipboard();
        let result = SetClipboardData(13, h);
        if temporary {
            exclude_from_history();
        }
        CloseClipboard();
        if result.is_null() {
            windows_sys::Win32::Foundation::GlobalFree(h);
            Err("Cannot write clipboard.".into())
        } else {
            Ok(())
        }
    }
}
pub fn pick_files() -> Result<Vec<PathBuf>, String> {
    pick_paths("Choose files for PieAll", true)
}
pub fn pick_engine() -> Result<Vec<PathBuf>, String> {
    pick_paths("Choose a trusted tool executable for PieAll", false)
}
pub fn pick_download_folder() -> Result<Option<PathBuf>, String> {
    use windows::Win32::{System::Com::*, UI::Shell::*};
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let result = (|| -> windows::core::Result<Option<PathBuf>> {
            let dialog: IFileOpenDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
            dialog.SetOptions(
                dialog.GetOptions()?
                    | FOS_PICKFOLDERS
                    | FOS_FORCEFILESYSTEM
                    | FOS_PATHMUSTEXIST
                    | FOS_NOCHANGEDIR,
            )?;
            dialog.SetTitle(windows::core::w!("Choose a folder for PieAll downloads"))?;
            if let Err(e) = dialog.Show(None) {
                if e.code().0 as u32 == 0x800704c7 {
                    return Ok(None);
                }
                return Err(e);
            }
            let raw = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
            let text = raw.to_string();
            CoTaskMemFree(Some(raw.0 as *const _));
            Ok(Some(PathBuf::from(text?)))
        })();
        if initialized {
            CoUninitialize();
        }
        result.map_err(|e| e.to_string())
    }
}
fn pick_paths(title: &str, multiple: bool) -> Result<Vec<PathBuf>, String> {
    unsafe {
        let mut buffer = vec![0u16; 65536];
        let mut o: OPENFILENAMEW = std::mem::zeroed();
        o.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        o.lpstrFile = buffer.as_mut_ptr();
        o.nMaxFile = buffer.len() as u32;
        let title = wide(title);
        o.lpstrTitle = title.as_ptr();
        o.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_NOCHANGEDIR;
        if multiple {
            o.Flags |= OFN_ALLOWMULTISELECT;
        }
        if GetOpenFileNameW(&mut o) == 0 {
            let code = CommDlgExtendedError();
            return if code == 0 {
                Ok(vec![])
            } else {
                Err(format!("File picker error {code}"))
            };
        }
        let parts: Vec<String> = buffer
            .split(|&c| c == 0)
            .take_while(|s| !s.is_empty())
            .map(String::from_utf16_lossy)
            .collect();
        if parts.len() == 1 {
            Ok(vec![PathBuf::from(&parts[0])])
        } else {
            Ok(parts[1..]
                .iter()
                .map(|p| PathBuf::from(&parts[0]).join(p))
                .collect())
        }
    }
}
pub fn capture_selection() -> Result<Vec<PathBuf>, String> {
    capture_selection_from(unsafe { GetForegroundWindow() } as usize)
}
pub fn capture_selection_from(source: usize) -> Result<Vec<PathBuf>, String> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    let mut child = Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
        .args(["--capture", &source.to_string()])
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or("Capture pipe unavailable")?;
    let reader = std::thread::spawn(move || {
        let mut data = Vec::new();
        stdout
            .take(1024 * 1024)
            .read_to_end(&mut data)
            .map(|_| data)
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1200);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err("Selection helper stopped unexpectedly.".into());
                }
                break;
            }
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err("Selection timed out. Drop files or use Open files.".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let data = reader
        .join()
        .map_err(|_| "Capture reader stopped")?
        .map_err(|e| e.to_string())?;
    serde_json::from_slice(&data).map_err(|_| "Invalid capture response".to_string())?
}
pub fn window_class(hwnd: usize) -> String {
    let mut buffer = [0u16; 128];
    let n = unsafe { GetClassNameW(hwnd as _, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..n.max(0) as usize])
}
/// Explorer windows and the desktop answer with file paths; everything else may have text.
pub fn is_shell_window(hwnd: usize) -> bool {
    matches!(
        window_class(hwnd).as_str(),
        "CabinetWClass" | "ExploreWClass" | "Progman" | "WorkerW"
    )
}
/// Ctrl+C means "interrupt" in terminals, so PieAll never sends it there.
pub fn is_terminal_window(hwnd: usize) -> bool {
    matches!(
        window_class(hwnd).as_str(),
        "ConsoleWindowClass"
            | "CASCADIA_HOSTING_WINDOW_CLASS"
            | "mintty"
            | "PuTTY"
            | "VirtualConsoleClass"
    )
}
pub fn capture_selection_direct(source: usize) -> Result<Vec<PathBuf>, String> {
    use windows::{
        core::Interface,
        Win32::{
            System::{Com::*, Variant::VARIANT},
            UI::Shell::*,
        },
    };
    unsafe {
        let foreground = source as isize;
        let desktop = matches!(window_class(source).as_str(), "Progman" | "WorkerW");
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let result = (|| -> windows::core::Result<Vec<PathBuf>> {
            let shell: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)?;
            let mut providers: Vec<IServiceProvider> = Vec::new();
            if desktop {
                let mut hwnd = 0i32;
                let dispatch = shell.FindWindowSW(
                    &VARIANT::from(CSIDL_DESKTOP as i32),
                    &VARIANT::default(),
                    SWC_DESKTOP,
                    &mut hwnd,
                    SWFO_NEEDDISPATCH,
                )?;
                providers.push(dispatch.cast()?);
            } else {
                for i in 0..shell.Count()? {
                    let variant = VARIANT::from(i);
                    let Ok(dispatch) = shell.Item(&variant) else {
                        continue;
                    };
                    let Ok(browser) = dispatch.cast::<IWebBrowserApp>() else {
                        continue;
                    };
                    if browser.HWND()?.0 == foreground {
                        providers.push(browser.cast()?);
                    }
                }
            }
            // Explorer tabs share one frame window; only the active tab's view is visible.
            let mut views = Vec::new();
            for provider in providers {
                let Ok(browser) = provider.QueryService::<IShellBrowser>(&SID_STopLevelBrowser)
                else {
                    continue;
                };
                let Ok(view) = browser.QueryActiveShellView() else {
                    continue;
                };
                if view.GetWindow().is_ok_and(|h| IsWindowVisible(h.0) != 0) {
                    views.push(view);
                }
            }
            if views.len() != 1 {
                return Ok(vec![]);
            }
            let folder: IFolderView2 = views[0].cast()?;
            let path_of = |item: IShellItem| -> Option<PathBuf> {
                let raw = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
                let text = raw.to_string();
                CoTaskMemFree(Some(raw.0 as *const _));
                Some(PathBuf::from(text.ok()?)).filter(|p| p.exists())
            };
            let mut paths = Vec::new();
            if let Ok(items) = folder.GetSelection(false) {
                if items.GetCount()? > 256 {
                    return Ok(vec![]);
                }
                for i in 0..items.GetCount()? {
                    if let Some(p) = path_of(items.GetItemAt(i)?) {
                        paths.push(p);
                    }
                }
            }
            // Nothing selected: offer the open folder itself.
            if paths.is_empty() {
                if let Some(p) = folder.GetFolder::<IShellItem>().ok().and_then(path_of) {
                    paths.push(p);
                }
            }
            Ok(paths)
        })();
        if initialized {
            CoUninitialize();
        }
        result.map_err(|_| "Selection unavailable. Drop files or use Open files.".into())
    }
}

/// Marks keystrokes PieAll injects so its own keyboard hook lets them through.
pub const INJECTED_MARKER: usize = 0x5049_4541;

/// Query the process behind the captured HWND; titles and frontend hints are not identity.
pub fn window_executable(source: usize, expected_pid: u32) -> Result<String, String> {
    use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::*};
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(source as _, &mut pid);
        if expected_pid == 0 || pid != expected_pid || IsWindow(source as _) == 0 {
            return Err("Source window changed or closed. Invoke PieAll again.".into());
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return Err("Cannot identify the source app.".into());
        }
        let mut path = vec![0u16; 32768];
        let mut length = path.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length);
        CloseHandle(process);
        if ok == 0 {
            return Err("Cannot identify the source app.".into());
        }
        let path = String::from_utf16_lossy(&path[..length as usize]);
        Ok(path.rsplit('\\').next().unwrap_or("").to_ascii_lowercase())
    }
}

pub fn window_profile(
    source: usize,
    pid: u32,
) -> Result<Option<&'static crate::shortcuts::Profile>, String> {
    let executable = window_executable(source, pid)?;
    Ok(
        crate::shortcuts::for_executable(&executable).filter(|profile| {
            // Explorer also hosts the desktop, where tab/address-bar commands do not apply.
            profile.id != "explorer"
                || matches!(
                    window_class(source).as_str(),
                    "CabinetWClass" | "ExploreWClass"
                )
        }),
    )
}

/// One bounded batch per chord, with foreground/process revalidation between chords.
/// The caller owns an operation lease and has already returned focus to the source.
pub fn send_app_shortcut(
    source: usize,
    pid: u32,
    action: &crate::shortcuts::Shortcut,
) -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    let sequence = crate::shortcuts::parse_keys(&action.keys)?;
    let validate = || -> Result<(), String> {
        if unsafe { GetForegroundWindow() } as usize != source {
            return Err("Focus changed. No further shortcut keys were sent.".into());
        }
        if window_profile(source, pid)?.is_none_or(|p| p.id != action.profile) {
            return Err("The captured app no longer matches this shortcut.".into());
        }
        Ok(())
    };
    // Let the invocation key and mouse/key activation come up; never release physical keys.
    // A slice clicked while ` is still held waits here for the user to let go.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        validate()?;
        let held = (1..=254).any(|key| unsafe { GetAsyncKeyState(key) } < 0);
        if !held {
            break;
        }
        if std::time::Instant::now() >= deadline {
            return Err("Release held keys and invoke the action again.".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let event = |vk: u16, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: (if up { KEYEVENTF_KEYUP } else { 0 })
                    | if matches!(vk, 0x21..=0x28 | 0x2D..=0x2E) {
                        KEYEVENTF_EXTENDEDKEY
                    } else {
                        0
                    },
                time: 0,
                dwExtraInfo: INJECTED_MARKER,
            },
        },
    };
    for (index, chord) in sequence.iter().enumerate() {
        if index != 0 {
            std::thread::sleep(std::time::Duration::from_millis(
                if action.category == "Screenshots" {
                    900
                } else {
                    70
                },
            ));
        }
        validate()?;
        let inputs: Vec<_> = chord
            .iter()
            .map(|&vk| event(vk, false))
            .chain(chord.iter().rev().map(|&vk| event(vk, true)))
            .collect();
        if (1..=254).any(|key| unsafe { GetAsyncKeyState(key) } < 0) {
            return Err("Input changed. No further shortcut keys were sent.".into());
        }
        let sent = unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            )
        };
        if sent != inputs.len() as u32 {
            // SendInput can partially insert a batch. Release only keys actually injected down.
            let mut down = Vec::new();
            for (position, &vk) in chord.iter().enumerate() {
                if position < sent as usize && chord.len() * 2 - 1 - position >= sent as usize {
                    down.push(event(vk, true));
                }
            }
            if !down.is_empty() {
                unsafe {
                    SendInput(
                        down.len() as u32,
                        down.as_ptr(),
                        std::mem::size_of::<INPUT>() as i32,
                    );
                }
            }
            return Err("Windows blocked shortcut input. The target may be elevated.".into());
        }
    }
    if action.category == "Screenshots" {
        return crate::browser_capture::finish(source, pid, action);
    }
    Ok(())
}
pub fn send_chord(modifier: u16, key: u16) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    let event = |vk: u16, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: INJECTED_MARKER,
            },
        },
    };
    let inputs = [
        event(modifier, false),
        event(key, false),
        event(key, true),
        event(modifier, true),
    ];
    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
    }
}
fn open_clipboard(owner: usize) -> bool {
    for _ in 0..20 {
        if unsafe { OpenClipboard(owner as _) } != 0 {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    false
}
fn exclude_from_history() {
    // Keeps PieAll's temporary clipboard use out of Win+V history and cloud sync.
    unsafe {
        let format =
            RegisterClipboardFormatW(wide("ExcludeClipboardContentFromMonitorProcessing").as_ptr());
        let h = GlobalAlloc(GMEM_MOVEABLE, 4);
        if format != 0 && !h.is_null() && SetClipboardData(format, h).is_null() {
            windows_sys::Win32::Foundation::GlobalFree(h);
        }
    }
}
/// A byte copy of every memory-backed clipboard format, so a borrowed clipboard can be put back.
pub struct ClipboardSnapshot(Vec<(u32, Vec<u8>)>);
impl ClipboardSnapshot {
    pub fn take(owner: usize) -> Option<Self> {
        if !open_clipboard(owner) {
            return None;
        }
        let mut formats = Vec::new();
        let mut total = 0usize;
        let mut format = 0;
        let complete = loop {
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break true;
            }
            // GDI handles and private handle formats are not memory blocks; Windows
            // regenerates the standard bitmap/metafile views from their DIB/EMF siblings.
            if matches!(format, 2 | 3 | 9 | 14 | 0x80..=0x8E | 0x200..=0x3FF) {
                continue;
            }
            unsafe {
                let h = GetClipboardData(format);
                if h.is_null() {
                    continue;
                }
                let size = GlobalSize(h);
                total += size;
                if total > 64 * 1024 * 1024 {
                    break false;
                }
                let p = GlobalLock(h) as *const u8;
                if p.is_null() {
                    continue;
                }
                formats.push((format, std::slice::from_raw_parts(p, size).to_vec()));
                GlobalUnlock(h);
            }
        };
        unsafe { CloseClipboard() };
        complete.then_some(Self(formats))
    }
    pub fn restore(&self, owner: usize) -> Result<(), String> {
        if !open_clipboard(owner) {
            return Err("Clipboard is busy.".into());
        }
        self.restore_opened();
        Ok(())
    }
    fn restore_if_unchanged(&self, owner: usize, sequence: u32) -> Result<(), String> {
        if !open_clipboard(owner) {
            return Err("Clipboard is busy.".into());
        }
        if unsafe { GetClipboardSequenceNumber() } != sequence {
            unsafe {
                CloseClipboard();
            }
            return Err("Clipboard changed. Choose Download again.".into());
        }
        self.restore_opened();
        Ok(())
    }
    fn restore_opened(&self) {
        unsafe {
            EmptyClipboard();
            for (format, bytes) in &self.0 {
                let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
                if h.is_null() {
                    continue;
                }
                let p = GlobalLock(h) as *mut u8;
                if p.is_null() {
                    windows_sys::Win32::Foundation::GlobalFree(h);
                    continue;
                }
                ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
                GlobalUnlock(h);
                if SetClipboardData(*format, h).is_null() {
                    windows_sys::Win32::Foundation::GlobalFree(h);
                }
            }
            exclude_from_history();
            CloseClipboard();
        }
    }
}
/// Borrow the captured browser's address bar only after the user chooses Download.
pub fn copy_browser_url(source: usize, pid: u32) -> Result<String, String> {
    let profile = window_profile(source, pid)?
        .filter(|p| p.sets.iter().any(|s| s == "browser" || s == "firefox"))
        .ok_or("Invoke Download video from a supported browser, or paste its URL in Browser downloads.")?;
    let shortcut = crate::shortcuts::find(&format!("app:{}:copy-url", profile.id))
        .ok_or("This browser has no Copy URL shortcut. Paste the URL in Browser downloads.")?;
    let snapshot = ClipboardSnapshot::take(source).ok_or("Clipboard is busy. Try again.")?;
    let before = unsafe { GetClipboardSequenceNumber() };
    let sent = send_app_shortcut(source, pid, shortcut);
    if sent.is_err() && unsafe { GetClipboardSequenceNumber() } == before {
        return sent.map(|_| String::new());
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
    while unsafe { GetClipboardSequenceNumber() } == before {
        if std::time::Instant::now() >= deadline {
            return Err(
                "The browser did not copy its address. Paste the URL in Browser downloads.".into(),
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let copied = unsafe { GetClipboardSequenceNumber() };
    let mut clipboard_pid = 0;
    unsafe {
        GetWindowThreadProcessId(GetClipboardOwner(), &mut clipboard_pid);
    }
    if clipboard_pid != pid {
        return Err("Another app changed the clipboard. Choose Download again.".into());
    }
    let text = clipboard_text();
    snapshot.restore_if_unchanged(source, copied)?;
    sent?;
    if unsafe { GetForegroundWindow() } as usize != source {
        return Err("Browser focus changed. Choose Download again.".into());
    }
    let text = text?;
    Ok(crate::web_proxy::web_url(text.trim())?.to_string())
}

/// Copies the source window's text selection with Ctrl+C, then puts the clipboard back.
/// Returns nothing when the app copied no text, which means nothing was selected.
/// `cancelled` lets a ring that already ran something else stop waiting; the
/// clipboard is still restored.
pub fn copy_selection(
    owner: usize,
    cancelled: impl Fn() -> bool,
) -> Option<(String, ClipboardSnapshot)> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL;
    if cancelled() {
        return None;
    }
    let snapshot = ClipboardSnapshot::take(owner)?;
    let before = unsafe { GetClipboardSequenceNumber() };
    if cancelled() {
        return None;
    }
    send_chord(VK_CONTROL, b'C' as u16);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(300);
    while unsafe { GetClipboardSequenceNumber() } == before {
        if cancelled() {
            let _ = snapshot.restore(owner);
            return None;
        }
        if std::time::Instant::now() > deadline {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    // Apps often write several formats in a row; let them finish.
    std::thread::sleep(std::time::Duration::from_millis(40));
    let text = clipboard_text().ok();
    let _ = snapshot.restore(owner);
    let text = text.filter(|t| !t.trim().is_empty() && t.len() <= 2 * 1024 * 1024)?;
    Some((text, snapshot))
}
/// Replaces the selection in the foreground window with `text`, then restores the clipboard.
/// Returns false when focus moved elsewhere; the text is then left on the clipboard.
pub fn paste_text(
    source: usize,
    text: &str,
    snapshot: &ClipboardSnapshot,
    owner: usize,
) -> Result<bool, String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL;
    set_text(text, true)?;
    std::thread::sleep(std::time::Duration::from_millis(80));
    if unsafe { GetForegroundWindow() } as usize != source {
        return Ok(false);
    }
    send_chord(VK_CONTROL, b'V' as u16);
    // Paste is read asynchronously by the target app; give it time before restoring.
    std::thread::sleep(std::time::Duration::from_millis(400));
    snapshot.restore(owner)?;
    Ok(true)
}
pub fn window_action(id: &str, raw: usize) -> Result<String, String> {
    use windows_sys::Win32::{Foundation::RECT, Graphics::Gdi::*};
    unsafe {
        let h = raw as windows_sys::Win32::Foundation::HWND;
        if h.is_null() || IsWindow(h) == 0 {
            return Err("Source window is no longer available.".into());
        }
        if id == "window_details" {
            let mut b = [0u16; 1024];
            let n = GetWindowTextW(h, b.as_mut_ptr(), b.len() as i32);
            return Ok(format!(
                "{}\nHWND: {raw}",
                String::from_utf16_lossy(&b[..n as usize])
            ));
        }
        if id == "always_top" {
            let on = GetWindowLongPtrW(h, GWL_EXSTYLE) & WS_EX_TOPMOST as isize != 0;
            SetWindowPos(
                h,
                if on { HWND_NOTOPMOST } else { HWND_TOPMOST },
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            return Ok(if on {
                "Window unpinned from the top.".into()
            } else {
                "Window pinned on top.".into()
            });
        }
        let monitor = MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return Err("Cannot inspect monitor.".into());
        }
        let mut r: RECT = std::mem::zeroed();
        GetWindowRect(h, &mut r);
        let w = (r.right - r.left).min(info.rcWork.right - info.rcWork.left);
        let height = (r.bottom - r.top).min(info.rcWork.bottom - info.rcWork.top);
        let (x, y, w, height) = if id == "center_window" {
            (
                info.rcWork.left + (info.rcWork.right - info.rcWork.left - w) / 2,
                info.rcWork.top + (info.rcWork.bottom - info.rcWork.top - height) / 2,
                w,
                height,
            )
        } else {
            if GetWindowLongPtrW(h, GWL_STYLE) & WS_THICKFRAME as isize == 0 {
                return Err("This window cannot be resized.".into());
            }
            (
                info.rcWork.left,
                info.rcWork.top,
                info.rcWork.right - info.rcWork.left,
                info.rcWork.bottom - info.rcWork.top,
            )
        };
        if SetWindowPos(
            h,
            ptr::null_mut(),
            x,
            y,
            w,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        ) == 0
        {
            return Err("Windows refused the window change.".into());
        }
        Ok(if id == "center_window" {
            "Window centered.".into()
        } else {
            "Window fills the screen.".into()
        })
    }
}
pub fn set_clipboard_image(width: u32, height: u32, bgra: &[u8]) -> Result<(), String> {
    unsafe {
        let size = (width as usize)
            .checked_mul(height as usize)
            .and_then(|v| v.checked_mul(4))
            .ok_or("Bitmap size overflow")?;
        if bgra.len() != size || size > 160_000_000 {
            return Err("Clipboard bitmap exceeds limit.".into());
        }
        let mut header = [0u8; 124];
        fn put(header: &mut [u8], at: usize, n: u32) {
            header[at..at + 4].copy_from_slice(&n.to_le_bytes());
        }
        put(&mut header, 0, 124);
        put(&mut header, 4, width);
        put(&mut header, 8, (-(height as i32)) as u32);
        header[12..14].copy_from_slice(&1u16.to_le_bytes());
        header[14..16].copy_from_slice(&32u16.to_le_bytes());
        put(&mut header, 16, 3);
        put(&mut header, 20, size as u32);
        put(&mut header, 40, 0x00ff0000);
        put(&mut header, 44, 0x0000ff00);
        put(&mut header, 48, 0x000000ff);
        put(&mut header, 52, 0xff000000);
        put(&mut header, 56, 0x73524742);
        put(&mut header, 108, 4);
        let memory = GlobalAlloc(GMEM_MOVEABLE, 124 + size);
        if memory.is_null() {
            return Err("Cannot allocate clipboard bitmap.".into());
        }
        let p = GlobalLock(memory) as *mut u8;
        if p.is_null() {
            windows_sys::Win32::Foundation::GlobalFree(memory);
            return Err("Cannot lock clipboard bitmap.".into());
        }
        ptr::copy_nonoverlapping(header.as_ptr(), p, 124);
        ptr::copy_nonoverlapping(bgra.as_ptr(), p.add(124), size);
        GlobalUnlock(memory);
        if OpenClipboard(ptr::null_mut()) == 0 {
            windows_sys::Win32::Foundation::GlobalFree(memory);
            return Err("Clipboard is busy.".into());
        }
        EmptyClipboard();
        let ok = SetClipboardData(17, memory);
        CloseClipboard();
        if ok.is_null() {
            windows_sys::Win32::Foundation::GlobalFree(memory);
            return Err("Cannot publish clipboard bitmap.".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod shortcut_tests {
    #[test]
    fn cancelled_selection_does_not_touch_the_clipboard() {
        let before = unsafe { super::GetClipboardSequenceNumber() };
        assert!(super::copy_selection(0, || true).is_none());
        assert_eq!(unsafe { super::GetClipboardSequenceNumber() }, before);
    }

    #[test]
    fn invalid_source_is_rejected_before_input_is_sent() {
        assert!(super::window_profile(0, 0).is_err());
        assert!(super::copy_browser_url(0, 0).is_err());
        let action = crate::shortcuts::find("app:figma:frame-tool").unwrap();
        assert!(super::send_app_shortcut(0, 0, action).is_err());
    }
}
