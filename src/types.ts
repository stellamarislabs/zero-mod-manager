export type Health = "good" | "warning" | "error" | "unknown";

export interface GameInfo {
  detected: boolean;
  path: string | null;
  steamBuildId: string | null;
  installState: string | null;
  engine: string;
  compatDataPath: string | null;
  source: "automatic" | "manual" | "ea" | "none";
  /** A recoverable discovery problem, such as a saved path moved elsewhere. */
  problemCode?: "game_path_invalid" | string | null;
  problem?: string | null;
}

export interface Ue4ssInfo {
  installed: boolean;
  /** Every file the runtime needs is present. Not a claim that it ever loaded. */
  healthy: boolean;
  modCount: number;
  /** UE4SS wrote its log, which is the only proof it actually loaded. */
  logFound: boolean;
  logPath: string | null;
  /** Proxy DLLs beside the game executable other than UE4SS's own dwmapi.dll. */
  extraLoaders: string[];
  /** Whether the Visual C++ runtime is present; null off Windows. */
  vcRuntime: boolean | null;
  protonOverride: boolean | null;
  message: string | null;
}

export interface Ue4ssInstallReport {
  installed: number;
  preserved: string[];
  protonHint: boolean;
}

export interface Links {
  ue4ssDownload: string;
  nexusGame: string;
  /** The manager's own Nexus Mods page, where a release lands as well. */
  nexusManager: string;
  project: string;
}

export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  releaseUrl: string;
  updateAvailable: boolean;
}

export interface LegacyImportStatus {
  available: boolean;
  dataDirectory: string | null;
  libraryDirectory: string | null;
  modCount: number;
  fileCount: number;
  canImport: boolean;
  reason: string | null;
}

export interface LegacyImportReport {
  importedMods: number;
  copiedFiles: number;
  copiedBytes: number;
  nexusKeyImported: boolean;
  backupPath: string;
}

export interface Dashboard {
  game: GameInfo;
  installedMods: number;
  enabledMods: number;
  conflictCount: number;
  ue4ss: Ue4ssInfo;
  previousBuildId: string | null;
  dataDirectory: string;
  storageMode: "platform" | "portable";
  retoc: ToolInfo;
  existingModScanPending: boolean;
}

export interface ToolInfo {
  found: boolean;
  path: string | null;
  version: string | null;
}

/** What a payload is and where it is deployed. */
export type ModType = "iostore" | "pak" | "ue4ss" | "gamedir" | "plugin" | "config";
/** A preview may also describe the UE4SS runtime, which is not a mod. */
export type PreviewType = ModType | "ue4ss-runtime";

export interface ModSummary {
  containerVerification?: string | null;
  id: string;
  /** Components installed from one archive in a single atomic operation share this id. */
  bundleId: string | null;
  name: string;
  version: string | null;
  modType: ModType;
  enabled: boolean;
  installedAt: string;
  installedBuild: string | null;
  packageCount: number;
  conflictCount: number;
  potentialConflictCount: number;
  loadPriority: number | null;
  /** The Nexus mod this came from, when known. Only these are update-checked. */
  nexusModId: number | null;
  /** That mod's page on Nexus, when it is linked to one. */
  nexusUrl: string | null;
  /** Taken out of update checking by the user, and never looked up again. */
  nexusIgnored: boolean;
  /** Kept out of the library list. Still installed, deployed, and ordered. */
  hidden: boolean;
  /** Its retained FOMOD installer can be opened again. */
  fomod: boolean;
  files: ModFile[];
}

export interface BundleInstallItem {
  allowUnverified?: boolean;
  stagingId: string;
  name: string | null;
}

export interface BundleInstallReport {
  bundleId: string;
  components: ModSummary[];
}

/** An installed mod a candidate would take the place of. */
export interface ReplacedMod {
  modId: string;
  name: string;
  version: string | null;
  reason: string;
}

export interface PreviewConflict {
  modId: string;
  name: string;
  packageCount: number;
}

export interface LoadOrderEntry {
  id: string;
  name: string;
  modType: "iostore" | "pak" | "ue4ss";
  /** Which UE4SS start pass a mod belongs to; null for packaged mods. */
  runtimeKind: "native" | "script" | "mixed" | null;
  enabled: boolean;
  priority: number | null;
  supported: boolean;
  supportReason: string | null;
  applied: boolean;
  activeConflictCount: number;
  potentialConflictCount: number;
}

export interface ConflictGroup {
  id: string;
  memberIds: string[];
  packageCount: number;
  active: boolean;
  potential: boolean;
  winnerId: string | null;
}

export interface LoadOrderState {
  entries: LoadOrderEntry[];
  /** UE4SS mods in the order the runtime starts them, first to last. */
  ue4ssEntries: LoadOrderEntry[];
  activeConflicts: ConflictGroup[];
  potentialConflicts: ConflictGroup[];
  unapplied: boolean;
}

export interface LoadOrderMove {
  modId: string;
  from: string;
  to: string;
}

export interface WinnerChange {
  conflictId: string;
  fromModId: string | null;
  toModId: string | null;
}

export interface LoadOrderPreview {
  orderedModIds: string[];
  moves: LoadOrderMove[];
  activeConflicts: ConflictGroup[];
  potentialConflicts: ConflictGroup[];
  winnerChanges: WinnerChange[];
}

export interface ModFile {
  name: string;
  destination: string;
  size: number;
  sha256: string;
}

export interface ModPreview {
  stagingId: string;
  /** The archive or folder this candidate was read from. */
  sourcePath: string;
  name: string;
  version: string | null;
  author: string | null;
  description: string | null;
  modType: PreviewType;
  files: string[];
  warnings: string[];
  valid: boolean;
  verification: "passed" | "failed" | "unavailable" | "not-required";
  verificationDetails: string | null;
  packageCount: number;
  packageNames: string[];
  compatibility: Health;
  compatibilityMessage: string;
  testedBuilds: string[];
  conflicts: PreviewConflict[];
  replaces: ReplacedMod | null;
  recommendedPriority: number | null;
  loadOrderSupported: boolean;
  loadOrderSupportReason: string | null;
  /** Folder label for one selectable option in a multi-option archive. */
  optionLabel: string | null;
}

/** One option a scripted installer offers inside a group. */
export interface FomodPlugin {
  id: string;
  name: string;
  description: string | null;
  /** A data: URL, since the sandbox is not reachable from the interface. */
  image: string | null;
  kind: "Required" | "Recommended" | "Optional" | "CouldBeUsable" | "NotUsable";
  /** Whether the author's own answer selects this option. */
  selected: boolean;
}

export type FomodGroupKind = "SelectExactlyOne" | "SelectAtMostOne" | "SelectAtLeastOne" | "SelectAny" | "SelectAll";

export interface FomodGroup {
  name: string;
  kind: FomodGroupKind;
  plugins: FomodPlugin[];
}

export interface FomodStep {
  index: number;
  name: string;
  groups: FomodGroup[];
}

/** One answered step, as it is handed back to the backend. */
export interface FomodAnswer {
  step: number;
  plugins: string[];
}

export interface FomodSession {
  sessionId: string;
  moduleName: string;
  moduleImage: string | null;
  author: string | null;
  version: string | null;
  description: string | null;
  /** The question awaiting an answer, or null once there are none left. */
  step: FomodStep | null;
  position: number;
  /** The most this installer can still ask; it falls as steps are skipped. */
  total: number;
  complete: boolean;
  warnings: string[];
}

/**
 * What reading a download produced: either the mods it contains, or the first
 * question its scripted installer asks.
 */
export interface Inspection {
  previews: ModPreview[];
  installer: FomodSession | null;
  package: PackageAssessment;
}

export interface PackageAssessment {
  role: "modBundle" | "runtime" | "externalTool" | "externalInstaller" | "unknown";
  title: string;
  reason: string;
  nativeFiles: string[];
}

/** A retained FOMOD reopened with the recipe used for its last installation. */
export interface FomodReconfiguration {
  previews: ModPreview[];
  installer: FomodSession | null;
  answers: FomodAnswer[];
}

export interface ExistingModCandidate {
  containerVerification?: string | null;
  id: string;
  name: string;
  version: string | null;
  modType: ModType;
  files: string[];
  enabled: boolean;
  packageCount: number;
  warnings: string[];
  adoptable: boolean;
  blockedReason: string | null;
  selectedByDefault: boolean;
  likelyRuntimeComponent: boolean;
  inferredPriority: number | null;
}

export interface ExistingModScan {
  scanId: string;
  candidates: ExistingModCandidate[];
  unsupported: string[];
  warnings: string[];
}

export interface AdoptionGroup {
  allowUnverified?: boolean;
  candidateIds: string[];
  name: string;
}

export interface AdoptionOutcome {
  candidateIds: string[];
  name: string;
  modSummary: ModSummary | null;
  error: string | null;
}

export interface AdoptionReport {
  outcomes: AdoptionOutcome[];
}

export interface DiagnosticItem {
  label: string;
  status: Health;
  value: string;
  action: string | null;
}

export interface DiagnosticReport {
  overall: "GOOD" | "NEEDS ATTENTION" | "BLOCKED";
  items: DiagnosticItem[];
  text: string;
}

export interface AppSettings {
  gamePath: string | null;
  customExecutablePath: string | null;
  retocPath: string | null;
  /** A 7-Zip executable the user pointed at, when the automatic search misses it. */
  sevenZipPath: string | null;
  logLevel: "normal" | "verbose" | "developer";
  advancedPackageNames: boolean;
  reducedMotion: boolean;
}

export interface ManagedLibraryInfo {
  path: string;
  defaultPath: string;
  isDefault: boolean;
}

export interface LaunchReport {
  method: "steam" | "ea" | "custom-executable";
  sessionId: string | null;
  mode: LaunchMode;
}

export type OperationalStatus = "ready" | "warning" | "blocked" | "unverified";
export type LaunchMode = "modded" | "vanilla" | "troubleshoot";

export interface ProfileSummary {
  id: string;
  name: string;
  notes: string;
  requiredRuntime: string | null;
  createdAt: string;
  updatedAt: string;
  active: boolean;
  enabledMods: number;
  totalMods: number;
}

export interface ProfileModState {
  modId: string;
  name: string;
  modType: ModType;
  enabled: boolean;
  loadPriority: number | null;
  fomodAnswers: string | null;
}

export interface ProfileDetail extends ProfileSummary {
  mods: ProfileModState[];
}

export interface ProfileChange {
  modId: string;
  name: string;
  fromEnabled: boolean;
  toEnabled: boolean;
  fromPriority: number | null;
  toPriority: number | null;
}

export interface ProfileSwitchPreview {
  profileId: string;
  profileName: string;
  changes: ProfileChange[];
  blocked: boolean;
  reasons: string[];
}

export interface SnapshotSummary {
  id: string;
  profileId: string | null;
  label: string;
  kind: string;
  createdAt: string;
  lastKnownGood: boolean;
}

export interface OperationRecord {
  id: string;
  kind: string;
  status: "pending" | "completed" | "failed" | "rolled-back";
  summary: string;
  detail: unknown;
  startedAt: string;
  finishedAt: string | null;
}

export interface CompatibilityIssue {
  id: string;
  status: OperationalStatus;
  ruleType: string;
  title: string;
  detail: string;
  source: "local-analysis" | "community-catalog" | "author-manifest" | string;
  evidenceUrl: string | null;
  memberIds: string[];
}

export interface CompatibilityReport {
  status: OperationalStatus;
  generatedAt: string;
  catalogState: string;
  issues: CompatibilityIssue[];
}

export interface ReadinessIssue {
  id: string;
  status: OperationalStatus;
  title: string;
  detail: string;
  action: string | null;
}

export interface LaunchPreflight {
  status: OperationalStatus;
  profileId: string | null;
  profileName: string | null;
  launcher: string;
  gameBuild: string | null;
  executableSha256: string | null;
  runtimeVersion: string | null;
  runtimeState: string;
  enabledMods: number;
  issues: ReadinessIssue[];
}

export interface LaunchSession {
  id: string;
  mode: LaunchMode;
  profileId: string | null;
  launcher: string;
  gameBuild: string | null;
  startedAt: string;
  endedAt: string | null;
  outcome: "worked" | "not-loaded" | "crashed" | "performance-issue" | "unknown" | null;
  logEvidence: string | null;
}

export interface IsolationObservation {
  phase: string;
  enabledModIds: string[];
  outcome: NonNullable<LaunchSession["outcome"]>;
}

export interface IsolationSession {
  id: string;
  profileId: string;
  phase: string;
  status: "active" | "completed" | "cancelled";
  candidateGroups: string[][];
  currentModIds: string[];
  observations: IsolationObservation[];
  suspectedModIds: string[];
  instruction: string;
  createdAt: string;
}

export interface ConfigDocument {
  path: string;
  format: "ini" | "json" | "toml" | "lua";
  content: string;
  sha256: string;
  writable: boolean;
}

export interface ConfigChangePreview {
  path: string;
  format: string;
  beforeSha256: string;
  afterSha256: string;
  diff: string[];
  valid: boolean;
  problem: string | null;
}

export interface ConfigPatchRecord {
  id: string;
  profileId: string;
  path: string;
  format: string;
  backupPath: string | null;
  createdAt: string;
}

export interface SupportBundlePreview {
  sections: string[];
  redactions: string[];
  estimatedFiles: number;
  includesSaveData: boolean;
}

export interface SupportBundleReport {
  path: string;
  files: number;
  redactionsApplied: number;
}
