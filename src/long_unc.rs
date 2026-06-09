use crate::OsStrExt;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use windows::Win32::Foundation::MAX_PATH;

/// Represents a path prefixed by `\\?\`.
/// Also called [Win32 File Namespace],
/// or Extended-Length Path.
///
/// [Win32 File Namespace]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#win32-file-namespaces
pub(crate) struct LongUnc<'a> {
    /// The path with the `\\?\` prefix stripped.
    stripped: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for LongUnc<'a> {
    type Error = ();

    fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        if let Some(suffix) = bytes.strip_prefix(Self::PREFIX) {
            return Ok(Self { stripped: suffix });
        }
        Err(())
    }
}

impl<'a, const N: usize> TryFrom<&'a [u8; N]> for LongUnc<'a> {
    type Error = ();

    fn try_from(bytes: &'a [u8; N]) -> Result<Self, Self::Error> {
        Self::try_from(bytes.as_slice())
    }
}

impl<'a> TryFrom<&'a str> for LongUnc<'a> {
    type Error = ();

    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        Self::try_from(str.as_bytes())
    }
}

impl<'a> TryFrom<&'a Path> for LongUnc<'a> {
    type Error = ();

    fn try_from(path: &'a Path) -> Result<Self, Self::Error> {
        Self::try_from(path.as_os_str().as_encoded_bytes())
    }
}

impl<'a> LongUnc<'a> {
    const PREFIX: &'static [u8] = br"\\?\";
    const UNC_SUB_PREFIX: &'static [u8] = br"UNC\";
    const SHORT_UNC_PREFIX: &'static [u8] = br"\\";
    const SHORT_UNC_LEN_SUB: usize = Self::UNC_SUB_PREFIX.len() - Self::SHORT_UNC_PREFIX.len();

    fn as_stripped_osstr(&self) -> &OsStr {
        unsafe { OsStr::from_encoded_bytes_unchecked(self.stripped) }
    }

    fn is_sub_prefix_unc(&self) -> bool {
        self.stripped.len() >= Self::UNC_SUB_PREFIX.len()
            && self.stripped[..Self::UNC_SUB_PREFIX.len()]
                .eq_ignore_ascii_case(Self::UNC_SUB_PREFIX)
    }

    fn is_stripped_longer_than_wide(&self, max: u32) -> bool {
        self.as_stripped_osstr().is_longer_than_wide(max)
    }

    fn is_short_unc_longer_than_max_path(&self) -> bool {
        self.is_stripped_longer_than_wide(MAX_PATH + Self::SHORT_UNC_LEN_SUB as u32)
    }

    pub(crate) fn to_short_unc_opt(&self, disallow_long: bool) -> Option<PathBuf> {
        if !self.is_sub_prefix_unc() {
            return None;
        }
        if disallow_long && self.is_short_unc_longer_than_max_path() {
            return None;
        }
        let capacity = self.stripped.len() - Self::SHORT_UNC_LEN_SUB;
        let mut result_bytes = Vec::with_capacity(capacity);
        result_bytes.extend_from_slice(Self::SHORT_UNC_PREFIX);
        result_bytes.extend_from_slice(&self.stripped[Self::UNC_SUB_PREFIX.len()..]);
        assert_eq!(result_bytes.len(), capacity);
        let os_str = unsafe { OsStr::from_encoded_bytes_unchecked(&result_bytes) };
        Some(PathBuf::from(os_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::ffi::OsStrExt;

    #[test]
    fn try_from() {
        assert!(LongUnc::try_from(br"\\?\server\share\dir").is_ok());
        assert!(LongUnc::try_from(br"\\?\C:\").is_ok());
        assert!(LongUnc::try_from(br"\\?\").is_ok());

        assert!(LongUnc::try_from(br"\\?").is_err());
        assert!(LongUnc::try_from(br"\\server\share\dir").is_err());
        assert!(LongUnc::try_from(br"\a\b").is_err());
    }

    fn from_bytes(bytes: &[u8]) -> LongUnc<'_> {
        LongUnc::try_from(bytes).unwrap()
    }

    fn from_str(str: &str) -> LongUnc<'_> {
        LongUnc::try_from(str).unwrap()
    }

    #[test]
    fn is_sub_prefix_unc() {
        assert!(from_bytes(br"\\?\UNC\").is_sub_prefix_unc());
        assert!(from_bytes(br"\\?\UNC\server\share\dir").is_sub_prefix_unc());
        assert!(from_bytes(br"\\?\unc\server\share\dir").is_sub_prefix_unc());
        assert!(from_bytes(br"\\?\uNc\server\share\dir").is_sub_prefix_unc());
        assert!(from_bytes(br"\\?\UnC\server\share\dir").is_sub_prefix_unc());

        assert!(!from_bytes(br"\\?\UN").is_sub_prefix_unc());
        assert!(!from_bytes(br"\\?\UNC").is_sub_prefix_unc());
        assert!(!from_bytes(br"\\?\UNCD\").is_sub_prefix_unc());
        assert!(!from_bytes(br"\\?\server\share\dir").is_sub_prefix_unc());
    }

    #[test]
    fn to_short_unc_opt() {
        assert_eq!(from_bytes(br"\\?\C:\foo").to_short_unc_opt(false), None);
        assert_eq!(
            from_bytes(br"\\?\UNC\server\share\dir").to_short_unc_opt(false),
            Some(PathBuf::from(r"\\server\share\dir"))
        );
    }

    #[test]
    fn to_short_unc_opt_long() {
        const PREFIX: &str = r"\\?\UNC\";
        const SHORT_PREFIX: &str = r"\\";
        const SERVER_SHARE: &str = r"server\share\";
        const PATH_MAX: usize = MAX_PATH as usize - SHORT_PREFIX.len() - SERVER_SHARE.len();

        for (suffix, wide_len) in [
            ("1".repeat(PATH_MAX), MAX_PATH),
            ("1".repeat(PATH_MAX + 1), MAX_PATH + 1),
            ("\u{3042}".repeat(PATH_MAX), MAX_PATH),
            ("\u{3042}".repeat(PATH_MAX + 1), MAX_PATH + 1),
        ] {
            let suffix = SERVER_SHARE.to_string() + &suffix;
            let long = PREFIX.to_string() + &suffix;
            let short = PathBuf::from(&(SHORT_PREFIX.to_string() + &suffix));
            assert_eq!(short.as_os_str().encode_wide().count(), wide_len as usize);
            if wide_len <= MAX_PATH {
                assert_eq!(from_str(&long).to_short_unc_opt(false), Some(short.clone()));
                assert_eq!(from_str(&long).to_short_unc_opt(true), Some(short));
            } else {
                assert_eq!(from_str(&long).to_short_unc_opt(false), Some(short));
                assert_eq!(from_str(&long).to_short_unc_opt(true), None);
            }
        }
    }
}
