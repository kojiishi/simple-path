use std::{
    ffi::{OsStr, OsString},
    os::windows::ffi::{OsStrExt as _, OsStringExt},
};
use windows::core::PWSTR;

pub(crate) trait OsStrExt {
    fn has_invalid_path_chars(&self) -> bool;
    fn is_reserved_name(&self) -> bool;
    fn is_longer_than_wide(&self, max: u32) -> bool;
    fn to_wide_vec_with_nul(&self) -> Vec<u16>;
}

impl OsStrExt for OsStr {
    fn has_invalid_path_chars(&self) -> bool {
        self.as_encoded_bytes()
            .iter()
            .any(|&ch| is_invalid_path_char(ch))
    }

    fn is_reserved_name(&self) -> bool {
        let bytes = self.as_encoded_bytes();
        // Windows reserved names are matched against the file/directory stem
        // (before the first period).
        let stem = match bytes.iter().position(|&b| b == b'.') {
            Some(idx) => &bytes[..idx],
            None => bytes,
        };
        // Trailing spaces are trimmed because Windows normalizes names by
        // stripping them (e.g. "CON " -> "CON").
        is_reserved_name(stem.trim_ascii_end())
    }

    fn is_longer_than_wide(&self, mut max: u32) -> bool {
        debug_assert!(self.len() >= self.encode_wide().count());
        if self.len() <= max as usize {
            return false;
        }
        for _ in self.encode_wide() {
            if max == 0 {
                return true;
            }
            max -= 1;
        }
        false
    }

    fn to_wide_vec_with_nul(&self) -> Vec<u16> {
        self.encode_wide().chain(Some(0)).collect()
    }
}

/// Valid characters as defined by [Naming Conventions].
///
/// Byte-comparison is safe because all invalid characters are in ASCII.
/// '/' and '\\' are excluded, as this function is for a path, not a file name.
///
/// [Naming Conventions]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#naming-conventions
fn is_invalid_path_char(ch: u8) -> bool {
    ch == b'<' || ch == b'>' || ch == b':' || ch == b'"' || ch == b'|' || ch == b'?' || ch == b'*'
}

fn is_reserved_name(bytes: &[u8]) -> bool {
    match bytes.len() {
        3 => {
            bytes.eq_ignore_ascii_case(b"CON")
                || bytes.eq_ignore_ascii_case(b"PRN")
                || bytes.eq_ignore_ascii_case(b"AUX")
                || bytes.eq_ignore_ascii_case(b"NUL")
        }
        4 => {
            // COM1..COM9, LPT1..LPT9
            let is_com_or_lpt =
                bytes[..3].eq_ignore_ascii_case(b"COM") || bytes[..3].eq_ignore_ascii_case(b"LPT");
            is_com_or_lpt && (b'1'..=b'9').contains(&bytes[3])
        }
        5 => {
            // COM¹, COM², COM³, LPT¹, LPT², LPT³
            // ¹ is C2 B9, ² is C2 B2, ³ is C2 B3
            let is_com_or_lpt =
                bytes[..3].eq_ignore_ascii_case(b"COM") || bytes[..3].eq_ignore_ascii_case(b"LPT");
            if !is_com_or_lpt {
                return false;
            }
            bytes[3] == 0xC2 && (bytes[4] == 0xB9 || bytes[4] == 0xB2 || bytes[4] == 0xB3)
        }
        _ => false,
    }
}

pub(crate) trait WinStrExt {
    fn to_os_string(&self) -> OsString;
}

impl WinStrExt for PWSTR {
    fn to_os_string(&self) -> OsString {
        if self.is_null() {
            return OsString::new();
        }
        unsafe {
            let slice = self.as_wide();
            OsString::from_wide(slice)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_longer_than_wide() {
        let is_longer_than_wide = |str, max| OsStr::new(str).is_longer_than_wide(max);
        assert!(!is_longer_than_wide("", 0));
        assert!(is_longer_than_wide("1", 0));

        assert!(!is_longer_than_wide("", 1));
        assert!(!is_longer_than_wide("1", 1));
        assert!(is_longer_than_wide("12", 1));

        let wide9 = "\u{3042}".repeat(9);
        assert!(!is_longer_than_wide(&wide9, 10));
        let wide10 = "\u{3042}".repeat(10);
        assert!(!is_longer_than_wide(&wide10, 10));
        let wide11 = "\u{3042}".repeat(11);
        assert!(is_longer_than_wide(&wide11, 10));
    }

    #[test]
    fn to_wide_vec_with_nul() {
        assert_eq!(OsStr::new("AB").to_wide_vec_with_nul(), vec![0x41, 0x42, 0]);
        assert_eq!(
            OsStr::new("\u{3042}\u{3043}").to_wide_vec_with_nul(),
            vec![0x3042, 0x3043, 0]
        );
    }
}
