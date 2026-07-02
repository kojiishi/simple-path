use crate::{DrivePath, LogicalDrive, NetResource, PathExt};
use std::{
    cmp::Ordering,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
    time::Instant,
};

static VOLUMES: LazyLock<Mutex<Option<Volumes>>> = LazyLock::new(|| Mutex::new(None));

#[derive(Clone, Debug)]
pub(crate) struct Volumes {
    volumes: Vec<Volume>,
}

impl Volumes {
    fn new() -> anyhow::Result<Self> {
        Ok(Self::with_volumes(Volume::get_remote_volumes()?))
    }

    fn with_volumes(mut volumes: Vec<Volume>) -> Self {
        volumes = Volume::sort(volumes);
        Self { volumes }
    }

    #[cfg(all(test, windows))]
    pub(crate) fn mock() -> Self {
        Self::with_volumes(vec![
            Volume::new('H', PathBuf::from(r"\\?\UNC\server\share\dir")),
            Volume::new('X', PathBuf::from(r"\\?\UNC\server\share")),
            Volume::new('Z', PathBuf::from(r"\\?\UNC\server2\share2")),
            Volume::new('\0', PathBuf::from(r"\\?\UNC\server0\share0")),
        ])
    }

    pub(crate) fn refresh() -> anyhow::Result<()> {
        Volumes::new()?.set_to_cache();
        Ok(())
    }

    fn set_to_cache(self) {
        let mut cache = VOLUMES.lock().unwrap();
        *cache = Some(self);
    }

    pub(crate) fn drive_path<'a>(path: &'a Path) -> anyhow::Result<Option<DrivePath<'a>>> {
        let drives = {
            let mut cache = VOLUMES.lock().unwrap();
            if cache.is_none() {
                *cache = Some(Volumes::new()?);
            }
            cache.as_ref().cloned().unwrap()
        };
        Ok(drives._drive_path(path))
    }

    pub(crate) fn _drive_path<'a>(&self, path: &'a Path) -> Option<DrivePath<'a>> {
        for volume in &self.volumes {
            if let Ok(suffix) = path.strip_prefix_fix(&volume.path) {
                return Some(DrivePath::new(volume.drive_letter, suffix));
            }
        }
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Volume {
    drive_letter: char,
    path: PathBuf,
}

impl Volume {
    pub(crate) fn new(drive_letter: char, path: impl AsRef<Path>) -> Self {
        Self {
            drive_letter,
            path: path.as_ref().into(),
        }
    }

    fn has_drive(&self) -> bool {
        self.drive_letter != '\0'
    }

    /// The shortest path wins.
    /// If paths are the same, the lowest drive letter wins.
    fn sort(mut volumes: Vec<Volume>) -> Vec<Volume> {
        volumes.sort_by(|a, b| {
            let mut cmp = a.path.as_os_str().len().cmp(&b.path.as_os_str().len());
            if cmp == Ordering::Equal {
                cmp = a.path.cmp(&b.path);
                if cmp == Ordering::Equal && a.drive_letter != b.drive_letter {
                    if !a.has_drive() {
                        cmp = Ordering::Greater;
                    } else if !b.has_drive() {
                        cmp = Ordering::Less;
                    } else {
                        cmp = a.drive_letter.cmp(&b.drive_letter);
                    }
                }
            }
            cmp
        });
        volumes
    }

    fn get_remote_volumes() -> anyhow::Result<Vec<Self>> {
        // Get both drives and net resources and unify them.
        // They often match, but there are cases where they don't, such as:
        // 1. Third-Party Redirectors and Virtual Filesystems.
        // 2. Offline/Disconnected Mapped Drives.
        // 3. When network services are disabled, unresponsive, or in a
        //    restricted context.
        let drives = Self::get_remote_drives()?;
        let net_resources = Self::get_net_resources()?;
        let volumes = drives.into_iter().chain(net_resources);
        let mut map: HashMap<PathBuf, Volume> = HashMap::new();
        for volume in volumes {
            match map.entry(volume.path.clone()) {
                std::collections::hash_map::Entry::Occupied(mut occ) => {
                    if !occ.get().has_drive() && volume.has_drive() {
                        *occ.get_mut() = volume;
                    }
                }
                std::collections::hash_map::Entry::Vacant(vac) => {
                    vac.insert(volume);
                }
            }
        }
        Ok(map.values().cloned().collect())
    }

    fn get_remote_drives() -> anyhow::Result<Vec<Self>> {
        let start = Instant::now();
        let mut drives = Vec::new();
        for drive in LogicalDrive::all()? {
            if drive.is_remote() {
                // Use `fs::canonicalize` to match the expected inputs.
                let canonicalized = fs::canonicalize(drive.path());
                log::trace!("Drive: {drive:?} -> {canonicalized:?}");
                let Ok(canonicalized) = canonicalized else {
                    // Skip failed drives. This could be, for example,
                    // a disconnected drive due to incorrect user/password.
                    continue;
                };
                drives.push(Self::new(drive.drive_letter, canonicalized));
            }
        }
        log::trace!("Drive: elapsed {:?}", start.elapsed());
        Ok(drives)
    }

    fn get_net_resources() -> anyhow::Result<Vec<Volume>> {
        let start = Instant::now();
        let mut volumes = Vec::new();
        for resource in NetResource::all()? {
            let resource = resource?;
            if resource.remote.is_empty() {
                log::warn!("Remote is empty for {resource:?}");
                continue;
            }
            volumes.push(Volume::new(
                resource.local_drive_letter(),
                resource.remote_canonicalized(),
            ));
        }
        log::trace!("NetResource: elapsed {:?}", start.elapsed());
        Ok(volumes)
    }
}

#[cfg(test)]
pub(crate) static TEST_LOG_INIT: LazyLock<bool> = LazyLock::new(|| {
    env_logger::init();
    true
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_remote_volumes() {
        // As the result depends on the machine configuration, all this test can
        // check is if it doesn't fail.
        // You can check the result manually by:
        // ```
        // cargo test -- print_remote_volumes --nocapture
        // ```
        assert!(*TEST_LOG_INIT);
        let start = Instant::now();
        let volumes = Volume::get_remote_volumes().unwrap();
        for volume in volumes {
            println!("{volume:?}");
        }
        println!("Volume: elapsed {:?}", start.elapsed());
    }

    #[test]
    fn sort() {
        assert_eq!(Volume::sort(vec![]), vec![]);
        assert_eq!(
            Volume::sort(vec![Volume::new('A', PathBuf::from("1"))]),
            vec![Volume::new('A', PathBuf::from("1"))]
        );
        assert_eq!(
            Volume::sort(vec![
                Volume::new('\0', PathBuf::from("124")),
                Volume::new('\0', PathBuf::from("123")),
                Volume::new('C', PathBuf::from("12")),
                Volume::new('A', PathBuf::from("123")),
                Volume::new('B', PathBuf::from("12")),
            ]),
            vec![
                Volume::new('B', PathBuf::from("12")),
                Volume::new('C', PathBuf::from("12")),
                Volume::new('A', PathBuf::from("123")),
                Volume::new('\0', PathBuf::from("123")),
                Volume::new('\0', PathBuf::from("124")),
            ]
        );
    }

    // [`fs::canonicalize`] emits the [Win32 File Namespaces].
    //
    // [`fs::canonicalize`]: https://doc.rust-lang.org/std/fs/fn.canonicalize.html
    // [Win32 File Namespaces]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file#win32-file-namespaces
    #[test]
    fn drive_path_win32_file_namespaces() {
        let volumes = Volumes::mock();
        assert_eq!(
            volumes._drive_path(Path::new(r"\\?\UNC\server\share\dir\file.txt")),
            Some(DrivePath::new('X', Path::new(r"dir\file.txt")))
        );
        assert_eq!(
            volumes._drive_path(Path::new(r"\\?\UNC\server2\share2\dir2\file2.txt")),
            Some(DrivePath::new('Z', Path::new(r"dir2\file2.txt"))),
        );
        assert_eq!(
            volumes._drive_path(Path::new(r"\\?\UNC\server3\share3\dir3\file3.txt")),
            None,
        );
        assert_eq!(volumes._drive_path(Path::new(r"C:\Windows\System32")), None);
        assert_eq!(
            volumes._drive_path(Path::new(r"\\?\UNC\server0\share0\dir\file.txt")),
            Some(DrivePath::new('\0', Path::new(r"dir\file.txt")))
        );
    }
}
