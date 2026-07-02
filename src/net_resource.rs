use crate::WinStrExt;
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
};
use windows::Win32::{
    Foundation::{ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS, HANDLE, NO_ERROR},
    NetworkManagement::WNet::{
        NETRESOURCEW, RESOURCE_CONNECTED, RESOURCETYPE_DISK, WNET_OPEN_ENUM_USAGE, WNetCloseEnum,
        WNetEnumResourceW, WNetOpenEnumW,
    },
};

/// Logical translation of the [`NETRESOURCEW``] structure.
///
/// [`NETRESOURCEW``]: https://learn.microsoft.com/en-us/windows/win32/api/winnetwk/ns-winnetwk-netresourcew
#[derive(Debug)]
pub(crate) struct NetResource {
    pub(crate) local: OsString,
    pub(crate) remote: OsString,
}

impl NetResource {
    pub(crate) fn all() -> windows::core::Result<NetResourceIter> {
        NetResourceIter::new()
    }

    pub(crate) fn local_drive_letter(&self) -> char {
        if self.local.len() == 2 && self.local.as_encoded_bytes()[1] == b':' {
            return self.local.as_encoded_bytes()[0] as char;
        }
        '\0'
    }

    pub(crate) fn remote_canonicalized<'a>(&'a self) -> Cow<'a, OsStr> {
        Self::normalize_remote(&self.remote)
    }

    fn normalize_remote<'a>(mut remote: &'a OsStr) -> Cow<'a, OsStr> {
        let mut bytes = remote.as_encoded_bytes();
        while matches!(bytes.last(), Some(b'\\')) {
            bytes = &bytes[..bytes.len() - 1];
        }
        if let Some(stripped) = bytes.strip_prefix(br"\\") {
            let mut path = OsString::from(r"\\?\UNC\");
            path.push(unsafe { OsStr::from_encoded_bytes_unchecked(stripped) });
            return Cow::Owned(path);
        }
        if bytes.len() != remote.len() {
            remote = unsafe { OsStr::from_encoded_bytes_unchecked(bytes) };
        }
        Cow::Borrowed(remote)
    }
}

#[derive(Debug)]
pub(crate) struct NetResourceIter {
    henum: HANDLE,
    index: u32,
    count: u32,
    buffer: Vec<u8>,
}

impl NetResourceIter {
    fn new() -> windows::core::Result<Self> {
        let mut henum = HANDLE::default();
        let res = unsafe {
            WNetOpenEnumW(
                RESOURCE_CONNECTED,
                RESOURCETYPE_DISK,
                WNET_OPEN_ENUM_USAGE(0),
                None,
                &mut henum,
            )
        };
        if res != NO_ERROR {
            return Err(windows::core::Error::from_hresult(res.to_hresult()));
        }
        Ok(Self {
            henum,
            index: 0,
            count: 0,
            buffer: vec![0u8; 16384],
        })
    }

    fn fetch(&mut self) -> windows::core::Result<bool> {
        assert!(self.index >= self.count);
        loop {
            let mut count = 0xFFFFFFFF;
            let mut buffer_size = self.buffer.len() as u32;
            let res = unsafe {
                WNetEnumResourceW(
                    self.henum,
                    &mut count,
                    self.buffer.as_mut_ptr() as *mut _,
                    &mut buffer_size,
                )
            };
            match res {
                NO_ERROR => {}
                ERROR_NO_MORE_ITEMS => return Ok(false),
                ERROR_MORE_DATA => {
                    self.buffer.resize(buffer_size as usize, 0);
                    continue;
                }
                _ => return Err(windows::core::Error::from_hresult(res.to_hresult())),
            }
            assert!(count > 0);
            self.count = count;
            self.index = 0;
            break;
        }
        Ok(true)
    }
}

impl Drop for NetResourceIter {
    fn drop(&mut self) {
        let _ = unsafe { WNetCloseEnum(self.henum) };
    }
}

impl Iterator for NetResourceIter {
    type Item = windows::core::Result<NetResource>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            match self.fetch() {
                Ok(true) => {}
                Ok(false) => return None,
                Err(error) => return Some(Err(error)),
            }
        }
        assert!(self.index < self.count);
        let ptr = self.buffer.as_ptr() as *const NETRESOURCEW;
        let src = unsafe { &*ptr.add(self.index as usize) };
        let resource = NetResource {
            local: src.lpLocalName.to_os_string(),
            remote: src.lpRemoteName.to_os_string(),
        };
        log::trace!("enum: {src:?}, {resource:?}");
        self.index += 1;
        Some(Ok(resource))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn print_net_resources() -> anyhow::Result<()> {
        // As the result depends on the machine configuration, all this test can
        // check is if it doesn't fail.
        // You can check the result manually by:
        // ```
        // cargo test -- print_net_resources --nocapture
        // ```
        assert!(*crate::TEST_LOG_INIT);
        let start = Instant::now();
        for resource in NetResource::all()? {
            println!("{resource:?}");
        }
        println!("NetResource: elapsed {:?}", start.elapsed());
        Ok(())
    }

    #[test]
    fn normalize_remote() {
        let test_cases = [
            (r"\\server\share", r"\\?\UNC\server\share"),
            (r"\\server\share\", r"\\?\UNC\server\share"),
            (r"\\server\share\\", r"\\?\UNC\server\share"),
            (r"C:\foo", r"C:\foo"),
            (r"C:\foo\", r"C:\foo"),
        ];
        for (input, expected) in test_cases {
            let res = NetResource::normalize_remote(OsStr::new(input));
            assert_eq!(&*res, OsStr::new(expected), "input: {}", input);
        }
    }
}
