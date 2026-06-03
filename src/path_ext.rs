use std::{os::windows::ffi::OsStrExt, path::Path};

pub(crate) trait PathExt {
    fn to_wide_vec_with_null(&self) -> Vec<u16>;
}

impl PathExt for Path {
    fn to_wide_vec_with_null(&self) -> Vec<u16> {
        self.as_os_str().encode_wide().chain(Some(0)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_wide_vec_with_null() {
        assert_eq!(Path::new("AB").to_wide_vec_with_null(), vec![0x41, 0x42, 0]);
        assert_eq!(
            Path::new("\u{3042}\u{3043}").to_wide_vec_with_null(),
            vec![0x3042, 0x3043, 0]
        );
    }
}
