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
    let mods = database::list_mods(conn)?;
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
    items.push(item("Steam manifest",if game.steam_build_id.is_some(){"good"}else{"warning"},game.steam_build_id.as_deref().map(|id|format!("Build {id}")).unwrap_or_else(||"Build ID unavailable".into()),game.steam_build_id.is_none().then(||"Manual installations work, but build compatibility cannot be assessed without an app manifest.".into())));
    if game.detected && game.steam_build_id.is_none() {
        items.push(item(
            "EA App / manual installation",
            "warning",
            "Experimental support",
            Some(
                "File deployment is supported, but launcher-specific UE4SS injection is not yet verified. Run the game once, then confirm that UE4SS.log appears before relying on runtime mods."
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
        } else if game.detected {
            "warning"
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
        format!(
            "{} managed, {} enabled",
            mods.len(),
            mods.iter().filter(|m| m.enabled).count()
        ),
        None,
    ));
    items.push(item("Package conflicts",if conflicts==0{"good"}else{"warning"},format!("{conflicts} overlapping file/package group(s)"),(conflicts>0).then(||"Open Mods to review the affected managers. Raw package names remain hidden by default.".into())));
    // A complete layout is not a loaded runtime. Reporting "Healthy" from file
    // presence alone told a user whose game never loaded UE4SS that nothing was
    // wrong, so the verdict now waits for the log UE4SS writes when it runs.
    items.push(item(
        "UE4SS",
        if ue4ss.healthy && ue4ss.log_found && ue4ss.extra_loaders.is_empty() {
            "good"
        } else if ue4ss.installed {
            "warning"
        } else {
            "unknown"
        },
        if ue4ss.healthy && ue4ss.log_found {
            format!("Loaded at least once; {} UE4SS mod(s)", ue4ss.mod_count)
        } else if ue4ss.healthy {
            format!(
                "Files complete, but no UE4SS log yet; {} UE4SS mod(s)",
                ue4ss.mod_count
            )
        } else if ue4ss.installed {
            "Incomplete installation".into()
        } else {
            "Not installed (optional)".into()
        },
        ue4ss.message.clone().or_else(|| {
            (!ue4ss.installed).then(|| {
                format!(
                    "Only needed for UE4SS script and DLL mods. Get the tested Zero Company build from {}, then use Home to install it.",
                    crate::ue4ss::DOWNLOAD_URL
                )
            })
        }),
    ));
    if ue4ss.installed {
        items.push(item(
            "UE4SS log",
            if ue4ss.log_found { "good" } else { "warning" },
            ue4ss
                .log_path
                .clone()
                .unwrap_or_else(|| "Not written yet".into()),
            (!ue4ss.log_found).then(|| {
                "UE4SS writes this the first time it loads. Its absence after starting the game \
                 means the loader never ran."
                    .into()
            }),
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
                    "UE4SS links against the Visual C++ 2015-2022 x64 runtime. Without it Windows \
                     cannot load the loader, and the game either hangs at start-up or runs with \
                     no UE4SS. Install it from Microsoft."
                        .into()
                }),
            ));
        }
        if !ue4ss.extra_loaders.is_empty() {
            items.push(item(
                "Proxy DLLs beside the game",
                "warning",
                ue4ss.extra_loaders.join(", "),
                Some(
                    "UE4SS loads only as dwmapi.dll. A copy renamed to another proxy name lets \
                     the game start while the runtime never loads. Remove the extra file unless \
                     another tool needs it."
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
    let mut text = format!("Zero Mod Manager — Mod Doctor\nOverall: {overall}\n\n");
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
