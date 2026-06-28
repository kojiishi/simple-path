//! Please see the [README] for a high-level overview,
//! and the [`SimplePath`] struct for the detailed features.
//!
//! [README]: https://github.com/kojiishi/simple-path

mod display;
#[cfg(windows)]
mod drive_path;
#[cfg(windows)]
mod net_resource;
#[cfg(windows)]
mod os_str_ext;
#[cfg(windows)]
mod path_ext;
mod simple_path;
#[cfg(windows)]
mod volume;
#[cfg(windows)]
mod win32_file_namespace_path;

pub use display::Display;
#[cfg(windows)]
pub(crate) use drive_path::DrivePath;
#[cfg(windows)]
pub(crate) use net_resource::NetResource;
#[cfg(windows)]
pub(crate) use os_str_ext::{OsStrExt, WinStrExt};
#[cfg(windows)]
pub(crate) use path_ext::PathExt;
pub use simple_path::SimplePath;
#[cfg(windows)]
pub(crate) use volume::Volumes;
#[cfg(windows)]
pub(crate) use win32_file_namespace_path::Win32FileNamespacePath;
