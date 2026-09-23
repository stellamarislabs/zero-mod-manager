//! A factory reset never uninstalls game payloads. It retires only trusted,
//! application-owned storage roots, before SQLite or deployment recovery opens.
//!
//! The immutable sibling journal contains no paths to execute. Targets are
//! re-derived from the platform/portable roots on every startup. Entire scoped
//! trees are renamed, not recursively deleted, so interrupted resets can resume
//! and recovery copies remain available without following directory links.
use crate::{
    error::{AppError, Result},
    storage::StorageRoots,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

const APP_ID: &str = "app.zeromodmanager.desktop";
const JOURNAL_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPreview {
    pub directories_to_reset: Vec<String>,
    pub recovery_directories: Vec<String>,
    /// Informational only: this path is never read or mutated by the reset.
    pub external_library_retained: Option<String>,
}

#[derive(Debug)]
pub struct ResetResult {
    pub recovery_directories: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u32,
    id: String,
    roots_digest: String,
    present: Vec<bool>,
    external_library_retained: Option<String>,
}

struct Layout {
    roots: Vec<PathBuf>,
    /// Resolver-created directories that may legitimately reappear between
    /// a completed rename and resuming after a crash.
    scaffold: Vec<PathBuf>,
    marker: PathBuf,
    digest: String,
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Other(format!(
        "Factory reset was not completed: {}",
        message.into()
    ))
}

fn path_key(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value
    }
}

fn contains(parent: &Path, child: &Path) -> bool {
    let parent = path_key(parent);
    let child = path_key(child);
    child == parent || child.starts_with(&(parent + "/"))
}

fn is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// Validate lexically and inspect every existing ancestor without following it.
fn check_path(path: &Path, directory: bool) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(invalid(
            "a storage path is not an absolute, normalized path",
        ));
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if is_link(&metadata) {
                    return Err(invalid(format!(
                        "{} is a link or reparse point; no linked data was touched",
                        current.display()
                    )));
                }
                if (directory || current != path) && !metadata.is_dir() {
                    return Err(invalid(format!("{} is not a directory", current.display())));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn platform_scoped(path: &Path) -> bool {
    let leaf = path.file_name().and_then(|part| part.to_str());
    leaf == Some(APP_ID)
        || (matches!(leaf, Some("mods" | "cache" | "logs"))
            && path
                .parent()
                .and_then(Path::file_name)
                .and_then(|part| part.to_str())
                == Some(APP_ID))
}

fn layout(storage: &StorageRoots) -> Result<Layout> {
    derive_layout(storage, true)
}

fn derive_layout(storage: &StorageRoots, inspect: bool) -> Result<Layout> {
    let raw = [&storage.data, &storage.cache, &storage.logs, &storage.mods];
    match storage.mode {
        "platform" if raw.iter().all(|path| platform_scoped(path)) => {}
        "portable"
            if storage.data.file_name().and_then(|name| name.to_str()) == Some("data")
                && storage.cache == storage.data.join("cache")
                && storage.logs == storage.data.join("logs")
                && storage.mods == storage.data.join("mods") => {}
        _ => {
            return Err(invalid(
                "the storage roots are not recognized app-owned directories",
            ))
        }
    }
    for path in raw {
        // Never accept a drive/filesystem root or its direct child as an app root.
        if path.parent().and_then(Path::parent).is_none() {
            return Err(invalid("a storage path is too broad to reset safely"));
        }
        if inspect {
            check_path(path, true)?;
        }
    }
    let scaffold: Vec<_> = raw.into_iter().cloned().collect();
    let mut candidates = scaffold.clone();
    candidates.sort_by_key(|path| (path.components().count(), path_key(path)));
    let mut roots: Vec<PathBuf> = Vec::new();
    for path in candidates {
        if !roots.iter().any(|parent| contains(parent, &path)) {
            roots.push(path);
        }
    }
    let anchor = roots
        .iter()
        .find(|root| contains(root, &storage.data))
        .ok_or_else(|| invalid("the data root could not be scoped"))?;
    let parent = anchor
        .parent()
        .ok_or_else(|| invalid("the data root has no parent"))?;
    let leaf = anchor
        .file_name()
        .ok_or_else(|| invalid("the data root has no name"))?;
    let marker = parent.join(format!(".{}.factory-reset.json", leaf.to_string_lossy()));
    if roots.iter().any(|root| contains(root, &marker)) {
        return Err(invalid("the reset journal would be inside a reset target"));
    }
    if inspect {
        check_path(&marker, false)?;
    }
    let mut digest = Sha256::new();
    digest.update(storage.mode.as_bytes());
    for root in &roots {
        digest.update([0]);
        digest.update(path_key(root).as_bytes());
    }
    Ok(Layout {
        roots,
        scaffold,
        marker,
        digest: hex::encode(digest.finalize()),
    })
}

fn recovery_path(root: &Path, id: &str) -> Result<PathBuf> {
    let name = root
        .file_name()
        .ok_or_else(|| invalid("a reset target has no name"))?;
    Ok(root.with_file_name(format!("{}.reset-recovery-{id}", name.to_string_lossy())))
}

fn read_journal(plan: &Layout) -> Result<Option<Journal>> {
    let metadata = match fs::symlink_metadata(&plan.marker) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if is_link(&metadata) || !metadata.is_file() || metadata.len() > 16 * 1024 {
        return Err(invalid("the reset journal is unsafe or too large"));
    }
    let journal: Journal = serde_json::from_slice(&fs::read(&plan.marker)?)?;
    if journal.version != JOURNAL_VERSION
        || journal.roots_digest != plan.digest
        || journal.present.len() != plan.roots.len()
        || uuid::Uuid::parse_str(&journal.id)
            .map(|id| id.to_string() != journal.id)
            .unwrap_or(true)
    {
        return Err(invalid(
            "the pending reset does not match this installation's storage",
        ));
    }
    Ok(Some(journal))
}

fn new_journal(plan: &Layout, library: &Path) -> Journal {
    Journal {
        version: JOURNAL_VERSION,
        id: uuid::Uuid::new_v4().to_string(),
        roots_digest: plan.digest.clone(),
        present: plan.roots.iter().map(|root| root.exists()).collect(),
        external_library_retained: (!plan.roots.iter().any(|root| contains(root, library)))
            .then(|| library.display().to_string()),
    }
}

fn describe(plan: &Layout, journal: &Journal) -> Result<ResetPreview> {
    let selected: Vec<_> = plan
        .roots
        .iter()
        .zip(&journal.present)
        .filter_map(|(root, present)| present.then_some(root))
        .collect();
    Ok(ResetPreview {
        directories_to_reset: selected
            .iter()
            .map(|root| root.display().to_string())
            .collect(),
        recovery_directories: selected
            .iter()
            .map(|root| Ok(recovery_path(root, &journal.id)?.display().to_string()))
            .collect::<Result<_>>()?,
        external_library_retained: journal.external_library_retained.clone(),
    })
}

pub fn preview(storage: &StorageRoots, active_library: &Path) -> Result<ResetPreview> {
    let plan = layout(storage)?;
    let journal = read_journal(&plan)?.unwrap_or_else(|| new_journal(&plan, active_library));
    describe(&plan, &journal)
}

pub fn is_pending(storage: &StorageRoots) -> Result<bool> {
    let plan = derive_layout(storage, false)?;
    if !marker_exists(&plan)? {
        return Ok(false);
    }
    Ok(read_journal(&layout(storage)?)?.is_some())
}

fn marker_exists(plan: &Layout) -> Result<bool> {
    match fs::symlink_metadata(&plan.marker) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Caller must reject in-flight operations and temporary-profile recovery,
/// then exit the application immediately after this succeeds. No reset target
/// is mutated here, while open database/webview handles may still own it.
pub fn schedule(storage: &StorageRoots, active_library: &Path) -> Result<ResetPreview> {
    let plan = layout(storage)?;
    if read_journal(&plan)?.is_some() {
        return Err(invalid(
            "a reset is already pending; close and reopen the manager",
        ));
    }
    let journal = new_journal(&plan, active_library);
    let description = describe(&plan, &journal)?;
    for root in &plan.roots {
        let destination = recovery_path(root, &journal.id)?;
        check_path(&destination, true)?;
        if destination.exists() {
            return Err(invalid("a recovery directory already exists"));
        }
    }
    let parent = plan
        .marker
        .parent()
        .ok_or_else(|| invalid("missing journal parent"))?;
    fs::create_dir_all(parent)?;
    check_path(parent, true)?;
    let temporary = parent.join(format!(".factory-reset-{}.pending", journal.id));
    let result = (|| -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&serde_json::to_vec_pretty(&journal)?)?;
        file.sync_all()?;
        drop(file);
        // The caller serializes schedule with all other manager mutations.
        // Windows rename refuses an existing destination, including on FAT/
        // exFAT portable drives where hard links are unavailable. Unix hard
        // link publishes atomically without rename's overwrite semantics.
        #[cfg(windows)]
        fs::rename(&temporary, &plan.marker)?;
        #[cfg(not(windows))]
        fs::hard_link(&temporary, &plan.marker)?;
        Ok(())
    })();
    let _ = fs::remove_file(&temporary);
    result?;
    Ok(description)
}

/// Remove only empty, known resolver directories (never a recursive delete).
/// This is necessary because portable storage resolution recreates its empty
/// tree before startup can notice a journal left immediately after a rename.
fn remove_empty_scaffold(root: &Path, plan: &Layout) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    check_path(root, true)?;
    let mut directories = vec![root.to_path_buf()];
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            let known = plan.scaffold.iter().any(|target| contains(&path, target));
            if is_link(&metadata) || !metadata.is_dir() || !known {
                return Err(invalid(format!(
                    "new or unrecognized data exists at {}; both it and the recovery copy were preserved",
                    path.display()
                )));
            }
            directories.push(path.clone());
            pending.push(path);
        }
    }
    directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for directory in directories {
        // Fails safely if a new file appeared after the inspection.
        fs::remove_dir(directory)?;
    }
    Ok(())
}

/// Must run before opening SQLite, emitting app logs, or recovering deployment
/// journals. On an error startup must stop, not initialize partial new state.
pub fn apply_pending(storage: &StorageRoots) -> Result<Option<ResetResult>> {
    // An ordinary startup must not impose factory-reset path restrictions on
    // installations using a linked home/AppData directory. Validate all link
    // ancestors strictly only if a reset marker actually exists.
    let candidate = derive_layout(storage, false)?;
    if !marker_exists(&candidate)? {
        return Ok(None);
    }
    let plan = layout(storage)?;
    let Some(journal) = read_journal(&plan)? else {
        return Ok(None);
    };
    let description = describe(&plan, &journal)?;
    for (root, was_present) in plan.roots.iter().zip(&journal.present) {
        let destination = recovery_path(root, &journal.id)?;
        check_path(root, true)?;
        check_path(&destination, true)?;
        if !was_present {
            // Never sweep data created since the request into a reset.
            remove_empty_scaffold(root, &plan)?;
            continue;
        }
        match (root.exists(), destination.exists()) {
            (true, false) => fs::rename(root, &destination)?,
            (false, true) => {} // Rename completed before a previous interruption.
            (true, true) => remove_empty_scaffold(root, &plan)?,
            (false, false) => {
                return Err(invalid(format!(
                    "both {} and its recovery directory are missing; no further state was changed",
                    root.display()
                )))
            }
        }
    }
    fs::remove_file(&plan.marker)?;
    Ok(Some(ResetResult {
        recovery_directories: description.recovery_directories,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn platform(root: &Path) -> StorageRoots {
        StorageRoots::platform(
            root.join("roaming").join(APP_ID),
            root.join("local").join(APP_ID),
            root.join("roaming").join(APP_ID).join("logs"),
            root.join("local").join(APP_ID).join("mods"),
        )
    }

    fn initialize(roots: &StorageRoots) {
        for path in [&roots.data, &roots.cache, &roots.logs, &roots.mods] {
            fs::create_dir_all(path).unwrap();
        }
        fs::write(roots.data.join("manager.sqlite3"), b"old database").unwrap();
        fs::write(roots.mods.join("owned-copy.pak"), b"mod bytes").unwrap();
    }

    #[test]
    fn resets_only_deduplicated_app_roots_with_recovery_copies() {
        let fixture = tempfile::tempdir().unwrap();
        let roots = platform(fixture.path());
        initialize(&roots);
        let game = fixture.path().join("game");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("installed.pak"), b"game mod").unwrap();
        let save = fixture.path().join("campaign.sav");
        fs::write(&save, b"save bytes").unwrap();
        let scheduled = schedule(&roots, &roots.mods).unwrap();
        assert_eq!(scheduled.directories_to_reset.len(), 2);
        assert!(roots.data.join("manager.sqlite3").exists());
        assert!(is_pending(&roots).unwrap());
        let completed = apply_pending(&roots).unwrap().unwrap();
        assert_eq!(
            completed.recovery_directories,
            scheduled.recovery_directories
        );
        assert!(!roots.data.exists());
        assert!(!roots.mods.exists());
        assert!(scheduled
            .recovery_directories
            .iter()
            .all(|path| Path::new(path).is_dir()));
        assert_eq!(fs::read(game.join("installed.pak")).unwrap(), b"game mod");
        assert_eq!(fs::read(save).unwrap(), b"save bytes");
        assert!(!is_pending(&roots).unwrap());
        assert!(apply_pending(&roots).unwrap().is_none());
    }

    #[test]
    fn external_library_and_user_archives_are_retained_explicitly() {
        let fixture = tempfile::tempdir().unwrap();
        let roots = platform(fixture.path());
        initialize(&roots);
        let external = fixture.path().join("My library and downloads");
        fs::create_dir_all(&external).unwrap();
        fs::write(external.join("user-archive.zip"), b"archive").unwrap();
        let description = schedule(&roots, &external).unwrap();
        assert_eq!(
            description.external_library_retained,
            Some(external.display().to_string())
        );
        apply_pending(&roots).unwrap();
        assert_eq!(
            fs::read(external.join("user-archive.zip")).unwrap(),
            b"archive"
        );
    }

    #[test]
    fn resumes_partial_moves_and_recreated_empty_portable_scaffold() {
        let fixture = tempfile::tempdir().unwrap();
        let roots = platform(fixture.path());
        initialize(&roots);
        schedule(&roots, &roots.mods).unwrap();
        let plan = layout(&roots).unwrap();
        let journal = read_journal(&plan).unwrap().unwrap();
        fs::rename(
            &plan.roots[0],
            recovery_path(&plan.roots[0], &journal.id).unwrap(),
        )
        .unwrap();
        apply_pending(&roots).unwrap();
        assert!(plan.roots.iter().all(|root| !root.exists()));

        let executable = fixture.path().join("portable");
        fs::create_dir_all(&executable).unwrap();
        fs::write(executable.join(crate::storage::PORTABLE_FLAG), []).unwrap();
        let portable = crate::storage::resolve(&executable, roots).unwrap();
        initialize(&portable);
        schedule(&portable, &portable.mods).unwrap();
        let plan = layout(&portable).unwrap();
        let journal = read_journal(&plan).unwrap().unwrap();
        fs::rename(
            &portable.data,
            recovery_path(&portable.data, &journal.id).unwrap(),
        )
        .unwrap();
        // A later startup resolver recreates exactly cache/logs/mods.
        let portable = crate::storage::resolve(&executable, portable).unwrap();
        assert!(apply_pending(&portable).unwrap().is_some());
        assert!(!portable.data.exists());
    }

    #[test]
    fn ambiguous_new_data_is_preserved_until_recovery_is_resolved() {
        let fixture = tempfile::tempdir().unwrap();
        let roots = platform(fixture.path());
        initialize(&roots);
        schedule(&roots, &roots.mods).unwrap();
        let plan = layout(&roots).unwrap();
        let journal = read_journal(&plan).unwrap().unwrap();
        let source = &plan.roots[0];
        let backup = recovery_path(source, &journal.id).unwrap();
        fs::rename(source, &backup).unwrap();
        fs::create_dir_all(source).unwrap();
        fs::write(source.join("new-user-data.txt"), b"new content").unwrap();
        assert!(apply_pending(&roots).is_err());
        assert_eq!(
            fs::read(source.join("new-user-data.txt")).unwrap(),
            b"new content"
        );
        assert!(backup.is_dir());
        assert!(is_pending(&roots).unwrap());
        fs::remove_file(source.join("new-user-data.txt")).unwrap();
        assert!(apply_pending(&roots).unwrap().is_some());
    }

    #[test]
    fn journal_cannot_inject_targets_or_retarget_a_different_installation() {
        let fixture = tempfile::tempdir().unwrap();
        let roots = platform(fixture.path());
        initialize(&roots);
        schedule(&roots, &roots.mods).unwrap();
        let plan = layout(&roots).unwrap();
        let mut marker: serde_json::Value =
            serde_json::from_slice(&fs::read(&plan.marker).unwrap()).unwrap();
        marker["roots_digest"] = "wrong installation".into();
        fs::write(&plan.marker, serde_json::to_vec(&marker).unwrap()).unwrap();
        assert!(apply_pending(&roots).is_err());
        assert!(roots.data.join("manager.sqlite3").is_file());
        marker["roots_digest"] = plan.digest.into();
        marker["targets"] = serde_json::json!([fixture.path()]);
        fs::write(&plan.marker, serde_json::to_vec(&marker).unwrap()).unwrap();
        assert!(apply_pending(&roots).is_err());
        assert!(roots.mods.join("owned-copy.pak").is_file());
    }

    #[test]
    fn rejects_broad_roots_and_second_pending_reset() {
        let fixture = tempfile::tempdir().unwrap();
        let mut roots = platform(fixture.path());
        initialize(&roots);
        schedule(&roots, &roots.mods).unwrap();
        assert!(schedule(&roots, &roots.mods).is_err());
        roots.data = fixture.path().to_path_buf();
        assert!(preview(&roots, &roots.mods).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn refuses_linked_root_or_ancestor_but_does_not_follow_internal_links() {
        use std::os::unix::fs::symlink;
        let fixture = tempfile::tempdir().unwrap();
        let roots = platform(fixture.path());
        initialize(&roots);
        let game = fixture.path().join("game");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("mod.pak"), b"untouched").unwrap();
        symlink(&game, roots.data.join("game-link")).unwrap();
        schedule(&roots, &roots.mods).unwrap();
        apply_pending(&roots).unwrap();
        assert_eq!(fs::read(game.join("mod.pak")).unwrap(), b"untouched");
        fs::create_dir_all(roots.data.parent().unwrap()).unwrap();
        symlink(&game, &roots.data).unwrap();
        assert!(preview(&roots, &roots.mods).is_err());
        assert!(apply_pending(&roots).unwrap().is_none());
    }

    #[cfg(windows)]
    #[test]
    fn windows_reparse_targets_are_never_followed_or_renamed() {
        use std::{
            os::windows::process::CommandExt,
            process::{Command, Stdio},
        };
        fn junction(link: &Path, target: &Path) {
            let result = Command::new("cmd.exe")
                .args(["/d", "/c", "mklink", "/J"])
                .arg(link)
                .arg(target)
                .creation_flags(0x08000000)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
            assert!(
                result.success(),
                "could not create test-only directory junction"
            );
        }
        let fixture = tempfile::tempdir().unwrap();
        let roots = platform(fixture.path());
        initialize(&roots);
        let game = fixture.path().join("game");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("mod.pak"), b"untouched").unwrap();
        junction(&roots.data.join("game-link"), &game);
        schedule(&roots, &roots.mods).unwrap();
        apply_pending(&roots).unwrap();
        assert_eq!(fs::read(game.join("mod.pak")).unwrap(), b"untouched");
        junction(&roots.data, &game);
        assert!(preview(&roots, &roots.mods).is_err());
        // Ordinary startup must not fail merely because a user redirects a
        // storage root: only destructive reset requests need this rejection.
        assert!(apply_pending(&roots).unwrap().is_none());
        fs::remove_dir(&roots.data).unwrap();

        let alias = fixture.path().join("redirected-parent");
        junction(&alias, roots.cache.parent().unwrap());
        let mut redirected = roots.clone();
        redirected.cache = alias.join(APP_ID);
        assert!(preview(&redirected, &roots.mods).is_err());
        fs::remove_dir(alias).unwrap();
        assert_eq!(fs::read(game.join("mod.pak")).unwrap(), b"untouched");
    }
}
