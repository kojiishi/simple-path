use crate::OsStrExt;
use std::path::{Component, Path, StripPrefixError};

pub(crate) trait PathExt {
    fn is_longer_than_wide(&self, max: u32) -> bool;
    fn to_wide_vec_with_nul(&self) -> Vec<u16>;

    fn strip_prefix_fix(&self, base: impl AsRef<Path>) -> Result<&Path, StripPrefixError>;
    fn trim_leading_separator(&self) -> &Path;
}

impl PathExt for Path {
    fn is_longer_than_wide(&self, max: u32) -> bool {
        self.as_os_str().is_longer_than_wide(max)
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
