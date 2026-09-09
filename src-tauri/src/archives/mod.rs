use crate::error::{AppError, Result};
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::RwLock,
};
use uuid::Uuid;
use walkdir::WalkDir;

pub struct Staging {
    pub root: PathBuf,
    pub warnings: Vec<String>,
    /// Archive-relative paths that carry native code. Whether those matter is a
    /// question about the mod layout rather than about the archive, so the
    /// judgement is left to `crate::mods`.
    pub executables: Vec<String>,
}

fn unsafe_name(path: &Path) -> bool {
    path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

/// Interprets an archive member name as a relative path.
///
/// ZIP mandates `/` as the separator, but Windows packaging tools regularly
/// write `\`. A non-Windows host then reads `ue4ss\Mods\X\main.dll` as a single
/// long file name, the mod folder never materializes, and detection fails on an
/// archive that installs correctly on Windows. Both separators are therefore
/// treated as directory boundaries on every platform.
pub(crate) fn archive_relative(name: &str) -> Option<PathBuf> {
    // An absolute member name is never legitimate, and rebasing it silently
    // would hide an archive that tried to write outside its own tree.
    if name.starts_with('/') || name.starts_with('\\') {
        return None;
    }
    let mut path = PathBuf::new();
    for part in name.split(['/', '\\']) {
        match part {
            "" | "." => continue,
            ".." => return None,
            // A Windows drive prefix survives as an ordinary component on other
            // platforms, so it is rejected by hand rather than by `unsafe_name`.
            _ if part.contains(':') || part.contains('\0') => return None,
            _ => path.push(part),
        }
    }
    (!path.as_os_str().is_empty() && !unsafe_name(&path)).then_some(path)
}

pub(crate) fn suspicious(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("exe" | "bat" | "cmd" | "ps1" | "dll" | "sh" | "msi" | "scr" | "vbs")
    )
}

fn note_executable(executables: &mut Vec<String>, relative: &Path) {
    if suspicious(relative) {
        executables.push(relative.display().to_string().replace('\\', "/"));
    }
}

fn copy_tree(source: &Path, destination: &Path, executables: &mut Vec<String>) -> Result<()> {
    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(|e| AppError::Other(e.to_string()))?;
        let rel = entry
            .path()
            .strip_prefix(source)
            .map_err(|e| AppError::Other(e.to_string()))?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        if unsafe_name(rel) {
            return Err(AppError::UnsafeArchive(rel.display().to_string()));
        }
        if entry.file_type().is_symlink() {
            return Err(AppError::UnsafeArchive(format!(
                "symbolic link {}",
                rel.display()
            )));
        }
        let target = destination.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            note_executable(executables, rel);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn extract_zip(source: &Path, destination: &Path, executables: &mut Vec<String>) -> Result<()> {
    let file = fs::File::open(source)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let enclosed = archive_relative(entry.name())
            .ok_or_else(|| AppError::UnsafeArchive(entry.name().to_string()))?;
        #[cfg(unix)]
        if entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err(AppError::UnsafeArchive(format!(
                "symbolic link {}",
                entry.name()
            )));
        }
        let target = destination.join(&enclosed);
        // A directory entry may also be spelled with a trailing separator that
        // `archive_relative` has already dropped, so both forms are checked.
        if entry.is_dir() || entry.name().ends_with(['/', '\\']) {
            fs::create_dir_all(&target)?;
            continue;
        }
        note_executable(executables, &enclosed);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = fs::File::create(target)?;
        std::io::copy(&mut entry, &mut output)?;
        output.flush()?;
    }
    Ok(())
}

/// A path the user pointed at their own 7-Zip build, held for the life of the
/// process.
///
/// Extraction runs far below the command layer, in code that has no database
/// handle, so the stored setting is published here at start-up and whenever it
/// is saved rather than threaded through every caller.
static CONFIGURED_7Z: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Records the 7-Zip executable the user chose. An empty or missing path
/// clears the override and returns discovery to the automatic search.
pub fn set_seven_zip_path(path: Option<&str>) {
    let chosen = path
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_file());
    if let Ok(mut held) = CONFIGURED_7Z.write() {
        *held = chosen;
    }
}

/// The executable names a usable 7-Zip build ships under.
///
/// `7z` is the full build; `7za` and `7zr` are the standalone ones some users
/// have instead, and both read the `.7z` archives this manager cares about.
/// NanaZip installs the same command-line tool under its own name.
const SEVEN_ZIP_NAMES: [&str; 4] = ["7z", "7za", "7zr", "NanaZipC"];

fn seven_zip_binaries() -> impl Iterator<Item = String> {
    SEVEN_ZIP_NAMES.into_iter().map(|name| {
        if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_string()
        }
    })
}

/// Builds a command that runs without opening a console window.
///
/// This is a windowless application, so every child process it starts would
/// otherwise flash a console over the game or the manager. Extraction runs
/// while the user is watching an install, and the archive-tool lookup runs on
/// every refresh, so both go through here.
pub(crate) fn quiet_command(program: &Path) -> Command {
    // Only the Windows arm mutates it, and Windows is the only platform where
    // the console window this suppresses exists.
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW
        command.creation_flags(0x0800_0000);
    }
    command
}

/// Directories a Windows 7-Zip installation lands in.
///
/// The 7-Zip installer does not put itself on `PATH`, so searching `PATH`
/// alone reported the tool as missing on a machine that plainly had it. These
/// are the standard per-machine and per-user locations, and cost nothing but a
/// few `is_file` calls.
#[cfg(windows)]
fn windows_install_dirs() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for variable in [
        "ProgramFiles",
        "ProgramW6432",
        "ProgramFiles(x86)",
        "LOCALAPPDATA",
        "APPDATA",
    ] {
        if let Some(value) = std::env::var_os(variable) {
            let base = PathBuf::from(value);
            roots.push(base.join("7-Zip"));
            roots.push(base.join("NanaZip"));
            roots.push(base.join("Programs").join("7-Zip"));
            roots.push(base.join("Programs").join("NanaZip"));
        }
    }
    roots
}

/// The directory the 7-Zip installer recorded, for an installation somewhere
/// the standard locations do not cover.
///
/// This shells out, so it is consulted only after every cheaper candidate has
/// missed rather than on each lookup.
#[cfg(windows)]
fn registered_install_dirs() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for key in [
        r"HKLM\SOFTWARE\7-Zip",
        r"HKLM\SOFTWARE\WOW6432Node\7-Zip",
        r"HKCU\SOFTWARE\7-Zip",
    ] {
        let Ok(output) = quiet_command(Path::new("reg"))
            .args(["query", key, "/v", "Path"])
            .output()
        else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            // REG_SZ values print as `    Path    REG_SZ    C:\Program Files\7-Zip\`.
            if let Some((_, value)) = line.split_once("REG_SZ") {
                let value = value.trim();
                if !value.is_empty() {
                    roots.push(PathBuf::from(value));
                }
            }
        }
    }
    roots
}

#[cfg(not(windows))]
fn windows_install_dirs() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(not(windows))]
fn registered_install_dirs() -> Vec<PathBuf> {
    Vec::new()
}

/// Locates a usable 7-Zip command-line tool.
///
/// The user's own choice wins, then `PATH`, then the places an installer puts
/// it. Each candidate is confirmed to be a file before it is returned, so a
/// stale registry entry left by an uninstall does not shadow a working build,
/// and neither does a configured path pointing at a tool since removed.
fn resolve_7z(configured: Option<&Path>) -> Option<PathBuf> {
    if let Some(configured) = configured.filter(|path| path.is_file()) {
        return Some(configured.to_path_buf());
    }
    let on_path = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>());
    let first_in = |directories: Vec<PathBuf>| {
        directories.into_iter().find_map(|directory| {
            seven_zip_binaries()
                .map(|binary| directory.join(binary))
                .find(|candidate| candidate.is_file())
        })
    };
    first_in(on_path.collect())
        .or_else(|| first_in(windows_install_dirs()))
        // Consulted last, because unlike the rest it starts a process.
        .or_else(|| first_in(registered_install_dirs()))
}

pub(crate) fn find_7z() -> Option<PathBuf> {
    let configured = CONFIGURED_7Z.read().ok().and_then(|held| held.clone());
    resolve_7z(configured.as_deref())
}

/// Version banners already read, keyed by the executable they came from.
///
/// Settings asks for this on every refresh, and running the tool each time to
/// re-read a string that cannot have changed is a process start the user pays
/// for after every mod action.
static SEVEN_ZIP_VERSIONS: RwLock<Option<(PathBuf, Option<String>)>> = RwLock::new(None);

/// The banner a 7-Zip build prints when run with no arguments.
fn seven_zip_version(path: &Path) -> Option<String> {
    if let Some((known, version)) = SEVEN_ZIP_VERSIONS.read().ok().and_then(|held| held.clone()) {
        if known == path {
            return version;
        }
    }
    let version = quiet_command(path)
        .output()
        .ok()
        .and_then(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .find(|line| line.contains("7-Zip") || line.contains("NanaZip"))
                .map(|line| line.trim().to_string())
        })
        .filter(|version| !version.is_empty());
    if let Ok(mut held) = SEVEN_ZIP_VERSIONS.write() {
        *held = Some((path.to_path_buf(), version.clone()));
    }
    version
}

/// What the interface shows for the archive tool, so a missing 7-Zip can be
/// pointed at from Settings rather than only reported when an install fails.
pub fn seven_zip_info() -> crate::models::ToolInfo {
    let path = find_7z();
    let version = path.as_deref().and_then(seven_zip_version);
    crate::models::ToolInfo {
        found: path.is_some(),
        path: path.map(|path| path.display().to_string()),
        version,
    }
}

/// Rebuilds directories from member names that kept `\` as their separator.
/// `extract_zip` handles this while reading, but an external extractor writes
/// whatever the archive contained, so the staged tree is repaired afterwards.
fn split_backslash_names(root: &Path) -> Result<()> {
    loop {
        let offender = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .find(|entry| {
                entry.file_type().is_file() && entry.file_name().to_string_lossy().contains('\\')
            })
            .map(|entry| entry.path().to_path_buf());
        let Some(offender) = offender else {
            return Ok(());
        };
        let name = offender.file_name().unwrap_or_default().to_string_lossy();
        let rebuilt =
            archive_relative(&name).ok_or_else(|| AppError::UnsafeArchive(name.to_string()))?;
        let target = offender.parent().unwrap_or(root).join(rebuilt);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&offender, &target)?;
    }
}

fn extract_7z(source: &Path, destination: &Path, executables: &mut Vec<String>) -> Result<()> {
    let seven = find_7z().ok_or(AppError::SevenZipNotFound)?;
    let listing = quiet_command(&seven)
        .args(["l", "-slt", "--"])
        .arg(source)
        .output()?;
    if !listing.status.success() {
        // RAR support is a separate, non-free codec that many 7-Zip builds omit,
        // and the failure otherwise looks like a corrupt download.
        return Err(AppError::Other(format!(
            "7z could not read this archive. {}",
            if source
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rar"))
            {
                "RAR needs a 7-Zip build with the RAR codec; extract it yourself and install the \
                 folder instead."
            } else {
                "It may be corrupt or use an unsupported compression method."
            }
        )));
    }
    let text = String::from_utf8_lossy(&listing.stdout);
    let mut entries = false;
    for line in text.lines() {
        if line.starts_with("----------") {
            entries = true;
            continue;
        }
        if !entries {
            continue;
        }
        if let Some(name) = line.strip_prefix("Path = ") {
            let path =
                archive_relative(name).ok_or_else(|| AppError::UnsafeArchive(name.to_string()))?;
            note_executable(executables, &path);
        }
    }
    let output = quiet_command(&seven)
        .args(["x", "-y", "-snl", "-snh"])
        .arg(format!("-o{}", destination.display()))
        .arg("--")
        .arg(source)
        .output()?;
    if !output.status.success() {
        return Err(AppError::Other(format!(
            "7z extraction failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    for entry in WalkDir::new(destination).follow_links(false) {
        let entry = entry.map_err(|e| AppError::Other(e.to_string()))?;
        if entry.file_type().is_symlink() {
            return Err(AppError::UnsafeArchive(
                "archive created a symbolic link".into(),
            ));
        }
    }
    split_backslash_names(destination)
}

pub fn stage(source: &Path, cache: &Path) -> Result<Staging> {
    // Explorer and archive utilities can materialize a dropped item a fraction
    // after WebView2 reports its path. Give that Windows shell handoff a small,
    // bounded chance to finish before treating the path as gone.
    #[cfg(target_os = "windows")]
    for _ in 0..4 {
        if source.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(75));
    }
    if !source.exists() {
        return Err(AppError::Other(
            "The selected source no longer exists. If it was dragged from inside 7-Zip, WinRAR, or another archive window, extract it first or drop the archive itself.".into(),
        ));
    }
    let root = cache.join("staging").join(Uuid::new_v4().to_string());
    fs::create_dir_all(&root)?;
    let mut executables = Vec::new();
    let result = if source.is_dir() {
        copy_tree(source, &root, &mut executables)
    } else {
        match source
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref()
        {
            Some("zip") => extract_zip(source, &root, &mut executables),
            Some("7z" | "rar") => extract_7z(source, &root, &mut executables),
            Some("pak" | "utoc" | "ucas") => {
                let stem = source.file_stem().unwrap_or_default();
                for ext in ["pak", "utoc", "ucas"] {
                    let candidate = source.with_extension(ext);
                    if candidate.is_file() {
                        fs::copy(
                            &candidate,
                            root.join(format!("{}.{}", stem.to_string_lossy(), ext)),
                        )?;
                    }
                }
                Ok(())
            }
            _ => Err(AppError::ModNotRecognized),
        }
    };
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&root);
        return Err(error);
    }
    Ok(Staging {
        root,
        warnings: Vec::new(),
        executables,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    fn zip_with(path: &Path, entries: &[(&str, &[u8])]) {
        let mut writer = zip::ZipWriter::new(fs::File::create(path).unwrap());
        for (name, body) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(body).unwrap();
        }
        writer.finish().unwrap();
    }

    /// 7-Zip's Windows installer does not put itself on `PATH`, so a machine
    /// that plainly had the tool was told archive support was unavailable. The
    /// configured path is the user's way out of any gap the search still has.
    #[test]
    fn a_configured_seven_zip_path_wins_over_the_search() {
        let d = tempdir().unwrap();
        let chosen = d.path().join(if cfg!(windows) { "7z.exe" } else { "7z" });
        fs::write(&chosen, b"").unwrap();
        assert_eq!(resolve_7z(Some(&chosen)), Some(chosen));
    }

    /// A tool the user pointed at and later uninstalled must not shadow one
    /// that is still there.
    #[test]
    fn a_configured_path_that_is_gone_falls_back_to_the_search() {
        let d = tempdir().unwrap();
        let missing = d.path().join("removed-7z");
        assert_eq!(resolve_7z(Some(&missing)), resolve_7z(None));
    }

    /// `set_seven_zip_path` only accepts a path that exists, so a stale
    /// setting never becomes the answer.
    #[test]
    fn an_empty_or_missing_configured_path_is_ignored() {
        assert_eq!(resolve_7z(Some(Path::new(""))), resolve_7z(None));
    }

    #[test]
    fn rejects_zip_traversal() {
        let d = tempdir().unwrap();
        let path = d.path().join("bad.zip");
        zip_with(&path, &[("../../evil.pak", b"malicious")]);
        assert!(matches!(
            stage(&path, d.path()),
            Err(AppError::UnsafeArchive(_))
        ));
    }

    #[test]
    fn rejects_traversal_spelled_with_backslashes() {
        let d = tempdir().unwrap();
        let path = d.path().join("bad.zip");
        zip_with(&path, &[("..\\..\\evil.pak", b"malicious")]);
        assert!(matches!(
            stage(&path, d.path()),
            Err(AppError::UnsafeArchive(_))
        ));
    }

    #[test]
    fn absolute_path_is_unsafe() {
        assert!(unsafe_name(Path::new("/tmp/evil")));
        assert!(unsafe_name(Path::new("../evil")));
        assert!(!unsafe_name(Path::new("safe/mod.pak")));
    }

    #[test]
    fn windows_separators_become_directories() {
        let d = tempdir().unwrap();
        let path = d.path().join("windows.zip");
        zip_with(
            &path,
            &[
                ("ue4ss\\Mods\\ZCUnlocked\\enabled.txt", b"1"),
                ("ue4ss\\Mods\\ZCUnlocked\\dlls\\main.dll", b"MZ"),
            ],
        );
        let staged = stage(&path, d.path()).unwrap();
        assert!(staged
            .root
            .join("ue4ss/Mods/ZCUnlocked/dlls/main.dll")
            .is_file());
        assert_eq!(
            staged.executables,
            vec!["ue4ss/Mods/ZCUnlocked/dlls/main.dll"]
        );
    }

    #[test]
    fn drive_letters_are_rejected() {
        assert!(archive_relative("C:\\Windows\\evil.dll").is_none());
        assert!(archive_relative("mods/Good_P.pak").is_some());
    }

    #[test]
    fn directory_symlink_is_rejected() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let s = tempdir().unwrap();
            let c = tempdir().unwrap();
            fs::write(s.path().join("real"), b"x").unwrap();
            symlink(s.path().join("real"), s.path().join("link")).unwrap();
            assert!(stage(s.path(), c.path()).is_err());
        }
    }

    #[test]
    fn rejects_absolute_zip_path() {
        let d = tempdir().unwrap();
        let path = d.path().join("absolute.zip");
        zip_with(&path, &[("/tmp/evil.pak", b"malicious")]);
        assert!(matches!(
            stage(&path, d.path()),
            Err(AppError::UnsafeArchive(_))
        ));
    }

    #[test]
    fn extracts_nested_7z_when_tool_is_available() {
        let Some(seven) = find_7z() else { return };
        let d = tempdir().unwrap();
        let input = d.path().join("input/SomeMod");
        fs::create_dir_all(&input).unwrap();
        fs::write(input.join("Nested_P.pak"), b"pak").unwrap();
        let archive = d.path().join("mod.7z");
        let status = Command::new(seven)
            .current_dir(d.path().join("input"))
            .args(["a", "-y"])
            .arg(&archive)
            .arg("SomeMod")
            .status()
            .unwrap();
        assert!(status.success());
        let staged = stage(&archive, d.path()).unwrap();
        assert!(staged.root.join("SomeMod/Nested_P.pak").is_file());
    }

    /// Only reachable where `\` is an ordinary filename character. Windows
    /// cannot hold such a name in the first place, so the repair is a no-op
    /// there and the scenario cannot be built.
    #[test]
    #[cfg(unix)]
    fn repairs_names_an_external_extractor_left_flat() {
        let d = tempdir().unwrap();
        let root = d.path().join("staged");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("ue4ss\\Mods\\Thing\\enabled.txt"), b"1").unwrap();
        split_backslash_names(&root).unwrap();
        assert!(root.join("ue4ss/Mods/Thing/enabled.txt").is_file());
    }
}
