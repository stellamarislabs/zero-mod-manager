use crate::{
    database,
    error::Result,
    models::{DiagnosticItem, DiagnosticReport, GameInfo, ToolInfo, Ue4ssInfo},
};
use rusqlite::Connection;
use std::path::Path;

fn item(
    label: &str,
    status: &str,
    value: impl Into<String>,
    action: Option<String>,
) -> DiagnosticItem {
    DiagnosticItem {
        label: label.into(),
        status: status.into(),
        value: value.into(),
        action,
    }
}
pub fn run(
    conn: &Connection,
    game: &GameInfo,
    ue4ss: &Ue4ssInfo,
    _tool: &ToolInfo,
) -> Result<DiagnosticReport> {
    let (managed, enabled) = database::counts(conn)?;
    let conflicts = database::conflict_count(conn)?;
    let mut items = Vec::new();
    items.push(item(
        "Game installation",
        if game.detected { "good" } else { "error" },
        if game.detected {
            "Valid Zero Company layout"
        } else {
            "Not detected"
        },
        (!game.detected).then(|| "Locate the game folder in Settings.".into()),
    ));
    items.push(item(
        "Game build",
        if game.steam_build_id.is_some() {
            "good"
        } else {
            "unknown"
        },
        game.steam_build_id
            .as_deref()
            .map(|id| format!("Build {id}"))
            .unwrap_or_else(|| "Build ID unavailable".into()),
        None,
    ));
    if game.detected && game.steam_build_id.is_none() {
        items.push(item(
            "EA App / manual installation",
            "unknown",
            "In-game behavior not verified",
            Some(
                "File checks do not establish in-game compatibility. Test the installed mods in the game; an existing log alone does not verify the current session."
                    .into(),
            ),
        ));
    }
    let mods_folder = game
        .path
        .as_deref()
        .map(Path::new)
        .map(|p| p.join("SWZeroCompany/Content/Paks/~mods"));
    items.push(item(
        "~mods folder",
        if mods_folder.as_ref().is_some_and(|p| p.is_dir()) {
            "good"
        } else {
            "unknown"
        },
        mods_folder
            .as_ref()
            .filter(|p| p.exists())
            .map(|_| "Present")
            .unwrap_or("Created on first packaged-mod install"),
        None,
    ));
    items.push(item(
        "Installed mods",
        "good",
        format!("{managed} managed, {enabled} enabled (including partially enabled bundles)"),
        None,
    ));
    items.push(item("Recorded file/package overlaps",if conflicts==0{"good"}else{"warning"},format!("{conflicts} overlapping file/package group(s)"),(conflicts>0).then(||"Open Library to review the affected mods. Raw package names remain hidden by default.".into())));
    // File presence and a historical log do not prove a successful game session.
    items.push(item(
        "UE4SS",
        if ue4ss.healthy {
            "good"
        } else if ue4ss.installed {
            "warning"
        } else {
            "unknown"
        },
        if ue4ss.healthy {
            format!("Expected files present; {} payload folder(s) detected", ue4ss.mod_count)
        } else if ue4ss.installed {
            "Incomplete installation".into()
        } else {
            "Not installed (optional)".into()
        },
        ue4ss.message.clone().or_else(|| {
            (!ue4ss.installed).then(|| {
                format!(
                    "Only needed for UE4SS script and DLL mods. Get the tested Zero Company build from {}, then use Command Center to install it.",
                    crate::ue4ss::DOWNLOAD_URL
                )
            })
        }),
    ));
    if ue4ss.installed {
        items.push(item(
            "UE4SS log",
            "unknown",
            ue4ss
                .log_path
                .clone()
                .unwrap_or_else(|| "Not found at known paths".into()),
            Some("File presence only; the log's contents, age and current-session success have not been verified.".into()),
        ));
        if let Some(present) = ue4ss.vc_runtime {
            // A warning rather than an error: this checks System32 only, and a
            // runtime deployed some other way would otherwise report the whole
            // installation as blocked when the game in fact works.
            items.push(item(
                "Visual C++ runtime",
                if present { "good" } else { "warning" },
                if present {
                    "Present"
                } else {
                    "vcruntime140 not found in System32"
                },
                (!present).then(|| {
                    "Expected runtime files were not found in System32. A local runtime was not checked. \
                     If UE4SS fails to load, install the x64 Visual C++ runtime from Microsoft."
                        .into()
                }),
            ));
        }
        if !ue4ss.extra_loaders.is_empty() {
            items.push(item(
                "Other proxy DLLs beside the game",
                "unknown",
                ue4ss.extra_loaders.join(", "),
                Some(
                    "These files may belong to other tools. File names alone do not establish a conflict; \
                     review their source before changing them."
                        .into(),
                ),
            ));
        }
    }
    if game.compat_data_path.is_some() {
        items.push(item("Proton compatibility prefix", "good", "Found", None));
        items.push(item(
            "DLL override",
            if ue4ss.proton_override == Some(true) {
                "good"
            } else if ue4ss.installed {
                "warning"
            } else {
                "unknown"
            },
            if ue4ss.proton_override == Some(true) {
                "dwmapi override detected"
            } else {
                "Not detected"
            },
            (ue4ss.installed && ue4ss.proton_override != Some(true)).then(|| {
                "Add to Steam launch options:\nWINEDLLOVERRIDES=\"dwmapi=n,b\" %command%".into()
            }),
        ));
    }
    let has_error = items.iter().any(|i| i.status == "error");
    let has_warning = items.iter().any(|i| i.status == "warning");
    let overall = if has_error {
        "BLOCKED"
    } else if has_warning {
        "NEEDS ATTENTION"
    } else {
        "GOOD"
    }
    .to_string();
    let mut text =
        format!("Zero Mod Manager — Mod Doctor\nOverall (local file checks): {overall}\n\n");
    for i in &items {
        text.push_str(&format!(
            "{:<30} {} — {}\n",
            i.label,
            status_symbol(&i.status),
            i.value
        ));
        if let Some(a) = &i.action {
            text.push_str(&format!("  Action: {a}\n"));
        }
    }
    Ok(DiagnosticReport {
        overall,
        items,
        text: sanitize(&text),
    })
}
fn status_symbol(status: &str) -> &'static str {
    match status {
        "good" => "OK",
        "warning" => "ATTENTION",
        "error" => "ERROR",
        _ => "INFO",
    }
}
fn sanitize(text: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        text.replace(&home.display().to_string(), "~")
    } else {
        text.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_file_checks_never_claim_session_success() {
        let dir = tempfile::tempdir().unwrap();
        let conn = database::open(&dir.path().join("state.sqlite3")).unwrap();
        let game = GameInfo {
            detected: true,
            ..Default::default()
        };
        let runtime = Ue4ssInfo {
            installed: true,
            healthy: true,
            log_found: true,
            ..Default::default()
        };
        let report = run(&conn, &game, &runtime, &ToolInfo::default()).unwrap();
        assert!(!report.text.contains("Loaded at least once"));
        assert!(report
            .items
            .iter()
            .any(|item| item.label == "UE4SS" && item.value.contains("Expected files present")));
        assert!(report
            .items
            .iter()
            .any(|item| item.label == "Game build" && item.status == "unknown"));
        assert!(report
            .items
            .iter()
            .any(|item| item.label == "UE4SS log" && item.status == "unknown"));
    }
}
