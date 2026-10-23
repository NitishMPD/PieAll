//! Windows owns app artwork; unresolved profiles retain their frontend monograms.
use base64::Engine;
use std::{
    collections::HashMap,
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, OnceLock},
};
use windows::{
    core::{Interface, PCWSTR},
    Win32::{Foundation::SIZE, System::Com::*, UI::Shell::*},
};
use windows_sys::Win32::{
    Foundation::CloseHandle,
    Graphics::Gdi::*,
    System::{Registry::*, Threading::*},
    UI::WindowsAndMessaging::*,
};

fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}
fn local_exe(path: &Path) -> bool {
    matches!(path.components().next(), Some(std::path::Component::Prefix(p))
        if matches!(p.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)))
        && path.is_absolute()
        && crate::output::plain_path(path).is_ok()
        && path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("exe"))
        && path.is_file()
}
fn remember(paths: &mut HashMap<String, PathBuf>, path: PathBuf) {
    if local_exe(&path) {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            paths.entry(name.to_ascii_lowercase()).or_insert(path);
        }
    }
}
unsafe extern "system" fn running(hwnd: windows_sys::Win32::Foundation::HWND, data: isize) -> i32 {
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if !process.is_null() {
        let mut text = vec![0u16; 32768];
        let mut size = text.len() as u32;
        if QueryFullProcessImageNameW(process, 0, text.as_mut_ptr(), &mut size) != 0 {
            remember(
                &mut *(data as *mut HashMap<String, PathBuf>),
                PathBuf::from(String::from_utf16_lossy(&text[..size as usize])),
            );
        }
        CloseHandle(process);
    }
    1
}
unsafe fn app_path(exe: &str) -> Option<PathBuf> {
    let key = wide(format!(
        "Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\{exe}"
    ));
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for view in [RRF_SUBKEY_WOW6464KEY, RRF_SUBKEY_WOW6432KEY] {
            let mut text = vec![0u16; 32768];
            let mut bytes = (text.len() * 2) as u32;
            if RegGetValueW(
                root,
                key.as_ptr(),
                ptr::null(),
                RRF_RT_REG_SZ | view,
                ptr::null_mut(),
                text.as_mut_ptr().cast(),
                &mut bytes,
            ) == 0
            {
                let n = text.iter().position(|&c| c == 0).unwrap_or(text.len());
                let path = PathBuf::from(String::from_utf16_lossy(&text[..n]).trim_matches('"'));
                if local_exe(&path) {
                    return Some(path);
                }
            }
        }
    }
    None
}
unsafe fn shortcuts(
    dir: &Path,
    depth: usize,
    budget: &mut usize,
    paths: &mut HashMap<String, PathBuf>,
) {
    if depth > 8 || *budget == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        use std::os::windows::fs::MetadataExt;
        if entry
            .metadata()
            .is_ok_and(|m| m.file_attributes() & 0x400 != 0)
        {
            continue;
        }
        if kind.is_dir() {
            shortcuts(&path, depth + 1, budget, paths);
        } else if path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("lnk"))
        {
            let Ok(link) =
                CoCreateInstance::<_, IShellLinkW>(&ShellLink, None, CLSCTX_INPROC_SERVER)
            else {
                continue;
            };
            let Ok(file) = link.cast::<IPersistFile>() else {
                continue;
            };
            if file.Load(PCWSTR(wide(&path).as_ptr()), STGM_READ).is_err() {
                continue;
            }
            let mut target = vec![0u16; 32768];
            if link.GetPath(&mut target, ptr::null_mut(), 0).is_ok() {
                let n = target.iter().position(|&c| c == 0).unwrap_or(target.len());
                remember(paths, PathBuf::from(String::from_utf16_lossy(&target[..n])));
            }
        }
    }
}
unsafe fn icon(path: &Path) -> Option<String> {
    let factory: IShellItemImageFactory =
        SHCreateItemFromParsingName(PCWSTR(wide(path).as_ptr()), None).ok()?;
    let bitmap = factory
        .GetImage(SIZE { cx: 48, cy: 48 }, SIIGBF_ICONONLY)
        .ok()?;
    let dc = CreateCompatibleDC(ptr::null_mut());
    let result = (|| {
        if dc.is_null() {
            return None;
        }
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        if GetDIBits(
            dc,
            bitmap.0,
            0,
            0,
            ptr::null_mut(),
            &mut info,
            DIB_RGB_COLORS,
        ) == 0
        {
            return None;
        }
        let width = info.bmiHeader.biWidth;
        let height = info.bmiHeader.biHeight.abs();
        if !(1..=256).contains(&width) || !(1..=256).contains(&height) {
            return None;
        }
        info.bmiHeader.biHeight = -height;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        if GetDIBits(
            dc,
            bitmap.0,
            0,
            height as u32,
            pixels.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        ) != height
        {
            return None;
        }
        for p in pixels.chunks_exact_mut(4) {
            p.swap(0, 2);
            // Shell bitmaps use premultiplied alpha; PNG expects straight alpha.
            if p[3] > 0 {
                for i in 0..3 {
                    p[i] = ((p[i] as u32 * 255) / p[3] as u32).min(255) as u8;
                }
            }
        }
        let image = image::RgbaImage::from_raw(width as u32, height as u32, pixels)?;
        let mut png = std::io::Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png).ok()?;
        Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png.into_inner())
        ))
    })();
    if !dc.is_null() {
        DeleteDC(dc);
    }
    DeleteObject(bitmap.0);
    result
}

pub fn load() -> HashMap<String, String> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, String>>> = OnceLock::new();
    // shortcut: icons stay cached until exit; refresh metadata if live icon updates are needed.
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    let mut result = HashMap::new();
    unsafe {
        if CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_err() {
            return result;
        }
        {
            let mut paths = HashMap::new();
            EnumWindows(Some(running), &mut paths as *mut _ as isize);
            let mut budget = 4096;
            for (env, suffix) in [
                ("APPDATA", "Microsoft/Windows/Start Menu/Programs"),
                ("ProgramData", "Microsoft/Windows/Start Menu/Programs"),
            ] {
                if let Some(root) = std::env::var_os(env) {
                    shortcuts(
                        &PathBuf::from(root).join(suffix),
                        0,
                        &mut budget,
                        &mut paths,
                    );
                }
            }
            let custom = crate::shortcuts::custom::apps();
            for app in &custom {
                if let Some(path) = &app.executable_path {
                    let path = PathBuf::from(path);
                    if local_exe(&path) {
                        paths.insert(app.executable.clone(), path);
                    }
                }
            }
            for profile in crate::shortcuts::catalog()
                .profiles
                .iter()
                .cloned()
                .chain(crate::shortcuts::custom::profiles())
            {
                for exe in &profile.executables {
                    let path = paths.get(exe).cloned().or_else(|| app_path(exe));
                    let Some(path) = path else {
                        continue;
                    };
                    let value = cache.get(&path).cloned().or_else(|| icon(&path));
                    if let Some(value) = value {
                        cache.insert(path, value.clone());
                        result.insert(profile.id.clone(), value);
                        break;
                    }
                }
            }
        }
        CoUninitialize();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_executable_produces_png_and_missing_apps_are_ignored() {
        let exe = std::env::current_exe().unwrap();
        assert!(local_exe(&exe));
        assert!(!local_exe(Path::new("missing.exe")));
        unsafe {
            assert!(CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok());
            let value = icon(&exe).expect("Windows executable icon");
            let png = base64::engine::general_purpose::STANDARD
                .decode(value.strip_prefix("data:image/png;base64,").unwrap())
                .unwrap();
            let decoded = image::load_from_memory(&png).unwrap().into_rgba8();
            assert!(decoded.pixels().any(|p| p[3] > 0));
            let dir = tempfile::tempdir().unwrap();
            let link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            link.SetPath(PCWSTR(wide(&exe).as_ptr())).unwrap();
            link.cast::<IPersistFile>()
                .unwrap()
                .Save(PCWSTR(wide(dir.path().join("app.lnk")).as_ptr()), true)
                .unwrap();
            let mut paths = HashMap::new();
            shortcuts(dir.path(), 0, &mut 10, &mut paths);
            assert_eq!(
                paths.get(
                    &exe.file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_ascii_lowercase()
                ),
                Some(&exe)
            );
            drop(link);
            CoUninitialize();
        }
    }
}
