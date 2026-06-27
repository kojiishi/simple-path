use std::{
    ffi::{OsStr, OsString},
    os::windows::ffi::{OsStrExt as _, OsStringExt},
};
use windows::core::PWSTR;

pub(crate) trait OsStrExt {
    fn has_win_invalid_chars(&self) -> bool;
    fn is_longer_than_wide(&self, max: u32) -> bool;
    fn to_wide_vec_with_nul(&self) -> Vec<u16>;
}

impl OsStrExt for OsStr {
    fn has_win_invalid_chars(&self) -> bool {
        self.as_encoded_bytes()
            .iter()
            .any(|&ch| is_win_invalid_path_char(ch))
    }

    fn is_longer_than_wide(&self, mut max: u32) -> bool {
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
fn is_win_invalid_path_char(ch: u8) -> bool {
    ch == b'<' || ch == b'>' || ch == b':' || ch == b'"' || ch == b'|' || ch == b'?' || ch == b'*'
}

pub(crate) trait PWSTRExt {
    fn to_os_string(&self) -> OsString;
}

impl PWSTRExt for PWSTR {
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
