//! Explicit organization of existing records. No deployment writes.
use crate::{
    database,
    deployment::sha256,
    error::{AppError, Result},
    models::ProfileLock,
    package_transaction,
};
use rusqlite::{params, Connection};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Component, Path},
};
use uuid::Uuid;

fn error(message: &str) -> AppError {
    AppError::Other(message.into())
}

// Retained for older clients. Arbitrary grouping provides no proof that entries
// belong to one mod and can make a remove/update action affect unrelated mods.
pub fn group(_conn: &mut Connection, _ids: &[String], _name: &str) -> Result<()> {
    Err(error("Manual merging is no longer supported. Existing mods stay separate; install an original bundle archive to retain its components."))
}

/// Separate existing component records without touching payloads, state or order.
pub fn ungroup(conn: &mut Connection, id: &str) -> Result<usize> {
    let mods = database::list_mods(conn)?;
    let anchor = mods
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| error("That mod no longer exists."))?;
    let bundle = anchor
        .bundle_id
        .as_ref()
        .ok_or_else(|| error("This mod is already separate."))?;
    let ids: HashSet<_> = mods
        .iter()
        .filter(|item| item.bundle_id.as_ref() == Some(bundle))
        .map(|item| item.id.as_str())
        .collect();
    let tx = conn.transaction()?;
    // Checkpoints retain membership and state; their presentation identity must
    // agree with the newly separated records too.
    let mut snapshots = Vec::new();
    {
        let mut statement = tx.prepare("SELECT id,payload_json FROM snapshots")?;
        for row in
            statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        {
            let (snapshot_id, json) = row?;
            let mut lock: ProfileLock = serde_json::from_str(&json)?;
            let mut changed = false;
            for item in &mut lock.mods {
                if ids.contains(item.id.as_str()) {
                    item.bundle_id = None;
                    changed = true;
                }
            }
            if changed {
                snapshots.push((snapshot_id, serde_json::to_string(&lock)?));
            }
        }
    }
    for id in &ids {
        tx.execute(
            "UPDATE mods SET bundle_id=NULL,bundle_name=NULL WHERE id=?1",
            [id],
        )?;
    }
    for (snapshot, json) in snapshots {
        tx.execute(
            "UPDATE snapshots SET payload_json=?1 WHERE id=?2",
            params![json, snapshot],
        )?;
    }
    tx.commit()?;
    Ok(ids.len())
}

type FileRow = (String, String, u64, String);

/// Legacy adoption flattened multiple container families into one row. Restore
/// only unambiguous additive packaged records; unknown provenance is not guessed.
pub fn restore_components(
    conn: &mut Connection,
    library: &Path,
    game: &Path,
    id: &str,
) -> Result<usize> {
    let old = database::list_mods(conn)?
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| error("That mod no longer exists."))?;
    let eligible: bool = conn.query_row("SELECT source_archive IS NULL AND manifest_json IS NULL AND mod_type IN ('pak','iostore') AND NOT EXISTS(SELECT 1 FROM mod_backups WHERE mod_id=mods.id) AND NOT EXISTS(SELECT 1 FROM mod_packages WHERE mod_id=mods.id) AND NOT EXISTS(SELECT 1 FROM fomod_installs WHERE mod_id=mods.id) FROM mods WHERE id=?1", [id], |r| r.get(0))?;
    if !eligible {
        return Err(error(
            "This is not a safely separable legacy adoption. Its records have been left unchanged.",
        ));
    }
    if conn.query_row("SELECT EXISTS(SELECT 1 FROM compatibility_rules WHERE subject_mod_id=?1 OR target_mod_id=?1)", [id], |r| r.get::<_,bool>(0))? {
        return Err(error("This record has compatibility rules that cannot be assigned to individual components safely. Nothing changed."));
    }
    if database::get_setting(conn, "pending_restore_profile")?.is_some() {
        return Err(error("Restore the temporary launch profile first."));
    }
    let rows = database::file_records(conn, id)?;
    let root = game.join("SWZeroCompany/Content/Paks/~mods");
    let mut families: BTreeMap<String, Vec<FileRow>> = BTreeMap::new();
    for row in rows {
        let relative = Path::new(&row.0);
        let destination = Path::new(&row.1);
        if relative.components().count() != 1
            || !matches!(relative.components().next(), Some(Component::Normal(_)))
            || destination.parent() != Some(root.as_path())
        {
            return Err(error("A file has an unexpected location. Nothing changed."));
        }
        let ext = relative
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "pak" | "utoc" | "ucas") {
            return Err(error("An unknown file prevents safe component recovery."));
        }
        let source = library.join(id).join("payload").join(relative);
        // Enroll before copying: this validates links and ensures a durable backup.
        package_transaction::protect_tree(&library.join(id))?;
        if sha256(&source)? != row.3 {
            return Err(error(
                "A managed copy changed. Restore the original copy before recovering components.",
            ));
        }
        if destination.exists()
            && (fs::symlink_metadata(destination)?.file_type().is_symlink()
                || sha256(destination)? != row.3)
        {
            return Err(error(
                "A deployed file changed. No component records were changed.",
            ));
        }
        if old.enabled && !destination.is_file() {
            return Err(error(
                "An enabled component is missing from the game. Nothing changed.",
            ));
        }
        let stem = relative
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_ascii_lowercase();
        families.entry(stem).or_default().push(row);
    }
    if families.len() < 2 || families.len() > 64 {
        return Err(error(
            "This mod does not contain multiple recoverable container families.",
        ));
    }
    for files in families.values() {
        let extensions: HashSet<_> = files
            .iter()
            .map(|r| {
                Path::new(&r.0)
                    .extension()
                    .unwrap()
                    .to_string_lossy()
                    .to_ascii_lowercase()
            })
            .collect();
        if extensions.len() != files.len()
            || extensions.contains("utoc") != extensions.contains("ucas")
        {
            return Err(error(
                "An incomplete or ambiguous container family prevents recovery.",
            ));
        }
    }
    let mut replacements = Vec::new();
    for (index, (stem, files)) in families.iter().enumerate() {
        let new_id = if index == 0 {
            id.to_string()
        } else {
            Uuid::new_v4().to_string()
        };
        let component_name = crate::mods::naming::display_name(stem);
        if index > 0 {
            let target = library.join(&new_id);
            package_transaction::protect_tree(&target)?;
            fs::create_dir_all(target.join("payload"))?;
            for row in files {
                let dest = target.join("payload").join(&row.0);
                fs::copy(library.join(id).join("payload").join(&row.0), &dest)?;
                if sha256(&dest)? != row.3 {
                    return Err(error("Component copy verification failed."));
                }
                fs::OpenOptions::new().write(true).open(&dest)?.sync_all()?;
            }
        }
        replacements.push((new_id, component_name, files));
    }
    // Keep all profile membership and in-app checkpoints valid. Existing session
    // history is evidence of the old state and is deliberately not rewritten.
    let mut snapshots = Vec::new();
    let mut statement = conn.prepare("SELECT id,payload_json FROM snapshots")?;
    for row in statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (snapshot_id, json) = row?;
        let mut lock: ProfileLock = serde_json::from_str(&json)?;
        if let Some(index) = lock.mods.iter().position(|m| m.id == id) {
            let original = lock.mods.remove(index);
            let expected: HashSet<_> = old.files.iter().map(|f| f.sha256.as_str()).collect();
            if original
                .file_hashes
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>()
                != expected
            {
                return Err(error("An older checkpoint refers to different files. Keep this record intact or update from the original archive."));
            }
            for (offset, (new_id, component_name, files)) in replacements.iter().enumerate() {
                let mut item = original.clone();
                item.id = new_id.clone();
                item.name = component_name.clone();
                item.bundle_id = None;
                item.mod_type = if files
                    .iter()
                    .any(|f| f.0.to_ascii_lowercase().ends_with(".utoc"))
                {
                    "iostore"
                } else {
                    "pak"
                }
                .into();
                item.file_hashes = files.iter().map(|f| f.3.clone()).collect();
                lock.mods.insert(index + offset, item);
            }
            snapshots.push((snapshot_id, serde_json::to_string(&lock)?));
        }
    }
    drop(statement);
    let tx = conn.transaction()?;
    for (index, (new_id, component_name, files)) in replacements.iter().enumerate() {
        let mod_type = if files
            .iter()
            .any(|f| f.0.to_ascii_lowercase().ends_with(".utoc"))
        {
            "iostore"
        } else {
            "pak"
        };
        if index > 0 {
            tx.execute("INSERT INTO mods(id,name,version,mod_type,deployment_key,source_archive,installed_at,enabled,installed_build,load_priority,nexus_mod_id,nexus_file_id,hidden,nexus_ignored,bundle_id,bundle_name) SELECT ?1,?2,version,?3,'',NULL,installed_at,enabled,installed_build,load_priority,nexus_mod_id,nexus_file_id,hidden,nexus_ignored,NULL,NULL FROM mods WHERE id=?4",params![new_id,component_name,mod_type,id])?;
            tx.execute("INSERT INTO profile_mods(profile_id,mod_id,enabled,load_priority,fomod_answers) SELECT profile_id,?1,enabled,load_priority,fomod_answers FROM profile_mods WHERE mod_id=?2",params![new_id,id])?;
            for row in *files {
                tx.execute(
                    "UPDATE mod_files SET mod_id=?1 WHERE mod_id=?2 AND library_relative=?3",
                    params![new_id, id, row.0],
                )?;
            }
        } else {
            tx.execute(
                "UPDATE mods SET name=?1,mod_type=?2,bundle_id=NULL,bundle_name=NULL WHERE id=?3",
                params![component_name, mod_type, id],
            )?;
        }
    }
    for (snapshot, json) in snapshots {
        tx.execute(
            "UPDATE snapshots SET payload_json=?1 WHERE id=?2",
            params![json, snapshot],
        )?;
    }
    tx.commit()?;
    for (_, _, files) in replacements.iter().skip(1) {
        for row in *files {
            fs::remove_file(library.join(id).join("payload").join(&row.0))?;
        }
    }
    Ok(replacements.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arbitrary_grouping_is_rejected_without_changing_records() {
        let temp = tempfile::tempdir().unwrap();
        let mut conn = database::open(&temp.path().join("db")).unwrap();
        for id in ["a", "b", "c", "d", "e"] {
            conn.execute("INSERT INTO mods(id,name,mod_type,installed_at,enabled) VALUES(?1,?1,'pak','now',1)",[id]).unwrap();
        }
        assert!(group(
            &mut conn,
            &["a".into(), "b".into(), "c".into(), "d".into(), "e".into()],
            "Armor"
        )
        .is_err());
        assert_eq!(database::counts(&conn).unwrap(), (5, 5));
    }

    #[test]
    fn ungroup_preserves_state_order_files_profiles_and_cannot_regroup_by_archive_path() {
        let temp = tempfile::tempdir().unwrap();
        let mut conn = database::open(&temp.path().join("db")).unwrap();
        for (id, enabled, rank) in [
            ("a", true, 3),
            ("b", false, 7),
            ("c", true, 9),
            ("d", false, 11),
            ("e", true, 13),
        ] {
            conn.execute("INSERT INTO mods(id,name,mod_type,installed_at,enabled,load_priority,bundle_id,bundle_name,source_archive) VALUES(?1,?1,'pak','now',?2,?3,'wrong-group','Armor','same-reused-path.zip')",params![id,enabled,rank]).unwrap();
            conn.execute("INSERT INTO mod_files(mod_id,library_relative,destination,size,sha256) VALUES(?1,?1,?1,3,'hash')",[id]).unwrap();
        }
        let profile = crate::profiles::create(&conn, "Campaign", "").unwrap();
        conn.execute("UPDATE profiles SET is_active=0", []).unwrap();
        conn.execute(
            "UPDATE profiles SET is_active=1 WHERE id=?1",
            [&profile.summary.id],
        )
        .unwrap();
        crate::profiles::capture_active(&conn).unwrap();
        let snapshot = crate::profiles::snapshot(&conn, "Before", "manual", true, None).unwrap();
        let before = database::list_mods(&conn).unwrap();
        assert_eq!(database::counts(&conn).unwrap(), (1, 1));
        assert_eq!(ungroup(&mut conn, "a").unwrap(), 5);
        let after = database::list_mods(&conn).unwrap();
        assert_eq!(database::counts(&conn).unwrap(), (5, 3));
        for item in &after {
            let old = before.iter().find(|old| old.id == item.id).unwrap();
            assert_eq!(old.enabled, item.enabled);
            assert_eq!(old.load_priority, item.load_priority);
            assert_eq!(old.files.len(), item.files.len());
            assert!(item.bundle_id.is_none() && item.bundle_name.is_none());
        }
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM profile_mods WHERE profile_id=?1",
                [&profile.summary.id],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            5
        );
        let json: String = conn
            .query_row(
                "SELECT payload_json FROM snapshots WHERE id=?1",
                [snapshot.id],
                |r| r.get(0),
            )
            .unwrap();
        let lock: ProfileLock = serde_json::from_str(&json).unwrap();
        assert_eq!(lock.mods.len(), 5);
        assert!(lock.mods.iter().all(|item| item.bundle_id.is_none()));
        assert!(ungroup(&mut conn, "a").is_err());
    }
}
