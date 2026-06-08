use std::{ffi::OsStr, os::windows::ffi::OsStrExt as _};

pub(crate) trait OsStrExt {
    fn is_longer_than_wide(&self, max: u32) -> bool;
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
}
