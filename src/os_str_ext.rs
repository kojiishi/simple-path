use std::{ffi::OsStr, os::windows::ffi::OsStrExt as _};

pub(crate) trait OsStrExt {
    fn is_longer_than_wide(&self, max: u32) -> bool;
    fn to_wide_vec_with_nul(&self) -> Vec<u16>;
}

impl OsStrExt for OsStr {
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
