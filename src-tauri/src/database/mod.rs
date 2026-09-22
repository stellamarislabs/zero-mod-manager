use crate::{
    error::Result,
    models::{AppSettings, ModFile, ModSummary, PreviewConflict},
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::{collections::BTreeMap, path::Path};

pub fn open(path: &Path) -> Result<Connection> {
    let mut connection = Connection::open(path)?;
    connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;
      CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS mods(id TEXT PRIMARY KEY, name TEXT NOT NULL, version TEXT, mod_type TEXT NOT NULL, deployment_key TEXT NOT NULL DEFAULT '', source_archive TEXT, installed_at TEXT NOT NULL, enabled INTEGER NOT NULL, installed_build TEXT);
      CREATE TABLE IF NOT EXISTS mod_files(id INTEGER PRIMARY KEY, mod_id TEXT NOT NULL REFERENCES mods(id) ON DELETE CASCADE, library_relative TEXT NOT NULL, destination TEXT NOT NULL, size INTEGER NOT NULL, sha256 TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS mod_packages(mod_id TEXT NOT NULL REFERENCES mods(id) ON DELETE CASCADE, package_id TEXT NOT NULL, PRIMARY KEY(mod_id, package_id));
      CREATE TABLE IF NOT EXISTS mod_backups(mod_id TEXT NOT NULL REFERENCES mods(id) ON DELETE CASCADE, destination TEXT NOT NULL, backup_relative TEXT NOT NULL, sha256 TEXT NOT NULL, PRIMARY KEY(mod_id, destination));
      CREATE TABLE IF NOT EXISTS fomod_installs(mod_id TEXT PRIMARY KEY REFERENCES mods(id) ON DELETE CASCADE, answers_json TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS container_checks(mod_id TEXT PRIMARY KEY REFERENCES mods(id) ON DELETE CASCADE, state TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS nexus_sources(archive_path TEXT PRIMARY KEY, nexus_mod_id INTEGER NOT NULL, nexus_file_id INTEGER NOT NULL, version TEXT, file_name TEXT NOT NULL, downloaded_at TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS nexus_identification(mod_id TEXT PRIMARY KEY REFERENCES mods(id) ON DELETE CASCADE, md5 TEXT NOT NULL, matched INTEGER NOT NULL, attempted_at TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS nexus_updates(nexus_mod_id INTEGER PRIMARY KEY, latest_file_id INTEGER NOT NULL, latest_version TEXT, latest_file_name TEXT NOT NULL, checked_at TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS nexus_file_updates(nexus_mod_id INTEGER NOT NULL, installed_file_id INTEGER NOT NULL, latest_file_id INTEGER NOT NULL, latest_version TEXT, latest_file_name TEXT NOT NULL, checked_at TEXT NOT NULL, PRIMARY KEY(nexus_mod_id, installed_file_id));
      INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES(1, datetime('now'));"
    )?;
    migrate_v2(&mut connection)?;
    migrate_v3(&mut connection)?;
    migrate_v4(&mut connection)?;
    migrate_v5(&mut connection)?;
    migrate_v6(&mut connection)?;
    migrate_v7(&mut connection)?;
    migrate_v8(&mut connection)?;
    migrate_v9(&mut connection)?;
    migrate_v10(&mut connection)?;
    Ok(connection)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(names.iter().any(|name| name == column))
}

fn migrate_v2(conn: &mut Connection) -> Result<()> {
    let applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=2)",
        [],
        |row| row.get(0),
    )?;
    if applied {
        return Ok(());
    }
    let transaction = conn.transaction()?;
    if !column_exists(&transaction, "mods", "load_priority")? {
        transaction.execute("ALTER TABLE mods ADD COLUMN load_priority INTEGER", [])?;
    }
    let ids = {
        let mut statement = transaction.prepare(
            "SELECT id FROM mods WHERE mod_type IN ('pak','iostore') ORDER BY installed_at ASC,id ASC",
        )?;
        let result = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        result
    };
    for (index, id) in ids.iter().enumerate() {
        transaction.execute(
            "UPDATE mods SET load_priority=?2 WHERE id=?1",
            params![id, (index + 1) as i64],
        )?;
    }
    transaction.execute(
        "INSERT INTO schema_migrations(version,applied_at) VALUES(2,datetime('now'))",
        [],
    )?;
    transaction.commit()?;
    Ok(())
}

/// Records where a mod came from on Nexus Mods. A mod installed from a file the
/// user picked by hand has no provenance and is simply never checked.
fn migrate_v3(conn: &mut Connection) -> Result<()> {
    let applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=3)",
        [],
        |row| row.get(0),
    )?;
    if applied {
        return Ok(());
    }
    let transaction = conn.transaction()?;
    for column in ["nexus_mod_id", "nexus_file_id"] {
        if !column_exists(&transaction, "mods", column)? {
            transaction.execute(&format!("ALTER TABLE mods ADD COLUMN {column} INTEGER"), [])?;
        }
    }
    transaction.execute(
        "INSERT INTO schema_migrations(version,applied_at) VALUES(3,datetime('now'))",
        [],
    )?;
    transaction.commit()?;
    Ok(())
}

/// Lets a mod be kept out of the library list without uninstalling it. A UE4SS
/// runtime mod adopted from the game folder is still deployed and still ordered;
/// it just does not need to be looked at every day.
fn migrate_v4(conn: &mut Connection) -> Result<()> {
    let applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=4)",
        [],
        |row| row.get(0),
    )?;
    if applied {
        return Ok(());
    }
    let transaction = conn.transaction()?;
    if !column_exists(&transaction, "mods", "hidden")? {
        transaction.execute(
            "ALTER TABLE mods ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    transaction.execute(
        "INSERT INTO schema_migrations(version,applied_at) VALUES(4,datetime('now'))",
        [],
    )?;
    transaction.commit()?;
    Ok(())
}

/// Lets a mod be taken out of update checking for good. Without it, a mod that
/// did not come from Nexus was offered to the MD5 lookup again on every check
/// the user asked for, and a mod that had been unlinked was simply matched and
/// linked all over again.
fn migrate_v5(conn: &mut Connection) -> Result<()> {
    let applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=5)",
        [],
        |row| row.get(0),
    )?;
    if applied {
        return Ok(());
    }
    let transaction = conn.transaction()?;
    if !column_exists(&transaction, "mods", "nexus_ignored")? {
        transaction.execute(
            "ALTER TABLE mods ADD COLUMN nexus_ignored INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    transaction.execute(
        "INSERT INTO schema_migrations(version,applied_at) VALUES(5,datetime('now'))",
        [],
    )?;
    transaction.commit()?;
    Ok(())
}

/// Stores update results for the exact Nexus file that was installed. A page
/// can publish several mutually exclusive main or optional files at once, so a
/// single newest-file row for the whole page cannot describe every install.
fn migrate_v6(conn: &mut Connection) -> Result<()> {
    let applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=6)",
        [],
        |row| row.get(0),
    )?;
    if applied {
        return Ok(());
    }
    let transaction = conn.transaction()?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS nexus_file_updates(
           nexus_mod_id INTEGER NOT NULL,
           installed_file_id INTEGER NOT NULL,
           latest_file_id INTEGER NOT NULL,
           latest_version TEXT,
           latest_file_name TEXT NOT NULL,
           checked_at TEXT NOT NULL,
           PRIMARY KEY(nexus_mod_id, installed_file_id)
         );",
    )?;
    transaction.execute(
        "INSERT INTO schema_migrations(version,applied_at) VALUES(6,datetime('now'))",
        [],
    )?;
    transaction.commit()?;
    Ok(())
}

/// Marks installs whose complete FOMOD source tree is retained in the managed
/// library and stores the answers needed to seed the wizard next time.
fn migrate_v7(conn: &mut Connection) -> Result<()> {
    let applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=7)",
        [],
        |row| row.get(0),
    )?;
    if applied {
        return Ok(());
    }
    let transaction = conn.transaction()?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS fomod_installs(
           mod_id TEXT PRIMARY KEY REFERENCES mods(id) ON DELETE CASCADE,
           answers_json TEXT NOT NULL
         );",
    )?;
    transaction.execute(
        "INSERT INTO schema_migrations(version,applied_at) VALUES(7,datetime('now'))",
        [],
    )?;
    transaction.commit()?;
    Ok(())
}

/// Adds the operational state used by profiles, recoverable activity,
/// launch evidence, configuration patches and compatibility metadata.
fn migrate_v8(conn: &mut Connection) -> Result<()> {
    let applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=8)",
        [],
        |row| row.get(0),
    )?;
    if applied {
        return Ok(());
    }
    let transaction = conn.transaction()?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS profiles(
           id TEXT PRIMARY KEY,
           name TEXT NOT NULL,
           notes TEXT NOT NULL DEFAULT '',
           required_runtime TEXT,
           created_at TEXT NOT NULL,
           updated_at TEXT NOT NULL,
           is_active INTEGER NOT NULL DEFAULT 0
         );
         CREATE UNIQUE INDEX IF NOT EXISTS one_active_profile
           ON profiles(is_active) WHERE is_active=1;
         CREATE TABLE IF NOT EXISTS profile_mods(
           profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
           mod_id TEXT NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
           enabled INTEGER NOT NULL,
           load_priority INTEGER,
           fomod_answers TEXT,
           PRIMARY KEY(profile_id,mod_id)
         );
         CREATE TABLE IF NOT EXISTS snapshots(
           id TEXT PRIMARY KEY,
           profile_id TEXT REFERENCES profiles(id) ON DELETE SET NULL,
           label TEXT NOT NULL,
           kind TEXT NOT NULL,
           payload_json TEXT NOT NULL,
           created_at TEXT NOT NULL,
           last_known_good INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS launch_sessions(
           id TEXT PRIMARY KEY,
           mode TEXT NOT NULL,
           profile_id TEXT REFERENCES profiles(id) ON DELETE SET NULL,
           launcher TEXT NOT NULL,
           game_build TEXT,
           executable_sha256 TEXT,
           runtime_version TEXT,
           mod_lock_json TEXT NOT NULL,
           started_at TEXT NOT NULL,
           ended_at TEXT,
           outcome TEXT,
           log_evidence TEXT
         );
         CREATE TABLE IF NOT EXISTS operations(
           id TEXT PRIMARY KEY,
           kind TEXT NOT NULL,
           status TEXT NOT NULL,
           summary TEXT NOT NULL,
           detail_json TEXT NOT NULL,
           recovery_json TEXT,
           started_at TEXT NOT NULL,
           finished_at TEXT
         );
         CREATE TABLE IF NOT EXISTS compatibility_rules(
           id TEXT PRIMARY KEY,
           subject_mod_id TEXT NOT NULL,
           target_mod_id TEXT,
           rule_type TEXT NOT NULL,
           severity TEXT NOT NULL,
           source TEXT NOT NULL,
           evidence_url TEXT,
           message TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS config_patches(
           id TEXT PRIMARY KEY,
           profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
           path TEXT NOT NULL,
           format TEXT NOT NULL,
           patch_json TEXT NOT NULL,
           backup_path TEXT,
           created_at TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS isolation_sessions(
           id TEXT PRIMARY KEY,
           profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
           original_lock_json TEXT NOT NULL,
           candidates_json TEXT NOT NULL,
           observations_json TEXT NOT NULL,
           current_json TEXT NOT NULL,
           phase TEXT NOT NULL,
           status TEXT NOT NULL,
           created_at TEXT NOT NULL
         );",
    )?;
    let profile_count: i64 =
        transaction.query_row("SELECT count(*) FROM profiles", [], |r| r.get(0))?;
    if profile_count == 0 {
        let profile_id = uuid::Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO profiles(id,name,created_at,updated_at,is_active) VALUES(?1,'Default',datetime('now'),datetime('now'),1)",
            [&profile_id],
        )?;
        transaction.execute(
            "INSERT INTO profile_mods(profile_id,mod_id,enabled,load_priority,fomod_answers)
             SELECT ?1,m.id,m.enabled,m.load_priority,f.answers_json
             FROM mods m LEFT JOIN fomod_installs f ON f.mod_id=m.id",
            [&profile_id],
        )?;
    }
    transaction.execute(
        "INSERT INTO schema_migrations(version,applied_at) VALUES(8,datetime('now'))",
        [],
    )?;
    transaction.commit()?;
    Ok(())
}

/// Retains stable manifest identity and the exact author contract so catalog
/// rules never depend on a random local database id.
fn migrate_v9(conn: &mut Connection) -> Result<()> {
    let applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=9)",
        [],
        |row| row.get(0),
    )?;
    if applied {
        return Ok(());
    }
    let transaction = conn.transaction()?;
    if !column_exists(&transaction, "mods", "manifest_id")? {
        transaction.execute("ALTER TABLE mods ADD COLUMN manifest_id TEXT", [])?;
    }
    if !column_exists(&transaction, "mods", "manifest_json")? {
        transaction.execute("ALTER TABLE mods ADD COLUMN manifest_json TEXT", [])?;
    }
    transaction.execute(
        "CREATE INDEX IF NOT EXISTS mods_manifest_id ON mods(manifest_id)",
        [],
    )?;
    transaction.execute(
        "INSERT INTO schema_migrations(version,applied_at) VALUES(9,datetime('now'))",
        [],
    )?;
    transaction.commit()?;
    Ok(())
}

/// Links components that came from one archive and were committed as one
/// operation. Existing installs remain valid standalone components.
fn migrate_v10(conn: &mut Connection) -> Result<()> {
    let applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=10)",
        [],
        |row| row.get(0),
    )?;
    if applied {
        return Ok(());
    }
    let transaction = conn.transaction()?;
    if !column_exists(&transaction, "mods", "bundle_id")? {
        transaction.execute("ALTER TABLE mods ADD COLUMN bundle_id TEXT", [])?;
    }
    transaction.execute(
        "CREATE INDEX IF NOT EXISTS mods_bundle_id ON mods(bundle_id)",
        [],
    )?;
    transaction.execute(
        "INSERT INTO schema_migrations(version,applied_at) VALUES(10,datetime('now'))",
        [],
    )?;
    transaction.commit()?;
    Ok(())
}

pub fn set_hidden(conn: &Connection, id: &str, hidden: bool) -> Result<()> {
    conn.execute("UPDATE mods SET hidden=?2 WHERE id=?1", params![id, hidden])?;
    Ok(())
}

/// Records manifest provenance for external links and compatibility rules.
pub fn set_nexus_ids(
    conn: &Connection,
    mod_id: &str,
    nexus_mod_id: u64,
    nexus_file_id: u64,
) -> Result<()> {
    conn.execute(
        "UPDATE mods SET nexus_mod_id=?2,nexus_file_id=?3 WHERE id=?1",
        params![mod_id, nexus_mod_id as i64, nexus_file_id as i64],
    )?;
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn.query_row("SELECT value FROM settings WHERE key=?1", [key], |r| r.get(0)).optional()?)
}

pub fn settings(conn: &Connection) -> Result<AppSettings> {
    let bool_value = |key: &str| get_setting(conn, key).map(|v| v.as_deref() == Some("true"));
    Ok(AppSettings {
        game_path: get_setting(conn, "game_path")?,
        custom_executable_path: get_setting(conn, "custom_executable_path")?
            .filter(|path| !path.trim().is_empty()),
        retoc_path: get_setting(conn, "retoc_path")?,
        seven_zip_path: get_setting(conn, "seven_zip_path")?.filter(|path| !path.trim().is_empty()),
        log_level: get_setting(conn, "log_level")?.unwrap_or_else(|| "normal".into()),
        advanced_package_names: bool_value("advanced_package_names")?,
        reduced_motion: bool_value("reduced_motion")?,
    })
}

pub fn save_settings(conn: &Connection, value: &AppSettings) -> Result<()> {
    let values = [
        ("game_path", value.game_path.clone().unwrap_or_default()),
        (
            "custom_executable_path",
            value.custom_executable_path.clone().unwrap_or_default(),
        ),
        ("retoc_path", value.retoc_path.clone().unwrap_or_default()),
        (
            "seven_zip_path",
            value.seven_zip_path.clone().unwrap_or_default(),
        ),
        ("log_level", value.log_level.clone()),
        (
            "advanced_package_names",
            value.advanced_package_names.to_string(),
        ),
        ("reduced_motion", value.reduced_motion.to_string()),
    ];
    for (key, val) in values {
        conn.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,val])?;
    }
    Ok(())
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,value])?;
    Ok(())
}

pub fn delete_setting(conn: &Connection, key: &str) -> Result<()> {
    conn.execute("DELETE FROM settings WHERE key=?1", params![key])?;
    Ok(())
}

pub fn counts(conn: &Connection) -> Result<(usize, usize)> {
    let total: i64 = conn.query_row("SELECT count(*) FROM mods", [], |r| r.get(0))?;
    let enabled: i64 = conn.query_row("SELECT count(*) FROM mods WHERE enabled=1", [], |r| {
        r.get(0)
    })?;
    Ok((total as usize, enabled as usize))
}

pub fn list_mods(conn: &Connection) -> Result<Vec<ModSummary>> {
    let mut stmt = conn.prepare("SELECT id,name,version,mod_type,enabled,installed_at,installed_build,load_priority,nexus_mod_id,hidden,nexus_ignored,EXISTS(SELECT 1 FROM fomod_installs f WHERE f.mod_id=mods.id),bundle_id FROM mods ORDER BY installed_at DESC")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, bool>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, Option<String>>(6)?,
            r.get::<_, Option<i64>>(7)?,
            r.get::<_, Option<i64>>(8)?,
            r.get::<_, bool>(9)?,
            r.get::<_, bool>(10)?,
            r.get::<_, bool>(11)?,
            r.get::<_, Option<String>>(12)?,
        ))
    })?;
    let mut result = Vec::new();
    for row in rows {
        let (
            id,
            name,
            version,
            mod_type,
            enabled,
            installed_at,
            installed_build,
            load_priority,
            nexus_mod_id,
            hidden,
            nexus_ignored,
            fomod,
            bundle_id,
        ) = row?;
        let mut fs = conn.prepare(
            "SELECT destination,size,sha256 FROM mod_files WHERE mod_id=?1 ORDER BY destination",
        )?;
        let files = fs
            .query_map([&id], |r| {
                let d: String = r.get(0)?;
                Ok(ModFile {
                    name: Path::new(&d)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    destination: d,
                    size: r.get::<_, i64>(1)? as u64,
                    sha256: r.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let package_count: i64 = conn.query_row(
            "SELECT count(*) FROM mod_packages WHERE mod_id=?1",
            [&id],
            |r| r.get(0),
        )?;
        let potential_conflict_count: i64=conn.query_row("SELECT count(DISTINCT other.mod_id) FROM mod_packages mine JOIN mod_packages other ON mine.package_id=other.package_id AND mine.mod_id<>other.mod_id WHERE mine.mod_id=?1",[&id],|r|r.get(0))?;
        let conflict_count: i64 = if enabled {
            conn.query_row("SELECT count(DISTINCT other.mod_id) FROM mod_packages mine JOIN mod_packages other ON mine.package_id=other.package_id AND mine.mod_id<>other.mod_id JOIN mods other_mod ON other_mod.id=other.mod_id WHERE mine.mod_id=?1 AND other_mod.enabled=1",[&id],|r|r.get(0))?
        } else {
            0
        };
        result.push(ModSummary {
            container_verification: conn.query_row("SELECT state FROM container_checks WHERE mod_id=?1", [&id], |row| row.get(0)).optional()?,
            id,
            bundle_id,
            name,
            version,
            mod_type,
            enabled,
            installed_at,
            installed_build,
            package_count: package_count as usize,
            conflict_count: conflict_count as usize,
            potential_conflict_count: potential_conflict_count as usize,
            load_priority,
            nexus_mod_id: nexus_mod_id.map(|id| id as u64),
            nexus_url: nexus_mod_id.map(|id| format!("https://www.nexusmods.com/starwarszerocompany/mods/{id}")),
            nexus_ignored,
            hidden,
            fomod,
            files,
        });
    }
    Ok(result)
}

pub fn set_bundle_id(conn: &Connection, mod_id: &str, bundle_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE mods SET bundle_id=?2 WHERE id=?1",
        params![mod_id, bundle_id],
    )?;
    Ok(())
}

pub fn record_fomod_install(tx: &Transaction<'_>, mod_id: &str, answers_json: &str) -> Result<()> {
    tx.execute(
        "INSERT INTO fomod_installs(mod_id,answers_json) VALUES(?1,?2)",
        params![mod_id, answers_json],
    )?;
    Ok(())
}

/// The source label and saved recipe for an installed FOMOD.
pub fn fomod_install(conn: &Connection, mod_id: &str) -> Result<Option<(Option<String>, String)>> {
    conn.query_row(
        "SELECT m.source_archive,f.answers_json FROM fomod_installs f JOIN mods m ON m.id=f.mod_id WHERE f.mod_id=?1",
        [mod_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(Into::into)
}

pub fn insert_mod(
    tx: &Transaction<'_>,
    summary: &ModSummary,
    deployment_key: &str,
    source: Option<&str>,
    file_rows: &[(String, String, u64, String)],
    packages: &[String],
) -> Result<()> {
    tx.execute("INSERT INTO mods(id,name,version,mod_type,deployment_key,source_archive,installed_at,enabled,installed_build,load_priority) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![summary.id,summary.name,summary.version,summary.mod_type,deployment_key,source,summary.installed_at,summary.enabled,summary.installed_build,summary.load_priority])?;
    if let Some(state) = &summary.container_verification {
        tx.execute("INSERT INTO container_checks(mod_id,state) VALUES(?1,?2)", params![summary.id, state])?;
    }
    for (library, destination, size, hash) in file_rows {
        tx.execute("INSERT INTO mod_files(mod_id,library_relative,destination,size,sha256) VALUES(?1,?2,?3,?4,?5)",params![summary.id,library,destination,*size as i64,hash])?;
    }
    for package in packages {
        tx.execute(
            "INSERT INTO mod_packages(mod_id,package_id) VALUES(?1,?2)",
            params![summary.id, package],
        )?;
    }
    Ok(())
}

pub fn set_enabled(conn: &Connection, id: &str, enabled: bool) -> Result<()> {
    conn.execute(
        "UPDATE mods SET enabled=?2 WHERE id=?1",
        params![id, enabled],
    )?;
    Ok(())
}
pub fn remove_mod(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM mods WHERE id=?1", [id])?;
    Ok(())
}
pub fn file_records(conn: &Connection, id: &str) -> Result<Vec<(String, String, u64, String)>> {
    let mut s = conn.prepare(
        "SELECT library_relative,destination,size,sha256 FROM mod_files WHERE mod_id=?1",
    )?;
    let rows = s
        .query_map([id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? as u64, r.get(3)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}
pub fn next_load_priority(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT coalesce(max(load_priority),0)+1 FROM mods WHERE mod_type IN ('pak','iostore')",
        [],
        |row| row.get(0),
    )?)
}

/// UE4SS start order is a separate sequence in the same column: the packaged
/// list is ranked highest-wins, while `mods.txt` is read first-to-last.
pub fn next_ue4ss_priority(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT coalesce(max(load_priority),0)+1 FROM mods WHERE mod_type='ue4ss'",
        [],
        |row| row.get(0),
    )?)
}
/// The packaged mod that already owns a payload file name, if any. `exclude`
/// skips the mod an installation is replacing, whose files are being taken over
/// rather than collided with.
pub fn packaged_source_name_owner(
    conn: &Connection,
    library_relative: &str,
    exclude: Option<&str>,
) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT m.id FROM mod_files f JOIN mods m ON m.id=f.mod_id WHERE m.mod_type IN ('pak','iostore') AND lower(f.library_relative)=lower(?1) AND m.id IS NOT ?2 LIMIT 1",
            params![library_relative, exclude],
            |row| row.get(0),
        )
        .optional()?)
}

/// The mod that already deploys to a path, if any.
pub fn destination_owner(
    conn: &Connection,
    destination: &str,
    exclude: Option<&str>,
) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT mod_id FROM mod_files WHERE destination=?1 AND mod_id IS NOT ?2 LIMIT 1",
            params![destination, exclude],
            |row| row.get(0),
        )
        .optional()?)
}

/// The UE4SS mod occupying a runtime folder name, if any.
pub fn ue4ss_folder_owner(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM mods WHERE mod_type='ue4ss' AND lower(deployment_key)=lower(?1) LIMIT 1",
            [key],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn summary_of(conn: &Connection, id: &str) -> Result<(String, Option<String>)> {
    Ok(
        conn.query_row("SELECT name,version FROM mods WHERE id=?1", [id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?,
    )
}
pub fn set_load_priority(tx: &Transaction<'_>, id: &str, priority: i64) -> Result<()> {
    tx.execute(
        "UPDATE mods SET load_priority=?2 WHERE id=?1",
        params![id, priority],
    )?;
    Ok(())
}
pub fn update_file_destination(
    tx: &Transaction<'_>,
    mod_id: &str,
    library_relative: &str,
    destination: &str,
) -> Result<()> {
    tx.execute(
        "UPDATE mod_files SET destination=?3 WHERE mod_id=?1 AND library_relative=?2",
        params![mod_id, library_relative, destination],
    )?;
    Ok(())
}
pub fn recorded_destination(
    conn: &Connection,
    mod_id: &str,
    library_relative: &str,
) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT destination FROM mod_files WHERE mod_id=?1 AND library_relative=?2",
            params![mod_id, library_relative],
            |row| row.get(0),
        )
        .optional()?)
}
pub fn conflicts_for_packages(
    conn: &Connection,
    packages: &[String],
) -> Result<Vec<PreviewConflict>> {
    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut statement = conn.prepare(
        "SELECT m.id,m.name FROM mod_packages p JOIN mods m ON m.id=p.mod_id WHERE p.package_id=?1",
    )?;
    for package in packages {
        for row in statement.query_map([package], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })? {
            *counts.entry(row?).or_default() += 1;
        }
    }
    Ok(counts
        .into_iter()
        .map(|((mod_id, name), package_count)| PreviewConflict {
            mod_id,
            name,
            package_count,
        })
        .collect())
}
pub fn package_members(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut statement =
        conn.prepare("SELECT package_id,mod_id FROM mod_packages ORDER BY package_id,mod_id")?;
    let result = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(result)
}
/// What the rest of the application needs to know about an installed mod
/// without loading its file list.
pub struct ModRecord {
    pub name: String,
    pub load_priority: Option<i64>,
    /// UE4SS mod folder names, one per line in the stored column. Rows written
    /// before an archive could hold several mods carry a single key, and rows
    /// that predate the column fall back to the mod name.
    pub keys: Vec<String>,
    pub mod_type: String,
    pub enabled: bool,
}

pub fn mod_record(conn: &Connection, id: &str) -> Result<ModRecord> {
    let (name, stored, mod_type, enabled, load_priority) = conn.query_row(
        "SELECT name,deployment_key,mod_type,enabled,load_priority FROM mods WHERE id=?1",
        [id],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, bool>(3)?,
                r.get::<_, Option<i64>>(4)?,
            ))
        },
    )?;
    let mut keys: Vec<String> = stored
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    if keys.is_empty() && mod_type == "ue4ss" {
        keys.push(name.clone());
    }
    Ok(ModRecord {
        name,
        load_priority,
        keys,
        mod_type,
        enabled,
    })
}

pub fn rename_mod(conn: &Connection, id: &str, name: &str) -> Result<()> {
    let changed = conn.execute("UPDATE mods SET name=?2 WHERE id=?1", params![id, name])?;
    if changed == 0 {
        return Err(crate::error::AppError::Other(
            "That mod is no longer installed.".into(),
        ));
    }
    Ok(())
}

pub fn record_backup(
    tx: &Connection,
    mod_id: &str,
    destination: &str,
    backup_relative: &str,
    sha256: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO mod_backups(mod_id,destination,backup_relative,sha256) VALUES(?1,?2,?3,?4) ON CONFLICT(mod_id,destination) DO UPDATE SET backup_relative=excluded.backup_relative,sha256=excluded.sha256",
        params![mod_id, destination, backup_relative, sha256],
    )?;
    Ok(())
}

/// The recorded original for one destination, as `(library path, checksum)`.
pub fn backup_for(
    conn: &Connection,
    mod_id: &str,
    destination: &str,
) -> Result<Option<(String, String)>> {
    Ok(conn
        .query_row(
            "SELECT backup_relative,sha256 FROM mod_backups WHERE mod_id=?1 AND destination=?2",
            params![mod_id, destination],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?)
}

pub fn backups(conn: &Connection, mod_id: &str) -> Result<Vec<(String, String, String)>> {
    let mut statement = conn.prepare(
        "SELECT destination,backup_relative,sha256 FROM mod_backups WHERE mod_id=?1 ORDER BY destination",
    )?;
    let rows = statement
        .query_map([mod_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn conflict_count(conn: &Connection) -> Result<usize> {
    let package:i64=conn.query_row("SELECT count(*) FROM (SELECT p.package_id FROM mod_packages p JOIN mods m ON m.id=p.mod_id WHERE m.enabled=1 GROUP BY p.package_id HAVING count(*)>1)",[],|r|r.get(0))?;
    let files:i64=conn.query_row("SELECT count(*) FROM (SELECT f.destination FROM mod_files f JOIN mods m ON m.id=f.mod_id WHERE m.enabled=1 GROUP BY f.destination HAVING count(*)>1)",[],|r|r.get(0))?;
    Ok((package + files) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModSummary;
    use tempfile::tempdir;

    fn summary(id: &str) -> ModSummary {
        ModSummary {
            container_verification: None,
            id: id.into(),
            bundle_id: None,
            name: id.into(),
            version: None,
            mod_type: "iostore".into(),
            enabled: true,
            installed_at: "2026-08-28T00:00:00Z".into(),
            installed_build: None,
            package_count: 0,
            conflict_count: 0,
            potential_conflict_count: 0,
            load_priority: None,
            nexus_mod_id: None,
            nexus_url: None,
            nexus_ignored: false,
            hidden: false,
            fomod: false,
            files: vec![],
        }
    }

    fn add(conn: &mut Connection, id: &str, destination: &str, packages: &[String]) {
        let tx = conn.transaction().unwrap();
        insert_mod(
            &tx,
            &summary(id),
            "",
            None,
            &[(
                format!("{id}.pak"),
                destination.into(),
                1,
                id.repeat(64 / id.len().max(1)),
            )],
            packages,
        )
        .unwrap();
        tx.commit().unwrap();
    }

    #[test]
    fn unverified_container_state_survives_restart_and_is_removed_with_mod() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("db");
        let mut conn = open(&path).unwrap();
        let mut item = summary("unchecked");
        item.container_verification = Some("unavailable".into());
        let tx = conn.transaction().unwrap();
        insert_mod(&tx, &item, "", None, &[], &[]).unwrap();
        tx.commit().unwrap();
        drop(conn);
        let conn = open(&path).unwrap();
        assert_eq!(list_mods(&conn).unwrap()[0].container_verification.as_deref(), Some("unavailable"));
        conn.execute("DELETE FROM mods WHERE id='unchecked'", []).unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM container_checks", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn custom_launch_executable_round_trips_through_settings() {
        let d = tempdir().unwrap();
        let conn = open(&d.path().join("db")).unwrap();
        let expected = AppSettings {
            custom_executable_path: Some("C:\\Games\\ZeroCompany.exe".into()),
            ..AppSettings::default()
        };

        save_settings(&conn, &expected).unwrap();

        assert_eq!(
            settings(&conn).unwrap().custom_executable_path,
            expected.custom_executable_path
        );
    }

    #[test]
    fn overlapping_packages_are_counted_without_names() {
        let d = tempdir().unwrap();
        let mut conn = open(&d.path().join("db")).unwrap();
        add(&mut conn, "a", "/mods/a.pak", &["hashed-package".into()]);
        add(&mut conn, "b", "/mods/b.pak", &["hashed-package".into()]);
        assert_eq!(conflict_count(&conn).unwrap(), 1);
    }

    #[test]
    fn duplicate_deployment_filename_is_counted() {
        let d = tempdir().unwrap();
        let mut conn = open(&d.path().join("db")).unwrap();
        add(&mut conn, "a", "/mods/shared.pak", &[]);
        add(&mut conn, "b", "/mods/shared.pak", &[]);
        assert_eq!(conflict_count(&conn).unwrap(), 1);
    }

    #[test]
    fn unrelated_mods_have_no_conflict() {
        let d = tempdir().unwrap();
        let mut conn = open(&d.path().join("db")).unwrap();
        add(&mut conn, "a", "/mods/a.pak", &["one".into()]);
        add(&mut conn, "b", "/mods/b.pak", &["two".into()]);
        assert_eq!(conflict_count(&conn).unwrap(), 0);
    }

    #[test]
    fn migrates_v1_and_assigns_newer_packaged_mods_higher_priority() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("db");
        let legacy = Connection::open(&path).unwrap();
        legacy.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
          CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
          CREATE TABLE mods(id TEXT PRIMARY KEY, name TEXT NOT NULL, version TEXT, mod_type TEXT NOT NULL, deployment_key TEXT NOT NULL DEFAULT '', source_archive TEXT, installed_at TEXT NOT NULL, enabled INTEGER NOT NULL, installed_build TEXT);
          CREATE TABLE mod_files(id INTEGER PRIMARY KEY, mod_id TEXT NOT NULL REFERENCES mods(id) ON DELETE CASCADE, library_relative TEXT NOT NULL, destination TEXT NOT NULL, size INTEGER NOT NULL, sha256 TEXT NOT NULL);
          CREATE TABLE mod_packages(mod_id TEXT NOT NULL REFERENCES mods(id) ON DELETE CASCADE, package_id TEXT NOT NULL, PRIMARY KEY(mod_id, package_id));
          INSERT INTO schema_migrations VALUES(1,datetime('now'));
          INSERT INTO mods VALUES('old','Old',NULL,'pak','',NULL,'2026-01-01',1,NULL);
          INSERT INTO mods VALUES('new','New',NULL,'iostore','',NULL,'2026-02-01',1,NULL);
          INSERT INTO mods VALUES('lua','Lua',NULL,'ue4ss','',NULL,'2026-03-01',1,NULL);").unwrap();
        drop(legacy);
        let migrated = open(&path).unwrap();
        let priorities = migrated
            .prepare("SELECT id,load_priority FROM mods ORDER BY id")
            .unwrap()
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
            })
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            priorities,
            vec![
                ("lua".into(), None),
                ("new".into(), Some(2)),
                ("old".into(), Some(1))
            ]
        );
    }

    #[test]
    fn hiding_a_mod_keeps_it_installed_and_ordered() {
        let directory = tempdir().unwrap();
        let mut conn = open(&directory.path().join("db.sqlite3")).unwrap();
        add(
            &mut conn,
            "runtime",
            "Content/Paks/~mods/Runtime_P.pak",
            &[],
        );
        assert!(!list_mods(&conn).unwrap()[0].hidden);

        set_hidden(&conn, "runtime", true).unwrap();
        let listed = list_mods(&conn).unwrap();
        // Still listed by the backend, still counted, still enabled: hiding is
        // a view decision the interface makes, not an uninstall.
        assert_eq!(listed.len(), 1);
        assert!(listed[0].hidden);
        assert!(listed[0].enabled);
        assert_eq!(counts(&conn).unwrap(), (1, 1));

        set_hidden(&conn, "runtime", false).unwrap();
        assert!(!list_mods(&conn).unwrap()[0].hidden);
    }
}
