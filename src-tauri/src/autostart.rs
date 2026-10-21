//! Start-with-Windows via the per-user Run key; no admin rights needed.
use std::{os::windows::ffi::OsStrExt, ptr};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE: &str = "PieAll";
const ERROR_SUCCESS: u32 = 0;
const ERROR_FILE_NOT_FOUND: u32 = 2;

fn wide(text: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    text.as_ref().encode_wide().chain(Some(0)).collect()
}

fn with_key<T>(access: u32, f: impl FnOnce(HKEY) -> T) -> Result<T, String> {
    let mut key = ptr::null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            0,
            ptr::null(),
            0,
            access,
            ptr::null(),
            &mut key,
            ptr::null_mut(),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!(
            "Windows startup setting unavailable (error {status})."
        ));
    }
    let out = f(key);
    unsafe { RegCloseKey(key) };
    Ok(out)
}

pub fn enabled() -> Result<bool, String> {
    with_key(KEY_QUERY_VALUE, |key| unsafe {
        RegQueryValueExW(
            key,
            wide(VALUE).as_ptr(),
            ptr::null(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        ) == ERROR_SUCCESS
    })
}

pub fn set_enabled(on: bool) -> Result<(), String> {
    let status = with_key(KEY_SET_VALUE, |key| -> Result<u32, String> {
        unsafe {
            if on {
                let exe = std::env::current_exe().map_err(|e| e.to_string())?;
                let mut command = wide(format!("\"{}\"", exe.display()));
                let bytes = (command.len() * 2) as u32;
                Ok(RegSetValueExW(
                    key,
                    wide(VALUE).as_ptr(),
                    0,
                    REG_SZ,
                    command.as_mut_ptr().cast(),
                    bytes,
                ))
            } else {
                Ok(match RegDeleteValueW(key, wide(VALUE).as_ptr()) {
                    ERROR_FILE_NOT_FOUND => ERROR_SUCCESS,
                    other => other,
                })
            }
        }
    })??;
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!(
            "Could not change the Windows startup setting (error {status})."
        ))
    }
}
