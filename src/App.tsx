import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import brandMark from "./assets/icon.svg";
import { Shell, type Page } from "./components/Shell";
import { LaunchControls } from "./components/LaunchControls";
import { AdoptionDialog } from "./components/AdoptionDialog";
import { StartupRecovery } from "./components/StartupRecovery";
import { LegacyImportDialog } from "./components/LegacyImportDialog";
import { useSubscription } from "./hooks/useSubscription";
import { HomePage } from "./pages/HomePage";
import { HealthPage } from "./pages/HealthPage";
import { ProfilesPage } from "./pages/ProfilesPage";
import { groupMods } from "./utils/modGroups";
import { InstallPage } from "./pages/InstallPage";
import { ModsPage } from "./pages/ModsPage";
import { SettingsPage } from "./pages/SettingsPage";
import { AboutPage } from "./pages/AboutPage";
import { backend, friendlyError, isChangedFileError } from "./services/backend";
import type { AdoptionGroup, AdoptionReport, AppSettings, CompatibilityReport, ConfigChangePreview, ConfigDocument, ConfigPatchRecord, Dashboard, DiagnosticReport, ExistingModScan, FomodAnswer, FomodSession, IsolationSession, LaunchMode, LaunchPreflight, LaunchSession, LegacyImportStatus, Links, LoadOrderPreview, LoadOrderState, ManagedLibraryInfo, ModPreview, ModSummary, OperationRecord, PackageAssessment, ProfileDetail, ProfileSummary, ProfileSwitchPreview, SnapshotSummary, SupportBundlePreview, ToolInfo, UpdateInfo } from "./types";

const defaultSettings: AppSettings = { gamePath: null, customExecutablePath: null, sevenZipPath: null, logLevel: "normal", advancedPackageNames: false, reducedMotion: false };
const defaultLinks: Links = { ue4ssDownload: "", nexusGame: "", nexusManager: "", project: "" };
const defaultLoadOrder: LoadOrderState = { entries: [], ue4ssEntries: [], activeConflicts: [], potentialConflicts: [], unapplied: false };

/**
 * WebView2 has occasionally kept a stale CSS viewport height after a mod
 * action. Fixed-position UI still uses the real window in that state, which is
 * why the toast in the report reaches the bottom while the shell stops early.
 * The CSS no longer uses `100vh`; this guard repairs any residual mismatch and
 * leaves measurements in the application log for a Windows follow-up.
 */
function ensureFullViewport(reason: string) {
  const shell = document.querySelector<HTMLElement>(".shell");
  if (!shell) return;
  const bounds = shell.getBoundingClientRect();
  const expected = window.innerHeight;
  if (Math.abs(bounds.height - expected) <= 2 && Math.abs(bounds.top) <= 2) return;
  const main = document.querySelector<HTMLElement>(".main")?.getBoundingClientRect();
  const context = [
    `reason=${reason}`,
    `inner=${window.innerWidth}x${window.innerHeight}`,
    `client=${document.documentElement.clientWidth}x${document.documentElement.clientHeight}`,
    `visual=${window.visualViewport ? `${window.visualViewport.width}x${window.visualViewport.height}` : "unavailable"}`,
    `dpr=${window.devicePixelRatio}`,
    `shell=${bounds.left},${bounds.top},${bounds.width},${bounds.height}`,
    `main=${main ? `${main.left},${main.top},${main.width},${main.height}` : "missing"}`,
  ].join(" ");
  // Fixed positioning uses the geometry that remained correct for the report's
  // toast, making this an immediate recovery as well as a diagnostic.
  shell.style.position = "fixed";
  shell.style.inset = "0";
  try { void backend.reportInterfaceLayout(context).catch(() => {}); }
  catch { /* Browser-only tests do not have the native logging bridge. */ }
}

export default function App() {
  const [page, setPage] = useState<Page>("home");
  const [dashboard, setDashboard] = useState<Dashboard | null>(null);
  const [mods, setMods] = useState<ModSummary[]>([]);
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [activeProfile, setActiveProfile] = useState<ProfileDetail | null>(null);
  const [selectedProfile, setSelectedProfile] = useState<ProfileDetail | null>(null);
  const [profilePreview, setProfilePreview] = useState<ProfileSwitchPreview | null>(null);
  const [snapshots, setSnapshots] = useState<SnapshotSummary[]>([]);
  const [compatibility, setCompatibility] = useState<CompatibilityReport | null>(null);
  const [preflight, setPreflight] = useState<LaunchPreflight | null>(null);
  const [activity, setActivity] = useState<OperationRecord[]>([]);
  const [launchSessions, setLaunchSessions] = useState<LaunchSession[]>([]);
  const [isolation, setIsolation] = useState<IsolationSession | null>(null);
  const [configs, setConfigs] = useState<ConfigDocument[]>([]);
  const [configHistory, setConfigHistory] = useState<ConfigPatchRecord[]>([]);
  const [configPreview, setConfigPreview] = useState<ConfigChangePreview | null>(null);
  const [supportPreview, setSupportPreview] = useState<SupportBundlePreview | null>(null);
  const [loadOrder, setLoadOrder] = useState<LoadOrderState>(defaultLoadOrder);
  const [orderPreview, setOrderPreview] = useState<LoadOrderPreview | null>(null);
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [sevenZip, setSevenZip] = useState<ToolInfo | null>(null);
  const [managedLibrary, setManagedLibrary] = useState<ManagedLibraryInfo | null>(null);
  const [movingLibrary, setMovingLibrary] = useState(false);
  const [links, setLinks] = useState<Links>(defaultLinks);
  const [previews, setPreviews] = useState<ModPreview[]>([]);
  const [packageTarget, setPackageTarget] = useState("");
  const [packageAssessment, setPackageAssessment] = useState<PackageAssessment | null>(null);
  // Names the person edited before installing, keyed by staging id. An archive
  // can hold several mods, so each keeps its own draft.
  const [names, setNames] = useState<Record<string, string>>({});
  const [installing, setInstalling] = useState<string | null>(null);
  // A scripted installer and the answers it has been given. The answers are
  // the whole of its state: the backend replays them from the start on every
  // call, so dropping the last one is all that going back takes.
  const [installer, setInstaller] = useState<FomodSession | null>(null);
  const [answers, setAnswers] = useState<FomodAnswer[]>([]);
  const [presetAnswers, setPresetAnswers] = useState<FomodAnswer[]>([]);
  const [restored, setRestored] = useState<FomodAnswer | null>(null);
  const [diagnostics, setDiagnostics] = useState<DiagnosticReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [busyMod, setBusyMod] = useState<string | null>(null);
  const [orderBusy, setOrderBusy] = useState(false);
  const [launching, setLaunching] = useState(false);
  const [updateChecking, setUpdateChecking] = useState(false);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [advanced, setAdvanced] = useState(false);
  const [existingScan, setExistingScan] = useState<ExistingModScan | null>(null);
  const [existingPrompt, setExistingPrompt] = useState(false);
  const [existingReview, setExistingReview] = useState(false);
  const [discoveringExisting, setDiscoveringExisting] = useState(false);
  const [adoptingExisting, setAdoptingExisting] = useState(false);
  const [toast, setToast] = useState<{ kind: "ok" | "error"; text: string } | null>(null);
  const [bootstrapError, setBootstrapError] = useState<string | null>(null);
  const [bootstrapping, setBootstrapping] = useState(true);
  const [legacyImport, setLegacyImport] = useState<LegacyImportStatus | null>(null);
  const [importingLegacy, setImportingLegacy] = useState(false);
  const toastTimer = useRef<number | null>(null);
  const updateCheckStarted = useRef(false);
  const automaticDiscoveryStarted = useRef(false);
  const existingDiscoveryAttempt = useRef(0);
  // Held in a ref as well as in state so an inspection that finishes while
  // another is starting can still find, and release, what it replaced.
  const previewsRef = useRef<ModPreview[]>([]);
  const installerRef = useRef<FomodSession | null>(null);
  const inspectAttempt = useRef(0);
  const legacyImportChecked = useRef(false);

  /**
   * Shows a message, and for a failure keeps it on screen until it is closed.
   *
   * A confirmation is worth a glance and disappears on its own. A failure is
   * the only account of what went wrong, and dismissing it after a few seconds
   * left users reporting an error they could not read, let alone quote. Every
   * failure is also appended to the application log, so it can still be found
   * after the message is closed.
   */
  const notify = (text: string, kind: "ok" | "error" = "ok") => {
    if (toastTimer.current) window.clearTimeout(toastTimer.current);
    setToast({ text, kind });
    if (kind === "error") {
      toastTimer.current = null;
      void backend.reportInterfaceError(text, null, "notification").catch(() => undefined);
      return;
    }
    toastTimer.current = window.setTimeout(() => setToast(null), 4500);
  };
  const refresh = useCallback(async () => {
    setBootstrapping(true);
    setBootstrapError(null);
    try {
      const results = await Promise.allSettled([
        backend.dashboard(),
        backend.mods(),
        backend.loadOrder(),
        backend.settings(),
        backend.links(),
        backend.managedLibrary(),
        backend.sevenZipStatus(),
        backend.profiles(),
        backend.activeProfile(),
        backend.snapshots(),
        backend.compatibility(),
        backend.launchPreflight(),
        backend.activity(),
        backend.launchSessions(),
        backend.activeGuidedIsolation(),
        backend.supportBundlePreview(),
      ] as const);
      const [dashboardResult, modsResult, loadOrderResult, settingsResult, linksResult, managedLibraryResult, sevenZipResult, profilesResult, activeProfileResult, snapshotsResult, compatibilityResult, preflightResult, activityResult, sessionsResult, isolationResult, supportResult] = results;

      if (dashboardResult.status === "rejected") {
        setDashboard(null);
        setBootstrapError(friendlyError(dashboardResult.reason));
        return;
      }

      setDashboard(dashboardResult.value);
      if (modsResult.status === "fulfilled") setMods(modsResult.value);
      if (loadOrderResult.status === "fulfilled") setLoadOrder(loadOrderResult.value);
      if (settingsResult.status === "fulfilled") {
        setSettings(settingsResult.value);
        document.documentElement.dataset.reduceMotion = String(settingsResult.value.reducedMotion);
      }
      if (linksResult.status === "fulfilled") setLinks(linksResult.value);
      if (managedLibraryResult.status === "fulfilled") setManagedLibrary(managedLibraryResult.value);
      if (sevenZipResult.status === "fulfilled") setSevenZip(sevenZipResult.value);
      if (profilesResult.status === "fulfilled") setProfiles(profilesResult.value);
      if (activeProfileResult.status === "fulfilled") {
        setActiveProfile(activeProfileResult.value);
        setSelectedProfile(current => current ?? activeProfileResult.value);
      }
      if (snapshotsResult.status === "fulfilled") setSnapshots(snapshotsResult.value);
      if (compatibilityResult.status === "fulfilled") setCompatibility(compatibilityResult.value);
      if (preflightResult.status === "fulfilled") setPreflight(preflightResult.value);
      if (activityResult.status === "fulfilled") setActivity(activityResult.value);
      if (sessionsResult.status === "fulfilled") setLaunchSessions(sessionsResult.value);
      if (isolationResult.status === "fulfilled") setIsolation(isolationResult.value);
      if (supportResult.status === "fulfilled") setSupportPreview(supportResult.value);

      const secondaryFailure = results.slice(1).find(result => result.status === "rejected");
      if (secondaryFailure?.status === "rejected") {
        notify(`Some library details could not be loaded. ${friendlyError(secondaryFailure.reason)}`, "error");
      }
    } finally {
      setBootstrapping(false);
    }
  }, []);

  useEffect(() => {
    void backend.frontendReady().catch(() => undefined);
    void refresh();
  }, [refresh]);
  useEffect(() => {
    if (page !== "diagnostics" || !dashboard?.game.detected) return;
    void Promise.allSettled([backend.configDocuments(), backend.configHistory(), backend.supportBundlePreview()]).then(([documents, history, support]) => {
      if (documents.status === "fulfilled") setConfigs(documents.value);
      if (history.status === "fulfilled") setConfigHistory(history.value);
      if (support.status === "fulfilled") setSupportPreview(support.value);
    });
  }, [page, dashboard?.game.detected]);
  useEffect(() => {
    if (!dashboard || legacyImportChecked.current) return;
    legacyImportChecked.current = true;
    void backend.legacyImportStatus()
      .then(status => { if (status.available && status.canImport) setLegacyImport(status); })
      .catch(error => void backend.reportInterfaceError(friendlyError(error), null, "legacy-import-status").catch(() => undefined));
  }, [dashboard]);
  useEffect(() => {
    // Wait through two paints: a toggle updates its busy state, refreshed mod
    // data, and toast in adjacent React commits.
    let settledFrame = 0;
    const frame = window.requestAnimationFrame(() => {
      settledFrame = window.requestAnimationFrame(() => ensureFullViewport("app-render"));
    });
    return () => {
      window.cancelAnimationFrame(frame);
      if (settledFrame) window.cancelAnimationFrame(settledFrame);
    };
  }, [busyMod, mods, page]);
  useEffect(() => {
    if (!dashboard?.game.detected || !dashboard.existingModScanPending || automaticDiscoveryStarted.current) return;
    automaticDiscoveryStarted.current = true;
    void discoverExisting(false);
  }, [dashboard?.game.detected, dashboard?.existingModScanPending]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (!links.project || updateCheckStarted.current) return;
    updateCheckStarted.current = true;
    void checkUpdates(false);
  }, [links.project]); // eslint-disable-line react-hooks/exhaustive-deps
  useSubscription(() => getCurrentWebview().onDragDropEvent(event => {
    if (event.payload.type === "drop" && event.payload.paths[0]) { setPage("install"); void inspect(event.payload.paths[0]); }
  }), []);
  useSubscription(() => listen<string>("zcom://refresh", () => void refresh()), [refresh]);
  async function inspect(path: string) {
    const attempt = ++inspectAttempt.current;
    setLoading(true);
    try {
      const found = await backend.inspect(path);
      // Another inspection started while this one was reading the archive, so
      // this result is stale: release its sandbox rather than leaving it behind.
      if (attempt !== inspectAttempt.current) { await release(found.previews); await releaseInstaller(found.installer); return; }
      const replaced = previewsRef.current;
      const replacedInstaller = installerRef.current;
      previewsRef.current = found.previews;
      setPreviews(found.previews);
      setPackageTarget("");
      setPackageAssessment(found.package);
      openInstaller(found.installer);
      setNames({});
      await release(replaced);
      await releaseInstaller(replacedInstaller);
    }
    catch (e) { if (attempt === inspectAttempt.current) notify(friendlyError(e), "error"); }
    finally { if (attempt === inspectAttempt.current) setLoading(false); }
  }
  /** Releases the sandbox a set of previews was extracted into. */
  async function release(staged: ModPreview[]) {
    if (staged.length === 0) return;
    try { await backend.discardPreviews(staged.map(preview => preview.stagingId)); }
    catch { /* the sandbox is a cache; a failure here is not worth a message */ }
  }
  /** Releases the sandbox an unfinished scripted installer was reading. */
  async function releaseInstaller(session: FomodSession | null) {
    if (!session) return;
    try { await backend.fomodCancel(session.sessionId); }
    catch { /* the sandbox is a cache; a failure here is not worth a message */ }
  }
  function openInstaller(session: FomodSession | null, presets: FomodAnswer[] = []) {
    installerRef.current = session;
    setInstaller(session);
    setAnswers([]);
    setPresetAnswers(presets);
    setRestored(session?.step ? presets.find(answer => answer.step === session.step!.index) ?? null : null);
  }
  async function discardPreviews() {
    const staged = previewsRef.current;
    const session = installerRef.current;
    previewsRef.current = [];
    setPreviews([]);
    setPackageAssessment(null);
    openInstaller(null);
    await release(staged);
    await releaseInstaller(session);
  }
  /**
   * Answers the question a scripted installer is on. When that was its last
   * question, the files the answers chose are read straight into previews, so
   * finishing the installer lands on the same review screen every other
   * download does.
   */
  async function answerInstaller(answer: FomodAnswer) {
    const session = installerRef.current;
    if (!session) return;
    const given = [...answers.filter(item => item.step !== answer.step), answer];
    setLoading(true);
    try {
      const next = await backend.fomodAdvance(session.sessionId, given);
      if (!next.complete) {
        installerRef.current = next;
        setInstaller(next);
        setAnswers(given);
        setRestored(next.step ? presetAnswers.find(item => item.step === next.step!.index) ?? null : null);
        return;
      }
      const found = await backend.fomodInstall(session.sessionId, given);
      installerRef.current = null;
      setInstaller(null);
      setAnswers([]);
      setPresetAnswers([]);
      setRestored(null);
      previewsRef.current = found;
      setPreviews(found);
      setNames({});
    }
    catch (e) { notify(friendlyError(e), "error"); }
    finally { setLoading(false); }
  }
  /** Takes back the last answer, which also undoes every question it decided. */
  async function backInstaller() {
    const session = installerRef.current;
    const previous = answers[answers.length - 1];
    if (!session || !previous) return;
    const given = answers.slice(0, -1);
    setLoading(true);
    try {
      const next = await backend.fomodAdvance(session.sessionId, given);
      installerRef.current = next;
      setInstaller(next);
      setAnswers(given);
      // The step being returned to is the one that answer belonged to, so it
      // is put back in front of the person exactly as they left it.
      setRestored(previous);
    }
    catch (e) { notify(friendlyError(e), "error"); }
    finally { setLoading(false); }
  }
  async function choose(options: { directory?: boolean; filters?: { name: string; extensions: string[] }[] } = {}) { const picked = await open({ multiple: false, ...options }); if (typeof picked === "string") await inspect(picked); }
  async function discoverExisting(interactive: boolean) {
    const attempt = ++existingDiscoveryAttempt.current;
    setDiscoveringExisting(true);
    try {
      const scan = await backend.discoverExistingMods();
      if (attempt !== existingDiscoveryAttempt.current) return;
      setExistingScan(scan);
      if (dashboard?.existingModScanPending) await backend.acknowledgeExistingModPrompt();
      if (!interactive) {
        setExistingPrompt(scan.candidates.length > 0 || scan.unsupported.length > 0);
      } else if (scan.candidates.length > 0 || scan.unsupported.length > 0) {
        setExistingReview(true);
      } else {
        notify("No unmanaged supported mods were found.");
      }
    } catch (e) {
      if (attempt === existingDiscoveryAttempt.current) notify(friendlyError(e), "error");
    } finally {
      if (attempt === existingDiscoveryAttempt.current) setDiscoveringExisting(false);
    }
  }
  async function adoptExisting(groups: AdoptionGroup[]): Promise<AdoptionReport> {
    if (!existingScan) return { outcomes: [] };
    setAdoptingExisting(true);
    try {
      const report = await backend.adoptExistingMods(existingScan.scanId, groups);
      const succeeded = report.outcomes.filter(outcome => outcome.modSummary);
      const succeededIds = new Set(succeeded.flatMap(outcome => outcome.candidateIds));
      setExistingScan(current => current ? { ...current, candidates: current.candidates.filter(candidate => !succeededIds.has(candidate.id)) } : null);
      await refresh();
      if (succeeded.length) notify(`${succeeded.length} existing mod${succeeded.length === 1 ? "" : "s"} adopted safely.`);
      const failures = report.outcomes.length - succeeded.length;
      if (failures) notify(`${failures} mod${failures === 1 ? "" : "s"} could not be adopted. Review the details and retry.`, "error");
      return report;
    } catch (e) {
      notify(friendlyError(e), "error");
      return { outcomes: groups.map(group => ({ candidateIds: group.candidateIds, name: group.name, modSummary: null, error: friendlyError(e) })) };
    } finally {
      setAdoptingExisting(false);
    }
  }
  async function locateGame() { const picked = await open({ directory: true, multiple: false, title: "Locate Star Wars Zero Company" }); if (typeof picked === "string") { try { await backend.setGamePath(picked); await refresh(); notify("Game installation connected."); } catch (e) { notify(friendlyError(e), "error"); } } }
  /**
   * Installs one preview, offering the override when the mod being replaced
   * has a deployed file that no longer matches what was recorded.
   *
   * A mod that keeps its own settings or data next to its scripts rewrites
   * them whenever the game runs, so the guard fires on every later update.
   * Refusing outright left such a mod impossible to upgrade, so the user is
   * told which file changed and asked whether to overwrite it.
   */
  async function installOnce(preview: ModPreview) {
    const name = names[preview.stagingId];
    try {
      return await backend.install(preview.stagingId, name, preview.replaces?.modId, false);
    } catch (e) {
      if (!isChangedFileError(e) || !preview.replaces) throw e;
      if (!window.confirm(`${friendlyError(e)}\n\nUpdate ${preview.replaces.name} anyway? The changed file is not carried over: the new version's own copy takes its place, or it goes if the new version ships none.`)) throw e;
      return await backend.install(preview.stagingId, name, preview.replaces.modId, true);
    }
  }
  async function install(preview: ModPreview) {
    setInstalling(preview.stagingId);
    try {
      const mod = await installOnce(preview);
      notify(preview.replaces ? `${mod.name} replaced ${preview.replaces.name}.` : `${mod.name} installed.`);
      const remaining = previewsRef.current.filter(item => item.stagingId !== preview.stagingId);
      previewsRef.current = remaining;
      setPreviews(remaining);
      await refresh();
      if (remaining.length === 0) setPage("mods");
    } catch (e) { notify(friendlyError(e), "error"); } finally { setInstalling(null); }
  }
  async function installAll(selected: ModPreview[]) {
    const owners = selected.flatMap(item => item.replaces ? mods.filter(mod => mod.id === item.replaces!.modId) : []);
    const bundles = [...new Set(owners.map(mod => mod.bundleId).filter(Boolean))];
    if (!packageTarget && owners.length && (bundles.length !== 1 || owners.some(mod => !mod.bundleId))) { notify("This archive overlaps separate mods. Review the components before updating.", "error"); return; }
    const replaceBundleId = packageTarget || bundles[0] || null;
    const previous = mods.filter(mod => replaceBundleId && mod.bundleId === replaceBundleId);
    if (replaceBundleId && !window.confirm(`Update the complete mod package?\n\nCurrent components:\n${previous.map(mod => mod.name).join("\n")}\n\nNew components:\n${selected.map(mod => mod.name).join("\n")}\n\nAll current components will be replaced. Components not included in this archive will be removed. If the operation fails, the previous package will be restored.`)) return;
    setInstalling("all");
    const components = [...selected].sort((left, right) => {
      const rank = (preview: ModPreview) => preview.modType === "ue4ss" ? 1 : 0;
      return rank(left) - rank(right);
    });
    try {
      const report = await backend.installBundle(components.map(preview => ({
        stagingId: preview.stagingId,
        name: names[preview.stagingId] || null,
      })), replaceBundleId);
      const installedIds = new Set(components.map(preview => preview.stagingId));
      const remaining = previewsRef.current.filter(item => !installedIds.has(item.stagingId));
      previewsRef.current = remaining;
      setPreviews(remaining);
      await refresh();
      notify(`Installed all ${report.components.length} components as one recoverable bundle.`);
      if (remaining.length === 0) setPage("mods");
    } catch (e) {
      await refresh();
      notify(friendlyError(e), "error");
    } finally {
      setInstalling(null);
    }
  }
  async function installRuntimeFrom(preview: ModPreview) {
    setInstalling(preview.stagingId);
    try { await applyUe4ssPackage(preview.sourcePath); await discardPreviews(); }
    finally { setInstalling(null); }
  }
  async function rename(mod: ModSummary) {
    const next = window.prompt(`Rename ${mod.name} to:`, mod.name);
    if (next === null || next.trim() === "" || next.trim() === mod.name) return;
    setBusyMod(mod.id);
    try { await backend.rename(mod.id, next.trim()); await refresh(); notify(`Renamed to ${next.trim()}.`); }
    catch (e) { notify(friendlyError(e), "error"); } finally { setBusyMod(null); }
  }
  /**
   * Enables or disables a mod, offering the same override as removal when one
   * of its deployed files no longer matches what was recorded. Without it a
   * mod that writes its own settings could be neither disabled, updated, nor
   * removed, which is what left users with an entry nothing worked on.
   */
  async function toggle(mod: ModSummary) {
    setBusyMod(mod.id);
    const done = () => notify(`${mod.name} ${mod.enabled ? "disabled" : "enabled"}.`);
    try {
      await backend.setEnabled(mod.id, !mod.enabled);
      await refresh();
      done();
    } catch (e) {
      if (!isChangedFileError(e)) { notify(friendlyError(e), "error"); return; }
      if (!window.confirm(`${friendlyError(e)}\n\nDisable ${mod.name} anyway? The changed file is removed with the rest of the payload, and enabling the mod again restores the version it was installed with rather than this one.`)) {
        notify(`${friendlyError(e)} The changed file was kept, and ${mod.name} is still enabled.`, "error");
        return;
      }
      try { await backend.setEnabled(mod.id, false, true); await refresh(); done(); }
      catch (retry) { notify(friendlyError(retry), "error"); }
    } finally { setBusyMod(null); }
  }
  /**
   * Removes a mod, and offers to remove it anyway when one of its deployed
   * files no longer matches what was recorded.
   *
   * The guard exists so a file the user edited is never deleted silently, but
   * a mod that writes its own settings or data trips it every time. Without
   * the override the entry could be neither updated nor removed, which left
   * the library with a mod there was no way to act on at all.
   */
  async function bundleAction(members: ModSummary[], action: "toggle" | "verify" | "hide" | "remove") {
    if (busyMod || loading || launching || installing || !members.length) return;
    setBusyMod("bundle");
    let completed = 0;
    try {
      if (action === "remove") {
        await backend.uninstallBundle(members[0].id);
        notify("Mod uninstalled, including all components.");
      } else {
        const enabled = !members.every(mod => mod.enabled);
        const hidden = !members.every(mod => mod.hidden);
        for (const mod of members) {
          if (action === "verify") await backend.verify(mod.id);
          if (action === "toggle" && mod.enabled !== enabled) await backend.setEnabled(mod.id, enabled);
          if (action === "hide") await backend.setHidden(mod.id, hidden);
          completed++;
        }
        notify(action === "verify" ? "All component files checked." : action === "toggle" ? `Mod ${enabled ? "enabled" : "disabled"}.` : "Library visibility updated.");
      }
    } catch (error) {
      notify(action === "remove" ? friendlyError(error) : `Stopped after ${completed} of ${members.length} components. ${friendlyError(error)} Earlier changes were not undone.`, "error");
    } finally { try { await refresh(); } finally { setBusyMod(null); } }
  }
  async function removeLibraryMods(selected: ModSummary[]) {
    if (busyMod || loading || launching || installing) return;
    setBusyMod("library-cleanup");
    let removed = 0;
    try {
      for (const mod of selected) { await backend.uninstall(mod.id, false); removed++; }
      notify(`${removed} mods removed.`);
    } catch (error) {
      notify(`Stopped after removing ${removed} of ${selected.length} mods. ${friendlyError(error)} Unprocessed mods were kept. Check the failed mod before using it again.`, "error");
    } finally {
      try { await refresh(); } finally { setBusyMod(null); }
    }
  }
  async function clearTemporaryInstallations() {
    if (busyMod || loading || launching || installing) return;
    setBusyMod("library-cleanup");
    try {
      const staged = previewsRef.current;
      if (staged.length) await backend.discardPreviews(staged.map(item => item.stagingId));
      previewsRef.current = [];
      setPreviews([]);
      setPackageAssessment(null);
      const session = installerRef.current;
      if (session) await backend.fomodCancel(session.sessionId);
      openInstaller(null);
      notify("Temporary installation files cleared. Installed mods were kept.");
    } catch (error) { notify(`Cleanup could not finish. ${friendlyError(error)}`, "error"); }
    finally { setBusyMod(null); }
  }
  async function uninstall(mod: ModSummary) {
    if (!window.confirm(`Uninstall ${mod.name}? Its managed library copy and unchanged deployed files will be removed.`)) return;
    setBusyMod(mod.id);
    try {
      await backend.uninstall(mod.id);
      await refresh();
      notify(`${mod.name} uninstalled.`);
    } catch (e) {
      if (!isChangedFileError(e)) { notify(friendlyError(e), "error"); return; }
      if (!window.confirm(`${friendlyError(e)}\n\nRemove ${mod.name} anyway? The changed file is deleted along with the rest of the mod.`)) {
        notify(`${friendlyError(e)} The changed file was kept, and ${mod.name} is still installed.`, "error");
        return;
      }
      try {
        await backend.uninstall(mod.id, true);
        await refresh();
        notify(`${mod.name} uninstalled, including the changed file.`);
      } catch (retry) { notify(friendlyError(retry), "error"); }
    } finally { setBusyMod(null); }
  }
  async function reconfigure(mod: ModSummary) {
    setBusyMod(mod.id);
    try {
      await discardPreviews();
      const found = await backend.reconfigureFomod(mod.id);
      previewsRef.current = found.previews;
      setPreviews(found.previews);
      setPackageAssessment({ role: "modBundle", title: "Scripted mod bundle", reason: "The retained FOMOD is ready to reconfigure.", nativeFiles: [] });
      setNames(Object.fromEntries(found.previews
        .filter(preview => preview.replaces?.modId === mod.id)
        .map(preview => [preview.stagingId, mod.name])));
      openInstaller(found.installer, found.answers);
      setPage("install");
    } catch (e) { notify(friendlyError(e), "error"); }
    finally { setBusyMod(null); }
  }
  async function verify(mod: ModSummary) { setBusyMod(mod.id); try { notify(await backend.verify(mod.id)); } catch (e) { notify(friendlyError(e), "error"); } finally { setBusyMod(null); } }
  async function previewOrder(ids: string[]) { setOrderBusy(true); try { setOrderPreview(await backend.previewLoadOrder(ids)); } catch (e) { notify(friendlyError(e), "error"); } finally { setOrderBusy(false); } }
  async function applyOrder(ids: string[]) { setOrderBusy(true); try { setLoadOrder(await backend.applyLoadOrder(ids)); setOrderPreview(null); await refresh(); notify("Load order applied safely."); } catch (e) { notify(friendlyError(e), "error"); } finally { setOrderBusy(false); } }
  async function applyUe4ssOrder(ids: string[]) { setOrderBusy(true); try { setLoadOrder(await backend.applyUe4ssOrder(ids)); await refresh(); notify("UE4SS start order written to mods.txt."); } catch (e) { notify(friendlyError(e), "error"); } finally { setOrderBusy(false); } }
  async function installUe4ss() {
    const picked = await open({ multiple: false, title: "Select the downloaded UE4SS package", filters: [{ name: "UE4SS package", extensions: ["zip", "7z"] }] });
    if (typeof picked !== "string") return;
    await applyUe4ssPackage(picked);
  }
  async function applyUe4ssPackage(picked: string) {
    setLoading(true);
    try {
      const report = await backend.installUe4ss(picked);
      const kept = report.preserved.length ? ` ${report.preserved.length} existing file${report.preserved.length === 1 ? "" : "s"} kept (${report.preserved.join(", ")}).` : "";
      const proton = report.protonHint ? ' Add WINEDLLOVERRIDES="dwmapi=n,b" %command% to the game\u2019s Steam launch options.' : "";
      await refresh();
      notify(`UE4SS runtime installed: ${report.installed} file${report.installed === 1 ? "" : "s"}.${kept}${proton}`);
    } catch (e) { notify(friendlyError(e), "error"); } finally { setLoading(false); }
  }
  // Hiding is a view decision: the mod stays installed, deployed, and ordered.
  async function setHidden(mod: ModSummary, hidden: boolean) {
    try {
      await backend.setHidden(mod.id, hidden);
      await refresh();
      notify(hidden ? `${mod.name} is hidden from the library list.` : `${mod.name} is shown again.`);
    } catch (e) { notify(friendlyError(e), "error"); }
  }
  function openExternal(url: string) { if (url) void openUrl(url).catch(e => notify(friendlyError(e), "error")); }

  async function runDiagnostics() { setLoading(true); try { setDiagnostics(await backend.diagnostics()); } catch (e) { notify(friendlyError(e), "error"); } finally { setLoading(false); } }
  async function saveSettings() { try { await backend.saveSettings(settings); await refresh(); notify("Settings saved."); } catch (e) { notify(friendlyError(e), "error"); } }
  async function moveLibrary(useDefault = false) {
    if (!managedLibrary) return;
    let destination = managedLibrary.defaultPath;
    if (!useDefault) {
      const picked = await open({ directory: true, multiple: false, title: "Select an empty managed mod library folder" });
      if (typeof picked !== "string") return;
      destination = picked;
    }
    if (destination === managedLibrary.path) return;
    if (!window.confirm("Move the managed mod library to this folder? Installed game files are not moved.")) return;
    setMovingLibrary(true);
    try {
      setManagedLibrary(await backend.moveManagedLibrary(destination));
      notify("Managed mod library moved and verified.");
    } catch (e) { notify(friendlyError(e), "error"); }
    finally { setMovingLibrary(false); }
  }
  async function openFolder(kind: Parameters<typeof backend.openManagedPath>[0]) {
    try { await backend.openManagedPath(kind); }
    catch (e) { notify(friendlyError(e), "error"); }
  }
  async function importLegacyData() {
    setImportingLegacy(true);
    try {
      const report = await backend.importLegacyData();
      setLegacyImport(null);
      await refresh();
      notify(`Imported ${report.importedMods} mod${report.importedMods === 1 ? "" : "s"} and verified ${report.copiedFiles} files.`);
    } catch (error) {
      notify(friendlyError(error), "error");
    } finally {
      setImportingLegacy(false);
    }
  }
  async function launchGame() {
    setLaunching(true);
    try {
      const result = await backend.launchGame();
      notify(result.method === "custom-executable" ? "Launching the custom game executable." : "Opening Zero Company in Steam.");
    }
    catch (e) { notify(friendlyError(e), "error"); }
    finally { setLaunching(false); }
  }
  async function launchMode(mode: LaunchMode) {
    setLaunching(true);
    try {
      const selected = mode === "troubleshoot" ? (activeProfile?.mods.filter(mod => mod.enabled).map(mod => mod.modId) ?? []) : [];
      const result = await backend.launchGameMode(mode, selected);
      notify(`${mode[0].toUpperCase() + mode.slice(1)} launch requested through ${result.method}.`);
      await refresh();
    } catch (error) { notify(friendlyError(error), "error"); }
    finally { setLaunching(false); }
  }
  async function selectProfile(id: string) {
    try { setSelectedProfile(await backend.profile(id)); setProfilePreview(null); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function createProfile(name: string, notes: string) {
    try { const profile = await backend.createProfile(name, notes); await refresh(); setSelectedProfile(profile); notify(`${profile.name} created.`); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function saveProfile(profile: ProfileDetail) {
    try { const saved = await backend.updateProfile(profile.id, profile.name, profile.notes, profile.requiredRuntime); setSelectedProfile(saved); await refresh(); notify("Profile details saved."); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function deleteProfile(profile: ProfileSummary) {
    setLoading(true);
    try { await backend.deleteProfile(profile.id); setSelectedProfile(activeProfile); await refresh(); notify(`${profile.name} deleted.`); }
    catch (error) { notify(friendlyError(error), "error"); }
    finally { setLoading(false); }
  }
  async function setProfileMod(modId: string, enabled: boolean, priority: number | null) {
    if (!selectedProfile) return;
    try { setSelectedProfile(await backend.setProfileModState(selectedProfile.id, modId, enabled, priority)); setProfilePreview(null); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function reviewProfile(id: string) {
    try { setProfilePreview(await backend.previewProfileSwitch(id)); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function activateSelectedProfile(id: string) {
    setLoading(true);
    try { const profile = await backend.activateProfile(id); setActiveProfile(profile); setSelectedProfile(profile); setProfilePreview(null); await refresh(); notify(`${profile.name} is now active.`); }
    catch (error) { notify(friendlyError(error), "error"); }
    finally { setLoading(false); }
  }
  async function exportProfile(id: string) {
    const path = await save({ title: "Export profile lockfile", defaultPath: "zero-mod-profile.lock.json", filters: [{ name: "Profile lock", extensions: ["json"] }] });
    if (!path) return;
    try { await backend.exportProfileLock(id, path); notify("Profile lockfile exported."); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function importProfile() {
    const path = await open({ multiple: false, title: "Import profile lockfile", filters: [{ name: "Profile lock", extensions: ["json"] }] });
    if (typeof path !== "string") return;
    try { const profile = await backend.importProfileLock(path); await refresh(); setSelectedProfile(profile); notify(`${profile.name} imported. Missing archives were not downloaded.`); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function createCheckpoint(label: string, lastKnownGood: boolean) {
    try { await backend.createSnapshot(label, lastKnownGood); await refresh(); notify(lastKnownGood ? "Last known good checkpoint updated." : "Checkpoint created."); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function restoreCheckpoint(snapshot: SnapshotSummary) {
    if (!window.confirm(`Restore ${snapshot.label} as a new active profile? The current deployment will be snapshotted first.`)) return;
    setLoading(true);
    try { const profile = await backend.restoreSnapshot(snapshot.id); setActiveProfile(profile); setSelectedProfile(profile); await refresh(); notify(`${snapshot.label} restored as ${profile.name}.`); }
    catch (error) { notify(friendlyError(error), "error"); }
    finally { setLoading(false); }
  }
  async function refreshOperationalHealth() {
    setLoading(true);
    try {
      const [report, ready, compatible, documents, history, recent, sessions, activeIsolation, support] = await Promise.all([
        backend.diagnostics(), backend.launchPreflight(), backend.compatibility(), backend.configDocuments(), backend.configHistory(), backend.activity(), backend.launchSessions(), backend.activeGuidedIsolation(), backend.supportBundlePreview(),
      ]);
      setDiagnostics(report); setPreflight(ready); setCompatibility(compatible); setConfigs(documents); setConfigHistory(history); setActivity(recent); setLaunchSessions(sessions); setIsolation(activeIsolation); setSupportPreview(support);
    } catch (error) { notify(friendlyError(error), "error"); }
    finally { setLoading(false); }
  }
  async function previewConfig(path: string, content: string) {
    try { setConfigPreview(await backend.previewConfigChange(path, content)); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function applyConfig(path: string, content: string, hash: string) {
    try { await backend.applyConfigChange(path, content, hash); setConfigPreview(null); await refreshOperationalHealth(); notify("Configuration applied with a rollback checkpoint."); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function rollbackConfig(id: string) {
    try { await backend.rollbackConfigChange(id); await refreshOperationalHealth(); notify("Configuration checkpoint restored."); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function saveSupportBundle() {
    const path = await save({ title: "Save support bundle", defaultPath: `zero-mod-manager-support-${new Date().toISOString().slice(0, 10)}.zip`, filters: [{ name: "ZIP archive", extensions: ["zip"] }] });
    if (!path) return;
    try { const report = await backend.createSupportBundle(path); notify(`Support bundle saved with ${report.files} files and ${report.redactionsApplied} redactions.`); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function refreshCatalog() {
    try { setCompatibility(await backend.updateCompatibilityCatalog()); notify("Signed compatibility catalog verified and refreshed."); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function completeSession(id: string, outcome: NonNullable<LaunchSession["outcome"]>) {
    try {
      await backend.completeLaunchSession(id, outcome);
      setLaunchSessions(await backend.launchSessions());
      notify("Launch result recorded. The manager will treat it as user evidence, not automatic blame.");
    } catch (error) { notify(friendlyError(error), "error"); }
  }
  async function startIsolation() {
    try { const session = await backend.startGuidedIsolation(); setIsolation(session); notify("Guided isolation started with a protected profile checkpoint."); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function runIsolationStep(session: IsolationSession) {
    setLaunching(true);
    try {
      const mode: LaunchMode = session.phase === "baseline" ? "vanilla" : "troubleshoot";
      await backend.launchGameMode(mode, session.currentModIds);
      setLaunchSessions(await backend.launchSessions());
      notify("Isolation test launched. Return here and label only what you observed.");
    } catch (error) { notify(friendlyError(error), "error"); }
    finally { setLaunching(false); }
  }
  async function recordIsolation(session: IsolationSession, outcome: NonNullable<LaunchSession["outcome"]>) {
    try { setIsolation(await backend.advanceGuidedIsolation(session.id, outcome)); setActivity(await backend.activity()); notify("Observation recorded; the next dependency-safe test set is ready."); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function cancelIsolation(id: string) {
    try { await backend.cancelGuidedIsolation(id); setIsolation(null); notify("Guided isolation cancelled; the active profile remains authoritative."); }
    catch (error) { notify(friendlyError(error), "error"); }
  }
  async function checkUpdates(announce: boolean) {
    setUpdateChecking(true); setUpdateError(null);
    try {
      const result = await backend.checkForUpdates();
      setUpdate(result);
      if (announce) notify(result.releaseAvailable === false ? "No public release is available yet." : result.updateAvailable ? `Version ${result.latestVersion} is available.` : "Zero Mod Manager is up to date.");
    } catch (e) {
      const message = friendlyError(e);
      setUpdateError(message);
      if (announce) notify("Updates could not be checked. Retry in About or open All releases.", "error");
    } finally { setUpdateChecking(false); }
  }

  if (!dashboard && bootstrapError) return <StartupRecovery
    message={bootstrapError}
    retrying={bootstrapping}
    onRetry={() => void refresh()}
    onOpenLogs={() => void backend.openManagedPath("logs").catch(error => {
      setBootstrapError(current => `${current ?? "Startup failed."}\n\nCould not open logs: ${friendlyError(error)}`);
    })}
  />;
  if (!dashboard) return <div className="splash" role="status"><img className="brand-mark" src={brandMark} alt="" width={96} height={96} /><p>Preparing your mod library…</p></div>;
  return <Shell page={page} onPage={setPage} gameReady={dashboard.game.detected} updateAvailable={update?.updateAvailable === true} toolbar={<LaunchControls
    status={preflight?.status ?? (dashboard.game.detected || settings.customExecutablePath ? "unverified" : "blocked")}
    canLaunch={dashboard.game.detected || !!settings.customExecutablePath}
    launching={launching || !!busyMod}
    onHealth={() => setPage("diagnostics")}
    onLaunchGame={() => void launchGame()}
    onLaunchMode={mode => void launchMode(mode)}
    onProfiles={() => setPage("profiles")}
  />}>
    {page === "home" && <HomePage
      hideLaunchControls
      data={dashboard}
      profile={activeProfile}
      preflight={preflight}
      recentActivity={activity}
      compatibility={compatibility}
      loadOrder={loadOrder}
      snapshots={snapshots}
      onInstall={() => setPage("install")}
      onDiagnose={() => setPage("diagnostics")}
      onProfiles={() => setPage("profiles")}
      onHealth={() => setPage("diagnostics")}
      onLibrary={() => setPage("mods")}
      onLaunchMode={mode => void launchMode(mode)}
      onLocate={locateGame}
      onOpenMods={() => void openFolder("mods")}
      onOpenGame={() => void openFolder("game")}
      onLaunchGame={() => void launchGame()}
      onGetUe4ss={() => openExternal(links.ue4ssDownload)}
      onInstallUe4ss={() => void installUe4ss()}
      busy={loading}
      launching={launching || !!busyMod}
      canLaunch={dashboard.game.detected || !!settings.customExecutablePath}
      existingModsFound={existingPrompt ? (existingScan?.candidates.length ?? 0) + (existingScan?.unsupported.length ?? 0) : 0}
      onDismissExisting={() => setExistingPrompt(false)}
      onReviewExisting={() => { setExistingPrompt(false); setPage("mods"); setExistingReview(true); }}
    />}
    {page === "mods" && <ModsPage onBundleAction={(members, action) => void bundleAction(members, action)} onBulkRemove={selected => void removeLibraryMods(selected)} onClearTemporary={() => void clearTemporaryInstallations()} temporaryCount={previews.length + (installer ? 1 : 0)} mods={mods} loadOrder={loadOrder} orderPreview={orderPreview} orderBusy={orderBusy} onPreviewOrder={ids => void previewOrder(ids)} onApplyOrder={ids => void applyOrder(ids)} onApplyUe4ssOrder={ids => void applyUe4ssOrder(ids)} onCancelOrder={() => setOrderPreview(null)} onBrowseNexus={() => openExternal(links.nexusGame)} busy={busyMod || (loading || launching || installing ? "operation" : null)} onInstall={() => setPage("install")} onDiscover={() => void discoverExisting(true)} discovering={discoveringExisting} onToggle={toggle} onUninstall={uninstall} onReconfigure={mod => void reconfigure(mod)} onVerify={verify} onRename={rename} onOpenInstalled={mod => void openFolder(`installed:${mod.id}`)} onOpenSource={mod => void openFolder(`mod:${mod.id}`)} onOpenModPage={mod => { if (mod.nexusUrl) openExternal(mod.nexusUrl); }} onSetHidden={(mod, hidden) => void setHidden(mod, hidden)} />}
    {page === "install" && <InstallPage packageTarget={packageTarget} onPackageTarget={setPackageTarget} packageTargets={groupMods(mods).filter(group => group.members[0].bundleId).map(group => ({ id: group.members[0].bundleId!, name: group.name }))} previews={previews} packageAssessment={packageAssessment} names={names} loading={loading} installer={installer} installerRestored={restored} installerCanGoBack={answers.length > 0} onInstallerNext={answer => void answerInstaller(answer)} onInstallerBack={() => void backInstaller()} advanced={advanced} installing={installing} onAdvanced={() => setAdvanced(!advanced)} onName={(stagingId, name) => setNames(current => ({ ...current, [stagingId]: name }))} onChooseFile={() => void choose({ filters: [{ name: "Supported mods", extensions: ["zip", "7z", "rar", "pak", "utoc", "ucas"] }] })} onChooseFolder={() => void choose({ directory: true })} onInstall={mod => void install(mod)} onInstallAll={mods => void installAll(mods)} onInstallRuntime={mod => void installRuntimeFrom(mod)} onCancel={() => void discardPreviews()} />}
    {page === "profiles" && <ProfilesPage profiles={profiles} selected={selectedProfile} preview={profilePreview} snapshots={snapshots} busy={loading} onSelect={id => void selectProfile(id)} onCreate={(name, notes) => void createProfile(name, notes)} onSave={profile => void saveProfile(profile)} onDelete={profile => void deleteProfile(profile)} onSetMod={(modId, enabled, priority) => void setProfileMod(modId, enabled, priority)} onPreview={id => void reviewProfile(id)} onActivate={id => void activateSelectedProfile(id)} onExport={id => void exportProfile(id)} onImport={() => void importProfile()} onSnapshot={(label, knownGood) => void createCheckpoint(label, knownGood)} onRestoreSnapshot={snapshot => void restoreCheckpoint(snapshot)} />}
    {page === "diagnostics" && <HealthPage diagnostics={diagnostics} preflight={preflight} compatibility={compatibility} ue4ss={dashboard.ue4ss} configs={configs} configHistory={configHistory} activity={activity} sessions={launchSessions} isolation={isolation} supportPreview={supportPreview} configPreview={configPreview} loading={loading || launching} onRefresh={() => void refreshOperationalHealth()} onCopyDiagnostics={() => void navigator.clipboard.writeText(diagnostics?.text ?? "").then(() => notify("Diagnostic report copied."))} onOpenUe4ssLog={() => void openFolder("ue4ss-log")} onOpenLogs={() => void openFolder("logs")} onPreviewConfig={(path, content) => void previewConfig(path, content)} onApplyConfig={(path, content, hash) => void applyConfig(path, content, hash)} onRollbackConfig={id => void rollbackConfig(id)} onCreateSupportBundle={() => void saveSupportBundle()} onCompleteSession={(id, outcome) => void completeSession(id, outcome)} onStartIsolation={() => void startIsolation()} onRunIsolationStep={session => void runIsolationStep(session)} onRecordIsolation={(session, outcome) => void recordIsolation(session, outcome)} onCancelIsolation={id => void cancelIsolation(id)} />}
    {page === "settings" && <SettingsPage settings={settings} sevenZip={sevenZip} managedLibrary={managedLibrary} movingLibrary={movingLibrary} onChange={setSettings} onSave={() => void saveSettings()} onPickGame={() => void locateGame()} onPickExecutable={async () => { const picked = await open({ multiple: false, title: "Select game executable or launcher" }); if (typeof picked === "string") setSettings({ ...settings, customExecutablePath: picked }); }} onPickSevenZip={async () => { const picked = await open({ multiple: false, title: "Select the 7-Zip command-line executable (7z.exe)" }); if (typeof picked === "string") setSettings({ ...settings, sevenZipPath: picked }); }} onMoveLibrary={() => void moveLibrary()} onUseDefaultLibrary={() => void moveLibrary(true)} onOpenLibrary={() => void openFolder("library")} onOpenLogs={() => void openFolder("logs")} onOpenData={() => void openFolder("data")} links={links} onOpenLink={openExternal} />}
    {page === "about" && <AboutPage projectUrl={links.project} nexusUrl={links.nexusManager} onOpenLink={openExternal} update={update} checking={updateChecking} error={updateError} onCheckUpdates={() => void checkUpdates(true)} />}
    {discoveringExisting && <div className="scan-indicator" role="status"><span className="spin" />Scanning game folders for existing mods…</div>}
    {existingReview && existingScan && <AdoptionDialog scan={existingScan} busy={adoptingExisting} onClose={() => setExistingReview(false)} onAdopt={adoptExisting} />}
    {legacyImport && <LegacyImportDialog status={legacyImport} busy={importingLegacy} onClose={() => setLegacyImport(null)} onImport={() => void importLegacyData()} />}
    {toast && <div className={`toast ${toast.kind}`} role={toast.kind === "error" ? "alert" : "status"}>
      <p>{toast.text}</p>
      {toast.kind === "error" && <div className="toast-actions">
        <button onClick={() => void navigator.clipboard.writeText(toast.text).catch(() => undefined)}>Copy</button>
        <button onClick={() => void openFolder("logs")}>Open logs</button>
        <button onClick={() => setToast(null)}>Dismiss</button>
      </div>}
    </div>}
  </Shell>;
}
