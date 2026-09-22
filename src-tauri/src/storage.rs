use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub const PORTABLE_FLAG: &str = "portable-data.flag";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageRoots {
    pub data: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub mods: PathBuf,
    pub mode: &'static str,
}

impl StorageRoots {
    pub fn platform(data: PathBuf, cache: PathBuf, logs: PathBuf, mods: PathBuf) -> Self {
        Self {
            data,
            cache,
            logs,
            mods,
            mode: "platform",
        }
    }
}

/// Selects the portable data root only when the executable directory contains
/// an explicit marker. A portable executable without the marker still keeps
/// user data in the platform directories, so replacing or moving the EXE does
/// not silently strand the library.
pub fn resolve(executable_dir: &Path, platform: StorageRoots) -> io::Result<StorageRoots> {
    if !executable_dir.join(PORTABLE_FLAG).is_file() {
        return Ok(platform);
    }

    let root = executable_dir.join("data");
    let portable = StorageRoots {
        data: root.clone(),
        cache: root.join("cache"),
        logs: root.join("logs"),
        mods: root.join("mods"),
        mode: "portable",
    };
    for directory in [
        &portable.data,
        &portable.cache,
        &portable.logs,
        &portable.mods,
    ] {
        fs::create_dir_all(directory).map_err(|error| portable_error(executable_dir, error))?;
    }
    prove_writable(&portable.data).map_err(|error| portable_error(executable_dir, error))?;
    Ok(portable)
}

fn prove_writable(directory: &Path) -> io::Result<()> {
    let probe = directory.join(format!(".write-test-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&probe)?;
        file.write_all(b"zero-mod-manager")?;
        file.sync_all()
    })();
    let cleanup = fs::remove_file(&probe);
    result.and(cleanup)
}

fn portable_error(executable_dir: &Path, source: io::Error) -> io::Error {
    io::Error::new(
        source.kind(),
        format!(
            "Portable data mode was requested by {} but this folder is not writable. Move Zero Mod Manager to a writable folder or remove {}. {source}",
            executable_dir.join(PORTABLE_FLAG).display(),
            PORTABLE_FLAG
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn platform(root: &Path) -> StorageRoots {
        StorageRoots::platform(
            root.join("platform-data"),
            root.join("platform-cache"),
            root.join("platform-logs"),
            root.join("platform-mods"),
        )
    }

    #[test]
    fn portable_executable_uses_platform_data_without_the_marker() {
        let root = tempfile::tempdir().unwrap();
        let defaults = platform(root.path());
        assert_eq!(resolve(root.path(), defaults.clone()).unwrap(), defaults);
    }

    #[test]
    fn marker_selects_a_self_contained_writable_data_tree() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(PORTABLE_FLAG), []).unwrap();

        let roots = resolve(root.path(), platform(root.path())).unwrap();

        assert_eq!(roots.mode, "portable");
        assert_eq!(roots.data, root.path().join("data"));
        assert!(roots.cache.is_dir());
        assert!(roots.logs.is_dir());
        assert!(roots.mods.is_dir());
        assert!(!roots.data.join(".write-test").exists());
    }
}
