use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

/// Represents a Windows UNC path,
/// which is prefixed by `\\`.
pub(crate) struct UncPath<'a> {
    path: &'a Path,
}

impl<'a> TryFrom<&'a [u8]> for UncPath<'a> {
    type Error = ();

    fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        if UncPath::is_unc_bytes(bytes) {
            let os_str = unsafe { OsStr::from_encoded_bytes_unchecked(bytes) };
            let path = Path::new(os_str);
            return Ok(Self { path });
        }
        Err(())
    }
}

impl<'a> TryFrom<&'a Path> for UncPath<'a> {
    type Error = ();

    fn try_from(path: &'a Path) -> Result<Self, Self::Error> {
        if UncPath::is_unc(path) {
            return Ok(Self { path });
        }
        Err(())
    }
}

#[cfg(test)]
impl<'a> TryFrom<&'a str> for UncPath<'a> {
    type Error = ();

    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        Self::try_from(Path::new(str))
    }
}

impl<'a> UncPath<'a> {
    const UNC_PREFIX: &'static [u8] = br"\\";
    const FILE_NAMESPACE_CHAR: u8 = b'?';
    const FILE_NAMESPACE_PREFIX: &'static [u8] = br"?\";
    const UNC_KEYWORD: &'static [u8] = br"UNC";

    #[inline]
    pub(crate) fn is_unc(path: impl AsRef<Path>) -> bool {
        Self::is_unc_bytes(path.as_ref().as_os_str().as_encoded_bytes())
    }

    #[inline]
    fn is_unc_bytes(bytes: &[u8]) -> bool {
        bytes.len() >= 2
            && std::path::is_separator(bytes[0] as char)
            && std::path::is_separator(bytes[1] as char)
    }

    #[inline]
    pub(crate) fn as_path(&self) -> &'a Path {
        self.path
    }

    fn as_encoded_bytes(&self) -> &'a [u8] {
        self.as_path().as_os_str().as_encoded_bytes()
    }

    fn as_stripped_encoded_bytes(&self) -> &'a [u8] {
        &self.as_encoded_bytes()[Self::UNC_PREFIX.len()..]
    }

    fn as_stripped_os_str(&self) -> &'a OsStr {
        let bytes = self.as_stripped_encoded_bytes();
        unsafe { OsStr::from_encoded_bytes_unchecked(bytes) }
    }

    /// True if it starts with `\\?\`.
    /// which is a [Win32 File Namespace] path.
    /// Also called Extended-Length Path.
    ///
    /// [Win32 File Namespace]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#win32-file-namespaces
    fn is_file_namespace(&self) -> bool {
        let bytes = self.as_stripped_encoded_bytes();
        bytes.len() >= 2
            && bytes[0] == Self::FILE_NAMESPACE_CHAR
            && std::path::is_separator(bytes[1] as char)
    }

    // Bytes with the `\\?\` prefix stripped.
    fn as_file_namespace_stripped_encoded_bytes(&self) -> Option<&'a [u8]> {
        if self.is_file_namespace() {
            Some(&self.as_stripped_encoded_bytes()[Self::FILE_NAMESPACE_PREFIX.len()..])
        } else {
            None
        }
    }

    /// True if it starts with `\\?\UNC\`.
    pub(crate) fn is_file_namespace_unc(&self) -> bool {
        if let Some(stripped) = self.as_file_namespace_stripped_encoded_bytes() {
            return Self::starts_with_unc_keyword_and_separator(stripped);
        }
        false
    }

    fn starts_with_unc_keyword_and_separator(str: &[u8]) -> bool {
        str.len() > Self::UNC_KEYWORD.len()
            && str[..Self::UNC_KEYWORD.len()].eq_ignore_ascii_case(Self::UNC_KEYWORD)
            && std::path::is_separator(str[Self::UNC_KEYWORD.len()] as char)
    }

    fn starts_with_drive(str: &[u8]) -> bool {
        str.len() >= 2 && str[1] == b':' && str[0].is_ascii_alphabetic()
    }

    /// Convert `\\?\UNC\` to `\\`, and `\\?\C:` to `C:`.
    pub(crate) fn to_short_unc(&self) -> Option<Cow<'a, Path>> {
        let file_ns_stripped = self.as_file_namespace_stripped_encoded_bytes()?;
        if Self::starts_with_unc_keyword_and_separator(file_ns_stripped) {
            let unc_stripped = &file_ns_stripped[Self::UNC_KEYWORD.len() + 1..];
            let capacity = unc_stripped.len() + Self::UNC_PREFIX.len();
            let mut result_bytes = Vec::with_capacity(capacity);
            result_bytes.extend_from_slice(Self::UNC_PREFIX);
            result_bytes.extend_from_slice(unc_stripped);
            debug_assert_eq!(result_bytes.len(), capacity);
            let os_str = unsafe { OsStr::from_encoded_bytes_unchecked(&result_bytes) };
            return Some(Cow::Owned(PathBuf::from(os_str)));
        } else if Self::starts_with_drive(file_ns_stripped) {
            let os_str = unsafe { OsStr::from_encoded_bytes_unchecked(file_ns_stripped) };
            return Some(Cow::Borrowed(Path::new(os_str)));
        }
        None
    }

    /// Convert `\\` to `\\?\UNC\`.
    pub(crate) fn to_filename_space_unc(&self) -> Option<PathBuf> {
        if !self.is_file_namespace() {
            let mut path = OsString::from(r"\\?\UNC\");
            path.push(self.as_stripped_os_str());
            return Some(PathBuf::from(path));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_unc_try_from() {
        let test_cases = [
            (r"\\", true),
            (r"\\?", true),
            (r"\\?\", true),
            (r"\\server\share\dir", true),
            (r"\\?\server\share\dir", true),
            (r"\\?\C:\", true),
            // Forward slashes.
            ("//", true),
            ("//?", true),
            ("//?/", true),
            ("//server/share/dir", true),
            ("//?/server/share/dir", true),
            ("//?/C:/", true),
            // Non-UNC paths.
            ("", false),
            ("/", false),
            (r"\", false),
            (r"C:\a\b", false),
            (r"\a\b", false),
            (r"a\b", false),
        ];
        for (str, expect) in test_cases {
            let path = Path::new(str);
            let bytes = str.as_bytes();
            assert_eq!(UncPath::is_unc(path), expect, "input: {str}");
            assert_eq!(UncPath::is_unc(str), expect, "input: {str}");
            assert_eq!(UncPath::is_unc_bytes(bytes), expect, "input: {str}");

            assert_eq!(UncPath::try_from(path).is_ok(), expect, "input: {str}");
            assert_eq!(UncPath::try_from(str).is_ok(), expect, "input: {str}");
            assert_eq!(UncPath::try_from(bytes).is_ok(), expect, "input: {str}");
        }
    }

    fn from_str(str: &str) -> UncPath<'_> {
        UncPath::try_from(str).unwrap()
    }

    #[test]
    fn is_file_namespace() {
        assert!(from_str(r"\\?\").is_file_namespace());
        assert!(from_str(r"\\?\C:\server\share\dir").is_file_namespace());
        assert!(from_str(r"\\?\UNC\server\share\dir").is_file_namespace());

        assert!(from_str(r"//?/").is_file_namespace());
        assert!(from_str(r"//?/C:/server/share/dir").is_file_namespace());
        assert!(from_str(r"//?/UNC/server/share/dir").is_file_namespace());

        assert!(!from_str(r"\\?").is_file_namespace());
        assert!(!from_str(r"\\??\").is_file_namespace());
        assert!(!from_str(r"\\server\share").is_file_namespace());
        assert!(!from_str(r"\\.\device").is_file_namespace());
    }

    #[test]
    fn is_file_namespace_unc() {
        assert!(from_str(r"\\?\UNC\").is_file_namespace_unc());
        assert!(from_str(r"\\?\UNC\server\share\dir").is_file_namespace_unc());
        assert!(from_str(r"\\?\unc\server\share\dir").is_file_namespace_unc());
        assert!(from_str(r"\\?\uNc\server\share\dir").is_file_namespace_unc());
        assert!(from_str(r"\\?\UnC\server\share\dir").is_file_namespace_unc());

        assert!(from_str(r"//?/UNC/server/share/dir").is_file_namespace_unc());
        assert!(from_str(r"//?/UnC/server/share/dir").is_file_namespace_unc());

        assert!(!from_str(r"\\?\UN").is_file_namespace_unc());
        assert!(!from_str(r"\\?\UNC").is_file_namespace_unc());
        assert!(!from_str(r"\\?\UNCD\").is_file_namespace_unc());
        assert!(!from_str(r"\\?\server\share\dir").is_file_namespace_unc());
    }

    #[test]
    fn to_short_unc() {
        assert_eq!(
            from_str(r"\\?\UNC\server\share\dir").to_short_unc(),
            Some(Cow::Owned(PathBuf::from(r"\\server\share\dir")))
        );
        assert_eq!(
            from_str(r"//?/UNC/server/share/dir").to_short_unc(),
            Some(Cow::Owned(PathBuf::from(r"\\server\share\dir")))
        );
        assert_eq!(from_str(r"\\server\share\dir").to_short_unc(), None);
        assert_eq!(from_str(r"\\.\device").to_short_unc(), None);
    }

    #[test]
    fn to_short_unc_drive() {
        assert_eq!(
            from_str(r"\\?\C:").to_short_unc(),
            Some(Cow::Borrowed(Path::new(r"C:")))
        );
        assert_eq!(
            from_str(r"\\?\C:\dir\file").to_short_unc(),
            Some(Cow::Borrowed(Path::new(r"C:\dir\file")))
        );
    }

    #[test]
    fn to_filename_space_unc() {
        assert_eq!(
            from_str(r"\\server\share\dir").to_filename_space_unc(),
            Some(PathBuf::from(r"\\?\UNC\server\share\dir"))
        );
        assert_eq!(
            from_str(r"//server/share/dir").to_filename_space_unc(),
            Some(PathBuf::from(r"\\?\UNC\server/share/dir"))
        );
        assert_eq!(
            from_str(r"\\?\UNC\server\share\dir").to_filename_space_unc(),
            None
        );
        assert_eq!(from_str(r"\\?\C:\dir").to_filename_space_unc(), None);
    }

    #[test]
    fn starts_with_drive() {
        assert!(UncPath::starts_with_drive(br"A:"));
        assert!(UncPath::starts_with_drive(br"C:"));
        assert!(UncPath::starts_with_drive(br"Z:"));
        assert!(UncPath::starts_with_drive(br"a:"));
        assert!(UncPath::starts_with_drive(br"z:"));

        assert!(!UncPath::starts_with_drive(br""));
        assert!(!UncPath::starts_with_drive(br"@:"));
        assert!(!UncPath::starts_with_drive(br"[:"));
        assert!(!UncPath::starts_with_drive(br"A"));
        assert!(!UncPath::starts_with_drive(br"UNC"));
    }
}
