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
        if OpenClipboard(ptr::null_mut()) == 0 {
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
        if OpenClipboard(ptr::null_mut()) == 0 {
            windows_sys::Win32::Foundation::GlobalFree(h);
            return Err("Clipboard is busy.".into());
        }
        EmptyClipboard();
        let result = SetClipboardData(13, h);
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
    unsafe {
        let mut buffer = vec![0u16; 65536];
        let mut o: OPENFILENAMEW = std::mem::zeroed();
        o.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        o.lpstrFile = buffer.as_mut_ptr();
        o.nMaxFile = buffer.len() as u32;
        let title = wide("Choose files for PieAll");
        o.lpstrTitle = title.as_ptr();
        o.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_ALLOWMULTISELECT | OFN_NOCHANGEDIR;
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
pub fn capture_selection_direct(source: usize) -> Result<Vec<PathBuf>, String> {
    use windows::{
        core::Interface,
        Win32::{System::Com::*, UI::Shell::*},
    };
    unsafe {
        let foreground = source as isize;
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let result = (|| -> windows::core::Result<Vec<PathBuf>> {
            let shell: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)?;
            let mut matches = Vec::new();
            for i in 0..shell.Count()? {
                let variant = windows::Win32::System::Variant::VARIANT::from(i);
                let dispatch = match shell.Item(&variant) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let browser = match dispatch.cast::<IWebBrowserApp>() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if browser.HWND()?.0 as isize != foreground {
                    continue;
                }
                matches.push(browser);
            }
            if matches.len() != 1 {
                return Ok(vec![]);
            }
            let provider: IServiceProvider = matches[0].cast()?;
            let browser: IShellBrowser = provider.QueryService(&SID_STopLevelBrowser)?;
            let view = browser.QueryActiveShellView()?;
            let view_hwnd = view.GetWindow()?;
            if IsWindowVisible(view_hwnd.0) == 0 {
                return Ok(vec![]);
            }
            let folder: IFolderView2 = view.cast()?;
            let items = folder.GetSelection(false)?;
            if items.GetCount()? > 256 {
                return Ok(vec![]);
            }
            let mut paths = Vec::new();
            for i in 0..items.GetCount()? {
                let item = items.GetItemAt(i)?;
                let raw = item.GetDisplayName(SIGDN_FILESYSPATH)?;
                let text = raw.to_string();
                CoTaskMemFree(Some(raw.0 as *const _));
                let p = PathBuf::from(text?);
                if p.exists() {
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
            return Ok("Always-on-top toggled.".into());
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
        Ok("Window updated.".into())
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
