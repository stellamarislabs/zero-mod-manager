mod adoption;
mod archives;
mod commands;
mod compatibility;
mod config_workbench;
mod credentials;
mod database;
mod deployment;
mod diagnostics;
mod error;
mod factory_reset;
mod fomod;
mod isolation;
mod launcher;
mod library_groups;
mod load_order;
mod migration;
mod models;
mod mods;
mod operations;
mod package_transaction;
mod packages;
mod profiles;
mod sessions;
mod steam;
mod storage;
mod support;
mod ue4ss;
mod window_branding;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Mutex, RwLock},
};
use tauri::Manager;

fn bootstrap_log(logs_dir: &Path, event: &str, detail: &str) {
    use std::io::Write;
    let record = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "level": "info",
        "event": event,
        "detail": detail
    });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(logs_dir.join("application.jsonl"))
    {
        let _ = writeln!(file, "{record}");
    }
}

pub struct AppContext {
    data_dir: PathBuf,
    cache_dir: PathBuf,
    mods_dir: RwLock<PathBuf>,
    default_mods_dir: PathBuf,
    logs_dir: PathBuf,
    db_path: PathBuf,
    previews: Mutex<HashMap<String, models::StagedMod>>,
    /// Scripted installers waiting on the answers that decide what to install.
    installers: Mutex<HashMap<String, fomod::Pending>>,
    discoveries: Mutex<HashMap<String, adoption::ScanSnapshot>>,
    previous_build_id: Option<String>,
    storage_mode: &'static str,
    reset_pending: std::sync::atomic::AtomicBool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let executable_dir = std::env::current_exe()?
                .parent()
                .map(Path::to_path_buf)
                .ok_or_else(|| {
                    std::io::Error::other("The application executable has no parent directory")
                })?;
            let roots = storage::resolve(
                &executable_dir,
                storage::StorageRoots::platform(
                    app.path().app_data_dir()?,
                    app.path().app_cache_dir()?,
                    app.path().app_log_dir()?,
                    app.path().app_local_data_dir()?.join("mods"),
                ),
            )?;
            // Reset is performed before opening any database, recovery journal,
            // log or WebView state. Live game files are never reset targets.
            let reset_result = factory_reset::apply_pending(&roots)
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            let data_dir = roots.data;
            let cache_dir = roots.cache;
            let default_mods_dir = roots.mods;
            let legacy_mods_dir = data_dir.join("mods");
            let logs_dir = roots.logs;
            for dir in [&data_dir, &cache_dir, &logs_dir] {
                std::fs::create_dir_all(dir)?
            }
            bootstrap_log(&logs_dir, "startup_begin", env!("CARGO_PKG_VERSION"));
            if let Some(reset) = reset_result {
                bootstrap_log(
                    &logs_dir,
                    "factory_reset_completed",
                    &format!(
                        "{} recovery folders retained",
                        reset.recovery_directories.len()
                    ),
                );
            }
            // Extractions from a previous run are only useful to previews that
            // no longer exist, so the sandbox starts empty.
            let _ = std::fs::remove_dir_all(cache_dir.join("staging"));
            let db_path = data_dir.join("zcom-mod-manager.sqlite3");
            if let Some(backup) =
                migration::backup_operational_upgrade(&db_path, &data_dir, &default_mods_dir)
                    .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?
            {
                bootstrap_log(
                    &logs_dir,
                    "pre_0_7_backup_completed",
                    &backup.display().to_string(),
                );
            }
            // Recover package files and SQLite together before migrations or
            // the independent load-order journal can observe partial state.
            {
                let mut recovery = rusqlite::Connection::open(&db_path)?;
                if package_transaction::recover(&mut recovery, &data_dir)
                    .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?
                {
                    bootstrap_log(
                        &logs_dir,
                        "package_recovered",
                        "Previous package state restored; recovery copies retained.",
                    );
                }
            }
            let mut conn = database::open(&db_path)
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            if let Err(error) = credentials::retire(&conn) {
                bootstrap_log(
                    &logs_dir,
                    "retired_credential_cleanup_pending",
                    &error.to_string(),
                );
            }
            load_order::recover(&conn, &data_dir.join("load-order-operation.json"))
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            adoption::recover(&conn, &data_dir)
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            let settings = database::settings(&conn)
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            // Archive extraction has no database handle of its own, so the
            // user's chosen 7-Zip is published to it once, here.
            archives::set_seven_zip_path(settings.seven_zip_path.as_deref());
            let configured_library = database::get_setting(&conn, "managed_library_path")
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?
                .filter(|path| !path.trim().is_empty())
                .map(PathBuf::from);
            // 0.5.0 stored the library below roaming AppData. Keep a populated
            // legacy library in place until the user chooses to move it, while
            // new installations use Local AppData so payloads are not roamed.
            let mods_dir = configured_library.unwrap_or_else(|| {
                if legacy_mods_dir != default_mods_dir && directory_has_entries(&legacy_mods_dir) {
                    legacy_mods_dir
                } else {
                    default_mods_dir.clone()
                }
            });
            std::fs::create_dir_all(&mods_dir)?;
            let detected = if let Some(path) = settings.game_path.filter(|p| !p.is_empty()) {
                steam::from_manual(std::path::Path::new(&path)).ok()
            } else {
                steam::discover().ok().flatten()
            };
            let detected_game_path = detected
                .as_ref()
                .and_then(|game| game.path.as_deref())
                .map(PathBuf::from);
            let current = detected
                .as_ref()
                .and_then(|game| game.steam_build_id.clone());
            let stored = conn
                .query_row(
                    "SELECT value FROM settings WHERE key='last_game_build'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .ok();
            let previous_build_id = match (&stored, &current) {
                (Some(old), Some(now)) if old != now => Some(old.clone()),
                _ => None,
            };
            if let Some(current) = current {
                let _ = database::set_setting(&conn, "last_game_build", &current);
            }
            if let (Some(profile_id), Some(game_path)) = (
                database::get_setting(&conn, "pending_restore_profile")
                    .ok()
                    .flatten(),
                detected_game_path.as_deref(),
            ) {
                if !deployment::game_is_running() {
                    match profiles::activate(
                        &mut conn,
                        &mods_dir,
                        game_path,
                        &data_dir.join("load-order-operation.json"),
                        &profile_id,
                        None,
                    ) {
                        Ok(_) => {
                            let _ = database::delete_setting(&conn, "pending_restore_profile");
                            bootstrap_log(&logs_dir, "temporary_profile_restored", &profile_id);
                        }
                        Err(error) => bootstrap_log(
                            &logs_dir,
                            "temporary_profile_restore_failed",
                            &error.to_string(),
                        ),
                    }
                }
            }
            if let Err(error) = compatibility::load_cached(
                &conn,
                &data_dir.join("catalog/compatibility.signed.json"),
            ) {
                bootstrap_log(&logs_dir, "catalog_cache_rejected", &error.to_string());
            }
            bootstrap_log(&logs_dir, "startup_ready", "application state initialized");
            app.manage(AppContext {
                data_dir,
                cache_dir,
                mods_dir: RwLock::new(mods_dir),
                default_mods_dir,
                logs_dir,
                db_path,
                previews: Mutex::new(HashMap::new()),
                installers: Mutex::new(HashMap::new()),
                discoveries: Mutex::new(HashMap::new()),
                previous_build_id,
                storage_mode: roots.mode,
                reset_pending: std::sync::atomic::AtomicBool::new(false),
            });
            // WebView2 owns files in Local AppData. Only create it after reset
            // has retired the old data tree and bootstrap state is ready.
            let main_window = app
                .config()
                .app
                .windows
                .first()
                .ok_or_else(|| std::io::Error::other("Missing main window configuration"))?
                .clone();
            let mut window = tauri::WebviewWindowBuilder::from_config(app.handle(), &main_window)?;
            let ctx = app.state::<AppContext>();
            if ctx.storage_mode == "portable" {
                window = window.data_directory(ctx.cache_dir.join("webview"));
            }
            let main = window.visible(false).build()?;
            if let Err(error) = window_branding::apply(&main, &ctx.cache_dir) {
                bootstrap_log(&ctx.logs_dir, "window_branding_failed", &error.to_string());
            }
            if main_window.visible {
                main.show()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard,
            commands::list_mods,
            commands::get_load_order_state,
            commands::preview_load_order,
            commands::apply_load_order,
            commands::apply_ue4ss_order,
            commands::inspect_mod,
            commands::fomod_advance,
            commands::fomod_install,
            commands::fomod_cancel,
            commands::reconfigure_fomod,
            commands::discover_existing_mods,
            commands::adopt_existing_mods,
            commands::group_existing_mods,
            commands::ungroup_existing_mods,
            commands::factory_reset_preview,
            commands::reset_application,
            commands::restore_adopted_components,
            commands::acknowledge_existing_mod_prompt,
            commands::install_mod,
            commands::install_bundle,
            commands::discard_previews,
            commands::rename_mod,
            commands::set_mod_enabled,
            commands::set_mod_hidden,
            commands::uninstall_mod,
            commands::uninstall_bundle,
            commands::verify_mod,
            commands::install_ue4ss,
            commands::get_links,
            commands::check_for_updates,
            commands::run_diagnostics,
            commands::diagnostic_report,
            commands::get_settings,
            commands::save_settings,
            commands::seven_zip_status,
            commands::set_game_path,
            commands::get_managed_library,
            commands::move_managed_library,
            commands::open_managed_path,
            commands::launch_game,
            commands::report_interface_error,
            commands::report_interface_layout,
            commands::frontend_ready,
            commands::legacy_import_status,
            commands::import_legacy_data,
            commands::list_profiles,
            commands::get_profile,
            commands::active_profile,
            commands::create_profile,
            commands::update_profile,
            commands::delete_profile,
            commands::set_profile_mod_state,
            commands::preview_profile_switch,
            commands::activate_profile,
            commands::export_profile_lock,
            commands::import_profile_lock,
            commands::list_snapshots,
            commands::create_snapshot,
            commands::restore_snapshot,
            commands::compatibility_report,
            commands::update_compatibility_catalog,
            commands::activity,
            commands::list_config_documents,
            commands::read_config_document,
            commands::preview_config_change,
            commands::apply_config_change,
            commands::config_history,
            commands::rollback_config_change,
            commands::launch_preflight,
            commands::launch_game_mode,
            commands::complete_launch_session,
            commands::launch_sessions,
            commands::start_guided_isolation,
            commands::active_guided_isolation,
            commands::advance_guided_isolation,
            commands::cancel_guided_isolation,
            commands::support_bundle_preview,
            commands::create_support_bundle
        ])
        .run(tauri::generate_context!())
        .expect("error while running Zero Mod Manager");
}

fn directory_has_entries(path: &Path) -> bool {
    std::fs::read_dir(path)
        .ok()
        .and_then(|mut entries| entries.next())
        .is_some()
}

#[cfg(test)]
mod public_contract_tests {
    use std::path::PathBuf;

    #[test]
    fn published_json_schemas_are_valid_json() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../schema");
        for name in [
            "zcom-mod.schema.json",
            "profile-lock.schema.json",
            "compatibility-catalog.schema.json",
        ] {
            let path = root.join(name);
            let body = std::fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!(
                    "could not read published schema {}: {error}",
                    path.display()
                )
            });
            serde_json::from_str::<serde_json::Value>(&body).unwrap_or_else(|error| {
                panic!(
                    "published schema {} is invalid JSON: {error}",
                    path.display()
                )
            });
        }
    }

    #[test]
    fn nexus_top_sixty_audit_has_sixty_unique_mod_ids() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../catalog/nexus-top60-audit.json");
        let body = std::fs::read_to_string(path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&body).unwrap();
        let ids = value["reviewedModIds"].as_array().unwrap();
        let unique = ids
            .iter()
            .filter_map(serde_json::Value::as_u64)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), 60);
        assert_eq!(unique.len(), 60);
        assert!(value["regressionShapes"].as_array().unwrap().len() >= 8);
    }
}
