use crate::OsStrExt;
use std::path::{
    Component, Path,
    Prefix::{UNC, Verbatim, VerbatimUNC},
    StripPrefixError,
};
use windows::Win32::Foundation::MAX_PATH;

pub(crate) trait PathExt {
    fn has_invalid_chars(&self) -> bool;
    fn has_reserved_names(&self) -> bool;

    fn is_longer_than_wide(&self, max: u32) -> bool;
    fn is_longer_than_win_max_path(&self) -> bool;
    fn to_wide_vec_with_nul(&self) -> Vec<u16>;

    fn strip_prefix_fix(&self, base: impl AsRef<Path>) -> Result<&Path, StripPrefixError>;
    fn trim_leading_separator(&self) -> &Path;
    fn trim_trailing_separator(&self) -> &Path;
}

impl PathExt for Path {
    fn has_invalid_chars(&self) -> bool {
        let mut components = self.components();
        match components.next() {
            None => false,
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Verbatim(str) => {
                    str.has_invalid_path_chars()
                        || components.as_path().as_os_str().has_invalid_path_chars()
                }
                UNC(server, share) | VerbatimUNC(server, share) => {
                    server.has_invalid_path_chars()
                        || share.has_invalid_path_chars()
                        || components.as_path().as_os_str().has_invalid_path_chars()
                }
                _ => components.as_path().as_os_str().has_invalid_path_chars(),
            },
            _ => self.as_os_str().has_invalid_path_chars(),
        }
    }

    fn has_reserved_names(&self) -> bool {
        self.components().any(|component| {
            if let Component::Normal(os_str) = component {
                os_str.is_reserved_name()
            } else {
                false
            }
        })
    }

    fn is_longer_than_wide(&self, max: u32) -> bool {
        self.as_os_str().is_longer_than_wide(max)
    }

    fn is_longer_than_win_max_path(&self) -> bool {
        self.is_longer_than_wide(MAX_PATH)
    }

    fn to_wide_vec_with_nul(&self) -> Vec<u16> {
        self.as_os_str().to_wide_vec_with_nul()
    }

    fn strip_prefix_fix(&self, base: impl AsRef<Path>) -> Result<&Path, StripPrefixError> {
        let result = self.strip_prefix(base);
        if let Ok(result) = result {
            // `strip_prefix` may leave the leading `\` for UNC paths.
            // https://github.com/rust-lang/rust/issues/155183
            return Ok(result.trim_leading_separator());
        }
        result
    }

    fn trim_leading_separator(&self) -> &Path {
        let mut components = self.components();
        if let Some(first) = components.next()
            && first == Component::RootDir
        {
            return components.as_path();
        }
        self
    }

    fn trim_trailing_separator(&self) -> &Path {
        let mut components = self.components();
        if let Some(last) = components.next_back()
            && last == Component::RootDir
        {
            return components.as_path();
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_invalid_chars() {
        assert!(Path::new(":").has_invalid_chars());
        assert!(Path::new(r"a\:").has_invalid_chars());
        assert!(Path::new(r"a\>").has_invalid_chars());
        assert!(Path::new(r"a\dir:dir").has_invalid_chars());
        assert!(Path::new(r"dir:\a").has_invalid_chars());
        assert!(Path::new(r"C:\dir:").has_invalid_chars());
        assert!(Path::new(r"\\server:\share\dir").has_invalid_chars());
        assert!(Path::new(r"\\server\share:\dir").has_invalid_chars());
        assert!(Path::new(r"\\server\share\dir:").has_invalid_chars());
        assert!(Path::new(r"\\?\UNC\server:\share\dir").has_invalid_chars());
        assert!(Path::new(r"\\?\UNC\server\share:\dir").has_invalid_chars());
        assert!(Path::new(r"\\?\UNC\server\share\dir:").has_invalid_chars());
        assert!(Path::new(r"\\?\UNC\foo:").has_invalid_chars());
        assert!(Path::new(r"\\?\UNC\foo>").has_invalid_chars());
        assert!(Path::new(r"\\?\C:\foo:").has_invalid_chars());
        assert!(Path::new(r"\\?\:\foo").has_invalid_chars());
        assert!(Path::new(r"\\?\>\foo").has_invalid_chars());

        assert!(!Path::new("").has_invalid_chars());
        assert!(!Path::new("a").has_invalid_chars());
        assert!(!Path::new(r"a\b").has_invalid_chars());
        assert!(!Path::new(r"\a\b").has_invalid_chars());
        assert!(!Path::new(r"C:\").has_invalid_chars());
        assert!(!Path::new(r"C:\dir").has_invalid_chars());
        assert!(!Path::new(r"\\server").has_invalid_chars());
        assert!(!Path::new(r"\\server\share").has_invalid_chars());
        assert!(!Path::new(r"\\server\share\dir").has_invalid_chars());
        assert!(!Path::new(r"\\?\").has_invalid_chars());
        assert!(!Path::new(r"\\?\foo").has_invalid_chars());
        assert!(!Path::new(r"\\?\foo\bar").has_invalid_chars());
        assert!(!Path::new(r"\\?\UNC\server\share\dir").has_invalid_chars());
        assert!(!Path::new(r"\\?\UNC\foo").has_invalid_chars());
        assert!(!Path::new(r"\\?\C:\").has_invalid_chars());
        assert!(!Path::new(r"\\?\C:\dir").has_invalid_chars());
        assert!(!Path::new(r"\\.\COM42").has_invalid_chars());
    }

    #[test]
    fn has_reserved_names() {
        assert!(Path::new("CON").has_reserved_names());
        assert!(Path::new("con").has_reserved_names());
        assert!(Path::new("PRN.txt").has_reserved_names());
        assert!(Path::new("aux.tar.gz").has_reserved_names());
        assert!(Path::new("NUL").has_reserved_names());
        assert!(Path::new("COM1").has_reserved_names());
        assert!(Path::new("com9").has_reserved_names());
        assert!(Path::new("LPT1").has_reserved_names());
        assert!(Path::new("lpt9").has_reserved_names());
        assert!(Path::new("COM¹").has_reserved_names());
        assert!(Path::new("lpt³").has_reserved_names());
        assert!(Path::new("con ").has_reserved_names());
        assert!(Path::new("con.").has_reserved_names());
        assert!(Path::new("con.txt. . ").has_reserved_names());
        assert!(Path::new(r"C:\foo\CON.txt").has_reserved_names());
        assert!(Path::new(r"\\server\share\COM1").has_reserved_names());

        assert!(!Path::new("").has_reserved_names());
        assert!(!Path::new("acon").has_reserved_names());
        assert!(!Path::new("conb").has_reserved_names());
        assert!(!Path::new("COM0").has_reserved_names());
        assert!(!Path::new("LPT0").has_reserved_names());
        assert!(!Path::new(".con").has_reserved_names());
        assert!(!Path::new("con1").has_reserved_names());
    }

    #[test]
    fn is_longer_than_wide() {
        assert!(!Path::new("").is_longer_than_wide(10));

        assert!(!Path::new(&"1".repeat(9)).is_longer_than_wide(10));
        assert!(!Path::new(&"1".repeat(10)).is_longer_than_wide(10));
        assert!(Path::new(&"1".repeat(11)).is_longer_than_wide(10));

        assert!(!Path::new(&"\u{3042}".repeat(9)).is_longer_than_wide(10));
        assert!(!Path::new(&"\u{3042}".repeat(10)).is_longer_than_wide(10));
        assert!(Path::new(&"\u{3042}".repeat(11)).is_longer_than_wide(10));
    }

    #[test]
    fn strip_prefix_fix() {
        let path = Path::new(r"\\?\UNC\server\share\dir");
        let base = Path::new(r"\\?\UNC\server\share");
        // `strip_prefix` may leave the leading `\` for UNC paths.
        // https://github.com/rust-lang/rust/issues/155183
        assert_eq!(path.strip_prefix(base), Ok(Path::new(r"\dir")));
        assert_eq!(path.strip_prefix_fix(base), Ok(Path::new(r"dir")));
    }

    #[test]
    fn trim_leading_separator() {
        assert_eq!(Path::new("/").trim_leading_separator(), Path::new(""));
        assert_eq!(Path::new("").trim_leading_separator(), Path::new(""));
        assert_eq!(Path::new("/a").trim_leading_separator(), Path::new("a"));
        assert_eq!(Path::new("a").trim_leading_separator(), Path::new("a"));
        assert_eq!(Path::new("/a/b").trim_leading_separator(), Path::new("a/b"));
        assert_eq!(Path::new("a/b").trim_leading_separator(), Path::new("a/b"));
    }

    #[test]
    fn trim_trailing_separator() {
        assert_eq!(Path::new("/").trim_trailing_separator(), Path::new(""));
        assert_eq!(Path::new("").trim_trailing_separator(), Path::new(""));
        assert_eq!(Path::new("a/").trim_trailing_separator(), Path::new("a"));
        assert_eq!(Path::new("a").trim_trailing_separator(), Path::new("a"));
        assert_eq!(Path::new("a//").trim_trailing_separator(), Path::new("a"));
        assert_eq!(
            Path::new("a/b/").trim_trailing_separator(),
            Path::new("a/b")
        );
        assert_eq!(Path::new("a/b").trim_trailing_separator(), Path::new("a/b"));
    }
}
