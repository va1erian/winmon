//! `HKEY_CURRENT_USER` values, for the per-user install (no elevation).

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_DWORD, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegDeleteTreeW,
    RegGetValueW, RegSetKeyValueW,
};
use windows::core::HSTRING;

fn check(e: WIN32_ERROR) -> std::io::Result<()> {
    if e == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(e.0 as i32))
    }
}

/// Sets `key\name` to the string `value`, creating `key` if needed.
pub fn set_string(key: &str, name: &str, value: &str) -> std::io::Result<()> {
    let data: Vec<u16> = value.encode_utf16().chain([0]).collect();
    // SAFETY: `data` is a NUL-terminated UTF-16 buffer of the stated size.
    check(unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(key),
            &HSTRING::from(name),
            REG_SZ.0,
            Some(data.as_ptr().cast()),
            (data.len() * 2) as u32,
        )
    })
}

/// Sets `key\name` to the DWORD `value`, creating `key` if needed.
pub fn set_dword(key: &str, name: &str, value: u32) -> std::io::Result<()> {
    // SAFETY: `value` is a live u32 of the stated size.
    check(unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(key),
            &HSTRING::from(name),
            REG_DWORD.0,
            Some((&raw const value).cast()),
            4,
        )
    })
}

/// The string at `key\name`, or `None` if it's missing or not a string.
pub fn get_string(key: &str, name: &str) -> Option<String> {
    let (key, name) = (HSTRING::from(key), HSTRING::from(name));
    let mut bytes = 0u32;
    // SAFETY: a size query (no buffer), then a read into a buffer of that size.
    unsafe {
        check(RegGetValueW(
            HKEY_CURRENT_USER,
            &key,
            &name,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut bytes),
        ))
        .ok()?;
        let mut buf = vec![0u16; (bytes as usize).div_ceil(2)];
        check(RegGetValueW(
            HKEY_CURRENT_USER,
            &key,
            &name,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut bytes),
        ))
        .ok()?;
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    }
}

/// Deletes `key\name`. A missing value is not an error.
pub fn delete_value(key: &str, name: &str) -> std::io::Result<()> {
    // SAFETY: plain call with two NUL-terminated strings.
    let e =
        unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(key), &HSTRING::from(name)) };
    if e == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    check(e)
}

/// Deletes `key` and everything under it. A missing key is not an error.
pub fn delete_key(key: &str) -> std::io::Result<()> {
    // SAFETY: plain call with a NUL-terminated string.
    let e = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(key)) };
    if e == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    check(e)
}
