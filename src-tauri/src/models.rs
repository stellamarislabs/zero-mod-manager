use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub detected: bool,
    pub path: Option<String>,
    pub steam_build_id: Option<String>,
    pub install_state: Option<String>,
    pub engine: String,
    pub compat_data_path: Option<String>,
    pub source: String,
    /// A recoverable discovery problem. This is data rather than a command
    /// failure so a stale saved path cannot block the whole interface.
    pub problem_code: Option<String>,
    pub problem: Option<String>,
}

impl Default for GameInfo {
    fn default() -> Self {
        Self {
            detected: false,
            path: None,
            steam_build_id: None,
            install_state: None,
            engine: "Unreal Engine 5 (minor version unverified)".into(),
            compat_data_path: None,
            source: "none".into(),
            problem_code: None,
            problem: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Ue4ssInfo {
    pub installed: bool,
    /// Whether every file the runtime needs is in place. This is a statement
    /// about the layout on disk and nothing more: it does not mean the runtime
    /// has ever loaded.
    pub healthy: bool,
    /// UE4SS mod folders present in the runtime, script and DLL mods alike.
    pub mod_count: usize,
    /// A log file exists at a known path. Its age, content and current-session
    /// provenance have not been verified.
    pub log_found: bool,
    /// Where that log is, so the interface can offer to open it.
    pub log_path: Option<String>,
    /// Proxy DLLs sitting next to the game executable other than the
    /// `dwmapi.dll` UE4SS ships. These may belong to unrelated tools.
    pub extra_loaders: Vec<String>,
    /// Whether the Visual C++ runtime UE4SS links against is present. `None`
    /// off Windows, where the question does not apply.
    pub vc_runtime: Option<bool>,
    pub proton_override: Option<bool>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Ue4ssInstallReport {
    /// Number of runtime files written into `Binaries/Win64`.
    pub installed: usize,
    /// Existing user-owned files that were kept instead of being overwritten.
    pub preserved: Vec<String>,
    /// Whether the Steam launch-option reminder applies to this platform.
    pub proton_hint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub game: GameInfo,
    pub installed_mods: usize,
    pub enabled_mods: usize,
    pub conflict_count: usize,
    pub ue4ss: Ue4ssInfo,
    pub previous_build_id: Option<String>,
    pub data_directory: String,
    pub storage_mode: String,
    /// Whether the one-time existing-mod discovery has not yet been shown.
    pub existing_mod_scan_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModFile {
    pub name: String,
    pub destination: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSummary {
    #[serde(default)]
    pub container_verification: Option<String>,
    pub id: String,
    /// Components installed from one archive in a single atomic operation
    /// share this stable local identity.
    pub bundle_id: Option<String>,
    #[serde(default)]
    pub bundle_name: Option<String>,
    pub name: String,
    pub version: Option<String>,
    pub mod_type: String,
    pub enabled: bool,
    pub installed_at: String,
    pub installed_build: Option<String>,
    pub package_count: usize,
    pub conflict_count: usize,
    pub potential_conflict_count: usize,
    pub load_priority: Option<i64>,
    /// The Nexus mod this was installed from, when it is known. Only a mod with
    /// one can be checked for updates.
    pub nexus_mod_id: Option<u64>,
    /// That mod's page, so the interface can offer to open it without knowing
    /// how a Nexus address is put together.
    pub nexus_url: Option<String>,
    /// Taken out of update checking by the user. Neither checked nor offered to
    /// the identification lookup again.
    pub nexus_ignored: bool,
    /// Kept out of the library list. Still installed, still deployed, and still
    /// ordered — only hidden from view.
    pub hidden: bool,
    /// Installed through a retained FOMOD recipe, so its guided installer can
    /// be opened again without the original download.
    pub fomod: bool,
    pub files: Vec<ModFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewConflict {
    pub mod_id: String,
    pub name: String,
    pub package_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadOrderEntry {
    pub id: String,
    pub name: String,
    pub mod_type: String,
    /// For a UE4SS mod, which of the runtime's start passes it belongs to:
    /// `native` for a DLL mod, `script` for a Lua mod, `mixed` when a mod
    /// ships both. `None` for packaged mods.
    pub runtime_kind: Option<String>,
    pub enabled: bool,
    pub priority: Option<i64>,
    pub supported: bool,
    pub support_reason: Option<String>,
    pub applied: bool,
    pub active_conflict_count: usize,
    pub potential_conflict_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConflictGroup {
    pub id: String,
    pub member_ids: Vec<String>,
    pub package_count: usize,
    pub active: bool,
    pub potential: bool,
    pub winner_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadOrderState {
    pub entries: Vec<LoadOrderEntry>,
    /// UE4SS mods in the order the runtime starts them, first to last. They are
    /// ordered by `mods.txt` rather than by deployed file name, so they are a
    /// separate list rather than another row in `entries`.
    pub ue4ss_entries: Vec<LoadOrderEntry>,
    pub active_conflicts: Vec<ConflictGroup>,
    pub potential_conflicts: Vec<ConflictGroup>,
    pub unapplied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoadOrderMove {
    pub mod_id: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WinnerChange {
    pub conflict_id: String,
    pub from_mod_id: Option<String>,
    pub to_mod_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadOrderPreview {
    pub ordered_mod_ids: Vec<String>,
    pub moves: Vec<LoadOrderMove>,
    pub active_conflicts: Vec<ConflictGroup>,
    pub potential_conflicts: Vec<ConflictGroup>,
    pub winner_changes: Vec<WinnerChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub game: Option<ManifestGame>,
    #[serde(rename = "type", default)]
    pub mod_types: Vec<String>,
    #[serde(default)]
    pub nexus: Option<ManifestNexus>,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub launchers: Vec<String>,
    #[serde(default)]
    pub runtime: Option<ManifestRuntime>,
    #[serde(default)]
    pub dependencies: Vec<ManifestRelation>,
    #[serde(default)]
    pub incompatibilities: Vec<ManifestRelation>,
    #[serde(default)]
    pub load_after: Vec<ManifestRelation>,
    #[serde(default)]
    pub config_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestNexus {
    pub mod_id: Option<u64>,
    pub file_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestRuntime {
    pub name: String,
    pub version: Option<String>,
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestRelation {
    pub id: String,
    pub version: Option<String>,
    pub reason: Option<String>,
    pub evidence_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestGame {
    pub app_id: u32,
    #[serde(default)]
    pub tested_builds: Vec<String>,
    pub strict: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PayloadFile {
    pub source: PathBuf,
    pub library_relative: PathBuf,
    pub destination_relative: PathBuf,
}

#[derive(Debug, Clone)]
pub struct StagedMod {
    pub staging_id: String,
    pub staging_root: PathBuf,
    pub source_archive: String,
    pub name: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    /// The author-supplied v1/v2 contract retained as compatibility provenance.
    pub manifest: Option<ModManifest>,
    pub mod_type: String,
    /// UE4SS mod folder names this payload owns. An archive regularly ships
    /// several, and every one needs its own line in `mods.txt`.
    pub deployment_keys: Vec<String>,
    pub files: Vec<PayloadFile>,
    pub packages: Vec<String>,
    pub verification: String,
    /// The complete extracted FOMOD package. Unlike `staging_root`, this holds
    /// every option, not only the payload selected by the current answers.
    pub fomod_source_root: Option<PathBuf>,
    /// The complete answer recipe, serialized for storage with the installed
    /// mod. `None` distinguishes an ordinary archive from a zero-question
    /// FOMOD, whose valid recipe is an empty JSON array.
    pub fomod_answers: Option<String>,
}

/// An installed mod that a candidate would take the place of, so the interface
/// can offer an upgrade instead of a deployment conflict.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReplacedMod {
    pub mod_id: String,
    pub name: String,
    pub version: Option<String>,
    /// Why this candidate lands on the same files.
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModPreview {
    pub staging_id: String,
    /// The archive or folder this candidate was read from, so the interface can
    /// hand a runtime package back to the UE4SS installer.
    pub source_path: String,
    pub name: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub mod_type: String,
    pub files: Vec<String>,
    pub warnings: Vec<String>,
    pub supplementary_files: Vec<String>,
    pub valid: bool,
    pub verification: String,
    pub verification_details: Option<String>,
    pub package_count: usize,
    pub package_names: Vec<String>,
    pub compatibility: String,
    pub compatibility_message: String,
    pub tested_builds: Vec<String>,
    pub conflicts: Vec<PreviewConflict>,
    /// The installed mod this one would replace, when there is one.
    pub replaces: Option<ReplacedMod>,
    pub recommended_priority: Option<i64>,
    pub load_order_supported: bool,
    pub load_order_support_reason: Option<String>,
    /// A containing folder that represents one selectable packaged option in
    /// an archive with several sibling variants/components.
    pub option_label: Option<String>,
}

/// What reading a download produced.
///
/// Most archives describe their contents directly and come back as previews. An
/// archive carrying a scripted installer instead comes back as the first
/// question that installer asks, and produces previews only once it is answered.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub previews: Vec<ModPreview>,
    pub installer: Option<crate::fomod::Session>,
    pub package: PackageAssessment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageAssessment {
    /// modBundle, runtime, externalTool, externalInstaller, or unknown.
    pub role: String,
    pub title: String,
    pub reason: String,
    /// Native or script payloads are names only. Nothing here is executed.
    pub native_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleInstallItem {
    #[serde(default)]
    pub allow_unverified: bool,
    pub staging_id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleInstallReport {
    pub bundle_id: String,
    pub components: Vec<ModSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExistingModCandidate {
    #[serde(default)]
    pub container_verification: Option<String>,
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub mod_type: String,
    /// Display-only paths relative to one of the controlled mod roots.
    pub files: Vec<String>,
    pub enabled: bool,
    pub package_count: usize,
    pub warnings: Vec<String>,
    pub adoptable: bool,
    pub blocked_reason: Option<String>,
    pub selected_by_default: bool,
    pub likely_runtime_component: bool,
    pub inferred_priority: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExistingModScan {
    pub scan_id: String,
    pub candidates: Vec<ExistingModCandidate>,
    pub unsupported: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionGroup {
    #[serde(default)]
    pub allow_unverified: bool,
    pub candidate_ids: Vec<String>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionOutcome {
    pub candidate_ids: Vec<String>,
    pub name: String,
    pub mod_summary: Option<ModSummary>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionReport {
    pub outcomes: Vec<AdoptionOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticItem {
    pub label: String,
    pub status: String,
    pub value: String,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub overall: String,
    pub items: Vec<DiagnosticItem>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub game_path: Option<String>,
    pub custom_executable_path: Option<String>,
    /// A 7-Zip executable the user pointed at, for the machines where 7-Zip is
    /// installed somewhere the automatic search does not reach.
    pub seven_zip_path: Option<String>,
    pub log_level: String,
    pub advanced_package_names: bool,
    pub reduced_motion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedLibraryInfo {
    /// The directory that currently holds managed payloads and backups.
    pub path: String,
    /// The per-user, non-roaming location used for a new installation.
    pub default_path: String,
    pub is_default: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            game_path: None,
            custom_executable_path: None,
            seven_zip_path: None,
            log_level: "normal".into(),
            advanced_package_names: false,
            reduced_motion: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchReport {
    pub method: String,
    pub session_id: Option<String>,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileModState {
    pub mod_id: String,
    pub name: String,
    pub mod_type: String,
    pub enabled: bool,
    pub load_priority: Option<i64>,
    pub fomod_answers: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub notes: String,
    pub required_runtime: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub active: bool,
    pub enabled_mods: usize,
    pub total_mods: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDetail {
    #[serde(flatten)]
    pub summary: ProfileSummary,
    pub mods: Vec<ProfileModState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileChange {
    pub mod_id: String,
    pub name: String,
    pub from_enabled: bool,
    pub to_enabled: bool,
    pub from_priority: Option<i64>,
    pub to_priority: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSwitchPreview {
    pub profile_id: String,
    pub profile_name: String,
    pub changes: Vec<ProfileChange>,
    pub blocked: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileLock {
    pub schema_version: u32,
    pub profile_name: String,
    pub notes: String,
    pub required_runtime: Option<String>,
    pub exported_at: String,
    pub game_build: Option<String>,
    pub mods: Vec<ProfileLockMod>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileLockMod {
    pub id: String,
    /// Local grouping identity for components installed by one atomic bundle
    /// operation. Optional so older Profile Lock v1 files stay valid.
    #[serde(default)]
    pub bundle_id: Option<String>,
    pub name: String,
    pub version: Option<String>,
    pub mod_type: String,
    pub enabled: bool,
    pub load_priority: Option<i64>,
    pub nexus_mod_id: Option<u64>,
    pub nexus_file_id: Option<u64>,
    pub file_hashes: Vec<String>,
    pub fomod_answers: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSummary {
    pub id: String,
    pub profile_id: Option<String>,
    pub label: String,
    pub kind: String,
    pub created_at: String,
    pub last_known_good: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OperationRecord {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub summary: String,
    pub detail: serde_json::Value,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessIssue {
    pub id: String,
    pub status: String,
    pub title: String,
    pub detail: String,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchPreflight {
    pub status: String,
    pub profile_id: Option<String>,
    pub profile_name: Option<String>,
    pub launcher: String,
    pub game_build: Option<String>,
    pub runtime_state: String,
    pub enabled_mods: usize,
    pub issues: Vec<ReadinessIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchSession {
    pub id: String,
    pub mode: String,
    pub profile_id: Option<String>,
    pub launcher: String,
    pub game_build: Option<String>,
    pub executable_sha256: Option<String>,
    pub runtime_version: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub outcome: Option<String>,
    pub log_evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IsolationObservation {
    pub phase: String,
    pub enabled_mod_ids: Vec<String>,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IsolationSession {
    pub id: String,
    pub profile_id: String,
    pub phase: String,
    pub status: String,
    pub candidate_groups: Vec<Vec<String>>,
    pub current_mod_ids: Vec<String>,
    pub observations: Vec<IsolationObservation>,
    pub suspected_mod_ids: Vec<String>,
    pub instruction: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityIssue {
    pub id: String,
    pub status: String,
    pub rule_type: String,
    pub title: String,
    pub detail: String,
    pub source: String,
    pub evidence_url: Option<String>,
    pub member_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityReport {
    pub status: String,
    pub generated_at: String,
    pub catalog_state: String,
    pub issues: Vec<CompatibilityIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDocument {
    pub path: String,
    pub format: String,
    pub content: String,
    pub sha256: String,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChangePreview {
    pub path: String,
    pub format: String,
    pub before_sha256: String,
    pub after_sha256: String,
    pub diff: Vec<String>,
    pub valid: bool,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SupportBundlePreview {
    pub sections: Vec<String>,
    pub redactions: Vec<String>,
    pub estimated_files: usize,
    pub includes_save_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SupportBundleReport {
    pub path: String,
    pub files: usize,
    pub redactions_applied: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPatchRecord {
    pub id: String,
    pub profile_id: String,
    pub path: String,
    pub format: String,
    pub backup_path: Option<String>,
    pub created_at: String,
}
