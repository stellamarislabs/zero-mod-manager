//! Explicit import from the final ZCOM Mod Manager data layout.
//!
//! The continuation has its own application identifier, so it never silently
//! takes ownership of the original application's database or secret. Import is
//! an opt-in operation, stages and verifies the managed library first, and
//! retains a database backup before replacing the empty continuation state.

use crate::{
    database, deployment,
    error::{AppError, Result},
    AppContext,
};
use chrono::Utc;
use rusqlite::{backup::Backup, Connection, OpenFlags};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;
use walkdir::WalkDir;

const LEGACY_IDENTIFIER: &str = "org.zcommodmanager.desktop";
const LEGACY_DATABASE: &str = "zcom-mod-manager.sqlite3";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyImportStatus {
    pub available: bool,
    pub data_directory: Option<String>,
    pub library_directory: Option<String>,
    pub mod_count: usize,
    pub file_count: usize,
    pub can_import: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyImportReport {
    pub imported_mods: usize,
    pub copied_files: usize,
    pub copied_bytes: u64,
    pub nexus_key_imported: bool,
    pub backup_path: String,
}

fn legacy_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|root| root.join(LEGACY_IDENTIFIER))
}

fn legacy_local_data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|root| root.join(LEGACY_IDENTIFIER))
}

fn open_read_only(path: &Path) -> Result<Connection> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?)
}

fn mod_count(conn: &Connection) -> Result<usize> {
    Ok(conn.query_row("SELECT COUNT(*) FROM mods", [], |row| row.get::<_, i64>(0))? as usize)
}

fn legacy_library(conn: &Connection, data_dir: &Path) -> PathBuf {
    database::get_setting(conn, "managed_library_path")
        .ok()
        .flatten()
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            let roaming = data_dir.join("mods");
            roaming.exists().then_some(roaming)
        })
        .or_else(|| legacy_local_data_dir().map(|path| path.join("mods")))
        .unwrap_or_else(|| data_dir.join("mods"))
}

fn file_count(path: &Path) -> usize {
    if !path.exists() {
        return 0;
    }
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .count()
}

pub fn status(ctx: &AppContext) -> Result<LegacyImportStatus> {
    let Some(data_dir) = legacy_data_dir() else {
        return Ok(unavailable(
            "The operating system did not provide a legacy data location.",
        ));
    };
    let db_path = data_dir.join(LEGACY_DATABASE);
    if !db_path.is_file() {
        return Ok(unavailable("No ZCOM Mod Manager 0.6.5 data was found."));
    }
    let legacy = open_read_only(&db_path)?;
    let mods = mod_count(&legacy)?;
    let library = legacy_library(&legacy, &data_dir);
    let current = database::open(&ctx.db_path)?;
    if database::get_setting(&current, "legacy_imported_from")?.is_some() {
        return Ok(unavailable(
            "ZCOM Mod Manager data has already been imported.",
        ));
    }
    let current_mods = mod_count(&current)?;
    let can_import = current_mods == 0;
    Ok(LegacyImportStatus {
        available: true,
        data_directory: Some(data_dir.to_string_lossy().into_owned()),
        library_directory: Some(library.to_string_lossy().into_owned()),
        mod_count: mods,
        file_count: file_count(&library),
        can_import,
        reason: (!can_import).then(|| {
            "This Zero Mod Manager library already contains mods, so legacy data cannot be imported over it.".into()
        }),
    })
}

fn unavailable(reason: &str) -> LegacyImportStatus {
    LegacyImportStatus {
        available: false,
        data_directory: None,
        library_directory: None,
        mod_count: 0,
        file_count: 0,
        can_import: false,
        reason: Some(reason.into()),
    }
}

fn copy_verified(source: &Path, destination: &Path) -> Result<(usize, u64)> {
    fs::create_dir_all(destination)?;
    if !source.exists() {
        return Ok((0, 0));
    }
    let mut files = 0;
    let mut bytes = 0;
    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(|error| AppError::Other(error.to_string()))?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .map_err(|error| AppError::Other(error.to_string()))?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);
        if entry.file_type().is_symlink() {
            return Err(AppError::Other(format!(
                "Legacy library contains a symbolic link and was not imported: {}",
                entry.path().display()
            )));
        }
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(entry.path(), &target)?;
        if deployment::sha256(entry.path())? != deployment::sha256(&target)? {
            return Err(AppError::Other(format!(
                "Verification failed while importing {}.",
                relative.display()
            )));
        }
        files += 1;
        bytes += entry
            .metadata()
            .map_err(|error| AppError::Other(error.to_string()))?
            .len();
    }
    Ok((files, bytes))
}

fn backup_database(source: &Connection, destination: &mut Connection) -> Result<()> {
    let backup = Backup::new(source, destination)?;
    backup.run_to_completion(64, std::time::Duration::from_millis(10), None)?;
    Ok(())
}

/// Creates the one-time, byte-verified safety copy required before the 0.7
/// operational schema can touch a 0.6.x database. The marker is written only
/// after both the SQLite backup and managed source library are complete.
pub fn backup_operational_upgrade(
    database_path: &Path,
    data_dir: &Path,
    default_library: &Path,
) -> Result<Option<PathBuf>> {
    if !database_path.is_file() {
        return Ok(None);
    }
    let marker = data_dir.join("operational-upgrade-backup.txt");
    if marker.is_file() {
        let saved = PathBuf::from(fs::read_to_string(&marker)?.trim());
        if saved.join("database.sqlite3").is_file() {
            return Ok(Some(saved));
        }
        return Err(AppError::Other("The recorded pre-0.7 backup is missing. Restore it or remove the stale marker before retrying the upgrade.".into()));
    }
    let source = open_read_only(database_path)?;
    let version = source
        .query_row(
            "SELECT coalesce(max(version),0) FROM schema_migrations",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0);
    if version >= 8 {
        return Ok(None);
    }
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let root = data_dir
        .join("migration-backups")
        .join(format!("pre-0.7-{stamp}"));
    fs::create_dir_all(&root)?;
    let mut database_backup = Connection::open(root.join("database.sqlite3"))?;
    backup_database(&source, &mut database_backup)?;

    let configured = database::get_setting(&source, "managed_library_path")?
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from);
    let legacy_local = data_dir.join("mods");
    let library = configured.unwrap_or_else(|| {
        if legacy_local.is_dir() {
            legacy_local
        } else {
            default_library.to_path_buf()
        }
    });
    let (files, bytes) = if library.is_dir() {
        copy_verified(&library, &root.join("managed-library"))?
    } else {
        (0, 0)
    };
    fs::write(
        root.join("backup.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": 1, "createdAt": Utc::now().to_rfc3339(),
            "sourceSchema": version, "libraryFiles": files, "libraryBytes": bytes,
            "originalDatabase": database_path, "originalLibrary": library,
        }))?,
    )?;
    fs::write(&marker, root.display().to_string())?;
    Ok(Some(root))
}

pub fn import(ctx: &AppContext, _include_nexus_key: bool) -> Result<LegacyImportReport> {
    let state = status(ctx)?;
    if !state.available || !state.can_import {
        return Err(AppError::Other(
            state
                .reason
                .unwrap_or_else(|| "Legacy data cannot be imported.".into()),
        ));
    }
    let legacy_data = PathBuf::from(state.data_directory.as_deref().unwrap_or_default());
    let legacy_library = PathBuf::from(state.library_directory.as_deref().unwrap_or_default());
    let legacy = open_read_only(&legacy_data.join(LEGACY_DATABASE))?;
    let stage = ctx
        .data_dir
        .join(format!(".legacy-library-import-{}", Uuid::new_v4()));
    let (copied_files, copied_bytes) = match copy_verified(&legacy_library, &stage) {
        Ok(result) => result,
        Err(error) => {
            let _ = fs::remove_dir_all(&stage);
            return Err(error);
        }
    };

    if ctx.default_mods_dir.exists() && ctx.default_mods_dir.read_dir()?.next().is_some() {
        let _ = fs::remove_dir_all(&stage);
        return Err(AppError::Other(
            "The new managed library is not empty. No legacy data was changed.".into(),
        ));
    }

    let backup_dir = ctx.data_dir.join("import-backups");
    fs::create_dir_all(&backup_dir)?;
    let backup_path = backup_dir.join(format!(
        "pre-legacy-import-{}.sqlite3",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    {
        let current = database::open(&ctx.db_path)?;
        let mut backup = Connection::open(&backup_path)?;
        backup_database(&current, &mut backup)?;
    }

    let import_result = (|| -> Result<()> {
        let mut current = database::open(&ctx.db_path)?;
        backup_database(&legacy, &mut current)?;
        database::set_setting(
            &current,
            "managed_library_path",
            &ctx.default_mods_dir.to_string_lossy(),
        )?;
        database::set_setting(&current, "legacy_imported_from", "ZCOM Mod Manager 0.6.5")?;
        // A database snapshot can contain the legacy plaintext fallback. It is
        // removed even when the old key lived in the OS keyring, then a new
        // credential is created only after explicit consent.
        for setting in ["nexus_api_key", "nexus_account_name", "nexus_premium"] {
            database::delete_setting(&current, setting)?;
        }
        drop(current);

        if ctx.default_mods_dir.exists() {
            fs::remove_dir(&ctx.default_mods_dir)?;
        }
        fs::rename(&stage, &ctx.default_mods_dir)?;
        *ctx.mods_dir
            .write()
            .map_err(|_| AppError::Other("managed library lock was poisoned".into()))? =
            ctx.default_mods_dir.clone();
        Ok(())
    })();

    if let Err(error) = import_result {
        let _ = fs::remove_dir_all(&stage);
        let backup = open_read_only(&backup_path)?;
        let mut current = database::open(&ctx.db_path)?;
        backup_database(&backup, &mut current)?;
        return Err(error);
    }

    Ok(LegacyImportReport {
        imported_mods: state.mod_count,
        copied_files,
        copied_bytes,
        nexus_key_imported: false,
        backup_path: backup_path.to_string_lossy().into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn legacy_library_copy_is_byte_verified() {
        let root = tempdir().unwrap();
        let source = root.path().join("old");
        let destination = root.path().join("staged");
        fs::create_dir_all(source.join("mod/payload")).unwrap();
        fs::write(source.join("mod/payload/main.lua"), b"return true").unwrap();

        let (files, bytes) = copy_verified(&source, &destination).unwrap();

        assert_eq!(files, 1);
        assert_eq!(bytes, 11);
        assert_eq!(
            deployment::sha256(&source.join("mod/payload/main.lua")).unwrap(),
            deployment::sha256(&destination.join("mod/payload/main.lua")).unwrap()
        );
    }

    #[test]
    fn sqlite_backup_contains_the_complete_source() {
        let source = Connection::open_in_memory().unwrap();
        source
            .execute("CREATE TABLE sample(value TEXT)", [])
            .unwrap();
        source
            .execute("INSERT INTO sample VALUES('legacy')", [])
            .unwrap();
        let mut destination = Connection::open_in_memory().unwrap();

        backup_database(&source, &mut destination).unwrap();

        let value: String = destination
            .query_row("SELECT value FROM sample", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "legacy");
    }

    #[test]
    fn operational_upgrade_creates_a_verified_one_time_backup() {
        let root = tempdir().unwrap();
        let data_dir = root.path().join("data");
        let library = root.path().join("managed-library");
        let database_path = data_dir.join("zcom-mod-manager.sqlite3");
        fs::create_dir_all(&data_dir).unwrap();
        fs::create_dir_all(library.join("example")).unwrap();
        fs::write(library.join("example/payload.pak"), b"managed payload").unwrap();
        {
            let source = Connection::open(&database_path).unwrap();
            source
                .execute_batch(
                    "CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY);\
                     INSERT INTO schema_migrations VALUES(7);\
                     CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);",
                )
                .unwrap();
            database::set_setting(&source, "managed_library_path", &library.to_string_lossy())
                .unwrap();
        }

        let backup = backup_operational_upgrade(&database_path, &data_dir, root.path())
            .unwrap()
            .unwrap();

        assert!(backup.join("database.sqlite3").is_file());
        assert_eq!(
            fs::read(backup.join("managed-library/example/payload.pak")).unwrap(),
            b"managed payload"
        );
        let copied = Connection::open(backup.join("database.sqlite3")).unwrap();
        let version: i64 = copied
            .query_row("SELECT max(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, 7);

        let repeated = backup_operational_upgrade(&database_path, &data_dir, root.path())
            .unwrap()
            .unwrap();
        assert_eq!(repeated, backup);
    }
}
