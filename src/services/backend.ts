import { invoke } from "@tauri-apps/api/core";
import type { AdoptionGroup, AdoptionReport, AppSettings, BundleInstallItem, BundleInstallReport, CompatibilityReport, ConfigChangePreview, ConfigDocument, ConfigPatchRecord, Dashboard, DiagnosticReport, ExistingModScan, FomodAnswer, FomodReconfiguration, FomodSession, Inspection, IsolationSession, LaunchMode, LaunchPreflight, LaunchReport, LaunchSession, LegacyImportReport, LegacyImportStatus, Links, LoadOrderPreview, LoadOrderState, ManagedLibraryInfo, ModPreview, ModSummary, OperationRecord, ProfileDetail, ProfileSummary, ProfileSwitchPreview, SnapshotSummary, SupportBundlePreview, SupportBundleReport, ToolInfo, Ue4ssInstallReport, UpdateInfo } from "../types";

export const backend = {
  frontendReady: () => invoke<void>("frontend_ready"),
  dashboard: () => invoke<Dashboard>("get_dashboard"),
  mods: () => invoke<ModSummary[]>("list_mods"),
  loadOrder: () => invoke<LoadOrderState>("get_load_order_state"),
  previewLoadOrder: (orderedModIds: string[]) => invoke<LoadOrderPreview>("preview_load_order", { orderedModIds }),
  applyLoadOrder: (orderedModIds: string[]) => invoke<LoadOrderState>("apply_load_order", { orderedModIds }),
  applyUe4ssOrder: (orderedModIds: string[]) => invoke<LoadOrderState>("apply_ue4ss_order", { orderedModIds }),
  inspect: (path: string) => invoke<Inspection>("inspect_mod", { path }),
  fomodAdvance: (sessionId: string, answers: FomodAnswer[]) => invoke<FomodSession>("fomod_advance", { sessionId, answers }),
  fomodInstall: (sessionId: string, answers: FomodAnswer[]) => invoke<ModPreview[]>("fomod_install", { sessionId, answers }),
  fomodCancel: (sessionId: string) => invoke<void>("fomod_cancel", { sessionId }),
  reconfigureFomod: (id: string) => invoke<FomodReconfiguration>("reconfigure_fomod", { id }),
  discoverExistingMods: () => invoke<ExistingModScan>("discover_existing_mods"),
  adoptExistingMods: (scanId: string, groups: AdoptionGroup[]) => invoke<AdoptionReport>("adopt_existing_mods", { scanId, groups }),
  acknowledgeExistingModPrompt: () => invoke<void>("acknowledge_existing_mod_prompt"),
  install: (stagingId: string, name?: string, replace?: string, force = false) => invoke<ModSummary>("install_mod", { stagingId, name: name ?? null, replace: replace ?? null, force }),
  installBundle: (items: BundleInstallItem[], replaceBundleId: string | null = null) => invoke<BundleInstallReport>("install_bundle", { items, replaceBundleId }),
  discardPreviews: (stagingIds: string[]) => invoke<void>("discard_previews", { stagingIds }),
  rename: (id: string, name: string) => invoke<void>("rename_mod", { id, name }),
  setEnabled: (id: string, enabled: boolean, force = false) => invoke<void>("set_mod_enabled", { id, enabled, force }),
  setHidden: (id: string, hidden: boolean) => invoke<void>("set_mod_hidden", { id, hidden }),
  uninstall: (id: string, force = false) => invoke<void>("uninstall_mod", { id, force }),
  uninstallBundle: (id: string) => invoke<void>("uninstall_bundle", { id }),
  verify: (id: string) => invoke<string>("verify_mod", { id }),
  installUe4ss: (path: string) => invoke<Ue4ssInstallReport>("install_ue4ss", { path }),
  links: () => invoke<Links>("get_links"),
  checkForUpdates: () => invoke<UpdateInfo>("check_for_updates"),
  diagnostics: () => invoke<DiagnosticReport>("run_diagnostics"),
  settings: () => invoke<AppSettings>("get_settings"),
  sevenZipStatus: () => invoke<ToolInfo>("seven_zip_status"),
  saveSettings: (settings: AppSettings) => invoke<void>("save_settings", { settings }),
  setGamePath: (path: string) => invoke<GameInfo>("set_game_path", { path }),
  managedLibrary: () => invoke<ManagedLibraryInfo>("get_managed_library"),
  moveManagedLibrary: (path: string) => invoke<ManagedLibraryInfo>("move_managed_library", { path }),
  copyDiagnostics: () => invoke<string>("diagnostic_report"),
  openManagedPath: (kind: "game" | "mods" | "logs" | "data" | "library" | "ue4ss-log" | `mod:${string}` | `installed:${string}`) => invoke<void>("open_managed_path", { kind }),
  launchGame: () => invoke<LaunchReport>("launch_game"),
  profiles: () => invoke<ProfileSummary[]>("list_profiles"),
  profile: (profileId: string) => invoke<ProfileDetail>("get_profile", { profileId }),
  activeProfile: () => invoke<ProfileDetail | null>("active_profile"),
  createProfile: (name: string, notes: string) => invoke<ProfileDetail>("create_profile", { name, notes }),
  updateProfile: (profileId: string, name: string, notes: string, requiredRuntime: string | null) => invoke<ProfileDetail>("update_profile", { profileId, name, notes, requiredRuntime }),
  deleteProfile: (profileId: string) => invoke<void>("delete_profile", { profileId }),
  setProfileModState: (profileId: string, modId: string, enabled: boolean, priority: number | null) => invoke<ProfileDetail>("set_profile_mod_state", { profileId, modId, enabled, priority }),
  previewProfileSwitch: (profileId: string) => invoke<ProfileSwitchPreview>("preview_profile_switch", { profileId }),
  activateProfile: (profileId: string) => invoke<ProfileDetail>("activate_profile", { profileId }),
  exportProfileLock: (profileId: string, path: string) => invoke<void>("export_profile_lock", { profileId, path }),
  importProfileLock: (path: string) => invoke<ProfileDetail>("import_profile_lock", { path }),
  snapshots: () => invoke<SnapshotSummary[]>("list_snapshots"),
  createSnapshot: (label: string, lastKnownGood: boolean) => invoke<SnapshotSummary>("create_snapshot", { label, lastKnownGood }),
  restoreSnapshot: (snapshotId: string) => invoke<ProfileDetail>("restore_snapshot", { snapshotId }),
  compatibility: () => invoke<CompatibilityReport>("compatibility_report"),
  updateCompatibilityCatalog: () => invoke<CompatibilityReport>("update_compatibility_catalog"),
  activity: () => invoke<OperationRecord[]>("activity"),
  configDocuments: () => invoke<ConfigDocument[]>("list_config_documents"),
  readConfigDocument: (path: string) => invoke<ConfigDocument>("read_config_document", { path }),
  previewConfigChange: (path: string, content: string) => invoke<ConfigChangePreview>("preview_config_change", { path, content }),
  applyConfigChange: (path: string, content: string, expectedSha256: string) => invoke<ConfigPatchRecord>("apply_config_change", { path, content, expectedSha256 }),
  configHistory: () => invoke<ConfigPatchRecord[]>("config_history"),
  rollbackConfigChange: (patchId: string) => invoke<void>("rollback_config_change", { patchId }),
  launchPreflight: () => invoke<LaunchPreflight>("launch_preflight"),
  launchGameMode: (mode: LaunchMode, enabledModIds: string[] = []) => invoke<LaunchReport>("launch_game_mode", { mode, enabledModIds }),
  completeLaunchSession: (sessionId: string, outcome: LaunchSession["outcome"], evidence: string | null = null) => invoke<LaunchSession>("complete_launch_session", { sessionId, outcome, evidence }),
  launchSessions: () => invoke<LaunchSession[]>("launch_sessions"),
  startGuidedIsolation: () => invoke<IsolationSession>("start_guided_isolation"),
  activeGuidedIsolation: () => invoke<IsolationSession | null>("active_guided_isolation"),
  advanceGuidedIsolation: (isolationId: string, outcome: NonNullable<LaunchSession["outcome"]>) => invoke<IsolationSession>("advance_guided_isolation", { isolationId, outcome }),
  cancelGuidedIsolation: (isolationId: string) => invoke<void>("cancel_guided_isolation", { isolationId }),
  supportBundlePreview: () => invoke<SupportBundlePreview>("support_bundle_preview"),
  createSupportBundle: (path: string) => invoke<SupportBundleReport>("create_support_bundle", { path }),
  reportInterfaceError: (message: string, stack: string | null, context: string) => invoke<void>("report_interface_error", { message, stack, context }),
  reportInterfaceLayout: (context: string) => invoke<void>("report_interface_layout", { context }),
  legacyImportStatus: () => invoke<LegacyImportStatus>("legacy_import_status"),
  importLegacyData: () => invoke<LegacyImportReport>("import_legacy_data")
};

interface GameInfo {
  detected: boolean;
  path: string | null;
  steamBuildId: string | null;
  installState: string | null;
  engine: string;
  compatDataPath: string | null;
  source: "automatic" | "manual" | "ea" | "none";
}

/**
 * Whether a failure is the guard on a managed file that changed on disk.
 *
 * A mod that writes its own settings or data files into its deployed folder
 * trips this every time, and without recognising it the interface could only
 * report the refusal, leaving the mod impossible to update or remove. The
 * message prefix is the contract; `AppError::ChecksumMismatch` owns the text.
 */
export function isChangedFileError(error: unknown): boolean {
  return friendlyError(error).startsWith("A managed file changed outside Zero Mod Manager:");
}

export function friendlyError(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "The operation could not be completed.";
}
