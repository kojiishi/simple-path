use crate::WinStrExt;
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    time::Instant,
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
    pub(crate) fn get_all() -> windows::core::Result<Vec<Self>> {
        let start = Instant::now();
        let mut resources = Vec::new();
        // for scope in [RESOURCE_CONNECTED, RESOURCE_REMEMBERED] {
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
        let mut buffer = vec![0u8; 16384];
        loop {
            let mut count = 0xFFFFFFFF;
            let mut buffer_size = buffer.len() as u32;
            let res = unsafe {
                WNetEnumResourceW(
                    henum,
                    &mut count,
                    buffer.as_mut_ptr() as *mut _,
                    &mut buffer_size,
                )
            };
            match res {
                NO_ERROR => {}
                ERROR_NO_MORE_ITEMS => break,
                ERROR_MORE_DATA => {
                    buffer.resize(buffer_size as usize, 0);
                    continue;
                }
                _ => return Err(windows::core::Error::from_hresult(res.to_hresult())),
            }
            let entries = count as usize;
            let ptr = buffer.as_ptr() as *const NETRESOURCEW;
            for i in 0..entries {
                let src = unsafe { &*ptr.add(i) };
                let resource = Self {
                    local: src.lpLocalName.to_os_string(),
                    remote: src.lpRemoteName.to_os_string(),
                };
                log::trace!("enum: {src:?}, {resource:?}");
                resources.push(resource);
            }
        }
        let _ = unsafe { WNetCloseEnum(henum) };
        log::trace!("get_all: elapsed {:?}", start.elapsed());
        Ok(resources)
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

#[cfg(test)]
mod tests {
    use super::*;

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
