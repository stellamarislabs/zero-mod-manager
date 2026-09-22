use crate::{
    compatibility,
    error::{AppError, Result},
    models::{
        AppSettings, GameInfo, LaunchPreflight, LaunchSession, ModManifest, ReadinessIssue,
        Ue4ssInfo,
    },
    profiles,
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

pub fn preflight(
    conn: &Connection,
    game: &GameInfo,
    ue4ss: &Ue4ssInfo,
    settings: &AppSettings,
) -> Result<LaunchPreflight> {
    let active = profiles::active(conn)?;
    let compatibility = compatibility::report(conn)?;
    let mut issues = Vec::new();
    if !game.detected
        && settings
            .custom_executable_path
            .as_deref()
            .is_none_or(str::is_empty)
    {
        issues.push(ReadinessIssue {
            id: "game-not-found".into(),
            status: "blocked".into(),
            title: "Game installation is not connected".into(),
            detail: game.problem.clone().unwrap_or_else(|| {
                "Locate a valid Zero Company installation before launching.".into()
            }),
            action: Some("settings".into()),
        });
    }
    if let Some(old) = crate::database::get_setting(conn, "last_acknowledged_game_build")? {
        if game
            .steam_build_id
            .as_deref()
            .is_some_and(|current| current != old)
        {
            issues.push(ReadinessIssue {
                id: "game-build-changed".into(), status: "warning".into(),
                title: "The game build changed".into(),
                detail: format!("Installed mods were last acknowledged for build {old}. Review compatibility before continuing."),
                action: Some("health".into()),
            });
        }
    }
    if ue4ss.installed && (!ue4ss.healthy || !ue4ss.log_found || !ue4ss.extra_loaders.is_empty()) {
        issues.push(ReadinessIssue {
            id: "runtime-unverified".into(),
            status: "warning".into(),
            title: "UE4SS is installed but not verified".into(),
            detail: ue4ss.message.clone().unwrap_or_else(|| {
                "A successful UE4SS log has not confirmed that the runtime loaded.".into()
            }),
            action: Some("health".into()),
        });
    }
    let launcher_identity = if settings
        .custom_executable_path
        .as_deref()
        .is_some_and(|path| !path.is_empty())
    {
        "manual"
    } else if game.source == "ea" {
        "ea"
    } else {
        "steam"
    };
    let mut manifests = conn.prepare(
        "SELECT id,name,manifest_json FROM mods WHERE enabled=1 AND manifest_json IS NOT NULL",
    )?;
    for row in manifests.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })? {
        let (id, name, raw) = row?;
        let Ok(manifest) = serde_json::from_str::<ModManifest>(&raw) else {
            continue;
        };
        if !manifest.launchers.is_empty()
            && !manifest
                .launchers
                .iter()
                .any(|value| value.eq_ignore_ascii_case(launcher_identity))
        {
            issues.push(ReadinessIssue {
                id: format!("launcher:{id}"),
                status: "warning".into(),
                title: "Mod is unverified for this launcher".into(),
                detail: format!(
                    "{name} declares {} support; the active launcher is {launcher_identity}.",
                    manifest.launchers.join(", ")
                ),
                action: Some("profiles".into()),
            });
        }
        if let Some(runtime) = manifest
            .runtime
            .filter(|runtime| runtime.required == Some(true))
        {
            if runtime.name.eq_ignore_ascii_case("ue4ss") && !ue4ss.installed {
                issues.push(ReadinessIssue {
                    id: format!("runtime:{id}"),
                    status: "blocked".into(),
                    title: "A required runtime is missing".into(),
                    detail: format!(
                        "{name} requires UE4SS{}.",
                        runtime
                            .version
                            .as_deref()
                            .map(|version| format!(" {version}"))
                            .unwrap_or_default()
                    ),
                    action: Some("health".into()),
                });
            } else if runtime.version.is_some() && ue4ss.installed {
                issues.push(ReadinessIssue {
                    id: format!("runtime-version:{id}"), status: "unverified".into(), title: "Runtime version could not be fingerprinted".into(),
                    detail: format!("{name} requires {} {}; the layout is present but its exact version is not proven.", runtime.name, runtime.version.unwrap_or_default()),
                    action: Some("health".into()),
                });
            }
        }
    }
    for issue in compatibility.issues {
        issues.push(ReadinessIssue {
            id: issue.id,
            status: issue.status,
            title: issue.title,
            detail: issue.detail,
            action: Some("library".into()),
        });
    }
    let status = if issues.iter().any(|issue| issue.status == "blocked") {
        "blocked"
    } else if issues.iter().any(|issue| issue.status == "warning") {
        "warning"
    } else if issues.iter().any(|issue| issue.status == "unverified") {
        "unverified"
    } else if ue4ss.installed && !ue4ss.log_found {
        "unverified"
    } else {
        "ready"
    };
    let launcher = if settings
        .custom_executable_path
        .as_deref()
        .is_some_and(|path| !path.is_empty())
    {
        "custom"
    } else if game.source == "ea" {
        "ea"
    } else {
        "steam"
    };
    Ok(LaunchPreflight {
        status: status.into(),
        profile_id: active.as_ref().map(|profile| profile.summary.id.clone()),
        profile_name: active.as_ref().map(|profile| profile.summary.name.clone()),
        launcher: launcher.into(),
        game_build: game.steam_build_id.clone(),
        runtime_state: if ue4ss.installed {
            if ue4ss.healthy && ue4ss.log_found {
                "verified"
            } else {
                "unverified"
            }
        } else {
            "not-installed"
        }
        .into(),
        enabled_mods: active
            .map(|profile| profile.summary.enabled_mods)
            .unwrap_or_default(),
        issues,
    })
}

pub fn begin(
    conn: &Connection,
    mode: &str,
    launcher: &str,
    game_build: Option<&str>,
    executable_sha256: Option<&str>,
    runtime_version: Option<&str>,
    lock_json: &str,
) -> Result<LaunchSession> {
    if !matches!(mode, "modded" | "vanilla" | "troubleshoot") {
        return Err(AppError::Other("Unknown launch mode.".into()));
    }
    let profile_id = conn
        .query_row("SELECT id FROM profiles WHERE is_active=1", [], |row| {
            row.get::<_, String>(0)
        })
        .optional()?;
    let id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO launch_sessions(id,mode,profile_id,launcher,game_build,executable_sha256,runtime_version,mod_lock_json,started_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![id, mode, profile_id, launcher, game_build, executable_sha256, runtime_version, lock_json, started_at],
    )?;
    Ok(LaunchSession {
        id,
        mode: mode.into(),
        profile_id,
        launcher: launcher.into(),
        game_build: game_build.map(str::to_string),
        executable_sha256: executable_sha256.map(str::to_string),
        runtime_version: runtime_version.map(str::to_string),
        started_at,
        ended_at: None,
        outcome: None,
        log_evidence: None,
    })
}

pub fn complete(
    conn: &Connection,
    session_id: &str,
    outcome: &str,
    evidence: Option<&str>,
) -> Result<LaunchSession> {
    if !matches!(
        outcome,
        "worked" | "not-loaded" | "crashed" | "performance-issue" | "unknown"
    ) {
        return Err(AppError::Other("Unknown launch outcome.".into()));
    }
    let ended = Utc::now().to_rfc3339();
    let changed = conn.execute(
        "UPDATE launch_sessions SET ended_at=?2,outcome=?3,log_evidence=?4 WHERE id=?1",
        params![session_id, ended, outcome, evidence],
    )?;
    if changed == 0 {
        return Err(AppError::Other(
            "That launch session no longer exists.".into(),
        ));
    }
    get(conn, session_id)
}

fn get(conn: &Connection, id: &str) -> Result<LaunchSession> {
    Ok(conn.query_row(
        "SELECT id,mode,profile_id,launcher,game_build,executable_sha256,runtime_version,started_at,ended_at,outcome,log_evidence FROM launch_sessions WHERE id=?1",
        [id], |row| Ok(LaunchSession {
            id: row.get(0)?, mode: row.get(1)?, profile_id: row.get(2)?, launcher: row.get(3)?, game_build: row.get(4)?,
            executable_sha256: row.get(5)?, runtime_version: row.get(6)?, started_at: row.get(7)?, ended_at: row.get(8)?, outcome: row.get(9)?, log_evidence: row.get(10)?,
        }))?)
}

pub fn list(conn: &Connection) -> Result<Vec<LaunchSession>> {
    let mut statement = conn.prepare(
        "SELECT id,mode,profile_id,launcher,game_build,executable_sha256,runtime_version,started_at,ended_at,outcome,log_evidence
         FROM launch_sessions ORDER BY started_at DESC LIMIT 50",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(LaunchSession {
                id: row.get(0)?,
                mode: row.get(1)?,
                profile_id: row.get(2)?,
                launcher: row.get(3)?,
                game_build: row.get(4)?,
                executable_sha256: row.get(5)?,
                runtime_version: row.get(6)?,
                started_at: row.get(7)?,
                ended_at: row.get(8)?,
                outcome: row.get(9)?,
                log_evidence: row.get(10)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}
