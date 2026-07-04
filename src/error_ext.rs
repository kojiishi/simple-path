use std::io;

pub(crate) trait ErrorExt {
    fn into_io_error(self) -> io::Error;
}

impl ErrorExt for anyhow::Error {
    fn into_io_error(self) -> io::Error {
        match self.downcast::<io::Error>() {
            Ok(io_error) => io_error,
            Err(other_error) => io::Error::other(other_error),
        }
    }
}
