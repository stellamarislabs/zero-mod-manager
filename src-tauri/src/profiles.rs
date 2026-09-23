use crate::{
    config_workbench, database, deployment,
    error::{AppError, Result},
    load_order,
    models::{
        ProfileChange, ProfileDetail, ProfileLock, ProfileLockMod, ProfileModState, ProfileSummary,
        ProfileSwitchPreview, SnapshotSummary,
    },
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

fn validate_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Other("A profile needs a name.".into()));
    }
    if name.chars().count() > 80 {
        return Err(AppError::Other(
            "Use 80 characters or fewer for a profile name.".into(),
        ));
    }
    Ok(name.to_string())
}

pub fn list(conn: &Connection) -> Result<Vec<ProfileSummary>> {
    let mut statement = conn.prepare(
        "SELECT p.id,p.name,p.notes,p.required_runtime,p.created_at,p.updated_at,p.is_active,
                count(DISTINCT CASE WHEN pm.enabled=1 THEN COALESCE(m.bundle_id, pm.mod_id) END),count(DISTINCT COALESCE(m.bundle_id, pm.mod_id))
         FROM profiles p LEFT JOIN profile_mods pm ON pm.profile_id=p.id LEFT JOIN mods m ON m.id=pm.mod_id
         GROUP BY p.id ORDER BY p.is_active DESC,lower(p.name),p.created_at",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(ProfileSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                notes: row.get(2)?,
                required_runtime: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                active: row.get(6)?,
                enabled_mods: row.get::<_, i64>(7)? as usize,
                total_mods: row.get::<_, i64>(8)? as usize,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn summary(conn: &Connection, profile_id: &str) -> Result<ProfileSummary> {
    list(conn)?
        .into_iter()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| AppError::Other("That profile no longer exists.".into()))
}

pub fn detail(conn: &Connection, profile_id: &str) -> Result<ProfileDetail> {
    let summary = summary(conn, profile_id)?;
    let mut statement = conn.prepare(
        "SELECT m.id,m.name,m.mod_type,coalesce(pm.enabled,0),pm.load_priority,pm.fomod_answers
         FROM mods m LEFT JOIN profile_mods pm ON pm.mod_id=m.id AND pm.profile_id=?1
         ORDER BY coalesce(pm.load_priority,9223372036854775807),lower(m.name)",
    )?;
    let mods = statement
        .query_map([profile_id], |row| {
            Ok(ProfileModState {
                mod_id: row.get(0)?,
                name: row.get(1)?,
                mod_type: row.get(2)?,
                enabled: row.get(3)?,
                load_priority: row.get(4)?,
                fomod_answers: row.get(5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(ProfileDetail { summary, mods })
}

pub fn active(conn: &Connection) -> Result<Option<ProfileDetail>> {
    let id = conn
        .query_row("SELECT id FROM profiles WHERE is_active=1", [], |row| {
            row.get::<_, String>(0)
        })
        .optional()?;
    id.map(|id| detail(conn, &id)).transpose()
}

fn capture_mods(conn: &Connection, profile_id: &str) -> Result<()> {
    conn.execute("DELETE FROM profile_mods WHERE profile_id=?1", [profile_id])?;
    conn.execute(
        "INSERT INTO profile_mods(profile_id,mod_id,enabled,load_priority,fomod_answers)
         SELECT ?1,m.id,m.enabled,m.load_priority,f.answers_json
         FROM mods m LEFT JOIN fomod_installs f ON f.mod_id=m.id",
        [profile_id],
    )?;
    conn.execute(
        "UPDATE profiles SET updated_at=datetime('now') WHERE id=?1",
        [profile_id],
    )?;
    Ok(())
}

pub fn capture_active(conn: &Connection) -> Result<()> {
    let active = conn
        .query_row("SELECT id FROM profiles WHERE is_active=1", [], |row| {
            row.get::<_, String>(0)
        })
        .optional()?;
    if let Some(id) = active {
        capture_mods(conn, &id)?;
    }
    Ok(())
}

pub fn create(conn: &Connection, name: &str, notes: &str) -> Result<ProfileDetail> {
    let name = validate_name(name)?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO profiles(id,name,notes,created_at,updated_at,is_active) VALUES(?1,?2,?3,?4,?4,0)",
        params![id, name, notes.trim(), now],
    )?;
    capture_mods(conn, &id)?;
    detail(conn, &id)
}

pub fn update(
    conn: &Connection,
    profile_id: &str,
    name: &str,
    notes: &str,
    required_runtime: Option<&str>,
) -> Result<ProfileDetail> {
    let name = validate_name(name)?;
    let changed = conn.execute(
        "UPDATE profiles SET name=?2,notes=?3,required_runtime=?4,updated_at=datetime('now') WHERE id=?1",
        params![profile_id, name, notes.trim(), required_runtime.filter(|v| !v.trim().is_empty())],
    )?;
    if changed == 0 {
        return Err(AppError::Other("That profile no longer exists.".into()));
    }
    detail(conn, profile_id)
}

pub fn remove(conn: &Connection, profile_id: &str) -> Result<()> {
    if database::get_setting(conn, "pending_restore_profile")?.as_deref() == Some(profile_id) {
        return Err(AppError::Other("This profile is needed to recover a temporary launch. Complete recovery before deleting it.".into()));
    }
    let active: bool = conn
        .query_row(
            "SELECT is_active FROM profiles WHERE id=?1",
            [profile_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::Other("That profile no longer exists.".into()))?;
    if active {
        return Err(AppError::Other(
            "Switch to another profile before deleting the active profile.".into(),
        ));
    }
    conn.execute("DELETE FROM profiles WHERE id=?1", [profile_id])?;
    Ok(())
}

pub fn set_mod_state(
    conn: &Connection,
    profile_id: &str,
    mod_id: &str,
    enabled: bool,
    priority: Option<i64>,
) -> Result<ProfileDetail> {
    summary(conn, profile_id)?;
    conn.execute(
        "INSERT INTO profile_mods(profile_id,mod_id,enabled,load_priority,fomod_answers)
         VALUES(?1,?2,?3,?4,(SELECT answers_json FROM fomod_installs WHERE mod_id=?2))
         ON CONFLICT(profile_id,mod_id) DO UPDATE SET enabled=excluded.enabled,load_priority=excluded.load_priority",
        params![profile_id, mod_id, enabled, priority],
    )?;
    conn.execute(
        "UPDATE profiles SET updated_at=datetime('now') WHERE id=?1",
        [profile_id],
    )?;
    detail(conn, profile_id)
}

struct ProfilePlan {
    profile: ProfileDetail,
    packaged_order: Option<Vec<String>>,
    ue4ss_order: Option<Vec<String>>,
}

/// Preview exactly the priorities the two order writers will record. Packaged
/// priorities run high to low; runtime start slots run low to high, DLLs first.
fn plan(conn: &Connection, profile_id: &str) -> Result<ProfilePlan> {
    let mut profile = detail(conn, profile_id)?;
    let current = database::list_mods(conn)?;
    let order = load_order::state(conn)?;
    let packaged = order
        .entries
        .iter()
        .filter(|entry| entry.supported)
        .collect::<Vec<_>>();
    let runtime = order.ue4ss_entries.iter().collect::<Vec<_>>();
    let changed = |entries: &[&crate::models::LoadOrderEntry]| {
        entries.iter().any(|entry| {
            profile
                .mods
                .iter()
                .any(|target| target.mod_id == entry.id && target.load_priority != entry.priority)
        })
    };
    let packaged_changed = changed(&packaged);
    let runtime_changed = changed(&runtime);
    let requested = profile
        .mods
        .iter()
        .map(|item| (item.mod_id.clone(), item.load_priority))
        .collect::<BTreeMap<_, _>>();
    for target in &profile.mods {
        let source = current.iter().find(|item| item.id == target.mod_id);
        if source.is_some_and(|source| source.load_priority != target.load_priority)
            && !packaged.iter().any(|entry| entry.id == target.mod_id)
            && !runtime.iter().any(|entry| entry.id == target.mod_id)
        {
            return Err(AppError::Other(format!(
                "Load order cannot be changed for {}. Keep its current priority.",
                target.name
            )));
        }
    }
    let packaged_order = if packaged_changed {
        let mut entries = packaged;
        // Stable ties keep the current order instead of guessing from names.
        entries.sort_by(|left, right| requested[&right.id].cmp(&requested[&left.id]));
        let count = entries.len() as i64;
        for (index, entry) in entries.iter().enumerate() {
            if let Some(target) = profile.mods.iter_mut().find(|item| item.mod_id == entry.id) {
                target.load_priority = Some(count - index as i64);
            }
        }
        Some(entries.into_iter().map(|entry| entry.id.clone()).collect())
    } else {
        None
    };
    let ue4ss_order = if runtime_changed {
        let mut entries = runtime;
        entries.sort_by(|left, right| {
            let pass = |kind: Option<&str>| {
                if matches!(kind, Some("native" | "mixed")) {
                    0
                } else {
                    1
                }
            };
            pass(left.runtime_kind.as_deref())
                .cmp(&pass(right.runtime_kind.as_deref()))
                .then_with(|| {
                    requested[&left.id]
                        .unwrap_or(i64::MAX)
                        .cmp(&requested[&right.id].unwrap_or(i64::MAX))
                })
        });
        for (index, entry) in entries.iter().enumerate() {
            if let Some(target) = profile.mods.iter_mut().find(|item| item.mod_id == entry.id) {
                target.load_priority = Some(index as i64 + 1);
            }
        }
        Some(entries.into_iter().map(|entry| entry.id.clone()).collect())
    } else {
        None
    };
    Ok(ProfilePlan {
        profile,
        packaged_order,
        ue4ss_order,
    })
}

pub fn preview(conn: &Connection, profile_id: &str) -> Result<ProfileSwitchPreview> {
    let profile = plan(conn, profile_id)?.profile;
    let current = database::list_mods(conn)?
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let mut changes = Vec::new();
    for target in &profile.mods {
        let Some(source) = current.get(&target.mod_id) else {
            continue;
        };
        if source.enabled != target.enabled || source.load_priority != target.load_priority {
            changes.push(ProfileChange {
                mod_id: target.mod_id.clone(),
                name: target.name.clone(),
                from_enabled: source.enabled,
                to_enabled: target.enabled,
                from_priority: source.load_priority,
                to_priority: target.load_priority,
            });
        }
    }
    let running = deployment::game_is_running();
    Ok(ProfileSwitchPreview {
        profile_id: profile.summary.id,
        profile_name: profile.summary.name,
        changes,
        blocked: running,
        reasons: if running {
            vec!["Close Star Wars: Zero Company before switching profiles.".into()]
        } else {
            Vec::new()
        },
    })
}

pub fn lockfile(
    conn: &Connection,
    profile_id: &str,
    game_build: Option<String>,
) -> Result<ProfileLock> {
    lockfile_from_profile(conn, detail(conn, profile_id)?, game_build)
}

fn lockfile_from_profile(
    conn: &Connection,
    profile: ProfileDetail,
    game_build: Option<String>,
) -> Result<ProfileLock> {
    let mut mods = Vec::new();
    for state in profile.mods {
        let row = conn.query_row(
            "SELECT version,mod_type,nexus_mod_id,nexus_file_id,manifest_id,bundle_id FROM mods WHERE id=?1",
            [&state.mod_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )?;
        let mut hashes =
            conn.prepare("SELECT sha256 FROM mod_files WHERE mod_id=?1 ORDER BY destination")?;
        let file_hashes = hashes
            .query_map([&state.mod_id], |r| r.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        mods.push(ProfileLockMod {
            id: row.4.unwrap_or(state.mod_id),
            bundle_id: row.5,
            name: state.name,
            version: row.0,
            mod_type: row.1,
            enabled: state.enabled,
            load_priority: state.load_priority,
            nexus_mod_id: row.2.map(|v| v as u64),
            nexus_file_id: row.3.map(|v| v as u64),
            file_hashes,
            fomod_answers: state.fomod_answers,
        });
    }
    Ok(ProfileLock {
        schema_version: 1,
        profile_name: profile.summary.name,
        notes: profile.summary.notes,
        required_runtime: profile.summary.required_runtime,
        exported_at: Utc::now().to_rfc3339(),
        game_build,
        mods,
    })
}

/// Current deployed mod state, with profile metadata but without applying or
/// overwriting any edits saved in the profile.
pub fn deployment_lockfile(
    conn: &Connection,
    profile_id: &str,
    game_build: Option<String>,
) -> Result<ProfileLock> {
    let mut profile = detail(conn, profile_id)?;
    let mut statement = conn.prepare(
        "SELECT m.id,m.name,m.mod_type,m.enabled,m.load_priority,f.answers_json
         FROM mods m LEFT JOIN fomod_installs f ON f.mod_id=m.id ORDER BY m.id",
    )?;
    profile.mods = statement
        .query_map([], |row| {
            Ok(ProfileModState {
                mod_id: row.get(0)?,
                name: row.get(1)?,
                mod_type: row.get(2)?,
                enabled: row.get(3)?,
                load_priority: row.get(4)?,
                fomod_answers: row.get(5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let local_ids = profile
        .mods
        .iter()
        .map(|item| item.mod_id.clone())
        .collect::<Vec<_>>();
    let mut lock = lockfile_from_profile(conn, profile, game_build)?;
    // Session/checkpoint evidence identifies actual local components even when
    // multiple components share one author manifest identity.
    for (item, local_id) in lock.mods.iter_mut().zip(local_ids) {
        item.id = local_id;
    }
    Ok(lock)
}

pub fn snapshot(
    conn: &Connection,
    label: &str,
    kind: &str,
    last_known_good: bool,
    game_build: Option<String>,
) -> Result<SnapshotSummary> {
    let active =
        active(conn)?.ok_or_else(|| AppError::Other("No active profile exists.".into()))?;
    let active_id = active.summary.id;
    let payload =
        serde_json::to_string_pretty(&deployment_lockfile(conn, &active_id, game_build)?)?;
    let id = Uuid::new_v4().to_string();
    if last_known_good {
        conn.execute("UPDATE snapshots SET last_known_good=0", [])?;
    }
    let created_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO snapshots(id,profile_id,label,kind,payload_json,created_at,last_known_good) VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![id, active_id, label.trim(), kind, payload, created_at, last_known_good],
    )?;
    Ok(SnapshotSummary {
        id,
        profile_id: Some(active_id),
        label: label.trim().to_string(),
        kind: kind.to_string(),
        created_at,
        last_known_good,
    })
}

pub fn snapshots(conn: &Connection) -> Result<Vec<SnapshotSummary>> {
    let mut statement = conn.prepare(
        "SELECT id,profile_id,label,kind,created_at,last_known_good FROM snapshots ORDER BY created_at DESC LIMIT 100",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(SnapshotSummary {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                label: row.get(2)?,
                kind: row.get(3)?,
                created_at: row.get(4)?,
                last_known_good: row.get(5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn restore_snapshot(
    conn: &mut Connection,
    library: &Path,
    game: &Path,
    journal: &Path,
    snapshot_id: &str,
    game_build: Option<String>,
) -> Result<ProfileDetail> {
    let (label, payload) = conn
        .query_row(
            "SELECT label,payload_json FROM snapshots WHERE id=?1",
            [snapshot_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::Other("That checkpoint no longer exists.".into()))?;
    let lock: ProfileLock = serde_json::from_str(&payload)?;
    let mut restored = import_lock(conn, &lock)?;
    let restored_name = validate_name(&format!(
        "{} (Restored)",
        label.chars().take(60).collect::<String>()
    ))?;
    conn.execute(
        "UPDATE profiles SET name=?2 WHERE id=?1",
        params![restored.summary.id, restored_name],
    )?;
    restored = activate(
        conn,
        library,
        game,
        journal,
        &restored.summary.id,
        game_build,
    )?;
    Ok(restored)
}

pub fn activate(
    conn: &mut Connection,
    library: &Path,
    game: &Path,
    journal: &Path,
    profile_id: &str,
    game_build: Option<String>,
) -> Result<ProfileDetail> {
    deployment::ensure_game_stopped()?;
    let plan = plan(conn, profile_id)?;
    let target = plan.profile;
    let original = database::list_mods(conn)?;
    let original_order = load_order::state(conn)?;
    snapshot(
        conn,
        &format!("Before switching to {}", target.summary.name),
        "profile-switch",
        false,
        game_build,
    )?;

    let config_paths = {
        let mut statement =
            conn.prepare("SELECT DISTINCT path FROM config_patches ORDER BY path")?;
        let paths = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        paths
    };
    let mut desired_configs = Vec::<(PathBuf, String)>::new();
    for path in config_paths {
        let patch = conn.query_row(
            "SELECT patch_json FROM config_patches WHERE profile_id=?1 AND path=?2 ORDER BY created_at DESC LIMIT 1",
            params![profile_id, path], |row| row.get::<_, String>(0),
        ).optional()?;
        let content = if let Some(patch) = patch {
            serde_json::from_str::<serde_json::Value>(&patch)?
                .get("content")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        } else {
            let backup = conn.query_row(
                "SELECT backup_path FROM config_patches WHERE path=?1 AND backup_path IS NOT NULL ORDER BY created_at ASC LIMIT 1",
                [&path], |row| row.get::<_, String>(0),
            ).optional()?;
            backup.map(fs::read_to_string).transpose()?
        };
        if let Some(content) = content {
            desired_configs.push((PathBuf::from(path), content));
        }
    }
    let mut config_originals = Vec::<(PathBuf, Vec<u8>)>::new();
    let mut packaged_attempted = false;
    let mut runtime_attempted = false;

    let apply_result = (|| -> Result<()> {
        for (path, content) in &desired_configs {
            let original = config_workbench::apply_profile_content(game, path, content)?;
            config_originals.push((path.clone(), original));
        }
        for state in &target.mods {
            let current = original.iter().find(|item| item.id == state.mod_id);
            if current.is_some_and(|item| item.enabled != state.enabled) {
                deployment::set_enabled(conn, library, game, &state.mod_id, state.enabled, false)?;
            }
        }
        if let Some(ordered) = &plan.packaged_order {
            packaged_attempted = true;
            load_order::apply(conn, ordered, journal)?;
        }
        if let Some(ordered) = &plan.ue4ss_order {
            runtime_attempted = true;
            load_order::apply_ue4ss_order(conn, game, ordered)?;
        }
        Ok(())
    })();

    if let Err(error) = apply_result {
        let mut recovery_errors = Vec::new();
        for (path, content) in config_originals.iter().rev() {
            if let Err(error) = config_workbench::restore_profile_content(game, path, content) {
                recovery_errors.push(error.to_string());
            }
        }
        for item in &original {
            match database::mod_record(conn, &item.id) {
                Ok(record) if record.enabled != item.enabled => {
                    if let Err(error) =
                        deployment::set_enabled(conn, library, game, &item.id, item.enabled, true)
                    {
                        recovery_errors.push(error.to_string());
                    }
                }
                Err(error) => recovery_errors.push(error.to_string()),
                _ => {}
            }
        }
        if packaged_attempted {
            if let Err(error) = load_order::apply(
                conn,
                &original_order
                    .entries
                    .iter()
                    .filter(|entry| entry.supported)
                    .map(|item| item.id.clone())
                    .collect::<Vec<_>>(),
                journal,
            ) {
                recovery_errors.push(error.to_string());
            }
        }
        if runtime_attempted {
            if let Err(error) = load_order::apply_ue4ss_order(
                conn,
                game,
                &original_order
                    .ue4ss_entries
                    .iter()
                    .map(|item| item.id.clone())
                    .collect::<Vec<_>>(),
            ) {
                recovery_errors.push(error.to_string());
            }
        }
        let recovery = if recovery_errors.is_empty() {
            "Restoration was attempted. Review Health before launching.".to_string()
        } else {
            format!("Restoration also reported: {}", recovery_errors.join("; "))
        };
        return Err(AppError::Other(format!(
            "Profile switch failed: {error}. {recovery}"
        )));
    }

    let transaction = conn.transaction()?;
    transaction.execute("UPDATE profiles SET is_active=0", [])?;
    transaction.execute(
        "UPDATE profiles SET is_active=1,updated_at=datetime('now') WHERE id=?1",
        [profile_id],
    )?;
    capture_mods(&transaction, profile_id)?;
    transaction.commit()?;
    detail(conn, profile_id)
}

pub fn import_lock(conn: &Connection, lock: &ProfileLock) -> Result<ProfileDetail> {
    if lock.schema_version != 1 {
        return Err(AppError::Other(format!(
            "Profile Lock schema {} is not supported.",
            lock.schema_version
        )));
    }
    let imported = create(
        conn,
        &format!("{} (Imported)", lock.profile_name),
        &lock.notes,
    )?;
    conn.execute(
        "DELETE FROM profile_mods WHERE profile_id=?1",
        [&imported.summary.id],
    )?;
    for item in &lock.mods {
        let installed = conn.query_row(
            "SELECT id FROM mods WHERE id=?1 OR manifest_id=?1 OR (?2 IS NOT NULL AND nexus_mod_id=?2) ORDER BY id=?1 DESC,manifest_id=?1 DESC LIMIT 1",
            params![item.id, item.nexus_mod_id.map(|v| v as i64)],
            |row| row.get::<_, String>(0),
        ).optional()?;
        if let Some(mod_id) = installed {
            conn.execute(
                "INSERT INTO profile_mods(profile_id,mod_id,enabled,load_priority,fomod_answers) VALUES(?1,?2,?3,?4,?5)",
                params![imported.summary.id, mod_id, item.enabled, item.load_priority, item.fomod_answers],
            )?;
        }
    }
    conn.execute(
        "UPDATE profiles SET required_runtime=?2,updated_at=datetime('now') WHERE id=?1",
        params![imported.summary.id, lock.required_runtime],
    )?;
    detail(conn, &imported.summary.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn first_database_has_an_active_default_profile() {
        let dir = tempdir().unwrap();
        let conn = database::open(&dir.path().join("profiles.sqlite3")).unwrap();
        let profiles = list(&conn).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "Default");
        assert!(profiles[0].active);
    }

    #[test]
    fn counts_only_explicit_bundles_not_reused_archive_names() {
        let dir = tempdir().unwrap();
        let conn = database::open(&dir.path().join("profiles.sqlite3")).unwrap();
        for (id, bundle, enabled) in [
            ("one", Some("bundle"), true),
            ("two", Some("bundle"), false),
            ("three", None, true),
            ("four", None, true),
        ] {
            conn.execute("INSERT INTO mods(id,name,mod_type,installed_at,enabled,bundle_id,source_archive) VALUES(?1,'Mod','pak','2026-09-22',?2,?3,'same.zip')",
                params![id, enabled, bundle]).unwrap();
        }
        capture_active(&conn).unwrap();
        let profile = active(&conn).unwrap().unwrap();
        assert_eq!(profile.summary.total_mods, 3);
        assert_eq!(profile.summary.enabled_mods, 3);
        assert_eq!(database::counts(&conn).unwrap(), (3, 3));
    }

    #[test]
    fn removal_preserves_active_and_recovery_profiles() {
        let dir = tempdir().unwrap();
        let conn = database::open(&dir.path().join("profiles.sqlite3")).unwrap();
        let current = active(&conn).unwrap().unwrap();
        assert!(remove(&conn, &current.summary.id).is_err());
        let other = create(&conn, "Other", "").unwrap();
        database::set_setting(&conn, "pending_restore_profile", &other.summary.id).unwrap();
        assert!(remove(&conn, &other.summary.id).is_err());
        assert!(detail(&conn, &other.summary.id).is_ok());
        database::delete_setting(&conn, "pending_restore_profile").unwrap();
        remove(&conn, &other.summary.id).unwrap();
        assert_eq!(list(&conn).unwrap().len(), 1);
    }

    #[test]
    fn profile_names_are_required_and_bounded() {
        assert!(validate_name("  ").is_err());
        assert!(validate_name(&"x".repeat(81)).is_err());
        assert_eq!(validate_name(" Campaign ").unwrap(), "Campaign");
    }

    #[test]
    fn profile_config_writes_participate_in_package_rollback() {
        let dir = tempdir().unwrap();
        let game = dir.path().join("game");
        let library = dir.path().join("library");
        let config = game.join("SWZeroCompany/Binaries/Win64/ue4ss/Mods/settings.ini");
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        fs::create_dir_all(&library).unwrap();
        fs::write(&config, b"[System]\nValue=original\n").unwrap();
        let mut conn = database::open(&dir.path().join("db")).unwrap();
        let result: Result<()> =
            crate::package_transaction::run(&mut conn, dir.path(), &library, &game, |_conn| {
                config_workbench::apply_profile_content(
                    &game,
                    &config,
                    "[System]\nValue=changed\n",
                )?;
                Err(AppError::Other("Injected later profile failure".into()))
            });
        assert!(result.is_err());
        assert_eq!(fs::read(&config).unwrap(), b"[System]\nValue=original\n");
        assert!(!config.with_extension("ini.zmm-new").exists());
        assert!(!fs::read_dir(config.parent().unwrap())
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("zmm-old")));
    }

    #[test]
    fn snapshots_record_deployed_state_without_overwriting_profile_edits() {
        let dir = tempdir().unwrap();
        let conn = database::open(&dir.path().join("db")).unwrap();
        conn.execute("INSERT INTO mods(id,name,mod_type,installed_at,enabled,load_priority,manifest_id) VALUES('local','Armor','iostore','2026-09-22',1,2,'author.identity')", []).unwrap();
        capture_active(&conn).unwrap();
        let profile = active(&conn).unwrap().unwrap();
        set_mod_state(&conn, &profile.summary.id, "local", false, Some(9)).unwrap();
        let saved = snapshot(&conn, "Current deployment", "manual", false, None).unwrap();
        let payload: String = conn
            .query_row(
                "SELECT payload_json FROM snapshots WHERE id=?1",
                [&saved.id],
                |row| row.get(0),
            )
            .unwrap();
        let lock: ProfileLock = serde_json::from_str(&payload).unwrap();
        assert_eq!(lock.mods[0].id, "local");
        assert!(lock.mods[0].enabled);
        assert_eq!(lock.mods[0].load_priority, Some(2));
        let draft = detail(&conn, &profile.summary.id).unwrap();
        assert!(!draft.mods[0].enabled);
        assert_eq!(draft.mods[0].load_priority, Some(9));
        assert_eq!(
            lockfile(&conn, &profile.summary.id, None).unwrap().mods[0].id,
            "author.identity"
        );
    }

    fn install_container(
        conn: &mut Connection,
        root: &Path,
        library: &Path,
        game: &Path,
        name: &str,
    ) -> crate::models::ModSummary {
        let source = root.join(name);
        fs::create_dir_all(&source).unwrap();
        let files = ["pak", "utoc", "ucas"]
            .into_iter()
            .map(|extension| {
                let filename = format!("{name}_P.{extension}");
                let path = source.join(&filename);
                fs::write(&path, format!("{name}-{extension}")).unwrap();
                crate::models::PayloadFile {
                    source: path,
                    library_relative: filename.clone().into(),
                    destination_relative: filename.into(),
                }
            })
            .collect();
        deployment::install(
            conn,
            library,
            game,
            &crate::models::StagedMod {
                staging_id: name.into(),
                staging_root: source.clone(),
                source_archive: source.display().to_string(),
                name: name.into(),
                version: None,
                author: None,
                description: None,
                manifest: None,
                mod_type: "iostore".into(),
                deployment_keys: Vec::new(),
                files,
                packages: Vec::new(),
                verification: "not-required".into(),
                fomod_source_root: None,
                fomod_answers: None,
            },
            None,
        )
        .unwrap()
    }

    #[test]
    fn profile_preview_and_activation_agree_on_normalized_highest_first_order() {
        let dir = tempdir().unwrap();
        let game = dir.path().join("game");
        let library = dir.path().join("library");
        fs::create_dir_all(game.join("SWZeroCompany/Content/Paks")).unwrap();
        fs::create_dir_all(&library).unwrap();
        let mut conn = database::open(&dir.path().join("db")).unwrap();
        let a = install_container(&mut conn, dir.path(), &library, &game, "Alpha");
        let b = install_container(&mut conn, dir.path(), &library, &game, "Bravo");
        conn.execute("INSERT INTO mods(id,name,mod_type,installed_at,enabled) VALUES('plain','Plain PAK','pak','2026-09-22',1)", []).unwrap();
        capture_active(&conn).unwrap();
        let target = create(&conn, "Reversed", "").unwrap();
        set_mod_state(&conn, &target.summary.id, &a.id, true, Some(99)).unwrap();
        set_mod_state(&conn, &target.summary.id, &b.id, true, Some(5)).unwrap();
        let review = preview(&conn, &target.summary.id).unwrap();
        assert_eq!(
            review
                .changes
                .iter()
                .find(|item| item.mod_id == a.id)
                .unwrap()
                .to_priority,
            Some(2)
        );
        assert_eq!(
            review
                .changes
                .iter()
                .find(|item| item.mod_id == b.id)
                .unwrap()
                .to_priority,
            Some(1)
        );
        let applied = activate(
            &mut conn,
            &library,
            &game,
            &dir.path().join("journal.json"),
            &target.summary.id,
            None,
        )
        .unwrap();
        assert_eq!(
            database::mod_record(&conn, &a.id).unwrap().load_priority,
            Some(2)
        );
        assert_eq!(
            database::mod_record(&conn, &b.id).unwrap().load_priority,
            Some(1)
        );
        assert_eq!(
            applied
                .mods
                .iter()
                .find(|item| item.mod_id == a.id)
                .unwrap()
                .load_priority,
            Some(2)
        );
        assert!(preview(&conn, &target.summary.id)
            .unwrap()
            .changes
            .is_empty());
        assert!(game
            .join("SWZeroCompany/Content/Paks/~mods/Alpha_0002_P.pak")
            .exists());
        assert!(game
            .join("SWZeroCompany/Content/Paks/~mods/Bravo_0001_P.pak")
            .exists());
    }

    #[test]
    fn unchanged_profile_order_does_not_rename_adopted_layouts() {
        let dir = tempdir().unwrap();
        let game = dir.path().join("game");
        let library = dir.path().join("library");
        fs::create_dir_all(game.join("SWZeroCompany/Content/Paks")).unwrap();
        fs::create_dir_all(&library).unwrap();
        let mut conn = database::open(&dir.path().join("db")).unwrap();
        let a = install_container(&mut conn, dir.path(), &library, &game, "Adopted");
        for file in &a.files {
            let before = PathBuf::from(&file.destination);
            let after = before.with_file_name(file.name.replace("_0001_P.", "_P."));
            fs::rename(&before, &after).unwrap();
            conn.execute(
                "UPDATE mod_files SET destination=?1 WHERE mod_id=?2 AND destination=?3",
                params![after.display().to_string(), a.id, file.destination],
            )
            .unwrap();
        }
        conn.execute("UPDATE mods SET load_priority=NULL WHERE id=?1", [&a.id])
            .unwrap();
        capture_active(&conn).unwrap();
        let target = create(&conn, "Same deployment", "").unwrap();
        assert!(preview(&conn, &target.summary.id)
            .unwrap()
            .changes
            .is_empty());
        activate(
            &mut conn,
            &library,
            &game,
            &dir.path().join("journal.json"),
            &target.summary.id,
            None,
        )
        .unwrap();
        assert_eq!(
            database::mod_record(&conn, &a.id).unwrap().load_priority,
            None
        );
        assert!(game
            .join("SWZeroCompany/Content/Paks/~mods/Adopted_P.pak")
            .exists());
    }

    #[test]
    fn runtime_preview_honors_native_pass_and_ascending_start_slots() {
        let dir = tempdir().unwrap();
        let conn = database::open(&dir.path().join("db")).unwrap();
        for (id, path, priority) in [
            ("lua-a", "A/Scripts/main.lua", 1),
            ("native", "N/dlls/main.dll", 2),
            ("lua-b", "B/Scripts/main.lua", 3),
        ] {
            conn.execute("INSERT INTO mods(id,name,mod_type,installed_at,enabled,load_priority) VALUES(?1,?1,'ue4ss','2026-09-22',1,?2)", params![id, priority]).unwrap();
            conn.execute("INSERT INTO mod_files(mod_id,library_relative,destination,size,sha256) VALUES(?1,?2,?2,0,'hash')", params![id, path]).unwrap();
        }
        capture_active(&conn).unwrap();
        let target = create(&conn, "Runtime order", "").unwrap();
        set_mod_state(&conn, &target.summary.id, "lua-b", true, Some(1)).unwrap();
        set_mod_state(&conn, &target.summary.id, "lua-a", true, Some(3)).unwrap();
        set_mod_state(&conn, &target.summary.id, "native", true, Some(9)).unwrap();
        let planned = plan(&conn, &target.summary.id).unwrap();
        assert_eq!(
            planned.ue4ss_order.unwrap(),
            vec!["native", "lua-b", "lua-a"]
        );
        let review = preview(&conn, &target.summary.id).unwrap();
        assert_eq!(
            review
                .changes
                .iter()
                .find(|item| item.mod_id == "native")
                .unwrap()
                .to_priority,
            Some(1)
        );
        assert_eq!(
            review
                .changes
                .iter()
                .find(|item| item.mod_id == "lua-b")
                .unwrap()
                .to_priority,
            Some(2)
        );
        assert_eq!(
            review
                .changes
                .iter()
                .find(|item| item.mod_id == "lua-a")
                .unwrap()
                .to_priority,
            Some(3)
        );
    }

    #[test]
    fn profile_switch_applies_and_restores_config_layers() {
        let dir = tempdir().unwrap();
        let game = dir.path().join("game");
        let config = game.join("SWZeroCompany/Binaries/Win64/ue4ss/Mods/settings.ini");
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        fs::write(&config, "[System]\nValue=0\n").unwrap();
        let library = dir.path().join("library");
        fs::create_dir_all(&library).unwrap();
        let mut conn = database::open(&dir.path().join("profiles.sqlite3")).unwrap();
        let default = active(&conn).unwrap().unwrap();
        let current = config_workbench::read(&game, &config).unwrap();
        config_workbench::apply(
            &conn,
            &game,
            dir.path(),
            &config,
            "[System]\nValue=1\n",
            &current.sha256,
        )
        .unwrap();

        let alternate = create(&conn, "Alternate", "").unwrap();
        activate(
            &mut conn,
            &library,
            &game,
            &dir.path().join("journal.json"),
            &alternate.summary.id,
            None,
        )
        .unwrap();
        assert!(fs::read_to_string(&config).unwrap().contains("Value=0"));
        let current = config_workbench::read(&game, &config).unwrap();
        config_workbench::apply(
            &conn,
            &game,
            dir.path(),
            &config,
            "[System]\nValue=2\n",
            &current.sha256,
        )
        .unwrap();

        activate(
            &mut conn,
            &library,
            &game,
            &dir.path().join("journal.json"),
            &default.summary.id,
            None,
        )
        .unwrap();
        assert!(fs::read_to_string(&config).unwrap().contains("Value=1"));
        activate(
            &mut conn,
            &library,
            &game,
            &dir.path().join("journal.json"),
            &alternate.summary.id,
            None,
        )
        .unwrap();
        assert!(fs::read_to_string(&config).unwrap().contains("Value=2"));
    }
}
