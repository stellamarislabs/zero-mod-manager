use crate::{
    error::{AppError, Result},
    models::{IsolationObservation, IsolationSession},
    profiles,
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

fn dependency_groups(conn: &Connection, enabled: &[String]) -> Result<Vec<Vec<String>>> {
    let enabled_set = enabled.iter().cloned().collect::<BTreeSet<_>>();
    let mut identity = BTreeMap::<String, String>::new();
    for id in enabled {
        identity.insert(id.clone(), id.clone());
        let row = conn.query_row(
            "SELECT manifest_id,nexus_mod_id FROM mods WHERE id=?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                ))
            },
        )?;
        if let Some(value) = row.0 {
            identity.insert(value, id.clone());
        }
        if let Some(value) = row.1 {
            identity.insert(format!("nexus:{value}"), id.clone());
        }
    }
    let mut groups = enabled
        .iter()
        .map(|id| vec![id.clone()])
        .collect::<Vec<_>>();
    let mut statement = conn.prepare(
        "SELECT subject_mod_id,target_mod_id FROM compatibility_rules WHERE rule_type='requires' AND target_mod_id IS NOT NULL",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (subject, target) = row?;
        let (Some(subject), Some(target)) = (identity.get(&subject), identity.get(&target)) else {
            continue;
        };
        if !enabled_set.contains(subject) || !enabled_set.contains(target) {
            continue;
        }
        let left = groups.iter().position(|group| group.contains(subject));
        let right = groups.iter().position(|group| group.contains(target));
        if let (Some(left), Some(right)) = (left, right) {
            if left != right {
                let (keep, remove) = if left < right {
                    (left, right)
                } else {
                    (right, left)
                };
                let merged = groups.remove(remove);
                groups[keep].extend(merged);
                groups[keep].sort();
                groups[keep].dedup();
            }
        }
    }
    groups.sort_by(|left, right| left[0].cmp(&right[0]));
    Ok(groups)
}

fn instruction(
    phase: &str,
    status: &str,
    current: &[String],
    candidates: &[Vec<String>],
) -> String {
    if status == "completed" {
        return match phase {
            "baseline-failed" => "The managed-mods-off baseline also failed. Runtime and unmanaged files were not removed, so this result cannot isolate a managed mod.".into(),
            "runtime-suspect" => "The repeated managed-mods-off test gave a different result. This does not isolate the runtime or a particular mod; establish a repeatable baseline first.".into(),
            _ => format!("Isolation narrowed the evidence to {} inseparable candidate group(s). Restore the original profile after reviewing the observations.", candidates.len()),
        };
    }
    match phase {
        "baseline" => "Test with manager-owned mods off. Runtime and unmanaged files remain. Use the main menu or a disposable test save; save files are not managed.".into(),
        "runtime" => "Repeat the managed-mods-off test to check consistency. The manager does not remove the runtime, so this is not an independent loader test.".into(),
        "bisect" => format!("Launch the next dependency-safe subset ({} mod entries), then label what happened.", current.len()),
        _ => "Review the isolation state.".into(),
    }
}

fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IsolationSession> {
    let candidates_json: String = row.get(3)?;
    let observations_json: String = row.get(4)?;
    let current_json: String = row.get(5)?;
    let phase: String = row.get(6)?;
    let status: String = row.get(7)?;
    let candidates: Vec<Vec<String>> = serde_json::from_str(&candidates_json).unwrap_or_default();
    let observations: Vec<IsolationObservation> =
        serde_json::from_str(&observations_json).unwrap_or_default();
    let current: Vec<String> = serde_json::from_str(&current_json).unwrap_or_default();
    let suspected = if status == "completed" && phase == "complete" {
        candidates.iter().flatten().cloned().collect()
    } else {
        Vec::new()
    };
    Ok(IsolationSession {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        phase: phase.clone(),
        status: status.clone(),
        candidate_groups: candidates.clone(),
        current_mod_ids: current.clone(),
        observations,
        suspected_mod_ids: suspected,
        instruction: instruction(&phase, &status, &current, &candidates),
        created_at: row.get(8)?,
    })
}

pub fn get(conn: &Connection, id: &str) -> Result<IsolationSession> {
    Ok(conn.query_row(
        "SELECT id,profile_id,original_lock_json,candidates_json,observations_json,current_json,phase,status,created_at FROM isolation_sessions WHERE id=?1",
        [id], from_row,
    )?)
}

pub fn active(conn: &Connection) -> Result<Option<IsolationSession>> {
    Ok(conn.query_row(
        "SELECT id,profile_id,original_lock_json,candidates_json,observations_json,current_json,phase,status,created_at FROM isolation_sessions WHERE status='active' ORDER BY created_at DESC LIMIT 1",
        [], from_row,
    ).optional()?)
}

pub fn start(conn: &Connection, game_build: Option<String>) -> Result<IsolationSession> {
    if active(conn)?.is_some() {
        return Err(AppError::Other(
            "A guided isolation session is already active.".into(),
        ));
    }
    let profile = profiles::active(conn)?
        .ok_or_else(|| AppError::Other("No active profile exists.".into()))?;
    let enabled = profile
        .mods
        .iter()
        .filter(|item| item.enabled)
        .map(|item| item.mod_id.clone())
        .collect::<Vec<_>>();
    if enabled.is_empty() {
        return Err(AppError::Other(
            "Guided isolation needs at least one enabled mod.".into(),
        ));
    }
    let candidates = dependency_groups(conn, &enabled)?;
    let lock = serde_json::to_string(&profiles::lockfile(conn, &profile.summary.id, game_build)?)?;
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO isolation_sessions(id,profile_id,original_lock_json,candidates_json,observations_json,current_json,phase,status,created_at)
         VALUES(?1,?2,?3,?4,'[]','[]','baseline','active',?5)",
        params![id, profile.summary.id, lock, serde_json::to_string(&candidates)?, created_at],
    )?;
    get(conn, &id)
}

pub fn advance(conn: &Connection, id: &str, outcome: &str) -> Result<IsolationSession> {
    if !matches!(
        outcome,
        "worked" | "not-loaded" | "crashed" | "performance-issue"
    ) {
        return Err(AppError::Other("Unknown isolation outcome.".into()));
    }
    let mut session = get(conn, id)?;
    if session.status != "active" {
        return Err(AppError::Other(
            "That isolation session is no longer active.".into(),
        ));
    }
    session.observations.push(IsolationObservation {
        phase: session.phase.clone(),
        enabled_mod_ids: session.current_mod_ids.clone(),
        outcome: outcome.into(),
    });
    let problem = outcome != "worked";
    match session.phase.as_str() {
        "baseline" if problem => {
            session.phase = "baseline-failed".into();
            session.status = "completed".into();
        }
        "baseline" => {
            session.phase = "runtime".into();
            session.current_mod_ids.clear();
        }
        "runtime" if problem => {
            session.phase = "runtime-suspect".into();
            session.status = "completed".into();
        }
        "runtime" => {
            session.phase = "bisect".into();
            let take = session.candidate_groups.len().div_ceil(2);
            session.current_mod_ids = session
                .candidate_groups
                .iter()
                .take(take)
                .flatten()
                .cloned()
                .collect();
        }
        "bisect" => {
            let tested = session
                .current_mod_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            session.candidate_groups.retain(|group| {
                let in_test = group.iter().any(|id| tested.contains(id));
                if problem {
                    in_test
                } else {
                    !in_test
                }
            });
            if session.candidate_groups.len() <= 1 {
                session.phase = "complete".into();
                session.status = "completed".into();
                session.current_mod_ids.clear();
            } else {
                let take = session.candidate_groups.len().div_ceil(2);
                session.current_mod_ids = session
                    .candidate_groups
                    .iter()
                    .take(take)
                    .flatten()
                    .cloned()
                    .collect();
            }
        }
        _ => {
            return Err(AppError::Other(
                "The isolation session has an invalid phase.".into(),
            ))
        }
    }
    conn.execute(
        "UPDATE isolation_sessions SET candidates_json=?2,observations_json=?3,current_json=?4,phase=?5,status=?6 WHERE id=?1",
        params![id, serde_json::to_string(&session.candidate_groups)?, serde_json::to_string(&session.observations)?, serde_json::to_string(&session.current_mod_ids)?, session.phase, session.status],
    )?;
    get(conn, id)
}

pub fn cancel(conn: &Connection, id: &str) -> Result<()> {
    let changed = conn.execute(
        "UPDATE isolation_sessions SET status='cancelled' WHERE id=?1 AND status='active'",
        [id],
    )?;
    if changed == 0 {
        return Err(AppError::Other(
            "That isolation session is not active.".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn bisection_never_splits_hard_dependencies() {
        let directory = tempdir().unwrap();
        let conn = crate::database::open(&directory.path().join("db.sqlite")).unwrap();
        for (id, manifest) in [("a", "suite.a"), ("b", "suite.b"), ("c", "suite.c")] {
            conn.execute("INSERT INTO mods(id,name,mod_type,installed_at,enabled,manifest_id) VALUES(?1,?1,'ue4ss',datetime('now'),1,?2)", params![id, manifest]).unwrap();
        }
        let profile: String = conn
            .query_row("SELECT id FROM profiles WHERE is_active=1", [], |row| {
                row.get(0)
            })
            .unwrap();
        conn.execute(
            "INSERT INTO profile_mods(profile_id,mod_id,enabled) SELECT ?1,id,1 FROM mods",
            [&profile],
        )
        .unwrap();
        conn.execute("INSERT INTO compatibility_rules(id,subject_mod_id,target_mod_id,rule_type,severity,source,message) VALUES('r','suite.a','suite.b','requires','blocked','author-manifest','required')", []).unwrap();
        let session = start(&conn, None).unwrap();
        assert!(session
            .candidate_groups
            .iter()
            .any(|group| group == &vec!["a".to_string(), "b".to_string()]));
        let session = advance(&conn, &session.id, "worked").unwrap();
        assert_eq!(session.phase, "runtime");
        let session = advance(&conn, &session.id, "worked").unwrap();
        assert_eq!(session.phase, "bisect");
        assert!(
            !session.current_mod_ids.contains(&"a".into())
                || session.current_mod_ids.contains(&"b".into())
        );
    }
}
