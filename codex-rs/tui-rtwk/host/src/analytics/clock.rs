//! The user's clock preference, detected once from the machine: the page's
//! times print on a 24-hour clock or a 12-hour one. Unknown preferences keep
//! the 12-hour output; detection never changes the process locale. Oracle:
//! `clock_format.rs`.

use std::sync::OnceLock;

/// Whether the times are shown on a 24-hour clock.
pub fn hour24() -> bool {
    static FORMAT: OnceLock<bool> = OnceLock::new();
    *FORMAT.get_or_init(|| detect().unwrap_or(false))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn detect() -> Option<bool> {
    let locale = ["LC_ALL", "LC_TIME", "LANG"]
        .into_iter()
        .find_map(|name| std::env::var_os(name).filter(|value| !value.is_empty()))?;
    let locale = locale.to_str()?;
    if matches!(locale, "C" | "POSIX" | "C.UTF-8" | "C.utf8") {
        return Some(true);
    }
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        let name = std::ffi::CString::new(locale).ok()?;
        // SAFETY: name is NUL-terminated, the mask is valid, and a null base
        // creates an independent locale. Its borrowed pattern is read before
        // freeing it.
        unsafe {
            let locale = libc::newlocale(libc::LC_TIME_MASK, name.as_ptr(), std::ptr::null_mut());
            if locale.is_null() {
                return None;
            }
            let pattern = libc::nl_langinfo_l(libc::T_FMT, locale);
            let result = if pattern.is_null() {
                None
            } else {
                parse_strftime(std::ffi::CStr::from_ptr(pattern).to_bytes())
            };
            libc::freelocale(locale);
            result
        }
    }
    #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
    None
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn parse_strftime(pattern: &[u8]) -> Option<bool> {
    let mut chars = pattern.iter().copied().peekable();
    let mut twelve = false;
    let mut twenty_four = false;
    while let Some(ch) = chars.next() {
        if ch != b'%' {
            continue;
        }
        while chars
            .next_if(|ch| matches!(ch, b'_' | b'-' | b'0' | b'^' | b'#') || ch.is_ascii_digit())
            .is_some()
        {}
        chars.next_if(|ch| matches!(ch, b'E' | b'O'));
        match chars.next()? {
            b'I' | b'l' | b'r' => twelve = true,
            b'H' | b'k' | b'R' | b'T' => twenty_four = true,
            b'%' | b'M' | b'S' | b'p' | b'P' | b'Z' | b'z' | b'n' | b't' => {}
            _ => return None,
        }
    }
    match (twelve, twenty_four) {
        (true, false) => Some(false),
        (false, true) => Some(true),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn detect() -> Option<bool> {
    use objc2_foundation::{NSDateFormatter, NSLocale, NSString};

    let locale = NSLocale::currentLocale();
    let pattern = NSDateFormatter::dateFormatFromTemplate_options_locale(
        &NSString::from_str("j"),
        /*opts*/ 0,
        Some(&locale),
    )?;
    parse_native_pattern(&pattern.to_string(), "hHKk")
}

#[cfg(windows)]
fn detect() -> Option<bool> {
    use windows_sys::Win32::Globalization::{GetLocaleInfoEx, LOCALE_SSHORTTIME};

    // SAFETY: the buffer is sized by the first call and the second writes at
    // most that many UTF-16 units.
    let pattern = unsafe {
        let length = GetLocaleInfoEx(
            std::ptr::null(),
            LOCALE_SSHORTTIME,
            std::ptr::null_mut(),
            /*cchdata*/ 0,
        );
        if length <= 1 {
            return None;
        }
        let mut buffer = vec![0; length as usize];
        let written = GetLocaleInfoEx(
            std::ptr::null(),
            LOCALE_SSHORTTIME,
            buffer.as_mut_ptr(),
            length,
        );
        if written <= 1 {
            return None;
        }
        String::from_utf16(&buffer[..written as usize - 1]).ok()?
    };
    parse_native_pattern(&pattern, "hH")
}

#[cfg(any(target_os = "macos", windows))]
fn parse_native_pattern(pattern: &str, hour_symbols: &str) -> Option<bool> {
    let mut quoted = false;
    let mut twelve = false;
    let mut twenty_four = false;
    for ch in pattern.chars() {
        if ch == '\'' {
            quoted = !quoted;
        } else if !quoted {
            if ch == ';' {
                break; // Windows lists the preferred pattern first.
            }
            if hour_symbols.contains(ch) {
                match ch {
                    'h' | 'K' => twelve = true,
                    'H' | 'k' => twenty_four = true,
                    _ => return None,
                }
            }
        }
    }
    match (quoted, twelve, twenty_four) {
        (false, true, false) => Some(false),
        (false, false, true) => Some(true),
        _ => None,
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    windows
)))]
fn detect() -> Option<bool> {
    None
}
