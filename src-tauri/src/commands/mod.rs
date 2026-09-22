use crate::{
    adoption, archives, compatibility, config_workbench, database, deployment, diagnostics,
    error::{AppError, Result},
    fomod, isolation, load_order,
    models::{
        AdoptionGroup, AdoptionReport, AppSettings, BundleInstallItem, BundleInstallReport,
        CompatibilityReport, ConfigChangePreview, ConfigDocument, ConfigPatchRecord, Dashboard,
        DiagnosticReport, ExistingModScan, GameInfo, Inspection, IsolationSession, LaunchPreflight,
        LaunchReport, LaunchSession, LoadOrderPreview, LoadOrderState, ManagedLibraryInfo,
        ModPreview, ModSummary, OperationRecord, PackageAssessment, ProfileDetail, ProfileLock,
        ProfileSummary, ProfileSwitchPreview, ReplacedMod, SnapshotSummary, StagedMod,
        SupportBundlePreview, SupportBundleReport, ToolInfo,
    },
    mods, operations, profiles, sessions, steam, support, ue4ss, AppContext,
};
use std::{
    path::{Path, PathBuf},
    sync::{MutexGuard, RwLockReadGuard},
    time::Duration,
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

fn package_operation<T>(ctx: &AppContext, action: impl FnOnce() -> Result<T>) -> Result<T> {
    let (_, game_path) = require_game(ctx)?;
    let library = mods_dir(ctx)?;
    let mut conn = connection(ctx)?;
    if database::get_setting(&conn, "pending_restore_profile")?.is_some() {
        return Err(AppError::Other(
            "Restore the temporary launch profile before changing mods.".into(),
        ));
    }
    crate::package_transaction::run(&mut conn, &ctx.data_dir, &library, &game_path, |_| action())
}

fn connection(ctx: &AppContext) -> Result<rusqlite::Connection> {
    database::open(&ctx.db_path)
}

/// Reads a saved manual path without letting a moved Steam library make every
/// startup command fail. The old path stays visible so the interface can show
/// exactly which saved location needs replacing.
fn manual_game_or_unavailable(path: &Path) -> Result<GameInfo> {
    match steam::from_manual(path) {
        Ok(info) => Ok(info),
        Err(AppError::InvalidGamePath(_)) => Ok(GameInfo {
            path: Some(path.display().to_string()),
            source: "manual".into(),
            problem_code: Some("game_path_invalid".into()),
            problem: Some(
                "The saved game location is no longer valid. The Steam library may have moved."
                    .into(),
            ),
            ..GameInfo::default()
        }),
        Err(error) => Err(error),
    }
}

fn game(ctx: &AppContext) -> Result<GameInfo> {
    let conn = connection(ctx)?;
    let settings = database::settings(&conn)?;
    if let Some(path) = settings.game_path.filter(|p| !p.is_empty()) {
        manual_game_or_unavailable(Path::new(&path))
    } else {
        Ok(steam::discover()?.unwrap_or_default())
    }
}
fn require_game(ctx: &AppContext) -> Result<(GameInfo, PathBuf)> {
    let info = game(ctx)?;
    if !info.detected {
        return Err(AppError::GameNotFound);
    }
    let path = info
        .path
        .as_ref()
        .map(PathBuf::from)
        .ok_or(AppError::GameNotFound)?;
    Ok((info, path))
}
fn tool(_ctx: &AppContext) -> Result<crate::models::ToolInfo> {
    Ok(crate::models::ToolInfo::default())
}
fn previews(
    ctx: &AppContext,
) -> Result<MutexGuard<'_, std::collections::HashMap<String, StagedMod>>> {
    ctx.previews
        .lock()
        .map_err(|_| AppError::Other("preview state lock was poisoned".into()))
}

fn mods_dir(ctx: &AppContext) -> Result<RwLockReadGuard<'_, PathBuf>> {
    ctx.mods_dir
        .read()
        .map_err(|_| AppError::Other("managed library lock was poisoned".into()))
}

fn discoveries(
    ctx: &AppContext,
) -> Result<MutexGuard<'_, std::collections::HashMap<String, adoption::ScanSnapshot>>> {
    ctx.discoveries
        .lock()
        .map_err(|_| AppError::Other("existing-mod discovery lock was poisoned".into()))
}

fn installers(
    ctx: &AppContext,
) -> Result<MutexGuard<'_, std::collections::HashMap<String, fomod::Pending>>> {
    ctx.installers
        .lock()
        .map_err(|_| AppError::Other("installer state lock was poisoned".into()))
}

#[tauri::command]
pub fn get_dashboard(ctx: State<'_, AppContext>) -> Result<Dashboard> {
    let conn = connection(&ctx)?;
    let game = game(&ctx)?;
    let (installed_mods, enabled_mods) = database::counts(&conn)?;
    let conflict_count = database::conflict_count(&conn)?;
    let game_path = game.path.as_deref().map(Path::new);
    let compat = game.compat_data_path.as_deref().map(Path::new);
    let ue4ss = ue4ss::detect(game_path, compat);
    let existing_mod_scan_pending =
        database::get_setting(&conn, "existing_mod_prompt_acknowledged")?.as_deref()
            != Some("true");
    Ok(Dashboard {
        game,
        installed_mods,
        enabled_mods,
        conflict_count,
        ue4ss,
        previous_build_id: ctx.previous_build_id.clone(),
        data_directory: ctx.data_dir.display().to_string(),
        storage_mode: ctx.storage_mode.into(),
        existing_mod_scan_pending,
    })
}
#[tauri::command]
pub fn list_mods(ctx: State<'_, AppContext>) -> Result<Vec<ModSummary>> {
    database::list_mods(&connection(&ctx)?)
}

#[tauri::command]
pub fn discover_existing_mods(ctx: State<'_, AppContext>) -> Result<ExistingModScan> {
    let (game_info, game_path) = require_game(&ctx)?;
    let conn = connection(&ctx)?;
    let scan_options = crate::models::ToolInfo::default();
    let (scan, snapshot) = adoption::discover(&conn, &game_path, &scan_options)?;
    let mut held = discoveries(&ctx)?;
    // Discovery snapshots point into the live game folder and are useful only
    // to the currently visible review. Dropping older scans also bounds memory.
    held.clear();
    held.insert(scan.scan_id.clone(), snapshot);
    drop(held);
    log(
        &ctx,
        "info",
        "existing_mods_discovered",
        &format!(
            "candidates={} unsupported={} build={}",
            scan.candidates.len(),
            scan.unsupported.len(),
            game_info.steam_build_id.as_deref().unwrap_or("unknown")
        ),
    );
    Ok(scan)
}

#[tauri::command]
pub fn acknowledge_existing_mod_prompt(ctx: State<'_, AppContext>) -> Result<()> {
    database::set_setting(
        &connection(&ctx)?,
        "existing_mod_prompt_acknowledged",
        "true",
    )
}

#[tauri::command]
pub fn adopt_existing_mods(
    scan_id: String,
    groups: Vec<AdoptionGroup>,
    ctx: State<'_, AppContext>,
) -> Result<AdoptionReport> {
    let (game_info, _) = require_game(&ctx)?;
    let snapshot = discoveries(&ctx)?.get(&scan_id).cloned().ok_or_else(|| {
        AppError::Other("That discovery expired. Scan the game folders again.".into())
    })?;
    let mut conn = connection(&ctx)?;
    let library = mods_dir(&ctx)?;
    let report = adoption::adopt(
        &mut conn,
        &library,
        &ctx.data_dir,
        &snapshot,
        &groups,
        game_info.steam_build_id,
    );
    let successful = report
        .outcomes
        .iter()
        .filter(|outcome| outcome.mod_summary.is_some())
        .flat_map(|outcome| outcome.candidate_ids.iter().cloned())
        .collect::<std::collections::HashSet<_>>();
    if !successful.is_empty() {
        let mut held = discoveries(&ctx)?;
        if let Some(snapshot) = held.get_mut(&scan_id) {
            snapshot
                .candidates
                .retain(|candidate, _| !successful.contains(candidate));
            if snapshot.candidates.is_empty() {
                held.remove(&scan_id);
            }
        }
    }
    log(
        &ctx,
        "info",
        "existing_mods_adopted",
        &format!(
            "succeeded={} failed={}",
            report
                .outcomes
                .iter()
                .filter(|outcome| outcome.mod_summary.is_some())
                .count(),
            report
                .outcomes
                .iter()
                .filter(|outcome| outcome.error.is_some())
                .count()
        ),
    );
    Ok(report)
}

#[tauri::command]
pub fn get_load_order_state(ctx: State<'_, AppContext>) -> Result<LoadOrderState> {
    load_order::state(&connection(&ctx)?)
}

#[tauri::command]
pub fn preview_load_order(
    ordered_mod_ids: Vec<String>,
    ctx: State<'_, AppContext>,
) -> Result<LoadOrderPreview> {
    load_order::preview(&connection(&ctx)?, &ordered_mod_ids)
}

/// Writes the UE4SS start order. Unlike the packaged order, this renames
/// nothing: the runtime reads `mods.txt` top to bottom, so there is no preview
/// step and nothing to roll back.
#[tauri::command]
pub fn apply_ue4ss_order(
    ordered_mod_ids: Vec<String>,
    ctx: State<'_, AppContext>,
) -> Result<LoadOrderState> {
    deployment::ensure_game_stopped()?;
    let (_, game_path) = require_game(&ctx)?;
    let mut conn = connection(&ctx)?;
    let state = load_order::apply_ue4ss_order(&mut conn, &game_path, &ordered_mod_ids)?;
    log(
        &ctx,
        "info",
        "ue4ss_order_applied",
        &format!("ordered_mods={}", ordered_mod_ids.len()),
    );
    profiles::capture_active(&conn)?;
    let _ = operations::record(
        &conn,
        "load-order",
        "completed",
        "UE4SS start order applied",
        serde_json::json!({"mods": ordered_mod_ids.len()}),
    );
    Ok(state)
}

#[tauri::command]
pub fn apply_load_order(
    ordered_mod_ids: Vec<String>,
    ctx: State<'_, AppContext>,
) -> Result<LoadOrderState> {
    deployment::ensure_game_stopped()?;
    let mut conn = connection(&ctx)?;
    let state = load_order::apply(
        &mut conn,
        &ordered_mod_ids,
        &ctx.data_dir.join("load-order-operation.json"),
    )?;
    log(
        &ctx,
        "info",
        "load_order_applied",
        &format!("ordered_mods={}", ordered_mod_ids.len()),
    );
    profiles::capture_active(&conn)?;
    let _ = operations::record(
        &conn,
        "load-order",
        "completed",
        "Packaged load order applied",
        serde_json::json!({"mods": ordered_mod_ids.len()}),
    );
    Ok(state)
}

/// Reads an archive or folder and reports every mod it contains.
///
/// One download regularly holds more than one installable mod: a UE4SS archive
/// with several script folders, or a package that ships both a `.pak` and a
/// loader mod. Each is previewed separately so the person can name and install
/// them individually.
#[tauri::command]
pub fn inspect_mod(path: String, ctx: State<'_, AppContext>) -> Result<Inspection> {
    log(
        &ctx,
        "info",
        "mod_inspection_started",
        &format!("source={} exists={}", path, Path::new(&path).exists()),
    );
    let source = PathBuf::from(&path);
    let staging = archives::stage(&source, &ctx.cache_dir).inspect_err(|error| {
        log(
            &ctx,
            "warn",
            "mod_inspection_failed",
            &format!("source={path} error={error}"),
        );
    })?;
    let assessment = mods::assess_package(&staging.executables);
    if matches!(
        assessment.role.as_str(),
        "externalInstaller" | "externalTool"
    ) {
        let _ = std::fs::remove_dir_all(&staging.root);
        log(
            &ctx,
            "info",
            "external_package_blocked",
            &format!("source={path} role={}", assessment.role),
        );
        return Ok(Inspection {
            previews: Vec::new(),
            installer: None,
            package: assessment,
        });
    }
    // A download that scripts its own installation answers "what does this
    // contain?" with a set of questions instead of a payload. Reading it as a
    // plain archive would offer every variant at once, which is the pile the
    // script exists to sort out.
    if let Some(package_root) = fomod::locate(&staging.root) {
        match fomod::parse(&package_root) {
            Ok(installer) => return begin_installer(&ctx, &source, installer, staging, assessment),
            // A script this manager cannot read is not a reason to refuse the
            // download: the archive still holds the files, and reading it the
            // ordinary way puts every option in front of the person by hand.
            Err(error) => log(
                &ctx,
                "warn",
                "fomod_script_unreadable",
                &format!("source={path} error={error}"),
            ),
        }
    }
    let found = scan_staged(&ctx, &source, staging).inspect_err(|error| {
        log(
            &ctx,
            "warn",
            "mod_inspection_failed",
            &format!("source={path} error={error}"),
        );
    })?;
    let previews = register_previews(&ctx, found)?;
    let assessment = mods::resolved_assessment(&previews, assessment);
    log(
        &ctx,
        "info",
        "mod_inspected",
        &format!(
            "mods={} types={}",
            previews.len(),
            previews
                .iter()
                .map(|preview| preview.mod_type.clone())
                .collect::<Vec<_>>()
                .join(",")
        ),
    );
    Ok(Inspection {
        previews,
        installer: None,
        package: assessment,
    })
}

/// Reads a staged tree with the current settings and game state.
fn scan_staged(
    ctx: &AppContext,
    source: &Path,
    staging: archives::Staging,
) -> Result<Vec<(StagedMod, ModPreview)>> {
    let conn = connection(ctx)?;
    let settings = database::settings(&conn)?;
    let game = game(ctx)?;
    let ue = ue4ss::detect(
        game.path.as_deref().map(Path::new),
        game.compat_data_path.as_deref().map(Path::new),
    );
    let tool = crate::models::ToolInfo::default();
    mods::scan_staged(
        source,
        staging,
        &tool,
        game.steam_build_id.as_deref(),
        ue.healthy,
        settings.advanced_package_names,
    )
}

/// Holds scanned candidates for installation and fills in what only the
/// library can answer: what they overlap, what they replace, and where they
/// would sit in the load order.
fn register_previews(
    ctx: &AppContext,
    found: Vec<(StagedMod, ModPreview)>,
) -> Result<Vec<ModPreview>> {
    let conn = connection(ctx)?;
    let mut result = Vec::new();
    let mut held = previews(ctx)?;
    for (staged, mut preview) in found {
        preview.conflicts = database::conflicts_for_packages(&conn, &staged.packages)?;
        preview.replaces = replaced_by(&conn, &staged)?;
        if preview.load_order_supported {
            preview.recommended_priority = Some(database::next_load_priority(&conn)?);
        }
        held.insert(staged.staging_id.clone(), staged);
        result.push(preview);
    }
    Ok(result)
}

/// Opens a scripted installer, or completes it outright when its script asks
/// nothing: a package whose every question is hidden has already been answered,
/// and showing an empty wizard would be a step with no purpose.
fn begin_installer(
    ctx: &AppContext,
    source: &Path,
    installer: fomod::Installer,
    staging: archives::Staging,
    mut assessment: PackageAssessment,
) -> Result<Inspection> {
    assessment.role = "modBundle".into();
    assessment.title = "Scripted mod bundle".into();
    assessment.reason = "The package uses a FOMOD selection script. Only files selected in the review flow are staged; no script is executed.".into();
    let session_id = uuid::Uuid::new_v4().to_string();
    let opened = installer.session(&session_id, &[]);
    let session = match opened {
        Ok(session) => session,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging.root);
            return Err(error);
        }
    };
    let pending = fomod::Pending {
        installer,
        staging_root: staging.root,
        source: source.to_path_buf(),
        reconfigure: None,
    };
    if session.complete {
        let previews = apply_installer(ctx, &pending, &[])?;
        return Ok(Inspection {
            previews,
            installer: None,
            package: assessment,
        });
    }
    log(
        ctx,
        "info",
        "fomod_started",
        &format!("module={} session={session_id}", session.module_name),
    );
    installers(ctx)?.insert(session_id, pending);
    Ok(Inspection {
        previews: Vec::new(),
        installer: Some(session),
        package: assessment,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FomodReconfiguration {
    previews: Vec<ModPreview>,
    installer: Option<fomod::Session>,
    answers: Vec<fomod::StepAnswer>,
}

/// Reopens the complete FOMOD tree retained alongside an installed mod. The
/// retained copy is staged again so cancellation and preview cleanup can never
/// remove or modify the library's durable source.
#[tauri::command]
pub fn reconfigure_fomod(id: String, ctx: State<'_, AppContext>) -> Result<FomodReconfiguration> {
    let conn = connection(&ctx)?;
    let summary = database::list_mods(&conn)?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| AppError::Other("That mod is no longer installed.".into()))?;
    let (source_archive, answers_json) = database::fomod_install(&conn, &id)?.ok_or_else(|| {
        AppError::Other("That mod was not installed through a retained FOMOD.".into())
    })?;
    let answers: Vec<fomod::StepAnswer> = serde_json::from_str(&answers_json)
        .map_err(|_| AppError::Other("The saved FOMOD choices are unreadable.".into()))?;
    let library = mods_dir(&ctx)?;
    let retained = library.join(&id).join("fomod-source");
    if !retained.is_dir() {
        return Err(AppError::Other(
            "The retained FOMOD source is missing. Reinstall this mod from its original archive once to restore reconfiguration.".into(),
        ));
    }
    let staging = archives::stage(&retained, &ctx.cache_dir)?;
    let package_root = fomod::locate(&staging.root).ok_or_else(|| {
        let _ = std::fs::remove_dir_all(&staging.root);
        AppError::Other("The retained source no longer contains a FOMOD installer.".into())
    })?;
    let installer = match fomod::parse(&package_root) {
        Ok(installer) => installer,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging.root);
            return Err(error);
        }
    };
    let session_id = uuid::Uuid::new_v4().to_string();
    let session = match installer.session(&session_id, &[]) {
        Ok(session) => session,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging.root);
            return Err(error);
        }
    };
    let target = ReplacedMod {
        mod_id: summary.id.clone(),
        name: summary.name.clone(),
        version: summary.version.clone(),
        reason: "Reconfiguring the retained guided installer.".into(),
    };
    let pending = fomod::Pending {
        installer,
        staging_root: staging.root,
        source: source_archive
            .filter(|source| !source.is_empty())
            .map(PathBuf::from)
            .unwrap_or(retained),
        reconfigure: Some((target, summary.mod_type)),
    };
    if session.complete {
        let previews = apply_installer(&ctx, &pending, &answers)?;
        return Ok(FomodReconfiguration {
            previews,
            installer: None,
            answers,
        });
    }
    installers(&ctx)?.insert(session_id, pending);
    log(&ctx, "info", "fomod_reconfigured", &format!("mod_id={id}"));
    Ok(FomodReconfiguration {
        previews: Vec::new(),
        installer: Some(session),
        answers,
    })
}

/// Writes the files a set of answers selects into a sandbox of their own, then
/// reads that back as though it were the archive the person downloaded.
fn apply_installer(
    ctx: &AppContext,
    pending: &fomod::Pending,
    answers: &[fomod::StepAnswer],
) -> Result<Vec<ModPreview>> {
    let root = ctx
        .cache_dir
        .join("staging")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&root)?;
    let built = match pending.installer.install(answers, &root) {
        Ok(built) => built,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&root);
            return Err(error);
        }
    };
    // `fomod/info.xml` is where a scripted package states its own title,
    // author, and version, and the selected files rarely carry a manifest of
    // their own. Writing one lets that metadata reach the preview through the
    // same path every other mod's metadata takes.
    if let Err(error) = write_installer_manifest(&pending.installer, &root) {
        log(
            ctx,
            "warn",
            "fomod_manifest_not_written",
            &error.to_string(),
        );
    }
    let staging = archives::Staging {
        root,
        warnings: built.warnings,
        executables: built.executables,
    };
    let mut found = scan_staged(ctx, &pending.source, staging)?;
    let answers_json = serde_json::to_string(answers)?;
    for (staged, _) in &mut found {
        staged.fomod_source_root = Some(pending.staging_root.clone());
        staged.fomod_answers = Some(answers_json.clone());
    }
    let mut previews = register_previews(ctx, found)?;
    if let Some((target, mod_type)) = &pending.reconfigure {
        let already_targeted = previews.iter().any(|preview| {
            preview
                .replaces
                .as_ref()
                .is_some_and(|item| item.mod_id == target.mod_id)
        });
        if !already_targeted {
            let named = previews
                .iter()
                .enumerate()
                .filter(|(_, preview)| preview.name.eq_ignore_ascii_case(&target.name))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let typed = previews
                .iter()
                .enumerate()
                .filter(|(_, preview)| preview.mod_type == *mod_type)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let index = if named.len() == 1 {
                named.first().copied()
            } else if typed.len() == 1 {
                typed.first().copied()
            } else if previews.len() == 1 {
                Some(0)
            } else {
                None
            };
            if let Some(index) = index {
                previews[index].replaces = Some(target.clone());
            }
        }
    }
    Ok(previews)
}

/// Records the script's own metadata as a manifest, unless the files it
/// installed already brought one.
fn write_installer_manifest(installer: &fomod::Installer, root: &Path) -> Result<()> {
    let existing = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .any(|entry| {
            entry.file_type().is_file() && entry.file_name().eq_ignore_ascii_case("zcom-mod.json")
        });
    if existing {
        return Ok(());
    }
    let info = &installer.info;
    let name = info
        .name
        .clone()
        .unwrap_or_else(|| installer.module_name.clone());
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": format!("fomod.{}", slug.trim_matches('-')),
        "name": name,
        "version": info.version,
        "author": info.author,
        "description": info.description,
    });
    std::fs::write(
        root.join("zcom-mod.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(())
}

/// Answers the question a scripted installer is on and reports the next one.
///
/// The answers given so far arrive in full on every call, and the script is
/// replayed from the start against them. That is what lets a person go back
/// and change an earlier answer: dropping it from the list is enough to undo
/// everything it decided, including which later questions get asked at all.
#[tauri::command]
pub fn fomod_advance(
    session_id: String,
    answers: Vec<fomod::StepAnswer>,
    ctx: State<'_, AppContext>,
) -> Result<fomod::Session> {
    let held = installers(&ctx)?;
    let pending = held.get(&session_id).ok_or(AppError::PreviewExpired)?;
    pending.installer.session(&session_id, &answers)
}

/// Finishes a scripted installer and hands back the mods its answers produced.
#[tauri::command]
pub fn fomod_install(
    session_id: String,
    answers: Vec<fomod::StepAnswer>,
    ctx: State<'_, AppContext>,
) -> Result<Vec<ModPreview>> {
    let pending = installers(&ctx)?
        .remove(&session_id)
        .ok_or(AppError::PreviewExpired)?;
    let result = apply_installer(&ctx, &pending, &answers);
    match &result {
        // The complete option tree remains referenced by the previews. It is
        // copied into the managed library on confirmation, then released after
        // the last preview from this installer is consumed.
        Ok(previews) => {
            log(
                &ctx,
                "info",
                "fomod_completed",
                &format!("session={session_id} mods={}", previews.len()),
            );
        }
        // A refused answer is worth another try, so the session goes back.
        Err(error) => {
            log(
                &ctx,
                "warn",
                "fomod_failed",
                &format!("session={session_id} error={error}"),
            );
            installers(&ctx)?.insert(session_id, pending);
        }
    }
    result
}

/// Abandons a scripted installer and removes the archive it was reading.
#[tauri::command]
pub fn fomod_cancel(session_id: String, ctx: State<'_, AppContext>) -> Result<()> {
    if let Some(pending) = installers(&ctx)?.remove(&session_id) {
        let _ = std::fs::remove_dir_all(&pending.staging_root);
    }
    Ok(())
}

/// Every orderable mod, highest priority first.
fn ordered_supported(conn: &rusqlite::Connection) -> Result<Vec<String>> {
    Ok(load_order::state(conn)?
        .entries
        .into_iter()
        .filter(|entry| entry.supported)
        .map(|entry| entry.id)
        .collect())
}

/// Puts a replacement in the slot its predecessor held. Anything newly
/// orderable, or orderable for the first time, falls in at the end.
fn keep_position(
    conn: &rusqlite::Connection,
    previous: &[String],
    old_id: &str,
    new_id: &str,
) -> Result<Vec<String>> {
    let supported = ordered_supported(conn)?;
    let mut ordered: Vec<String> = previous
        .iter()
        .map(|id| {
            if id == old_id {
                new_id.to_string()
            } else {
                id.clone()
            }
        })
        .filter(|id| supported.contains(id))
        .collect();
    let appended: Vec<String> = supported
        .into_iter()
        .filter(|id| !ordered.contains(id))
        .collect();
    ordered.extend(appended);
    Ok(ordered)
}

/// The installed mod a candidate would land on top of.
///
/// A newer build of a mod occupies exactly the same runtime folder or payload
/// file names as the one already installed. Reporting that as a deployment
/// conflict made updating a mod a two-step chore, so it is surfaced as the
/// upgrade it is.
fn replaced_by(conn: &rusqlite::Connection, staged: &StagedMod) -> Result<Option<ReplacedMod>> {
    let found = match staged.mod_type.as_str() {
        "ue4ss" => staged
            .deployment_keys
            .iter()
            .find_map(|key| database::ue4ss_folder_owner(conn, key).transpose())
            .transpose()?
            .map(|id| (id, "It uses the same UE4SS mod folder.".to_string())),
        "pak" | "iostore" => staged
            .files
            .iter()
            .find_map(|file| {
                database::packaged_source_name_owner(
                    conn,
                    &file.library_relative.display().to_string(),
                    None,
                )
                .transpose()
            })
            .transpose()?
            .map(|id| (id, "It ships the same container files.".to_string())),
        "gamedir" | "plugin" | "config" => {
            let game = match game_path(conn)? {
                Some(path) => path,
                None => return Ok(None),
            };
            staged
                .files
                .iter()
                .find_map(|file| {
                    database::destination_owner(
                        conn,
                        &deployment::destination_base(&game, &staged.mod_type)
                            .join(&file.destination_relative)
                            .display()
                            .to_string(),
                        None,
                    )
                    .transpose()
                })
                .transpose()?
                .map(|id| (id, "It writes to the same place in the game folder.".into()))
        }
        _ => None,
    };
    found
        .map(|(mod_id, reason)| {
            let (name, version) = database::summary_of(conn, &mod_id)?;
            Ok(ReplacedMod {
                mod_id,
                name,
                version,
                reason,
            })
        })
        .transpose()
}

/// The game folder as the database and Steam currently resolve it, without
/// failing when no game is connected: an inspection is still useful then.
fn game_path(conn: &rusqlite::Connection) -> Result<Option<PathBuf>> {
    let settings = database::settings(conn)?;
    let info = if let Some(path) = settings.game_path.filter(|p| !p.is_empty()) {
        steam::from_manual(Path::new(&path)).ok()
    } else {
        steam::discover().ok().flatten()
    };
    Ok(info.and_then(|info| info.path).map(PathBuf::from))
}

/// Drops the staged copy of an archive once no preview still refers to it.
/// Several previews share one extraction, so the sandbox outlives the first
/// installation and is cleaned up after the last.
fn release_staging(ctx: &AppContext, root: &Path) {
    let still_needed = previews(ctx)
        .map(|held| {
            held.values().any(|staged| {
                staged.staging_root == root || staged.fomod_source_root.as_deref() == Some(root)
            })
        })
        .unwrap_or(true);
    if !still_needed {
        let _ = std::fs::remove_dir_all(root);
    }
}

/// Forgets every preview taken from one archive, and removes its sandbox.
#[tauri::command]
pub fn discard_previews(staging_ids: Vec<String>, ctx: State<'_, AppContext>) -> Result<()> {
    let mut roots = Vec::new();
    {
        let mut held = previews(&ctx)?;
        for id in &staging_ids {
            if let Some(staged) = held.remove(id) {
                roots.push(staged.staging_root);
                if let Some(root) = staged.fomod_source_root {
                    roots.push(root);
                }
            }
        }
    }
    roots.sort();
    roots.dedup();
    for root in roots {
        release_staging(&ctx, &root);
    }
    Ok(())
}

/// Renames an installed mod. Only the label changes: deployed file names, the
/// UE4SS folder names, and every recorded checksum stay exactly as they are.
#[tauri::command]
pub fn rename_mod(id: String, name: String, ctx: State<'_, AppContext>) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Other("A mod needs a name.".into()));
    }
    if name.chars().count() > 120 {
        return Err(AppError::Other(
            "That name is too long. Use 120 characters or fewer.".into(),
        ));
    }
    database::rename_mod(&connection(&ctx)?, &id, name)?;
    log(&ctx, "info", "mod_renamed", &format!("mod_id={id}"));
    Ok(())
}

/// Installs a staged mod, optionally over the one it supersedes.
///
/// `replace` carries the id the preview reported in `replaces`. Passing it
/// upgrades in place; leaving it out installs alongside, which fails with the
/// usual deployment conflict when the two really do collide.
///
/// `force` overrides the checksum guard on the mod being replaced. A mod that
/// writes its own settings or data files into its deployed folder changes them
/// while the game runs, and without an override the user could neither update
/// nor remove it.
#[tauri::command]
pub fn install_mod(
    staging_id: String,
    name: Option<String>,
    replace: Option<String>,
    force: bool,
    ctx: State<'_, AppContext>,
) -> Result<ModSummary> {
    package_operation(&ctx, || {
        let mut staged = previews(&ctx)?
            .get(&staging_id)
            .cloned()
            .ok_or(AppError::PreviewExpired)?;
        if let Some(name) = name
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
        {
            staged.name = name.chars().take(120).collect();
        }
        if staged.mod_type == "ue4ss-runtime" {
            return Err(AppError::Other(
                "That archive is the UE4SS runtime. Install it with the UE4SS button instead."
                    .into(),
            ));
        }
        let (game_info, game_path) = require_game(&ctx)?;
        let mut conn = connection(&ctx)?;
        // Position the replacement where the mod it supersedes sat, rather than at
        // the top, so an upgrade does not silently change which mod wins.
        let previous_order = replace
            .as_ref()
            .map(|_| ordered_supported(&conn))
            .transpose()?;
        let library = mods_dir(&ctx)?;
        let result = match replace.as_deref() {
            Some(old_id) => deployment::replace(
                &mut conn,
                &library,
                &game_path,
                old_id,
                &staged,
                game_info.steam_build_id,
                force,
            ),
            None => deployment::install(
                &mut conn,
                &library,
                &game_path,
                &staged,
                game_info.steam_build_id,
            ),
        };
        let summary = result?;
        if let Some(manifest) = &staged.manifest {
            if let Err(error) = compatibility::install_author_manifest(&conn, &summary.id, manifest)
            {
                let _ = deployment::uninstall(&conn, &library, &summary.id, true, Some(&game_path));
                return Err(error);
            }
            if let Some(nexus) = &manifest.nexus {
                if let (Some(mod_id), Some(file_id)) = (nexus.mod_id, nexus.file_id) {
                    database::set_nexus_ids(&conn, &summary.id, mod_id, file_id)?;
                }
            }
        }
        if summary.mod_type == "ue4ss" {
            // The recorded start order is the source of truth, so mods.txt is
            // rewritten from it. For a fresh install that only confirms the entry
            // just appended; for an upgrade it restores the slot it inherited.
            let ordered = load_order::state(&conn)?
                .ue4ss_entries
                .into_iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>();
            if let Err(error) = load_order::apply_ue4ss_order(&mut conn, &game_path, &ordered) {
                log(&ctx, "warn", "ue4ss_order_not_written", &error.to_string());
            }
        }
        if matches!(summary.mod_type.as_str(), "pak" | "iostore") {
            let ordered = match (previous_order, replace.as_deref()) {
                (Some(previous), Some(old_id)) => {
                    keep_position(&conn, &previous, old_id, &summary.id)?
                }
                _ => ordered_supported(&conn)?,
            };
            if let Err(error) = load_order::apply(
                &mut conn,
                &ordered,
                &ctx.data_dir.join("load-order-operation.json"),
            ) {
                let _ = deployment::uninstall(&conn, &library, &summary.id, true, Some(&game_path));
                return Err(error);
            }
        }
        // Keep a failed preview available for retry. Only a fully successful
        // deployment consumes its staging id and may release the shared bundle
        // sandbox.
        previews(&ctx)?.remove(&staging_id);
        release_staging(&ctx, &staged.staging_root);
        if let Some(root) = &staged.fomod_source_root {
            release_staging(&ctx, root);
        }
        log(
            &ctx,
            "info",
            "mod_installed",
            &format!("mod_id={} type={}", summary.id, summary.mod_type),
        );
        profiles::capture_active(&conn)?;
        let _ = operations::record(
            &conn,
            "install",
            "completed",
            &format!("Installed {}", summary.name),
            serde_json::json!({"modId": &summary.id, "type": &summary.mod_type}),
        );
        Ok(summary)
    })
}

/// Deploys a complete inspected package, with explicit whole-package replacement
/// consent and durable file/database rollback covering metadata and load order.
#[tauri::command]
pub fn install_bundle(
    items: Vec<BundleInstallItem>,
    replace_bundle_id: Option<String>,
    ctx: State<'_, AppContext>,
) -> Result<BundleInstallReport> {
    if !(1..=64).contains(&items.len()) {
        return Err(AppError::Other(
            "A bundle install needs between 1 and 64 components.".into(),
        ));
    }
    let mut unique = std::collections::HashSet::new();
    if items
        .iter()
        .any(|item| !unique.insert(item.staging_id.as_str()))
    {
        return Err(AppError::Other(
            "A bundle cannot contain the same staged component twice.".into(),
        ));
    }
    let staged = {
        let held = previews(&ctx)?;
        items
            .iter()
            .map(|item| {
                let mut component = held
                    .get(&item.staging_id)
                    .cloned()
                    .ok_or(AppError::PreviewExpired)?;
                if let Some(name) = item
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                {
                    component.name = name.chars().take(120).collect();
                }
                Ok(component)
            })
            .collect::<Result<Vec<_>>>()?
    };
    let source = staged
        .first()
        .map(|component| component.source_archive.clone())
        .ok_or_else(|| AppError::Other("The bundle has no components.".into()))?;
    if staged
        .iter()
        .any(|component| component.source_archive != source)
    {
        return Err(AppError::Other(
            "Atomic bundle install only accepts components from the same inspected archive.".into(),
        ));
    }
    for component in &staged {
        if component.mod_type == "ue4ss-runtime" {
            return Err(AppError::Other(
                "A UE4SS runtime cannot be installed as a mod bundle component.".into(),
            ));
        }
    }

    let (game_info, game_path) = require_game(&ctx)?;
    deployment::ensure_game_stopped()?;
    let mut conn = connection(&ctx)?;
    if database::get_setting(&conn, "pending_restore_profile")?.is_some() {
        return Err(AppError::Other(
            "Restore the temporary launch profile before updating mods.".into(),
        ));
    }
    let all = database::list_mods(&conn)?;
    let matched = staged
        .iter()
        .map(|item| replaced_by(&conn, item).map(|r| r.map(|r| r.mod_id)))
        .collect::<Result<Vec<_>>>()?;
    let old = match &replace_bundle_id {
        Some(bundle) => {
            let members = all
                .iter()
                .filter(|item| item.bundle_id.as_ref() == Some(bundle))
                .cloned()
                .collect::<Vec<_>>();
            if members.is_empty() {
                return Err(AppError::Other(
                    "The selected package no longer exists. Inspect the archive again.".into(),
                ));
            }
            if matched
                .iter()
                .flatten()
                .any(|id| !members.iter().any(|m| &m.id == id))
            {
                return Err(AppError::Other(
                    "This archive also replaces another mod. No files were changed.".into(),
                ));
            }
            members
        }
        None if matched.iter().any(Option::is_some) => {
            return Err(AppError::Other(
                "Confirm the complete package update before installing.".into(),
            ))
        }
        None => Vec::new(),
    };
    let bundle_id = replace_bundle_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let library = mods_dir(&ctx)?;
    let installed =
        crate::package_transaction::run(&mut conn, &ctx.data_dir, &library, &game_path, |conn| {
            let installed = crate::packages::deploy(
                conn,
                &library,
                &game_path,
                &staged,
                &old,
                &matched,
                game_info.steam_build_id.clone(),
                &bundle_id,
            )?;
            for (component, summary) in staged.iter().zip(&installed) {
                if let Some(manifest) = &component.manifest {
                    compatibility::install_author_manifest(conn, &summary.id, manifest)?;
                    if let Some(nexus) = &manifest.nexus {
                        if let (Some(mod_id), Some(file_id)) = (nexus.mod_id, nexus.file_id) {
                            database::set_nexus_ids(conn, &summary.id, mod_id, file_id)?;
                        }
                    }
                }
            }
            let ue4ss_order = load_order::state(conn)?
                .ue4ss_entries
                .into_iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>();
            load_order::apply_ue4ss_order(conn, &game_path, &ue4ss_order)?;
            let packaged_order = ordered_supported(conn)?;
            load_order::apply(
                conn,
                &packaged_order,
                &ctx.data_dir.join("load-order-operation.json"),
            )?;
            profiles::capture_active(conn)?;
            Ok(installed)
        })?;

    // Staging is consumed only after deployment, metadata, ordering and active
    // profile capture all succeed.
    let ids = items
        .iter()
        .map(|item| item.staging_id.clone())
        .collect::<Vec<_>>();
    let mut roots = Vec::new();
    {
        let mut held = previews(&ctx)?;
        for id in &ids {
            if let Some(component) = held.remove(id) {
                roots.push(component.staging_root);
                if let Some(root) = component.fomod_source_root {
                    roots.push(root);
                }
            }
        }
    }
    roots.sort();
    roots.dedup();
    for root in roots {
        release_staging(&ctx, &root);
    }
    let _ = operations::record(
        &conn,
        "bundle-install",
        "completed",
        &format!("Installed {} bundle components", installed.len()),
        serde_json::json!({"bundleId": &bundle_id, "modIds": installed.iter().map(|item| &item.id).collect::<Vec<_>>()}),
    );
    log(
        &ctx,
        "info",
        "bundle_installed",
        &format!("bundle_id={bundle_id} components={}", installed.len()),
    );
    Ok(BundleInstallReport {
        bundle_id,
        components: installed,
    })
}

#[tauri::command]
pub fn set_mod_enabled(
    id: String,
    enabled: bool,
    force: bool,
    ctx: State<'_, AppContext>,
) -> Result<()> {
    package_operation(&ctx, || {
        let (_, game_path) = require_game(&ctx)?;
        let conn = connection(&ctx)?;
        let library = mods_dir(&ctx)?;
        deployment::set_enabled(&conn, &library, &game_path, &id, enabled, force)?;
        profiles::capture_active(&conn)?;
        let _ = operations::record(
            &conn,
            if enabled { "enable" } else { "disable" },
            "completed",
            if enabled {
                "Mod enabled"
            } else {
                "Mod disabled"
            },
            serde_json::json!({"modId": &id}),
        );
        log(
            &ctx,
            "info",
            if enabled {
                "mod_enabled"
            } else {
                "mod_disabled"
            },
            &format!("mod_id={id}"),
        );
        Ok(())
    })
}

/// Keeps a mod out of the library list without touching what is deployed.
/// Existing-mod discovery adopts the UE4SS runtime's own bundled mods, which
/// have to stay installed and ordered but do not need to be looked at.
#[tauri::command]
pub fn set_mod_hidden(id: String, hidden: bool, ctx: State<'_, AppContext>) -> Result<()> {
    database::set_hidden(&connection(&ctx)?, &id, hidden)?;
    log(
        &ctx,
        "info",
        if hidden { "mod_hidden" } else { "mod_shown" },
        &format!("mod_id={id}"),
    );
    Ok(())
}

#[tauri::command]
pub fn uninstall_bundle(id: String, ctx: State<'_, AppContext>) -> Result<()> {
    let game_path = game(&ctx)?.path.map(PathBuf::from).ok_or_else(|| {
        AppError::Other("Connect the game folder before removing a package.".into())
    })?;
    let library = mods_dir(&ctx)?;
    let mut conn = connection(&ctx)?;
    if database::get_setting(&conn, "pending_restore_profile")?.is_some() {
        return Err(AppError::Other(
            "Restore the temporary launch profile before removing mods.".into(),
        ));
    }
    let mods = database::list_mods(&conn)?;
    let anchor = mods
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| AppError::Other("This mod is no longer installed.".into()))?;
    let ids = mods
        .iter()
        .filter(|item| {
            item.id == id || (anchor.bundle_id.is_some() && item.bundle_id == anchor.bundle_id)
        })
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    deployment::validate_removal(&conn, &ids)?;
    crate::package_transaction::run(&mut conn, &ctx.data_dir, &library, &game_path, |conn| {
        for member in &ids {
            deployment::uninstall(conn, &library, member, false, Some(&game_path))?;
        }
        profiles::capture_active(conn)?;
        operations::record(
            conn,
            "bundle-uninstall",
            "completed",
            "Package removed",
            serde_json::json!({"modIds":ids}),
        )?;
        Ok(())
    })
}

#[tauri::command]
pub fn uninstall_mod(id: String, force: bool, ctx: State<'_, AppContext>) -> Result<()> {
    package_operation(&ctx, || {
        if database::get_setting(&connection(&ctx)?, "pending_restore_profile")?.is_some() {
            return Err(AppError::Other(
                "Restore the temporary launch profile before removing mods.".into(),
            ));
        }
        let game_path = game(&ctx)?.path.map(PathBuf::from);
        let conn = connection(&ctx)?;
        let library = mods_dir(&ctx)?;
        deployment::uninstall(&conn, &library, &id, force, game_path.as_deref())?;
        profiles::capture_active(&conn)?;
        let _ = operations::record(
            &conn,
            "uninstall",
            "completed",
            "Mod uninstalled",
            serde_json::json!({"modId": &id}),
        );
        log(&ctx, "info", "mod_uninstalled", &format!("mod_id={id}"));
        Ok(())
    })
}

#[tauri::command]
pub fn verify_mod(id: String, ctx: State<'_, AppContext>) -> Result<String> {
    deployment::verify(&connection(&ctx)?, &id)
}

/// Installs a user-downloaded UE4SS package. Nothing is fetched from the
/// network: the user supplies the archive, and it is staged in the same
/// sandbox that mod archives use before any file reaches the game folder.
#[tauri::command]
pub fn install_ue4ss(
    path: String,
    ctx: State<'_, AppContext>,
) -> Result<crate::models::Ue4ssInstallReport> {
    deployment::ensure_game_stopped()?;
    let (_, game_path) = require_game(&ctx)?;
    let report = ue4ss::install_from(Path::new(&path), &game_path, &ctx.cache_dir)?;
    log(
        &ctx,
        "info",
        "ue4ss_installed",
        &format!(
            "files={} preserved={}",
            report.installed,
            report.preserved.len()
        ),
    );
    let conn = connection(&ctx)?;
    let _ = operations::record(
        &conn,
        "runtime",
        "completed",
        "UE4SS runtime installed",
        serde_json::json!({"files": report.installed, "preserved": report.preserved.len()}),
    );
    Ok(report)
}

/// External resources the interface links to. Kept on this side so the
/// download target has a single definition shared with diagnostics.
#[tauri::command]
pub fn get_links() -> Links {
    Links {
        ue4ss_download: ue4ss::DOWNLOAD_URL.into(),
        nexus_game: "https://www.nexusmods.com/games/starwarszerocompany/mods".into(),
        // The manager's own Nexus page. A release reaches Nexus and GitHub
        // alike, and someone who found the manager on Nexus expects to update
        // it there.
        nexus_manager: option_env!("ZERO_MOD_MANAGER_NEXUS_URL")
            .unwrap_or("")
            .into(),
        project: option_env!("ZERO_MOD_MANAGER_PROJECT_URL")
            .unwrap_or("")
            .into(),
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    pub ue4ss_download: String,
    pub nexus_game: String,
    pub nexus_manager: String,
    pub project: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub update_available: bool,
    pub release_available: bool,
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    match (
        semver::Version::parse(latest.trim_start_matches(['v', 'V'])),
        semver::Version::parse(current.trim_start_matches(['v', 'V'])),
    ) {
        (Ok(latest), Ok(current)) => latest > current,
        _ => false,
    }
}
fn select_release(value: &serde_json::Value, allow_prerelease: bool) -> Option<&serde_json::Value> {
    let items = value
        .as_array()
        .map(|items| items.iter().collect::<Vec<_>>())
        .unwrap_or_else(|| vec![value]);
    items
        .into_iter()
        .filter(|item| item["draft"].as_bool() != Some(true))
        .filter_map(|item| {
            let version =
                semver::Version::parse(item["tag_name"].as_str()?.trim_start_matches(['v', 'V']))
                    .ok()?;
            if !allow_prerelease
                && (!version.pre.is_empty() || item["prerelease"].as_bool() == Some(true))
            {
                return None;
            }
            Some((version, item))
        })
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, item)| item)
}

/// Queries the latest published GitHub release. The interface calls this once
/// at startup and also exposes an explicit retry on the About page.
#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateInfo> {
    let release_api = option_env!("ZERO_MOD_MANAGER_RELEASE_API").ok_or_else(|| {
        AppError::Other(
            "Update checking is disabled until the continuation repository is published.".into(),
        )
    })?;
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let allow_prerelease = !semver::Version::parse(&current_version)
        .map_err(|e| AppError::Other(e.to_string()))?
        .pre
        .is_empty();
    let endpoint = if allow_prerelease {
        format!("{}?per_page=30", release_api.trim_end_matches("/latest"))
    } else {
        release_api.to_string()
    };
    let unavailable = || UpdateInfo {
        current_version: current_version.clone(),
        latest_version: current_version.clone(),
        release_url: format!(
            "{}/releases",
            option_env!("ZERO_MOD_MANAGER_PROJECT_URL").unwrap_or("")
        ),
        update_available: false,
        release_available: false,
    };
    let response = reqwest::Client::builder()
        .user_agent(format!("zero-mod-manager/{current_version}"))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| AppError::Network(error.to_string()))?
        .get(endpoint)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| AppError::Network(error.to_string()))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(unavailable());
    }
    let response = response
        .error_for_status()
        .map_err(|error| AppError::Network(error.to_string()))?;
    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|error| AppError::Network(error.to_string()))?;
    let Some(release) = select_release(&payload, allow_prerelease) else {
        return Ok(unavailable());
    };
    let tag = release["tag_name"]
        .as_str()
        .ok_or_else(|| AppError::Other("GitHub returned a release without a version.".into()))?;
    let release_url = release["html_url"]
        .as_str()
        .ok_or_else(|| AppError::Other("GitHub returned a release without a link.".into()))?;
    let latest_version = tag.trim_start_matches(['v', 'V']).to_string();
    Ok(UpdateInfo {
        update_available: version_is_newer(&latest_version, &current_version),
        release_available: true,
        current_version,
        latest_version,
        release_url: release_url.to_string(),
    })
}
#[tauri::command]
pub fn run_diagnostics(ctx: State<'_, AppContext>) -> Result<DiagnosticReport> {
    let conn = connection(&ctx)?;
    let game = game(&ctx)?;
    let ue = ue4ss::detect(
        game.path.as_deref().map(Path::new),
        game.compat_data_path.as_deref().map(Path::new),
    );
    diagnostics::run(&conn, &game, &ue, &tool(&ctx)?)
}
#[tauri::command]
pub fn diagnostic_report(ctx: State<'_, AppContext>) -> Result<String> {
    Ok(run_diagnostics(ctx)?.text)
}

#[tauri::command]
pub fn legacy_import_status(
    ctx: State<'_, AppContext>,
) -> Result<crate::migration::LegacyImportStatus> {
    crate::migration::status(&ctx)
}

#[tauri::command]
pub fn import_legacy_data(
    ctx: State<'_, AppContext>,
) -> Result<crate::migration::LegacyImportReport> {
    let report = crate::migration::import(&ctx, false)?;
    log(
        &ctx,
        "info",
        "legacy_data_imported",
        &format!(
            "mods={} files={} bytes={} nexus_key={}",
            report.imported_mods,
            report.copied_files,
            report.copied_bytes,
            report.nexus_key_imported
        ),
    );
    let conn = connection(&ctx)?;
    profiles::capture_active(&conn)?;
    let _ = operations::record(
        &conn,
        "migration",
        "completed",
        "Legacy ZCOM data imported",
        serde_json::json!({"mods": report.imported_mods, "files": report.copied_files}),
    );
    Ok(report)
}
#[tauri::command]
pub fn get_settings(ctx: State<'_, AppContext>) -> Result<AppSettings> {
    database::settings(&connection(&ctx)?)
}
#[tauri::command]
pub fn save_settings(mut settings: AppSettings, ctx: State<'_, AppContext>) -> Result<()> {
    if settings.game_path.as_deref() == Some("") {
        settings.game_path = None
    }
    if settings
        .seven_zip_path
        .as_deref()
        .is_some_and(|path| path.trim().is_empty())
    {
        settings.seven_zip_path = None
    }
    if settings
        .custom_executable_path
        .as_deref()
        .is_some_and(|path| path.trim().is_empty())
    {
        settings.custom_executable_path = None
    }
    database::save_settings(&connection(&ctx)?, &settings)?;
    // Extraction reads this from a process-wide slot, so the new choice has to
    // reach it here rather than at the next start-up.
    archives::set_seven_zip_path(settings.seven_zip_path.as_deref());
    Ok(())
}

/// Reports the archive tool the manager would use, so Settings can show
/// whether 7-Zip was found before an install runs into it.
#[tauri::command]
pub fn seven_zip_status() -> ToolInfo {
    archives::seven_zip_info()
}
#[tauri::command]
pub fn set_game_path(path: String, ctx: State<'_, AppContext>) -> Result<GameInfo> {
    let info = steam::from_manual(Path::new(&path))?;
    database::set_setting(&connection(&ctx)?, "game_path", &path)?;
    Ok(info)
}

fn library_info(ctx: &AppContext, path: &Path) -> ManagedLibraryInfo {
    ManagedLibraryInfo {
        path: path.display().to_string(),
        default_path: ctx.default_mods_dir.display().to_string(),
        is_default: paths_are_same(path, &ctx.default_mods_dir),
    }
}

fn paths_are_same(left: &Path, right: &Path) -> bool {
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

#[tauri::command]
pub fn get_managed_library(ctx: State<'_, AppContext>) -> Result<ManagedLibraryInfo> {
    let library = mods_dir(&ctx)?;
    Ok(library_info(&ctx, &library))
}

/// Copies every managed payload and backup to a verified staging directory,
/// then swaps that directory into the selected empty destination. The source
/// is intentionally retained until both the database and in-memory path point
/// at the completed copy.
fn copy_library_for_move(source: &Path, destination: &Path) -> Result<()> {
    if !destination.is_absolute() {
        return Err(AppError::Other(
            "Choose an absolute path for the managed mod library.".into(),
        ));
    }
    std::fs::create_dir_all(destination)?;
    if std::fs::symlink_metadata(source)?.file_type().is_symlink()
        || std::fs::symlink_metadata(destination)?
            .file_type()
            .is_symlink()
    {
        return Err(AppError::Other(
            "A symbolic link cannot be used as a managed library location.".into(),
        ));
    }
    let source_real = std::fs::canonicalize(source)?;
    let destination_real = std::fs::canonicalize(destination)?;
    if source_real == destination_real {
        return Err(AppError::Other(
            "That folder is already the managed mod library.".into(),
        ));
    }
    if destination_real.starts_with(&source_real) || source_real.starts_with(&destination_real) {
        return Err(AppError::Other(
            "The new library cannot contain the current library, or be inside it.".into(),
        ));
    }
    if std::fs::read_dir(destination)?.next().is_some() {
        return Err(AppError::Other(
            "The selected library folder is not empty. Create or select an empty folder.".into(),
        ));
    }
    let parent = destination.parent().ok_or_else(|| {
        AppError::Other("A drive root cannot be used as the managed library folder.".into())
    })?;
    let temporary = parent.join(format!(".zcom-library-migration-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&temporary)?;
    let copied = (|| -> Result<()> {
        for entry in walkdir::WalkDir::new(source).follow_links(false) {
            let entry = entry.map_err(|error| {
                AppError::Other(format!("The managed library could not be read: {error}"))
            })?;
            if entry.path() == source {
                continue;
            }
            let relative = entry.path().strip_prefix(source).map_err(|_| {
                AppError::Other("A managed library entry escaped its source folder.".into())
            })?;
            let target = temporary.join(relative);
            if entry.file_type().is_symlink() {
                return Err(AppError::Other(format!(
                    "The managed library contains a symbolic link and was not moved: {}",
                    relative.display()
                )));
            }
            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&target)?;
            } else if entry.file_type().is_file() {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(entry.path(), &target)?;
                if crate::deployment::sha256(entry.path())? != crate::deployment::sha256(&target)? {
                    return Err(AppError::Other(format!(
                        "Verification failed while copying {}.",
                        relative.display()
                    )));
                }
            } else {
                return Err(AppError::Other(format!(
                    "The managed library contains an unsupported filesystem entry: {}",
                    relative.display()
                )));
            }
        }
        std::fs::remove_dir(destination)?;
        std::fs::rename(&temporary, destination)?;
        Ok(())
    })();
    if copied.is_err() {
        let _ = std::fs::remove_dir_all(&temporary);
        if !destination.exists() {
            let _ = std::fs::create_dir_all(destination);
        }
    }
    copied
}

fn move_managed_library_inner(path: String, ctx: &AppContext) -> Result<ManagedLibraryInfo> {
    let destination = PathBuf::from(path.trim());
    let mut library = ctx
        .mods_dir
        .write()
        .map_err(|_| AppError::Other("managed library lock was poisoned".into()))?;
    if paths_are_same(&library, &destination) {
        return Ok(library_info(ctx, &library));
    }
    let source = library.clone();
    copy_library_for_move(&source, &destination)?;
    let persisted = connection(ctx).and_then(|conn| {
        if paths_are_same(&destination, &ctx.default_mods_dir) {
            database::delete_setting(&conn, "managed_library_path")
        } else {
            database::set_setting(
                &conn,
                "managed_library_path",
                &destination.display().to_string(),
            )
        }
    });
    if let Err(error) = persisted {
        // The destination was an empty user-selected folder before this
        // operation, and contains only the copy just made.
        let _ = std::fs::remove_dir_all(&destination);
        return Err(error);
    }
    *library = destination.clone();
    if let Err(error) = std::fs::remove_dir_all(&source) {
        log(
            ctx,
            "warn",
            "managed_library_old_copy_kept",
            &format!("path={} error={error}", source.display()),
        );
    }
    log(
        ctx,
        "info",
        "managed_library_moved",
        &format!("path={}", destination.display()),
    );
    Ok(library_info(ctx, &destination))
}

#[tauri::command]
pub async fn move_managed_library(path: String, app: AppHandle) -> Result<ManagedLibraryInfo> {
    // A library may contain gigabytes of payloads. Keep copying and hashing off
    // the webview event loop so the move screen can repaint and stay usable.
    tauri::async_runtime::spawn_blocking(move || {
        let ctx = app.state::<AppContext>();
        move_managed_library_inner(path, &ctx)
    })
    .await
    .map_err(|error| AppError::Other(format!("The library move task failed: {error}")))?
}

fn managed_path_for(kind: &str, ctx: &AppContext) -> Result<PathBuf> {
    let library = mods_dir(ctx)?;
    // The UE4SS log is a file rather than a folder, and it exists only once the
    // runtime has actually loaded, so it returns before the directory is
    // created: creating an empty one would answer the question wrongly.
    if kind == "ue4ss-log" {
        let game = game(ctx)?
            .path
            .map(PathBuf::from)
            .ok_or(AppError::GameNotFound)?;
        return ue4ss::detect(Some(&game), None)
            .log_path
            .map(PathBuf::from)
            .ok_or_else(|| {
                AppError::Other(
                    "UE4SS has not written a log. It writes one the first time it loads, so                      start the game once."
                        .into(),
                )
            });
    }
    let path = if let Some(id) = kind.strip_prefix("mod:") {
        let conn = connection(ctx)?;
        database::mod_record(&conn, id)?;
        let path = library.join(id);
        if !path.is_dir() {
            return Err(AppError::Other(
                "Managed mod source folder was not found.".into(),
            ));
        }
        path
    } else if let Some(id) = kind.strip_prefix("installed:") {
        let conn = connection(ctx)?;
        database::mod_record(&conn, id)?;
        let destination = database::file_records(&conn, id)?
            .into_iter()
            .next()
            .map(|(_, destination, _, _)| PathBuf::from(destination))
            .ok_or_else(|| AppError::Other("This mod has no managed files.".into()))?;
        destination
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| AppError::Other("The installed mod folder was not found.".into()))?
    } else {
        match kind {
            "logs" => ctx.logs_dir.clone(),
            "data" => ctx.data_dir.clone(),
            "library" => library.clone(),
            "mods" => game(ctx)?
                .path
                .map(PathBuf::from)
                .map(|p| p.join("SWZeroCompany/Content/Paks/~mods"))
                .unwrap_or_else(|| library.clone()),
            "game" => game(ctx)?
                .path
                .map(PathBuf::from)
                .ok_or(AppError::GameNotFound)?,
            _ => return Err(AppError::Other("unknown managed path".into())),
        }
    };
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

#[tauri::command]
pub fn open_managed_path(kind: String, app: AppHandle, ctx: State<'_, AppContext>) -> Result<()> {
    let path = managed_path_for(&kind, &ctx)?;
    #[cfg(target_os = "linux")]
    if steam::running_from_appimage() {
        return steam::open_path_from_appimage(&path);
    }
    app.opener()
        .open_path(path.display().to_string(), None::<String>)
        .map_err(|error| AppError::Other(format!("The folder could not be opened: {error}")))?;
    Ok(())
}

fn configured_executable(settings: &AppSettings) -> Result<Option<(PathBuf, PathBuf)>> {
    let Some(path) = settings
        .custom_executable_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
    else {
        return Ok(None);
    };
    let executable = PathBuf::from(path);
    if !executable.is_file() {
        return Err(AppError::Other(
            "The custom game executable no longer exists. Choose it again in Settings or clear it to use Steam."
                .into(),
        ));
    }
    let working_directory = executable
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            AppError::Other("The custom game executable has no containing folder.".into())
        })?
        .to_path_buf();
    Ok(Some((executable, working_directory)))
}

fn validate_launch_file(executable: &Path) -> Result<()> {
    if !executable.is_file() || std::fs::metadata(executable)?.len() == 0 {
        return Err(AppError::Other("The game executable is missing or empty. Choose a valid game installation in Settings before launching.".into()));
    }
    Ok(())
}

fn perform_launch(app: &AppHandle, ctx: &AppContext) -> Result<String> {
    let settings = database::settings(&connection(ctx)?)?;
    if let Some((executable, working_directory)) = configured_executable(&settings)? {
        validate_launch_file(&executable)?;
        crate::launcher::launch(&executable, &working_directory)
            .map_err(|error| AppError::Other(crate::launcher::error_message(&error)))?;
        log(
            ctx,
            "info",
            "game_launch_requested",
            &format!("source=custom_executable path={}", executable.display()),
        );
        return Ok("custom-executable".into());
    }
    let (game_info, game_path) = require_game(ctx)?;
    validate_launch_file(&game_path.join("SWZeroCompany/Binaries/Win64/SWZeroCompany.exe"))?;
    if game_info.source == "ea" {
        let executable = game_path.join("SWZeroCompany/Binaries/Win64/SWZeroCompany.exe");
        let working_directory = executable.parent().unwrap_or(&game_path);
        std::process::Command::new(&executable)
            .current_dir(working_directory)
            .spawn()
            .map_err(|error| {
                AppError::Other(format!(
                    "EA App could not launch the game executable: {error}"
                ))
            })?;
        log(
            ctx,
            "info",
            "game_launch_requested",
            "source=ea_installation",
        );
        return Ok("ea".into());
    }
    #[cfg(target_os = "linux")]
    if steam::running_from_appimage() {
        steam::launch_from_appimage()?;
        log(
            ctx,
            "info",
            "game_launch_requested",
            "source=steam_uri appimage_environment=sanitized",
        );
        return Ok("steam".into());
    }
    app.opener()
        .open_url(steam::launch_url(), None::<String>)
        .map_err(|error| AppError::Other(format!("Steam could not launch the game: {error}")))?;
    log(ctx, "info", "game_launch_requested", "source=steam_uri");
    Ok("steam".into())
}

#[tauri::command]
pub fn launch_game(app: AppHandle, ctx: State<'_, AppContext>) -> Result<LaunchReport> {
    Ok(LaunchReport {
        method: perform_launch(&app, &ctx)?,
        session_id: None,
        mode: "modded".into(),
    })
}

pub fn log(ctx: &AppContext, level: &str, event: &str, detail: &str) {
    use std::io::Write;
    let detail = dirs::home_dir()
        .map(|h| detail.replace(&h.display().to_string(), "~"))
        .unwrap_or_else(|| detail.into());
    let record = serde_json::json!({"timestamp":chrono::Utc::now().to_rfc3339(),"level":level,"event":event,"detail":detail});
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ctx.logs_dir.join("application.jsonl"))
    {
        let _ = writeln!(file, "{record}");
    }
}

/// Records a failure the interface showed the user.
///
/// This covers both a JavaScript crash and an ordinary operation that failed:
/// the message a user sees is otherwise the only account of what went wrong,
/// and a user who cannot reproduce it on demand had nothing to quote. The
/// event name keeps the two apart, because a failed install is not a crash.
#[tauri::command]
pub fn report_interface_error(
    message: String,
    stack: Option<String>,
    context: String,
    ctx: State<'_, AppContext>,
) {
    let truncate = |value: &str, limit: usize| value.chars().take(limit).collect::<String>();
    let detail = format!(
        "message={} stack={} context={}",
        truncate(&message, 4_000),
        truncate(stack.as_deref().unwrap_or(""), 12_000),
        truncate(&context, 2_000)
    );
    let event = if context == "notification" {
        "operation_failed"
    } else {
        "interface_error"
    };
    log(&ctx, "error", event, &detail);
}

/// Records the dimensions from a WebView layout mismatch without treating it
/// as a JavaScript crash. The frontend also repairs the shell immediately.
#[tauri::command]
pub fn report_interface_layout(context: String, ctx: State<'_, AppContext>) {
    let detail = context.chars().take(2_000).collect::<String>();
    log(&ctx, "warn", "interface_layout_repaired", &detail);
}

/// The release smoke test waits for this event. Backend startup alone cannot
/// prove that the production WebView loaded the embedded frontend rather than
/// a dead development URL.
#[tauri::command]
pub fn frontend_ready(ctx: State<'_, AppContext>) {
    log(
        &ctx,
        "info",
        "frontend_ready",
        &format!("storage_mode={}", ctx.storage_mode),
    );
}

// Operational Readiness API -------------------------------------------------

#[tauri::command]
pub fn list_profiles(ctx: State<'_, AppContext>) -> Result<Vec<ProfileSummary>> {
    profiles::list(&connection(&ctx)?)
}

#[tauri::command]
pub fn get_profile(profile_id: String, ctx: State<'_, AppContext>) -> Result<ProfileDetail> {
    profiles::detail(&connection(&ctx)?, &profile_id)
}

#[tauri::command]
pub fn active_profile(ctx: State<'_, AppContext>) -> Result<Option<ProfileDetail>> {
    profiles::active(&connection(&ctx)?)
}

#[tauri::command]
pub fn create_profile(
    name: String,
    notes: String,
    ctx: State<'_, AppContext>,
) -> Result<ProfileDetail> {
    let conn = connection(&ctx)?;
    let profile = profiles::create(&conn, &name, &notes)?;
    let _ = operations::record(
        &conn,
        "profile",
        "completed",
        &format!("Created profile {}", profile.summary.name),
        serde_json::json!({"profileId": &profile.summary.id}),
    );
    Ok(profile)
}

#[tauri::command]
pub fn update_profile(
    profile_id: String,
    name: String,
    notes: String,
    required_runtime: Option<String>,
    ctx: State<'_, AppContext>,
) -> Result<ProfileDetail> {
    profiles::update(
        &connection(&ctx)?,
        &profile_id,
        &name,
        &notes,
        required_runtime.as_deref(),
    )
}

#[tauri::command]
pub fn delete_profile(profile_id: String, ctx: State<'_, AppContext>) -> Result<()> {
    profiles::remove(&connection(&ctx)?, &profile_id)
}

#[tauri::command]
pub fn set_profile_mod_state(
    profile_id: String,
    mod_id: String,
    enabled: bool,
    priority: Option<i64>,
    ctx: State<'_, AppContext>,
) -> Result<ProfileDetail> {
    profiles::set_mod_state(&connection(&ctx)?, &profile_id, &mod_id, enabled, priority)
}

#[tauri::command]
pub fn preview_profile_switch(
    profile_id: String,
    ctx: State<'_, AppContext>,
) -> Result<ProfileSwitchPreview> {
    profiles::preview(&connection(&ctx)?, &profile_id)
}

#[tauri::command]
pub fn activate_profile(profile_id: String, ctx: State<'_, AppContext>) -> Result<ProfileDetail> {
    let (game_info, game_path) = require_game(&ctx)?;
    let mut conn = connection(&ctx)?;
    let library = mods_dir(&ctx)?;
    let result = profiles::activate(
        &mut conn,
        &library,
        &game_path,
        &ctx.data_dir.join("load-order-operation.json"),
        &profile_id,
        game_info.steam_build_id,
    )?;
    let _ = operations::record(
        &conn,
        "profile-switch",
        "completed",
        &format!("Activated profile {}", result.summary.name),
        serde_json::json!({"profileId": &result.summary.id}),
    );
    Ok(result)
}

#[tauri::command]
pub fn export_profile_lock(
    profile_id: String,
    path: String,
    ctx: State<'_, AppContext>,
) -> Result<()> {
    let game_build = game(&ctx)?.steam_build_id;
    let lock = profiles::lockfile(&connection(&ctx)?, &profile_id, game_build)?;
    std::fs::write(path, serde_json::to_vec_pretty(&lock)?)?;
    Ok(())
}

#[tauri::command]
pub fn import_profile_lock(path: String, ctx: State<'_, AppContext>) -> Result<ProfileDetail> {
    let lock: ProfileLock = serde_json::from_slice(&std::fs::read(path)?)?;
    profiles::import_lock(&connection(&ctx)?, &lock)
}

#[tauri::command]
pub fn list_snapshots(ctx: State<'_, AppContext>) -> Result<Vec<SnapshotSummary>> {
    profiles::snapshots(&connection(&ctx)?)
}

#[tauri::command]
pub fn create_snapshot(
    label: String,
    last_known_good: bool,
    ctx: State<'_, AppContext>,
) -> Result<SnapshotSummary> {
    profiles::snapshot(
        &connection(&ctx)?,
        &label,
        "manual",
        last_known_good,
        game(&ctx)?.steam_build_id,
    )
}

#[tauri::command]
pub fn restore_snapshot(snapshot_id: String, ctx: State<'_, AppContext>) -> Result<ProfileDetail> {
    let (game_info, game_path) = require_game(&ctx)?;
    let library = mods_dir(&ctx)?;
    let mut conn = connection(&ctx)?;
    let profile = profiles::restore_snapshot(
        &mut conn,
        &library,
        &game_path,
        &ctx.data_dir.join("load-order-operation.json"),
        &snapshot_id,
        game_info.steam_build_id,
    )?;
    let _ = operations::record(
        &conn,
        "snapshot-restore",
        "completed",
        &format!("Restored checkpoint as {}", profile.summary.name),
        serde_json::json!({"snapshotId": snapshot_id, "profileId": &profile.summary.id}),
    );
    Ok(profile)
}

#[tauri::command]
pub fn compatibility_report(ctx: State<'_, AppContext>) -> Result<CompatibilityReport> {
    compatibility::report(&connection(&ctx)?)
}

#[tauri::command]
pub async fn update_compatibility_catalog(app: AppHandle) -> Result<CompatibilityReport> {
    let url = option_env!("ZERO_MOD_MANAGER_CATALOG_URL")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::Other(
                "No signed compatibility catalog URL is configured for this build.".into(),
            )
        })?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| AppError::Network(error.to_string()))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| AppError::Network(error.to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "catalog request returned HTTP {}",
            response.status()
        )));
    }
    if response
        .content_length()
        .is_some_and(|length| length > 8 * 1024 * 1024)
    {
        return Err(AppError::CatalogUntrusted(
            "catalog response exceeds the 8 MiB safety limit".into(),
        ));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| AppError::Network(error.to_string()))?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(AppError::CatalogUntrusted(
            "catalog response exceeds the 8 MiB safety limit".into(),
        ));
    }
    let ctx = app.state::<AppContext>();
    let conn = connection(&ctx)?;
    let version = compatibility::install(
        &conn,
        &bytes,
        &ctx.data_dir.join("catalog/compatibility.signed.json"),
    )?;
    let _ = operations::record(
        &conn,
        "catalog",
        "completed",
        &format!("Compatibility catalog {version} verified"),
        serde_json::json!({"version": version}),
    );
    compatibility::report(&conn)
}

#[tauri::command]
pub fn activity(ctx: State<'_, AppContext>) -> Result<Vec<OperationRecord>> {
    operations::list(&connection(&ctx)?, 100)
}

#[tauri::command]
pub fn list_config_documents(ctx: State<'_, AppContext>) -> Result<Vec<ConfigDocument>> {
    let (_, game_path) = require_game(&ctx)?;
    config_workbench::list(&game_path)
}

#[tauri::command]
pub fn read_config_document(path: String, ctx: State<'_, AppContext>) -> Result<ConfigDocument> {
    let (_, game_path) = require_game(&ctx)?;
    config_workbench::read(&game_path, Path::new(&path))
}

#[tauri::command]
pub fn preview_config_change(
    path: String,
    content: String,
    ctx: State<'_, AppContext>,
) -> Result<ConfigChangePreview> {
    let (_, game_path) = require_game(&ctx)?;
    config_workbench::preview(&game_path, Path::new(&path), &content)
}

#[tauri::command]
pub fn apply_config_change(
    path: String,
    content: String,
    expected_sha256: String,
    ctx: State<'_, AppContext>,
) -> Result<ConfigPatchRecord> {
    let (_, game_path) = require_game(&ctx)?;
    let conn = connection(&ctx)?;
    profiles::snapshot(
        &conn,
        "Before configuration change",
        "config",
        false,
        game(&ctx)?.steam_build_id,
    )?;
    let patch = config_workbench::apply(
        &conn,
        &game_path,
        &ctx.data_dir,
        Path::new(&path),
        &content,
        &expected_sha256,
    )?;
    let _ = operations::record(
        &conn,
        "config",
        "completed",
        "Configuration change applied",
        serde_json::json!({"path": &patch.path, "patchId": &patch.id}),
    );
    Ok(patch)
}

#[tauri::command]
pub fn config_history(ctx: State<'_, AppContext>) -> Result<Vec<ConfigPatchRecord>> {
    config_workbench::history(&connection(&ctx)?)
}

#[tauri::command]
pub fn rollback_config_change(patch_id: String, ctx: State<'_, AppContext>) -> Result<()> {
    let (_, game_path) = require_game(&ctx)?;
    let conn = connection(&ctx)?;
    config_workbench::rollback(&conn, &game_path, &patch_id)?;
    let _ = operations::record(
        &conn,
        "config",
        "rolled-back",
        "Configuration checkpoint restored",
        serde_json::json!({"patchId": patch_id}),
    );
    Ok(())
}

#[tauri::command]
pub fn launch_preflight(ctx: State<'_, AppContext>) -> Result<LaunchPreflight> {
    let conn = connection(&ctx)?;
    let game = game(&ctx)?;
    let settings = database::settings(&conn)?;
    let runtime = ue4ss::detect(
        game.path.as_deref().map(Path::new),
        game.compat_data_path.as_deref().map(Path::new),
    );
    sessions::preflight(&conn, &game, &runtime, &settings)
}

fn restore_temporary_profile(
    db_path: PathBuf,
    library: PathBuf,
    game_path: PathBuf,
    data_dir: PathBuf,
    profile_id: String,
) {
    let mut seen_running = false;
    for _ in 0..120 {
        if deployment::game_is_running() {
            seen_running = true;
            break;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    if seen_running {
        while deployment::game_is_running() {
            std::thread::sleep(Duration::from_secs(2));
        }
    }
    if let Ok(mut conn) = database::open(&db_path) {
        let result = profiles::activate(
            &mut conn,
            &library,
            &game_path,
            &data_dir.join("load-order-operation.json"),
            &profile_id,
            None,
        );
        let status = if result.is_ok() {
            "completed"
        } else {
            "failed"
        };
        let _ = operations::record(
            &conn,
            "temporary-launch-restore",
            status,
            "Restored the profile after a temporary launch",
            serde_json::json!({"profileId": profile_id, "error": result.err().map(|error| error.to_string())}),
        );
        if status == "completed" {
            let _ = database::delete_setting(&conn, "pending_restore_profile");
        }
    }
}

#[tauri::command]
pub fn launch_game_mode(
    mode: String,
    enabled_mod_ids: Vec<String>,
    app: AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<LaunchReport> {
    let preflight = launch_preflight(ctx.clone())?;
    if preflight.status == "blocked" {
        return Err(AppError::Other(
            "Launch readiness is blocked. Open Health to resolve the listed problems.".into(),
        ));
    }
    let (game_info, game_path) = require_game(&ctx)?;
    let conn = connection(&ctx)?;
    if database::get_setting(&conn, "pending_restore_profile")?.is_some() {
        return Err(AppError::Other("The previous temporary launch is still restoring its profile. Wait a moment and refresh Health before starting another test.".into()));
    }
    let active = profiles::active(&conn)?
        .ok_or_else(|| AppError::Other("No active profile exists.".into()))?;
    let lock = profiles::lockfile(&conn, &active.summary.id, game_info.steam_build_id.clone())?;
    let lock_json = serde_json::to_string(&lock)?;
    let settings = database::settings(&conn)?;
    let launcher = if settings
        .custom_executable_path
        .as_deref()
        .is_some_and(|value| !value.is_empty())
    {
        "custom"
    } else if game_info.source == "ea" {
        "ea"
    } else {
        "steam"
    };
    let executable = settings
        .custom_executable_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| game_path.join("SWZeroCompany/Binaries/Win64/SWZeroCompany.exe"));
    validate_launch_file(&executable)?;
    let executable_sha256 = executable
        .is_file()
        .then(|| deployment::sha256(&executable))
        .transpose()?;
    let session = sessions::begin(
        &conn,
        &mode,
        launcher,
        game_info.steam_build_id.as_deref(),
        executable_sha256.as_deref(),
        active.summary.required_runtime.as_deref(),
        &lock_json,
    )?;

    if mode != "modded" {
        deployment::ensure_game_stopped()?;
        profiles::snapshot(
            &conn,
            &format!("Before {mode} launch"),
            "temporary-launch",
            false,
            game_info.steam_build_id.clone(),
        )?;
        let selected = enabled_mod_ids
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        let library = mods_dir(&ctx)?;
        for installed in database::list_mods(&conn)? {
            let desired = mode == "troubleshoot" && selected.contains(&installed.id);
            if installed.enabled != desired {
                deployment::set_enabled(
                    &conn,
                    &library,
                    &game_path,
                    &installed.id,
                    desired,
                    false,
                )?;
            }
        }
        database::set_setting(&conn, "pending_restore_profile", &active.summary.id)?;
    }

    let method = match perform_launch(&app, &ctx) {
        Ok(method) => method,
        Err(error) => {
            if mode != "modded" {
                let mut rollback = connection(&ctx)?;
                let library = mods_dir(&ctx)?;
                let restored = profiles::activate(
                    &mut rollback,
                    &library,
                    &game_path,
                    &ctx.data_dir.join("load-order-operation.json"),
                    &active.summary.id,
                    game_info.steam_build_id,
                );
                if restored.is_ok() {
                    let _ = database::delete_setting(&rollback, "pending_restore_profile");
                }
            }
            return Err(error);
        }
    };
    if mode != "modded" {
        let db_path = ctx.db_path.clone();
        let library = mods_dir(&ctx)?.clone();
        let data_dir = ctx.data_dir.clone();
        let profile_id = active.summary.id.clone();
        std::thread::spawn(move || {
            restore_temporary_profile(db_path, library, game_path, data_dir, profile_id)
        });
    }
    let _ = operations::record(
        &conn,
        "launch",
        "completed",
        &format!("Requested {mode} launch"),
        serde_json::json!({"sessionId": &session.id, "launcher": launcher}),
    );
    Ok(LaunchReport {
        method,
        session_id: Some(session.id),
        mode,
    })
}

#[tauri::command]
pub fn complete_launch_session(
    session_id: String,
    outcome: String,
    evidence: Option<String>,
    ctx: State<'_, AppContext>,
) -> Result<LaunchSession> {
    sessions::complete(
        &connection(&ctx)?,
        &session_id,
        &outcome,
        evidence.as_deref(),
    )
}

#[tauri::command]
pub fn launch_sessions(ctx: State<'_, AppContext>) -> Result<Vec<LaunchSession>> {
    sessions::list(&connection(&ctx)?)
}

#[tauri::command]
pub fn start_guided_isolation(ctx: State<'_, AppContext>) -> Result<IsolationSession> {
    deployment::ensure_game_stopped()?;
    let game = game(&ctx)?;
    let conn = connection(&ctx)?;
    profiles::snapshot(
        &conn,
        "Before guided isolation",
        "guided-isolation",
        false,
        game.steam_build_id.clone(),
    )?;
    let result = isolation::start(&conn, game.steam_build_id)?;
    let _ = operations::record(
        &conn,
        "isolation",
        "pending",
        "Guided isolation started",
        serde_json::json!({"isolationId": &result.id, "groups": result.candidate_groups.len()}),
    );
    Ok(result)
}

#[tauri::command]
pub fn active_guided_isolation(ctx: State<'_, AppContext>) -> Result<Option<IsolationSession>> {
    isolation::active(&connection(&ctx)?)
}

#[tauri::command]
pub fn advance_guided_isolation(
    isolation_id: String,
    outcome: String,
    ctx: State<'_, AppContext>,
) -> Result<IsolationSession> {
    deployment::ensure_game_stopped()?;
    let conn = connection(&ctx)?;
    if database::get_setting(&conn, "pending_restore_profile")?.is_some() {
        return Err(AppError::Other("The temporary launch is still restoring the original profile. Wait a moment, then record the observation.".into()));
    }
    let result = isolation::advance(&conn, &isolation_id, &outcome)?;
    let _ = operations::record(
        &conn,
        "isolation-observation",
        "completed",
        "Guided isolation observation recorded",
        serde_json::json!({"isolationId": isolation_id, "outcome": outcome, "phase": &result.phase, "remainingGroups": result.candidate_groups.len()}),
    );
    Ok(result)
}

#[tauri::command]
pub fn cancel_guided_isolation(isolation_id: String, ctx: State<'_, AppContext>) -> Result<()> {
    isolation::cancel(&connection(&ctx)?, &isolation_id)
}

#[tauri::command]
pub fn support_bundle_preview(ctx: State<'_, AppContext>) -> Result<SupportBundlePreview> {
    let conn = connection(&ctx)?;
    let game = game(&ctx)?;
    let runtime = ue4ss::detect(
        game.path.as_deref().map(Path::new),
        game.compat_data_path.as_deref().map(Path::new),
    );
    Ok(support::preview(
        profiles::active(&conn)?.is_some(),
        runtime
            .log_path
            .as_deref()
            .is_some_and(|path| Path::new(path).is_file()),
    ))
}

#[tauri::command]
pub fn create_support_bundle(
    path: String,
    ctx: State<'_, AppContext>,
) -> Result<SupportBundleReport> {
    let conn = connection(&ctx)?;
    let game = game(&ctx)?;
    let runtime = ue4ss::detect(
        game.path.as_deref().map(Path::new),
        game.compat_data_path.as_deref().map(Path::new),
    );
    let diagnostics = diagnostics::run(&conn, &game, &runtime, &tool(&ctx)?)?;
    let compatibility = compatibility::report(&conn)?;
    let active = profiles::active(&conn)?;
    let lock = active
        .as_ref()
        .map(|profile| profiles::lockfile(&conn, &profile.summary.id, game.steam_build_id.clone()))
        .transpose()?;
    let activity = operations::list(&conn, 100)?;
    let launch_sessions = sessions::list(&conn)?;
    let application = serde_json::json!({
        "application": "Zero Mod Manager", "version": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS, "architecture": std::env::consts::ARCH,
        "game": game, "runtime": runtime,
    });
    let compatibility_value = serde_json::to_value(compatibility)?;
    let lock_value = lock.map(serde_json::to_value).transpose()?;
    let activity_value = serde_json::to_value(activity)?;
    let sessions_value = serde_json::to_value(launch_sessions)?;
    support::create(
        Path::new(&path),
        support::BundleContent {
            application: &application,
            diagnostics: &diagnostics.text,
            compatibility: &compatibility_value,
            profile_lock: lock_value.as_ref(),
            activity: &activity_value,
            sessions: &sessions_value,
            application_log: &ctx.logs_dir.join("application.jsonl"),
            ue4ss_log: runtime.log_path.as_deref().map(Path::new),
        },
    )
}

#[cfg(test)]
mod update_tests {
    use super::{
        configured_executable, copy_library_for_move, manual_game_or_unavailable, replaced_by,
        version_is_newer,
    };
    use crate::{
        database,
        models::{AppSettings, PayloadFile, StagedMod},
    };
    use std::{fs, path::PathBuf};
    use tempfile::tempdir;

    #[test]
    fn compares_release_versions_numerically() {
        assert!(version_is_newer("v0.2.0", "0.1.4"));
        assert!(version_is_newer("0.1.10", "0.1.9"));
        assert!(!version_is_newer("v0.1.4", "0.1.4"));
        assert!(!version_is_newer("0.1.3", "0.1.4"));
        assert!(version_is_newer("0.7.0", "0.7.0-rc.2"));
        assert!(version_is_newer("0.7.0-rc.10", "0.7.0-rc.2"));
        assert!(!version_is_newer("0.7.0-rc.2", "0.7.0"));
        assert!(!version_is_newer("invalid", "0.7.0"));
    }

    #[test]
    fn a_moved_manual_game_path_is_recoverable_startup_state() {
        let directory = tempdir().unwrap();
        let moved = directory
            .path()
            .join("old-steam-library/Star Wars Zero Company");

        let info = manual_game_or_unavailable(&moved).unwrap();

        assert!(!info.detected);
        assert_eq!(info.path.as_deref(), Some(moved.to_string_lossy().as_ref()));
        assert_eq!(info.source, "manual");
        assert_eq!(info.problem_code.as_deref(), Some("game_path_invalid"));
        assert!(info
            .problem
            .as_deref()
            .is_some_and(|message| message.contains("moved")));
    }

    #[test]
    fn a_library_move_is_verified_before_the_destination_replaces_the_empty_folder() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("old-library");
        let destination = directory.path().join("new-library");
        fs::create_dir_all(source.join("mod-a/payload")).unwrap();
        fs::create_dir(&destination).unwrap();
        fs::write(source.join("mod-a/payload/mod.pak"), b"managed payload").unwrap();

        copy_library_for_move(&source, &destination).unwrap();

        assert_eq!(
            fs::read(destination.join("mod-a/payload/mod.pak")).unwrap(),
            b"managed payload"
        );
        assert!(source.join("mod-a/payload/mod.pak").is_file());
    }

    #[test]
    fn a_library_move_refuses_to_mix_with_an_existing_folder() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("old-library");
        let destination = directory.path().join("not-empty");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("user-file.txt"), b"keep me").unwrap();

        let error = copy_library_for_move(&source, &destination)
            .unwrap_err()
            .to_string();

        assert!(error.contains("not empty"));
        assert!(destination.join("user-file.txt").is_file());
    }

    #[test]
    fn a_library_copy_never_treats_its_source_as_a_completed_move() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("library");
        fs::create_dir(&source).unwrap();

        let error = copy_library_for_move(&source, &source)
            .unwrap_err()
            .to_string();

        assert!(error.contains("already"));
        assert!(source.is_dir());
    }

    #[test]
    fn validates_a_configured_launch_executable_and_uses_its_folder() {
        let directory = tempdir().unwrap();
        let executable = directory.path().join("ZeroCompany.exe");
        fs::write(&executable, b"test executable").unwrap();
        let settings = AppSettings {
            custom_executable_path: Some(executable.display().to_string()),
            ..AppSettings::default()
        };

        let (selected, working_directory) = configured_executable(&settings).unwrap().unwrap();
        assert_eq!(selected, executable);
        assert_eq!(working_directory, directory.path());
    }

    #[test]
    fn missing_or_empty_launch_file_is_rejected_before_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("game.exe");
        assert!(super::validate_launch_file(&exe).is_err());
        std::fs::write(&exe, []).unwrap();
        assert!(super::validate_launch_file(&exe).is_err());
        std::fs::write(&exe, b"test-fixture").unwrap();
        assert!(super::validate_launch_file(&exe).is_ok());
    }

    #[test]
    fn reports_a_missing_custom_launch_executable() {
        let settings = AppSettings {
            custom_executable_path: Some("/missing/ZeroCompany.exe".into()),
            ..AppSettings::default()
        };
        let error = configured_executable(&settings).unwrap_err().to_string();
        assert!(error.contains("no longer exists"));
        assert!(error.contains("clear it to use Steam"));
    }

    #[test]
    fn one_bundle_matches_both_entries_from_an_old_split_install() {
        let directory = tempdir().unwrap();
        let connection = database::open(&directory.path().join("mods.sqlite3")).unwrap();
        connection.execute(
            "INSERT INTO mods(id,name,version,mod_type,deployment_key,source_archive,installed_at,enabled,load_priority) VALUES('old-core','Squad Six - Core','1.0.1','iostore','','core.zip','now',1,1)",
            [],
        ).unwrap();
        connection.execute(
            "INSERT INTO mod_files(mod_id,library_relative,destination,size,sha256) VALUES('old-core','pakchunk99-ZCOMSquadSix_P.pak','/game/~mods/pakchunk99-ZCOMSquadSix_P.pak',1,'hash')",
            [],
        ).unwrap();
        connection.execute(
            "INSERT INTO mods(id,name,version,mod_type,deployment_key,source_archive,installed_at,enabled,load_priority) VALUES('old-runtime','Squad Six - Runtime','1.0.1','ue4ss','ZCOMSquadSix','runtime.zip','now',1,1)",
            [],
        ).unwrap();

        let component = |kind: &str, keys: Vec<String>, files: Vec<PayloadFile>| StagedMod {
            staging_id: kind.into(),
            staging_root: PathBuf::from("/staging"),
            source_archive: "Squad-Six-ZCOM-Manager.zip".into(),
            name: format!("Squad Six - {kind}"),
            version: Some("1.1.1".into()),
            author: None,
            description: None,
            manifest: None,
            mod_type: kind.to_ascii_lowercase(),
            deployment_keys: keys,
            files,
            packages: Vec::new(),
            verification: "passed".into(),
            fomod_source_root: None,
            fomod_answers: None,
        };
        let core = component(
            "IoStore",
            Vec::new(),
            vec![PayloadFile {
                source: PathBuf::from("/staging/Core/pakchunk99-ZCOMSquadSix_P.pak"),
                library_relative: PathBuf::from("pakchunk99-ZCOMSquadSix_P.pak"),
                destination_relative: PathBuf::from("pakchunk99-ZCOMSquadSix_P.pak"),
            }],
        );
        let runtime = component("UE4SS", vec!["ZCOMSquadSix".into()], Vec::new());

        assert_eq!(
            replaced_by(&connection, &core).unwrap().unwrap().mod_id,
            "old-core"
        );
        assert_eq!(
            replaced_by(&connection, &runtime).unwrap().unwrap().mod_id,
            "old-runtime"
        );
    }
}
