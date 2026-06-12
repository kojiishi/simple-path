#![cfg_attr(not(target_os = "windows"), allow(unused))]
use crate::Display;
#[cfg(windows)]
use crate::{LongUnc, PathExt, Volumes};
use std::{
    borrow::Cow,
    fs, io,
    path::{Path, PathBuf, StripPrefixError},
};

/// Simplifies [Win32 File Namespaces] paths (the "`\\?\`" prefix)
/// for better readability and compatibility.
///
/// The following code is a snap-in replacement of [`fs::canonicalize`].
/// ```no_run
/// # use simple_path::SimplePath;
/// # let path = "";
/// SimplePath::default().canonicalize(path);
/// ```
///
/// If you have `net use Z: \\server\share`:
/// | | `C:\dir` | `Z:\x` |
/// | --- | --- | --- |
/// | [`fs::canonicalize`] | `\\?\C:\dir` | `\\?\UNC\server\share\x` |
/// | `SimplePath` | `C:\dir` | `\\server\share\x` |
/// | `SimplePath` with [`map_to_drive`] | `C:\dir` | `Z:\x` |
///
/// [`map_to_drive`]: `SimplePath::map_to_drive`
/// [Win32 File Namespaces]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#win32-file-namespaces
#[derive(Clone, Debug, Default)]
pub struct SimplePath {
    /// Disallow simplifications
    /// if the result is a "long path" (longer than 260 characters).
    /// Initially `false`.
    ///
    /// Long paths may not be supported by some programs and APIs.
    /// In such cases, using the [Win32 File Namespaces] (the "`\\?\`" prefix)
    /// can often work around the limitation.
    /// Setting this option to `true` can improve
    /// the compatibility with such cases.
    ///
    /// On the other hand, some other programs such as PowerShell v7
    /// can handle long paths,
    /// but they can't handle the "`\\?\`" prefix.
    /// They work best with `false`.
    ///
    /// [Win32 File Namespaces]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#win32-file-namespaces
    pub disallow_long: bool,

    /// Simplify all long UNC paths (prefixed by "`\\?\UNC\`").
    /// Initially `false`.
    ///
    /// Technically speaking,
    /// since the "`\\?\`" prefix ([Win32 File Namespaces])
    /// disables all string parsing and
    /// sends the following string directly to the file system,
    /// simplifying the path is not always guaranteed to be safe or equivalent.
    ///
    /// For this reason,
    /// the `SimplePath` simplifies connected network shares only by default.
    /// Set this option to `true`
    /// to simplify all paths prefixed by "`\\?\UNC\`".
    ///
    /// Please also see the [safety] note.
    ///
    /// # Examples
    /// ```
    /// # use simple_path::SimplePath;
    /// # use std::path::Path;
    /// let path = Path::new(r"\\?\UNC\server\share\dir");
    /// let simple = SimplePath { allow_unknown_unc: true, ..Default::default() };
    /// #[cfg(windows)]
    /// assert_eq!(&*simple.simplify(path).unwrap().unwrap(), r"\\server\share\dir");
    /// ```
    ///
    /// [safety]: https://github.com/kojiishi/simple-path#safety
    /// [Win32 File Namespaces]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#win32-file-namespaces
    pub allow_unknown_unc: bool,

    /// Map to network share drive names when possible.
    /// Initially `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use simple_path::SimplePath;
    /// # fn test() -> std::io::Result<()> {
    /// let path = "file.txt";
    /// let simple = SimplePath { map_to_drive: true, ..Default::default() };
    /// let canonicalized = simple.canonicalize(path)?;
    /// # Ok(())
    /// # }
    /// ```
    /// If the `file.txt` is in a network drive,
    /// the result is `Z:\dir\file.txt`
    /// instead of `\\server\share\dir\file.txt`.
    ///
    /// The following code tries to preserve the original form of the `path`.
    /// ```
    /// # use simple_path::SimplePath;
    /// # fn test(path: &std::path::Path) -> std::io::Result<()> {
    /// SimplePath {
    ///     map_to_drive: !path.as_os_str().as_encoded_bytes().starts_with(br"\\"),
    ///     ..Default::default()
    /// }.canonicalize(path)?;
    /// # Ok(())
    /// # }
    /// ```
    pub map_to_drive: bool,

    /// Skip the [`dunce`] simplification.
    /// Initially `false`.
    ///
    /// [`dunce`]: https://crates.io/crates/dunce
    pub skip_dunce: bool,

    /// It is highly recommended to always use `, ..Default::default()`.
    /// Otherwise builds fail when new fields are added.
    ///
    /// This field is not used in any ways,
    /// but exists to allow using `, ..Default::default()`
    /// even when all other fields are specified.
    pub _unused: bool,

    #[cfg(all(test, windows))]
    volumes: Option<Volumes>,
}

impl SimplePath {
    #[cfg(all(test, windows))]
    pub(crate) fn mock() -> SimplePath {
        SimplePath {
            volumes: Some(Volumes::mock()),
            ..Default::default()
        }
    }

    /// A snap-in replacement for [`fs::canonicalize`].
    /// It calls [`fs::canonicalize`] and [`simplify`].
    ///
    /// On other platforms than Windows,
    /// this is equivalent to [`fs::canonicalize`].
    ///
    /// # Examples
    /// ```
    /// # fn test(path: &std::path::Path) -> std::io::Result<()> {
    /// use simple_path::SimplePath;
    /// let canonicalized = SimplePath::default().canonicalize(path)?;
    /// println!("{}", canonicalized.display());
    /// # Ok(()) }
    /// ```
    ///
    /// [`fs::canonicalize`]: https://doc.rust-lang.org/std/fs/fn.canonicalize.html
    /// [`simplify`]: SimplePath::simplify
    pub fn canonicalize(&self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        let canonicalized = fs::canonicalize(path)?;
        #[cfg(windows)]
        if let Some(simplified) = self.simplify(&canonicalized)? {
            return Ok(simplified.into_owned());
        }
        Ok(canonicalized)
    }

    /// Try to simplify the given `path`.
    ///
    /// Returns `Ok(None)`
    /// if no simplification is applied,
    /// or on other platforms than Windows.
    pub fn simplify<'a>(&self, path: &'a Path) -> io::Result<Option<Cow<'a, Path>>> {
        #[cfg(windows)]
        return self._simplify(path).map_err(io_error_from_anyhow);
        #[cfg(not(windows))]
        Ok(None)
    }

    #[cfg(windows)]
    fn _simplify<'a>(&self, path: &'a Path) -> anyhow::Result<Option<Cow<'a, Path>>> {
        if let Ok(long_unc) = LongUnc::try_from(path)
            && long_unc.is_sub_prefix_unc()
        {
            // Try mapped network drives.
            let drive_path = if !self.allow_unknown_unc || self.map_to_drive {
                self.drive_path(path)?
            } else {
                None
            };
            if self.map_to_drive
                && let Some(drive_path) = &drive_path
                && drive_path.has_drive()
                && !drive_path.has_invalid_chars()
                && (!self.disallow_long || !drive_path.is_longer_than_max_path())
            {
                return Ok(Some(Cow::Owned(drive_path.to_path_buf())));
            }

            // Try short UNC (`\\server\share`).
            if (self.allow_unknown_unc || drive_path.is_some())
                && !long_unc.has_invalid_chars()
                && (!self.disallow_long || !long_unc.is_short_unc_longer_than_max_path())
            {
                return Ok(Some(Cow::Owned(long_unc.to_short_unc())));
            }
        }

        // Try `dunce::simplified`.
        if !self.skip_dunce {
            let simplified = dunce::simplified(path);
            if !std::ptr::eq(path, simplified) {
                return Ok(Some(Cow::Borrowed(simplified)));
            }
        }
        Ok(None)
    }

    #[cfg(windows)]
    fn drive_path<'a>(&self, path: &'a Path) -> anyhow::Result<Option<crate::DrivePath<'a>>> {
        #[cfg(test)]
        if let Some(volumes) = &self.volumes {
            return Ok(volumes._drive_path(path));
        }
        Volumes::drive_path(path)
    }

    /// Refreshes the cached information.
    pub fn refresh() -> io::Result<()> {
        #[cfg(windows)]
        Volumes::refresh().map_err(io_error_from_anyhow)?;
        Ok(())
    }

    /// Returns an object that implements [`Display`][`core::fmt::Display`]
    /// for printing simplified paths.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::path::Path;
    /// # use simple_path::SimplePath;
    /// # fn test() -> std::io::Result<()> {
    /// let path = Path::new("file").canonicalize()?;
    /// println!("{}", SimplePath::default().display(&path));
    /// # Ok(())
    /// # }
    /// ```
    pub fn display<'a>(&'a self, path: &'a Path) -> Display<'a> {
        Display::new(self, path)
    }

    /// A snap-in replacement for [`Path::strip_prefix`]
    /// with a fix for [a leading directory separator "`\`" left for UNC paths
    /// on Windows](https://github.com/rust-lang/rust/issues/155183).
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::path::{Path, StripPrefixError};
    /// # use simple_path::SimplePath;
    /// # fn t<'a>(path: &'a Path, base: &'a Path) -> Result<&'a Path, StripPrefixError> {
    /// SimplePath::strip_prefix(path, base)
    /// # }
    /// ```
    pub fn strip_prefix(path: &Path, base: impl AsRef<Path>) -> Result<&Path, StripPrefixError> {
        #[cfg(windows)]
        return PathExt::strip_prefix_fix(path, base);
        #[cfg(not(windows))]
        path.strip_prefix(base)
    }
}

fn io_error_from_anyhow(error: anyhow::Error) -> io::Error {
    match error.downcast::<io::Error>() {
        Ok(io_error) => io_error,
        Err(other_error) => io::Error::other(other_error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn simplify_drive() {
        let mut simple = SimplePath::mock();
        assert_eq!(simple.simplify(Path::new(r"C:\foo")).unwrap(), None);
        simple.allow_unknown_unc = true;
        assert_eq!(simple.simplify(Path::new(r"C:\foo")).unwrap(), None);
    }

    #[cfg(windows)]
    #[test]
    fn simplify_drive_unc() {
        let mut simple = SimplePath::mock();
        let path = Path::new(r"\\?\UNC\server\share\foo");
        let path2 = Path::new(r"\\?\UNC\server2\share2\foo2");
        assert_eq!(
            simple.simplify(path).unwrap(),
            Some(Cow::Owned(PathBuf::from(r"\\server\share\foo")))
        );
        assert_eq!(
            simple.simplify(path2).unwrap(),
            Some(Cow::Owned(PathBuf::from(r"\\server2\share2\foo2")))
        );

        simple.map_to_drive = true;
        assert_eq!(
            simple.simplify(path).unwrap(),
            Some(Cow::Owned(PathBuf::from(r"X:\foo")))
        );
        assert_eq!(
            simple.simplify(path2).unwrap(),
            Some(Cow::Owned(PathBuf::from(r"Z:\foo2")))
        );
    }

    #[cfg(windows)]
    #[test]
    fn simplify_dunce() {
        let simple = SimplePath::default();
        assert_eq!(
            simple.simplify(Path::new(r"\\?\C:\foo")).unwrap(),
            Some(Cow::Borrowed(Path::new(r"C:\foo")))
        );
    }

    #[cfg(windows)]
    #[test]
    fn simplify_dunce_skip() {
        let simple = SimplePath {
            skip_dunce: true,
            ..Default::default()
        };
        assert_eq!(simple.simplify(Path::new(r"\\?\C:\foo")).unwrap(), None);
    }

    #[cfg(windows)]
    #[test]
    fn simplify_unmapped_connected_share() {
        let mut simple = SimplePath::mock();
        let path = Path::new(r"\\?\UNC\server0\share0\foo");
        assert_eq!(
            simple.simplify(path).unwrap(),
            Some(Cow::Owned(PathBuf::from(r"\\server0\share0\foo")))
        );

        // Even with map_to_drive = true, it should simplify to the UNC path,
        // because the drive letter is '\0'.
        simple.map_to_drive = true;
        assert_eq!(
            simple.simplify(path).unwrap(),
            Some(Cow::Owned(PathBuf::from(r"\\server0\share0\foo")))
        );
    }

    #[cfg(windows)]
    #[test]
    fn simplify_unknown_unc() -> anyhow::Result<()> {
        let mut simple = SimplePath::mock();
        let unknown = Path::new(r"\\?\UNC\server\unknown\foo");
        let mapped = Path::new(r"\\?\UNC\server\share\foo");
        assert_eq!(simple.simplify(unknown)?, None);

        // `unknown` should be simplified if `allow_unknown_unc`.
        simple.allow_unknown_unc = true;
        assert_eq!(
            simple.simplify(unknown)?,
            Some(Cow::Owned(PathBuf::from(r"\\server\unknown\foo")))
        );
        assert_eq!(
            simple.simplify(mapped)?,
            Some(Cow::Owned(PathBuf::from(r"\\server\share\foo")))
        );

        // `map_to_drive` should still be in effect.
        simple.map_to_drive = true;
        assert_eq!(
            simple.simplify(mapped)?,
            Some(Cow::Owned(PathBuf::from(r"X:\foo")))
        );

        // `allow_unknown_unc` should simplify only for "`\\?\UNC\`".
        assert_eq!(simple.simplify(Path::new(r"\\.\COM1:"))?, None);
        simple.skip_dunce = true;
        assert_eq!(simple.simplify(Path::new(r"\\?\C:\foo"))?, None);
        Ok(())
    }
}
