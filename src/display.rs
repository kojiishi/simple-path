use crate::SimplePath;
use std::{
    fmt,
    path::{self, Path},
};

/// Helper struct for printing simplified paths with [`format!`] and `{}`.
///
/// Please see [`SimplePath::display`].
#[derive(Debug)]
pub struct Display<'a> {
    #[cfg(windows)]
    simple: &'a SimplePath,
    path: &'a Path,
}

impl<'a> Display<'a> {
    #[cfg(windows)]
    pub(crate) fn new(simple: &'a SimplePath, path: &'a Path) -> Self {
        Self { simple, path }
    }
    #[cfg(not(windows))]
    pub(crate) fn new(_unc: &'a SimplePath, path: &'a Path) -> Self {
        Self { path }
    }
}

impl fmt::Display for Display<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(windows)]
        if let Ok(Some(simplified)) = self.simple.simplify(self.path) {
            return path::Display::fmt(&simplified.display(), f);
        };
        path::Display::fmt(&self.path.display(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_not_simplified() {
        let simple = SimplePath::default();
        let path = Path::new("foo");
        assert_eq!(format!("{}", simple.display(path)), "foo");
    }

    #[cfg(windows)]
    #[test]
    fn display_drive_unc() {
        let mut simple = SimplePath::mock();
        let path = Path::new(r"\\?\UNC\server\share\foo");
        assert_eq!(format!("{}", simple.display(path)), r"\\server\share\foo");

        simple.map_to_drive = true;
        assert_eq!(format!("{}", simple.display(path)), r"X:\foo");
    }
}
