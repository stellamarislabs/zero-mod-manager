//! Write-ahead file snapshots plus a SQLite snapshot for package operations.
//! Payload writes enroll their exact targets before mutation; saves are never
//! scanned. A failed/ interrupted operation restores its pre-operation state.
use crate::{
    deployment,
    error::{AppError, Result},
};
use rusqlite::{backup::Backup, Connection};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    time::Duration,
};

thread_local! { static ACTIVE: RefCell<Option<Journal>> = const { RefCell::new(None) }; }
const DIR: &str = "package-operation";
static OPERATION: std::sync::Mutex<()> = std::sync::Mutex::new(());
struct ResetActive;
impl Drop for ResetActive {
    fn drop(&mut self) {
        ACTIVE.with(|cell| {
            cell.borrow_mut().take();
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    path: PathBuf,
    tree: bool,
    existed: bool,
    backup: String,
    digest: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Journal {
    version: u32,
    root: PathBuf,
    library: PathBuf,
    roots: Vec<PathBuf>,
    committed: bool,
    database_digest: String,
    entries: Vec<Entry>,
}

fn digest(path: &Path, tree: bool) -> Result<String> {
    if !tree {
        return deployment::sha256(path);
    }
    use sha2::{Digest, Sha256};
    let mut files = Vec::new();
    for item in walkdir::WalkDir::new(path).follow_links(false) {
        let item = item.map_err(|e| error(&e.to_string()))?;
        safe(item.path(), &[path.parent().unwrap().into()])?;
        if item.file_type().is_file() {
            files.push((
                item.path().strip_prefix(path).unwrap().to_path_buf(),
                deployment::sha256(item.path())?,
            ));
        }
    }
    files.sort();
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&files)?)))
}

fn error(message: &str) -> AppError {
    AppError::Other(message.into())
}
fn safe(path: &Path, roots: &[PathBuf]) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
        || !roots
            .iter()
            .any(|root| path.starts_with(root) && path != root)
    {
        return Err(error(
            "Package transaction target is outside its allowed folders.",
        ));
    }
    for parent in path.ancestors() {
        if let Ok(metadata) = fs::symlink_metadata(parent) {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err(error("Linked transaction paths are not supported."));
                }
            }
            if metadata.file_type().is_symlink() {
                return Err(error("Linked transaction paths are not supported."));
            }
        }
    }
    Ok(())
}
fn copy_file(source: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, target)?;
    fs::OpenOptions::new()
        .write(true)
        .open(target)?
        .sync_all()?;
    if deployment::sha256(source)? != deployment::sha256(target)? {
        return Err(error("Transaction backup verification failed."));
    }
    Ok(())
}
fn copy_tree(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)?;
    for item in walkdir::WalkDir::new(source).follow_links(false) {
        let item = item.map_err(|e| error(&e.to_string()))?;
        safe(item.path(), &[source.parent().unwrap().to_path_buf()])?;
        let to = target.join(
            item.path()
                .strip_prefix(source)
                .map_err(|e| error(&e.to_string()))?,
        );
        if item.file_type().is_dir() {
            fs::create_dir_all(to)?;
        } else if item.file_type().is_file() {
            copy_file(item.path(), &to)?;
        } else {
            return Err(error("Unsupported transaction file type."));
        }
    }
    Ok(())
}
impl Journal {
    fn save(&self) -> Result<()> {
        let temp = self.root.join("journal.next");
        let mut file = fs::File::create(&temp)?;
        file.write_all(&serde_json::to_vec(self)?)?;
        file.sync_all()?;
        fs::rename(temp, self.root.join("journal.json"))?;
        Ok(())
    }
    fn protect(&mut self, path: &Path, tree: bool) -> Result<()> {
        safe(path, &self.roots)?;
        if tree {
            self.safe_tree(path)?;
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.path == path || (entry.tree && path.starts_with(&entry.path)))
        {
            return Ok(());
        }
        // Enrolling a parent after a child would snapshot already-mutated data.
        if tree
            && self
                .entries
                .iter()
                .any(|entry| entry.path.starts_with(path))
        {
            return Err(error("Transaction snapshot order is invalid."));
        }
        let backup = format!("entry-{}", self.entries.len());
        let existed = path.exists();
        if existed {
            if tree {
                copy_tree(path, &self.root.join(&backup))?;
            } else {
                copy_file(path, &self.root.join(&backup))?;
            }
        }
        let digest = if existed {
            Some(digest(&self.root.join(&backup), tree)?)
        } else {
            None
        };
        self.entries.push(Entry {
            path: path.into(),
            tree,
            existed,
            backup,
            digest,
        });
        self.save() // Durable record exists before caller may mutate the target.
    }
    fn safe_tree(&self, path: &Path) -> Result<()> {
        let name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
        if path.parent() != Some(self.library.as_path())
            || uuid::Uuid::parse_str(name.strip_prefix(".replacing-").unwrap_or(name)).is_err()
        {
            return Err(error(
                "Only individual managed mod folders may be restored.",
            ));
        }
        Ok(())
    }
}
pub fn protect_file(path: &Path) -> Result<()> {
    ACTIVE.with(|cell| match cell.borrow_mut().as_mut() {
        Some(j) => j.protect(path, false),
        None => Ok(()),
    })
}
pub fn protect_tree(path: &Path) -> Result<()> {
    ACTIVE.with(|cell| match cell.borrow_mut().as_mut() {
        Some(j) => j.protect(path, true),
        None => Ok(()),
    })
}

pub fn check_available(database: &Path) -> Result<()> {
    if !ACTIVE.with(|cell| cell.borrow().is_some())
        && database
            .parent()
            .is_some_and(|p| p.join(DIR).join("journal.json").exists())
    {
        return Err(error("A package operation or recovery is pending. Wait for it to finish, or restart the manager to recover it."));
    }
    Ok(())
}
fn rollback(conn: &mut Connection, journal: &Journal) -> Result<()> {
    deployment::ensure_game_stopped()?;
    if deployment::sha256(&journal.root.join("database.sqlite3"))? != journal.database_digest {
        return Err(error("Database recovery backup is damaged."));
    }
    for entry in &journal.entries {
        safe(&entry.path, &journal.roots)?;
        if entry.tree {
            journal.safe_tree(&entry.path)?;
        }
        if entry.backup.contains(['/', '\\']) || !entry.backup.starts_with("entry-") {
            return Err(error("Invalid recovery backup name."));
        }
        if entry.existed && !journal.root.join(&entry.backup).exists() {
            return Err(error(
                "A recovery backup is missing; nothing further was restored.",
            ));
        }
        if entry.existed
            && entry.digest.as_ref()
                != Some(&digest(&journal.root.join(&entry.backup), entry.tree)?)
        {
            return Err(error(
                "File recovery backup is damaged; no recovery writes were started.",
            ));
        }
    }
    for entry in journal.entries.iter().rev() {
        let backup = journal.root.join(&entry.backup);
        // Preserve the interrupted state too, so external edits made after a
        // crash are recoverable rather than silently lost.
        let interrupted = journal.root.join(format!("interrupted-{}", entry.backup));
        if entry.path.exists() && !interrupted.exists() {
            if entry.tree {
                copy_tree(&entry.path, &interrupted)?;
            } else {
                copy_file(&entry.path, &interrupted)?;
            }
        }
        if entry.tree {
            if entry.path.exists() {
                fs::remove_dir_all(&entry.path)?;
            }
            if entry.existed {
                copy_tree(&backup, &entry.path)?;
            }
        } else if entry.existed {
            copy_file(&backup, &entry.path)?;
        } else if entry.path.exists() {
            fs::remove_file(&entry.path)?;
        }
    }
    let saved = Connection::open(journal.root.join("database.sqlite3"))?;
    Backup::new(&saved, conn)?.run_to_completion(64, Duration::from_millis(10), None)?;
    Ok(())
}
fn archive(data: &Path, journal: &Journal) -> Result<()> {
    let history = data.join("package-recovery");
    fs::create_dir_all(&history)?;
    fs::rename(
        &journal.root,
        history.join(uuid::Uuid::new_v4().to_string()),
    )?;
    Ok(())
}
pub fn recover(conn: &mut Connection, data: &Path) -> Result<bool> {
    let root = data.join(DIR);
    let path = root.join("journal.json");
    if !path.exists() {
        return Ok(false);
    }
    let journal: Journal = serde_json::from_slice(&fs::read(path)?)?;
    if journal.version != 1 || journal.root != root {
        return Err(error("Invalid package recovery journal."));
    }
    if !journal.committed {
        rollback(conn, &journal)?;
    }
    archive(data, &journal)?;
    Ok(!journal.committed)
}
pub fn run<T>(
    conn: &mut Connection,
    data: &Path,
    library: &Path,
    game: &Path,
    action: impl FnOnce(&mut Connection) -> Result<T>,
) -> Result<T> {
    let _operation = OPERATION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    deployment::ensure_game_stopped()?;
    let root = data.join(DIR);
    if root.join("journal.json").exists() {
        return Err(error(
            "Recover the previous package operation before continuing.",
        ));
    }
    if root.exists() {
        fs::rename(
            &root,
            data.join(format!("abandoned-package-{}", uuid::Uuid::new_v4())),
        )?;
    }
    fs::create_dir(&root)?;
    let mut saved = Connection::open(root.join("database.sqlite3"))?;
    Backup::new(conn, &mut saved)?.run_to_completion(64, Duration::from_millis(10), None)?;
    drop(saved);
    let database_digest = deployment::sha256(&root.join("database.sqlite3"))?;
    let journal = Journal {
        version: 1,
        root,
        library: library.into(),
        roots: vec![
            data.into(),
            library.into(),
            game.into(),
            deployment::config_root(game),
        ],
        committed: false,
        database_digest,
        entries: vec![],
    };
    journal.save()?;
    ACTIVE.with(|cell| *cell.borrow_mut() = Some(journal));
    let _reset = ResetActive;
    let result = action(conn);
    let mut journal = ACTIVE
        .with(|cell| cell.borrow_mut().take())
        .ok_or_else(|| error("Missing package journal"))?;
    match result {
        Ok(value) => {
            journal.committed = true;
            journal.save()?;
            archive(data, &journal)?;
            Ok(value)
        }
        Err(failure) => {
            rollback(conn, &journal).map_err(|recovery| error(&format!("Operation failed: {failure}. Recovery pending: {recovery}. Restart the manager before continuing.")))?;
            journal.committed = true;
            journal.save()?;
            archive(data, &journal)?;
            Err(error(&format!(
                "No package changes were kept. Previous files and library were restored. {failure}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (tempfile::TempDir, Connection, PathBuf, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        let library = temp.path().join("library");
        let game = temp.path().join("game");
        for path in [&data, &library, &game] {
            fs::create_dir_all(path).unwrap();
        }
        let conn = Connection::open(data.join("db.sqlite")).unwrap();
        conn.execute_batch("CREATE TABLE state(value TEXT); INSERT INTO state VALUES('before');")
            .unwrap();
        (temp, conn, data, library, game)
    }
    #[test]
    fn failure_restores_database_files_and_library_and_removes_new_payload() {
        let (_temp, mut conn, data, library, game) = setup();
        let original = game.join("original.bin");
        fs::write(&original, b"before").unwrap();
        let new = game.join("new.bin");
        let mod_dir = library.join(uuid::Uuid::new_v4().to_string());
        let result: Result<()> = run(&mut conn, &data, &library, &game, |conn| {
            protect_file(&original)?;
            fs::write(&original, b"after")?;
            protect_file(&new)?;
            fs::write(&new, b"new")?;
            protect_tree(&mod_dir)?;
            fs::create_dir(&mod_dir)?;
            fs::write(mod_dir.join("payload"), b"data")?;
            conn.execute("UPDATE state SET value='after'", [])?;
            Err(error("injected disk failure"))
        });
        assert!(result.unwrap_err().to_string().contains("restored"));
        assert_eq!(fs::read(original).unwrap(), b"before");
        assert!(!new.exists());
        assert!(!mod_dir.exists());
        assert_eq!(
            conn.query_row("SELECT value FROM state", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "before"
        );
        assert!(!data.join(DIR).exists());
    }
    #[test]
    fn interrupted_operation_recovers_on_restart_and_recovery_is_idempotent() {
        let (_temp, mut conn, data, library, game) = setup();
        let file = game.join("payload.bin");
        fs::write(&file, b"before").unwrap();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _: Result<()> = run(&mut conn, &data, &library, &game, |conn| {
                protect_file(&file)?;
                fs::write(&file, b"partial")?;
                conn.execute("UPDATE state SET value='partial'", [])?;
                panic!("simulated process loss");
            });
        }));
        ACTIVE.with(|cell| cell.borrow_mut().take());
        assert!(check_available(&data.join("db.sqlite")).is_err());
        assert!(recover(&mut conn, &data).unwrap());
        assert!(!recover(&mut conn, &data).unwrap());
        assert_eq!(fs::read(file).unwrap(), b"before");
        assert_eq!(
            conn.query_row("SELECT value FROM state", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "before"
        );
    }
    #[test]
    fn successful_commit_keeps_changes_and_broad_deletion_is_rejected() {
        let (_temp, mut conn, data, library, game) = setup();
        run(&mut conn, &data, &library, &game, |conn| {
            assert!(protect_tree(&library).is_err());
            assert!(protect_file(&game.join("../outside")).is_err());
            conn.execute("UPDATE state SET value='after'", [])?;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            conn.query_row("SELECT value FROM state", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "after"
        );
        assert!(!recover(&mut conn, &data).unwrap());
    }

    #[test]
    fn damaged_backup_stops_recovery_before_touching_live_files() {
        let (_temp, mut conn, data, library, game) = setup();
        let file = game.join("payload.bin");
        fs::write(&file, b"before").unwrap();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _: Result<()> = run(&mut conn, &data, &library, &game, |_| {
                protect_file(&file)?;
                fs::write(&file, b"partial")?;
                panic!("simulated interruption");
            });
        }));
        fs::write(data.join(DIR).join("entry-0"), b"damaged").unwrap();
        assert!(recover(&mut conn, &data)
            .unwrap_err()
            .to_string()
            .contains("damaged"));
        assert_eq!(fs::read(file).unwrap(), b"partial");
        assert!(data.join(DIR).join("journal.json").is_file());
    }
}
