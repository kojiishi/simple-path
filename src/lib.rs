//! Please see the [README] for a high-level overview,
//! and the [`SimplePath`] struct for the detailed features.
//!
//! [README]: https://github.com/kojiishi/simple-path

mod display;
#[cfg(windows)]
mod drive_path;
#[cfg(windows)]
mod path_ext;
mod simple_path;
#[cfg(windows)]
mod volume;

pub use display::Display;
#[cfg(windows)]
pub(crate) use drive_path::DrivePath;
#[cfg(windows)]
pub(crate) use path_ext::PathExt;
pub use simple_path::SimplePath;
#[cfg(windows)]
pub(crate) use volume::Volumes;
