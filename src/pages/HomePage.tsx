import { useMemo, useState } from "react";
import { Activity, Boxes, CheckCircle2, ChevronRight, CircleHelp, CircleX, Download, ExternalLink, FileArchive, FileText, FolderOpen, Gamepad2, Layers3, ListTree, Puzzle, Search, ShieldCheck, SlidersHorizontal, TriangleAlert, Wrench, type LucideIcon } from "lucide-react";
import { LaunchControls } from "../components/LaunchControls";
import type { CompatibilityReport, Dashboard, LaunchMode, LaunchPreflight, LoadOrderState, OperationRecord, OperationalStatus, ProfileDetail, SnapshotSummary } from "../types";

type CommandView = "readiness" | "operations" | "activity";
type SystemId = "game" | "runtime" | "profile" | "load-order" | "compatibility";

interface Props {
  data: Dashboard;
  onInstall: () => void;
  onDiagnose: () => void;
  onLocate: () => void;
  onOpenMods: () => void;
  onOpenGame: () => void;
  onLaunchGame: () => void;
  onGetUe4ss: () => void;
  onInstallUe4ss: () => void;
  busy: boolean;
  launching: boolean;
  canLaunch?: boolean;
  existingModsFound?: number;
  onReviewExisting?: () => void;
  onDismissExisting?: () => void;
  profile?: ProfileDetail | null;
  preflight?: LaunchPreflight | null;
  recentActivity?: OperationRecord[];
  compatibility?: CompatibilityReport | null;
  loadOrder?: LoadOrderState | null;
  snapshots?: SnapshotSummary[];
  onLaunchMode?: (mode: LaunchMode) => void;
  onProfiles?: () => void;
  onHealth?: () => void;
  onLibrary?: () => void;
  hideLaunchControls?: boolean;
}

interface SystemZone {
  id: SystemId;
  label: string;
  status: OperationalStatus;
  metric: string;
  summary: string;
  evidence: string;
  icon: LucideIcon;
  actionLabel: string;
  action?: () => void;
  secondaryLabel?: string;
  secondaryAction?: () => void;
}

const statusCopy: Record<OperationalStatus, string> = {
  ready: "Ready",
  warning: "Warning",
  blocked: "Blocked",
  unverified: "Unverified",
};

function formatDate(value?: string | null) {
  if (!value) return "No record";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

export function HomePage({
  data,
  onInstall,
  onDiagnose,
  onLocate,
  onOpenMods,
  onOpenGame,
  onLaunchGame,
  onGetUe4ss,
  onInstallUe4ss,
  busy,
  launching,
  canLaunch = data.game.detected,
  existingModsFound,
  onReviewExisting,
  onDismissExisting,
  profile,
  preflight,
  recentActivity = [],
  compatibility,
  loadOrder,
  snapshots = [],
  onLaunchMode,
  onProfiles,
  onHealth,
  onLibrary,
  hideLaunchControls = false,
}: Props) {
  const { game, ue4ss } = data;
  const [view, setView] = useState<CommandView>("readiness");
  const [selectedSystem, setSelectedSystem] = useState<SystemId>("compatibility");

  // A log file can be left behind by an earlier run. File presence is not
  // evidence that the current profile or runtime has loaded successfully.
  const runtimeFilesReady = ue4ss.installed && ue4ss.healthy && ue4ss.vcRuntime !== false && ue4ss.protonOverride !== false;
  const activeConflicts = loadOrder?.activeConflicts.length;
  const lastSnapshot = snapshots[0];
  const lastOperation = recentActivity[0];

  const systems = useMemo<SystemZone[]>(() => [
    {
      id: "game",
      label: "Game",
      status: game.detected ? "ready" : "blocked",
      metric: game.detected ? "Game found" : "Not found",
      summary: game.detected ? `${game.source === "ea" ? "EA App" : game.source === "manual" ? "Manual" : game.source === "automatic" ? "Steam" : "Game"} installation found. In-game mod loading is not checked here.` : "Zero Company installation is not verified.",
      evidence: game.detected ? (game.path ?? "Detected installation") : (game.problem ?? "No valid executable location is available."),
      icon: Gamepad2,
      actionLabel: game.detected ? "Open game folder" : "Locate game",
      action: game.detected ? onOpenGame : onLocate,
    },
    {
      id: "runtime",
      label: "UE4SS",
      status: runtimeFilesReady ? "ready" : ue4ss.installed ? "warning" : "unverified",
      metric: runtimeFilesReady ? "Runtime files found" : ue4ss.installed ? "Needs checking" : "Not installed",
      summary: runtimeFilesReady ? "Required runtime files are present. This does not confirm in-game loading." : ue4ss.installed ? "Runtime installation checks need attention. Open Health for details." : "UE4SS is optional until a mod requires it.",
      evidence: ue4ss.logFound ? "A UE4SS log file exists; it may be from an earlier session." : "No UE4SS log file found. This alone does not indicate an installation problem.",
      icon: Puzzle,
      actionLabel: "Open Health",
      action: onHealth ?? onDiagnose,
      secondaryLabel: ue4ss.installed ? "Update package" : "Install package",
      secondaryAction: onInstallUe4ss,
    },
    {
      id: "profile",
      label: "Profile",
      status: profile ? "ready" : "unverified",
      metric: profile?.name ?? "No profile selected",
      summary: profile ? `${profile.name} is the current named profile. Saved selections may differ from the installed state.` : "Active profile information is unavailable. Open Profiles to check it.",
      evidence: profile ? profile.requiredRuntime ? `Requires runtime ${profile.requiredRuntime}` : "No profile-specific runtime requirement" : "Profile requirements are not available",
      icon: Layers3,
      actionLabel: "Manage profiles",
      action: onProfiles,
    },
    {
      id: "load-order",
      label: "Load Order",
      status: activeConflicts ? "warning" : loadOrder ? "ready" : "unverified",
      metric: activeConflicts ? `${activeConflicts} recorded overlap${activeConflicts === 1 ? "" : "s"}` : loadOrder ? "No recorded overlaps" : "Not checked",
      summary: !loadOrder ? "Load-order information is unavailable." : activeConflicts ? "Recorded package overlaps are present. Review the intended priority in Library." : "No active overlaps in the available package metadata. This is not a check of every asset inside a mod.",
      evidence: loadOrder ? `${loadOrder.entries.length} packaged and ${loadOrder.ue4ssEntries.length} runtime entries tracked` : "Load-order graph has not been loaded",
      icon: ListTree,
      actionLabel: "Open Library",
      action: onLibrary,
      secondaryLabel: "Open mods folder",
      secondaryAction: onOpenMods,
    },
    {
      id: "compatibility",
      label: "Compatibility",
      status: compatibility?.status ?? "unverified",
      metric: compatibility ? compatibility.issues.length ? `${compatibility.issues.length} ${compatibility.issues.length === 1 ? "check" : "checks"} to review` : "No reported issues" : "Awaiting analysis",
      summary: compatibility ? (compatibility.issues.length ? "Some compatibility checks need your attention." : "No issues in the available rules and local checks. In-game compatibility is not guaranteed.") : "Compatibility has not been checked yet.",
      evidence: compatibility ? `Catalog: ${compatibility.catalogState} · ${formatDate(compatibility.generatedAt)}` : "Run Health to produce current evidence",
      icon: Puzzle,
      actionLabel: "Review compatibility",
      action: onHealth ?? onDiagnose,
    },
  ], [compatibility, game, loadOrder, activeConflicts, onDiagnose, onHealth, onInstallUe4ss, onLibrary, onLocate, onOpenGame, onOpenMods, onProfiles, profile, runtimeFilesReady, ue4ss]);

  const activeSystem = systems.find(system => system.id === selectedSystem) ?? systems[0];
  const ActiveSystemIcon = activeSystem.icon;
  const findings = compatibility?.issues ?? [];
  const evidenceRows = selectedSystem === "compatibility" ? [
    { title: "Compatibility catalog", detail: compatibility?.catalogState ? `Recorded catalog state: ${compatibility.catalogState}` : "No catalog evidence available", status: "unverified" },
    { title: "Recorded package overlaps", detail: loadOrder ? `${activeConflicts} active overlap${activeConflicts === 1 ? "" : "s"} in available metadata` : "Load-order information is unavailable", status: loadOrder ? activeConflicts ? "warning" : "ready" : "unverified" },
    { title: "Rule findings", detail: compatibility ? `${findings.length} finding${findings.length === 1 ? "" : "s"} in the available rules` : "Analysis has not been loaded", status: compatibility?.status ?? "unverified" },
    { title: "Game build", detail: game.steamBuildId ?? "Build not identified", status: !game.detected ? "blocked" : game.steamBuildId ? "ready" : "unverified" },
  ] : [
    { title: activeSystem.label, detail: activeSystem.evidence, status: activeSystem.status },
    { title: "Active profile", detail: profile?.name ?? "No named profile loaded", status: profile ? "ready" : "unverified" },
    { title: "Runtime log", detail: ue4ss.logFound ? "Log file found; current-session loading is not verified" : "No log file found", status: "unverified" },
    { title: "Last checkpoint", detail: lastSnapshot?.label ?? "No checkpoint recorded", status: lastSnapshot ? "ready" : "unverified" },
  ];
  const detailStats = selectedSystem === "compatibility" && findings.length > 0 ? [
    { value: compatibility ? findings.filter(issue => issue.status === "warning").length : "?", label: "Warnings", note: "Review recommended", icon: TriangleAlert, status: "warning" },
    { value: compatibility ? findings.filter(issue => issue.status === "blocked").length : "?", label: "Launch blockers", note: "Blocking rule findings", icon: CircleX, status: "blocked" },
  ].filter(stat => Number(stat.value) > 0) : [];

  return <div className="page command-center-page">
    {!hideLaunchControls && <LaunchControls status={preflight?.status ?? (canLaunch ? "unverified" : "blocked")} canLaunch={canLaunch} launching={launching} onHealth={onHealth ?? onDiagnose} onLaunchGame={onLaunchGame} onLaunchMode={onLaunchMode} onProfiles={onProfiles} />}

    <header className="page-header"><div><h1>Command Center</h1><p className="mod-summary" title="Counts each explicit bundle once. A partly enabled bundle is included in the enabled count.">{data.enabledMods} enabled mods · {data.installedMods} mods in library</p></div></header>

    {!game.detected && <section className="callout warning" role="alert"><div><h2>{game.problemCode === "game_path_invalid" ? "Your saved game location is unavailable" : "Locate your game installation"}</h2><p>{game.problem ?? "Automatic discovery did not find a valid Zero Company installation."}</p>{game.problemCode === "game_path_invalid" && game.path && <small className="stale-path">Saved location: <code>{game.path}</code></small>}</div><button className="primary" onClick={onLocate}>Locate game</button></section>}
    {!!existingModsFound && <section className="callout warning existing-mod-callout"><div><h2>Existing mods found</h2><p>{existingModsFound} detected entr{existingModsFound === 1 ? "y" : "ies"} to review. Some may be components of the same mod.</p></div><button onClick={onDismissExisting}>Not now</button><button className="primary" onClick={onReviewExisting}>Review existing mods</button></section>}
    {data.previousBuildId && game.steamBuildId && data.previousBuildId !== game.steamBuildId && <section className="callout warning"><div><h2>Zero Company updated</h2><p>Build {data.previousBuildId} → {game.steamBuildId}. Review installed mods before playing.</p></div><button onClick={onDiagnose}>Review</button></section>}

    <header className="command-heading">
      <div className="command-tabs" role="tablist" aria-label="Command Center views">
        {(["readiness", "operations", "activity"] as CommandView[]).map((tab, index, tabs) => <button key={tab} id={`command-tab-${tab}`} role="tab" aria-controls="command-view" tabIndex={view === tab ? 0 : -1} aria-selected={view === tab} className={view === tab ? "active" : ""} onClick={() => setView(tab)} onKeyDown={event => {
          const next = event.key === "ArrowRight" ? (index + 1) % tabs.length : event.key === "ArrowLeft" ? (index + tabs.length - 1) % tabs.length : event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : -1;
          if (next >= 0) { event.preventDefault(); setView(tabs[next]); document.getElementById(`command-tab-${tabs[next]}`)?.focus(); }
        }}><span>{tab}</span></button>)}
      </div>
    </header>

    <div id="command-view" role="tabpanel" aria-labelledby={`command-tab-${view}`}>
    {view === "readiness" && <>
      <section className="readiness-layout" aria-label="Deployment readiness">
        <div className="system-overview">
          <h2>Systems</h2>
          <div className="system-list" aria-label="Deployment systems">{systems.map(system => <button key={system.id} className={selectedSystem === system.id ? "selected" : ""} aria-label={`${system.label}: ${system.status}. ${system.metric}`} aria-pressed={selectedSystem === system.id} onClick={() => setSelectedSystem(system.id)}><system.icon aria-hidden size={20} /><span><b>{system.label}</b><small>{system.metric}</small></span><span className={`op-state ${system.status}`}>{statusCopy[system.status]}</span></button>)}</div>
        </div>

        <aside className={`system-detail ${activeSystem.status}`} aria-live="polite">
          <div className="system-detail-heading"><ActiveSystemIcon className="detail-icon" aria-hidden size={43} /><div><h2>{activeSystem.label}</h2><p>{activeSystem.metric}</p></div><span className={`system-state ${activeSystem.status}`}><i aria-hidden="true" />{statusCopy[activeSystem.status]}</span></div>
          <p className="system-summary">{activeSystem.summary}</p>
          {detailStats.length > 0 && <div className="system-stats">{detailStats.map(stat => <div key={stat.label} className={stat.status}><stat.icon aria-hidden size={30} /><strong>{stat.value}</strong><b>{stat.label}</b></div>)}</div>}
          <details className="evidence-details"><summary>Technical details</summary>
          <div className="system-evidence">{evidenceRows.map(row => <div key={row.title}><FileText aria-hidden size={23} /><span><b>{row.title}</b><small>{row.detail}</small></span>{row.status === "ready" ? <CheckCircle2 className="signal-ready" aria-label="Ready" size={19} /> : row.status === "blocked" ? <CircleX className="signal-blocked" aria-label="Blocked" size={19} /> : row.status === "warning" ? <TriangleAlert className="signal-warning" aria-label="Warning" size={19} /> : <CircleHelp aria-label="Unverified" size={19} />}</div>)}</div>
          </details>
          <div className="system-actions"><button className="primary" onClick={activeSystem.action} disabled={!activeSystem.action}><Search aria-hidden size={24} />{activeSystem.actionLabel}<ChevronRight aria-hidden size={20} /></button>{activeSystem.secondaryLabel && <button onClick={activeSystem.secondaryAction} disabled={!activeSystem.secondaryAction || busy || (selectedSystem === "runtime" && !game.detected)}>{activeSystem.secondaryLabel}</button>}</div>
        </aside>
      </section>

      <section className="command-footer" aria-label="Current deployment summary">
        <article><h3>Active profile</h3><Layers3 aria-hidden /><span><b>{profile?.name ?? "Not loaded"}</b><em title={profile ? "Includes partly enabled bundles." : undefined}>{profile ? `${profile.enabledMods} / ${profile.totalMods} mods selected in saved profile` : "Open Profiles to check the active profile"}</em></span><button onClick={onProfiles} disabled={!onProfiles}>Manage <ChevronRight aria-hidden size={16} /></button></article>
        <details className="summary-extra"><summary>Recovery and recent activity</summary><article><h3>Latest snapshot</h3><Boxes aria-hidden /><span><b>{lastSnapshot?.label ?? "No checkpoint"}</b><em>{lastSnapshot ? formatDate(lastSnapshot.createdAt) : "Create a recovery point in Profiles"}</em></span><button onClick={onProfiles} disabled={!onProfiles}>Snapshots <ChevronRight aria-hidden size={16} /></button></article>
        <article><h3>Recent operation</h3><Activity aria-hidden /><span><b>{lastOperation?.summary ?? "No managed operation"}</b><em>{lastOperation ? `${lastOperation.status} · ${formatDate(lastOperation.startedAt)}` : "Your managed changes will appear here"}</em></span><button onClick={() => setView("activity")}>View log <ChevronRight aria-hidden size={16} /></button></article></details>
      </section>
    </>}

    {view === "operations" && <section className="operations-view">
      <article className="operation-primary"><div><h2>Install a downloaded mod</h2></div><button className="primary large" onClick={onInstall}><Download aria-hidden size={18} />Install mod</button></article>
      <div className="operation-grid">
        <article><Wrench aria-hidden /><div><h3>Runtime & tools</h3><p>{runtimeFilesReady ? "Required UE4SS files are present. Check Health for installation details." : "Review runtime installation checks before using dependent mods."}</p></div><button onClick={onDiagnose}>Run Mod Doctor</button><button className="link-button" onClick={onGetUe4ss}><ExternalLink aria-hidden size={14} />UE4SS download</button><button className="link-button" onClick={onInstallUe4ss} disabled={!game.detected || busy}><FileArchive aria-hidden size={14} />Install downloaded package</button></article>
        <article><SlidersHorizontal aria-hidden /><div><h3>Deployment access</h3></div><button onClick={onLibrary} disabled={!onLibrary}>Open Library</button><button onClick={onOpenMods}><FolderOpen aria-hidden size={16} />Open mods folder</button></article>
      </div>
      {preflight?.issues.length ? <div className="preflight-list"><h2>Launch checks</h2>{preflight.issues.map(issue => <article key={issue.id}><span className={`op-state ${issue.status}`}>{statusCopy[issue.status]}</span><div><b>{issue.title}</b><p>{issue.detail}</p></div></article>)}</div> : <div className="quiet-state"><ShieldCheck aria-hidden />{preflight ? "No launch issues reported." : "Launch checks have not run yet."}</div>}
    </section>}

    {view === "activity" && <section className="activity-view">
      <div className="section-heading"><div><h2>Managed operation timeline</h2></div><button onClick={onHealth ?? onDiagnose}>Open Health</button></div>
      {recentActivity.length ? <div className="command-timeline">{recentActivity.slice(0, 10).map(item => <article key={item.id}><span className={`timeline-node ${item.status === "completed" ? "ready" : item.status === "failed" ? "blocked" : "warning"}`} aria-hidden="true" /><div><b>{item.summary}</b><small>{item.kind} · {formatDate(item.startedAt)}</small></div><span className={`op-state ${item.status === "completed" ? "ready" : item.status === "failed" ? "blocked" : "warning"}`}>{item.status}</span></article>)}</div> : <div className="quiet-state"><Activity aria-hidden />No managed operations recorded yet.</div>}
    </section>}

    </div>
    <div className="command-paths">
      <button className="path-line command-path" onClick={onOpenMods}><FolderOpen aria-hidden size={15} /><span>Open mods folder</span></button>
      {game.path && <button className="path-line command-path" onClick={onOpenGame} title="Open game folder"><Gamepad2 aria-hidden size={15} /><span>Game folder</span></button>}
    </div>
  </div>;
}
