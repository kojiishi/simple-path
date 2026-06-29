use crate::PathExt;
use std::{iter::FusedIterator, path::Path};
use windows::{
    Win32::{
        Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives},
        System::WindowsProgramming::DRIVE_REMOTE,
    },
    core::PCWSTR,
};

#[derive(Debug)]
pub(crate) struct LogicalDrive {
    pub(crate) drive_letter: char,
    path_str: String,
}

impl LogicalDrive {
    fn new(drive_letter: char) -> Self {
        Self {
            drive_letter,
            path_str: format!(r"{drive_letter}:\"),
        }
    }

    pub(crate) fn all() -> Result<LogicalDriveIter, windows::core::Error> {
        LogicalDriveIter::new()
    }

    #[inline]
    pub(crate) fn path(&self) -> &Path {
        Path::new(&self.path_str)
    }

    fn drive_type(&self) -> u32 {
        let path = self.path();
        let path_u16 = path.to_wide_vec_with_nul();
        unsafe { GetDriveTypeW(PCWSTR(path_u16.as_ptr())) }
    }

    pub(crate) fn is_remote(&self) -> bool {
        self.drive_type() == DRIVE_REMOTE
    }
}

#[derive(Debug)]
pub(crate) struct LogicalDriveIter {
    mask: u32,
    drive_letter: u8,
}

impl LogicalDriveIter {
    fn new() -> Result<Self, windows::core::Error> {
        let drive_mask = unsafe { GetLogicalDrives() };
        if drive_mask == 0 {
            return Err(windows::core::Error::from_thread());
        }
        Ok(Self::with_mask(drive_mask))
    }

    fn with_mask(mask: u32) -> Self {
        Self {
            mask,
            drive_letter: b'A',
        }
    }
}

impl Iterator for LogicalDriveIter {
    type Item = LogicalDrive;

    fn next(&mut self) -> Option<Self::Item> {
        while self.mask != 0 {
            if self.mask & 1 != 0 {
                let drive_letter = self.drive_letter as char;
                self.mask >>= 1;
                self.drive_letter += 1;
                return Some(LogicalDrive::new(drive_letter));
            }
            self.mask >>= 1;
            self.drive_letter += 1;
        }
        None
    }
}

impl FusedIterator for LogicalDriveIter {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_logical_drives() -> anyhow::Result<()> {
        // As the result depends on the machine configuration, all this test can
        // check is if it doesn't fail.
        // You can check the result manually by:
        // ```
        // cargo test -- print_logical_drives --nocapture
        // ```
        assert!(*crate::TEST_LOG_INIT);
        for drive in LogicalDrive::all()? {
            println!("{drive:?} {}", drive.drive_type());
        }
        Ok(())
    }

    #[test]
    fn iterator() {
        let vec_from_mask = |mask| -> Vec<char> {
            LogicalDriveIter::with_mask(mask)
                .map(|d| d.drive_letter)
                .collect()
        };
        assert_eq!(vec_from_mask(0b0), vec![]);
        assert_eq!(vec_from_mask(0b1), vec!['A']);
        assert_eq!(vec_from_mask(0b10), vec!['B']);
        assert_eq!(vec_from_mask(0b11), vec!['A', 'B']);
        assert_eq!(vec_from_mask(0b101), vec!['A', 'C']);
        assert_eq!(vec_from_mask(0b1010), vec!['B', 'D']);
        assert_eq!(vec_from_mask(0b1100), vec!['C', 'D']);
    }
}
