use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DrivePath<'a> {
    drive_letter: char,
    path: &'a Path,
}

impl<'a> DrivePath<'a> {
    pub(crate) fn new(drive_letter: char, path: &'a Path) -> Self {
        Self { drive_letter, path }
    }

    pub(crate) fn to_path_buf(&self) -> PathBuf {
        let mut drive_path = PathBuf::from(format!(r"{}:\", self.drive_letter));
        drive_path.push(self.path);
        drive_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_path_buf() {
        assert_eq!(
            DrivePath::new('X', Path::new(r"dir\file.txt")).to_path_buf(),
            PathBuf::from(r"X:\dir\file.txt")
        );
    }

    #[test]
    fn to_path_buf_abs() {
        // `strip_prefix` may leave the leading `\`.
        // https://github.com/rust-lang/rust/issues/155183
        assert_eq!(
            DrivePath::new('X', Path::new(r"\dir\file.txt")).to_path_buf(),
            PathBuf::from(r"X:\dir\file.txt")
        );
    }
}
