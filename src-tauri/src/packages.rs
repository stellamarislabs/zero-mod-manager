use crate::{
    deployment,
    error::{AppError, Result},
    models::{ModSummary, StagedMod},
};
use rusqlite::{params, Connection};
use std::{collections::BTreeMap, path::Path};

/// Replaces exactly the package the user approved. A caller must wrap this
/// in package_transaction::run, including metadata/order/profile finalization.
#[allow(clippy::too_many_arguments)] // Explicit transaction inputs; no hidden global state.
pub fn deploy(
    conn: &mut Connection,
    library: &Path,
    game: &Path,
    staged: &[StagedMod],
    old: &[ModSummary],
    matched: &[Option<String>],
    build: Option<String>,
    bundle: &str,
) -> Result<Vec<ModSummary>> {
    let ids = old.iter().map(|m| m.id.clone()).collect::<Vec<_>>();
    deployment::validate_removal(conn, &ids)?;
    type ProfileRow = (String, bool, Option<i64>, Option<String>);
    let mut memberships: BTreeMap<String, Vec<ProfileRow>> = BTreeMap::new();
    let mut package_profiles: BTreeMap<String, bool> = BTreeMap::new();
    for item in old {
        let mut query = conn.prepare("SELECT profile_id,enabled,load_priority,fomod_answers FROM profile_mods WHERE mod_id=?1")?;
        let rows = query
            .query_map([&item.id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, bool>(1)?,
                    r.get::<_, Option<i64>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for (profile, enabled, _, _) in &rows {
            *package_profiles.entry(profile.clone()).or_default() |= enabled;
        }
        memberships.insert(item.id.clone(), rows);
    }
    for item in old {
        deployment::uninstall(conn, library, &item.id, false, Some(game))?;
    }
    let mut installed = deployment::install_bundle(conn, library, game, staged, build, bundle)?;
    for (index, summary) in installed.iter_mut().enumerate() {
        let previous = matched
            .get(index)
            .and_then(|id| id.as_ref())
            .and_then(|id| old.iter().find(|m| &m.id == id));
        let enabled = previous.map_or(old.is_empty() || old.iter().any(|m| m.enabled), |m| {
            m.enabled
        });
        if !enabled {
            deployment::set_enabled(conn, library, game, &summary.id, false, false)?;
            summary.enabled = false;
        }
        if let Some(previous) = previous {
            if let Some(priority) = previous.load_priority {
                conn.execute(
                    "UPDATE mods SET load_priority=?2 WHERE id=?1",
                    params![summary.id, priority],
                )?;
                summary.load_priority = Some(priority);
            }
        }
        let rows = previous
            .and_then(|m| memberships.get(&m.id))
            .cloned()
            .unwrap_or_else(|| {
                package_profiles
                    .iter()
                    .map(|(id, enabled)| (id.clone(), *enabled, summary.load_priority, None))
                    .collect()
            });
        for (profile, enabled, priority, answers) in rows {
            conn.execute("INSERT INTO profile_mods(profile_id,mod_id,enabled,load_priority,fomod_answers) VALUES(?1,?2,?3,?4,?5)", params![profile,summary.id,enabled,priority,answers])?;
        }
    }
    if installed.len() != staged.len() {
        return Err(AppError::Other("Incomplete package deployment.".into()));
    }
    Ok(installed)
}
