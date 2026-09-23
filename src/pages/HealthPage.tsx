import { Activity, Clipboard, FileCheck2, FileText, FolderOpen, RefreshCw, Save, ShieldAlert, Stethoscope, Undo2 } from "lucide-react";
import { useEffect, useState } from "react";
import type { CompatibilityReport, ConfigChangePreview, ConfigDocument, ConfigPatchRecord, DiagnosticReport, IsolationSession, LaunchPreflight, LaunchSession, OperationRecord, SupportBundlePreview, Ue4ssInfo } from "../types";

type Tab = "readiness" | "isolation" | "config" | "activity" | "support";
interface Props {
  diagnostics: DiagnosticReport | null;
  preflight: LaunchPreflight | null;
  compatibility: CompatibilityReport | null;
  ue4ss: Ue4ssInfo | null;
  configs: ConfigDocument[];
  configHistory: ConfigPatchRecord[];
  activity: OperationRecord[];
  sessions: LaunchSession[];
  supportPreview: SupportBundlePreview | null;
  isolation: IsolationSession | null;
  configPreview: ConfigChangePreview | null;
  loading: boolean;
  onRefresh: () => void;
  onCopyDiagnostics: () => void;
  onOpenLogs: () => void;
  onOpenUe4ssLog: () => void;
  onPreviewConfig: (path: string, content: string) => void;
  onApplyConfig: (path: string, content: string, hash: string) => void;
  onRollbackConfig: (id: string) => void;
  onCreateSupportBundle: () => void;
  onCompleteSession: (id: string, outcome: NonNullable<LaunchSession["outcome"]>) => void;
  onStartIsolation: () => void;
  onRunIsolationStep: (session: IsolationSession) => void;
  onRecordIsolation: (session: IsolationSession, outcome: NonNullable<LaunchSession["outcome"]>) => void;
  onCancelIsolation: (id: string) => void;
}

function displayDate(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "Date unavailable" : date.toLocaleString();
}

function outcomeLabel(outcome: LaunchSession["outcome"]): string {
  if (!outcome || outcome === "unknown") return "Result not recorded";
  const labels = { worked: "Worked", "not-loaded": "Mods did not load", crashed: "Crashed", "performance-issue": "Performance issue" };
  return `User reported: ${labels[outcome]}`;
}

export function HealthPage(props: Props) {
  const [tab, setTab] = useState<Tab>("readiness");
  const [selectedPath, setSelectedPath] = useState("");
  const selected = props.configs.find(config => config.path === selectedPath) ?? props.configs[0] ?? null;
  const [content, setContent] = useState(selected?.content ?? "");
  useEffect(() => { if (selected) setContent(selected.content); }, [selected?.path, selected?.sha256]);
  const [hashUnavailable, setHashUnavailable] = useState(false);
  const [editorHash, setEditorHash] = useState<{ content: string; hash: string } | null>(null);
  useEffect(() => {
    let cancelled = false;
    setEditorHash(null);
    setHashUnavailable(!globalThis.crypto?.subtle);
    if (globalThis.crypto?.subtle) {
      void globalThis.crypto.subtle.digest("SHA-256", new TextEncoder().encode(content)).then(buffer => {
        if (!cancelled) setEditorHash({ content, hash: Array.from(new Uint8Array(buffer), byte => byte.toString(16).padStart(2, "0")).join("") });
      }).catch(() => { if (!cancelled) setHashUnavailable(true); });
    }
    return () => { cancelled = true; };
  }, [content]);
  const previewCurrent = !!selected && props.configPreview?.path === selected.path
    && props.configPreview.beforeSha256 === selected.sha256
    && editorHash?.content === content && editorHash.hash === props.configPreview.afterSha256;
  const state = props.preflight?.status ?? "unverified";
  return <div className="page health-page"><header className="page-header"><div><h1>Health</h1></div><button className="primary" disabled={props.loading} onClick={props.onRefresh}><RefreshCw className={props.loading ? "spin" : ""} size={17} />Refresh all checks</button></header>
    <nav className="page-tabs" aria-label="Health sections">{(["readiness", "isolation", "config", "activity", "support"] as Tab[]).map(item => <button key={item} className={tab === item ? "active" : ""} onClick={() => setTab(item)}>{item === "config" ? "Config Workbench" : item === "isolation" ? "Guided Isolation" : item[0].toUpperCase() + item.slice(1)}</button>)}</nav>
    {tab === "readiness" && <div className="health-grid"><section className={`readiness-banner ${state}`}><div><span>Launch readiness</span><strong>{state}</strong><p>{props.preflight?.profileName ? `${props.preflight.profileName} · ${props.preflight.enabledMods} enabled mods · ${props.preflight.launcher} launcher` : "Run checks after connecting the game."}</p></div><Stethoscope size={34} /></section><section className="readiness-list">{props.preflight?.issues.map(issue => <article key={issue.id}><span className={`op-state ${issue.status}`}>{issue.status}</span><div><b>{issue.title}</b><p>{issue.detail}</p></div></article>)}{props.preflight && !props.preflight.issues.length && <div className="quiet-state"><FileCheck2 size={23} /><span>No blocking preflight issue was found.</span></div>}</section><section className="panel runtime-evidence"><h2>Runtime evidence</h2><dl><div><dt>Required files</dt><dd>{!props.ue4ss ? "Not checked" : props.ue4ss.healthy ? "Present" : props.ue4ss.installed ? "Incomplete" : "Not found"}</dd></div><div><dt>UE4SS log file</dt><dd>{!props.ue4ss ? "Not checked" : props.ue4ss.logFound ? "Found (may be from an earlier session)" : "Not found"}</dd></div><div><dt>Other loader files</dt><dd>{!props.ue4ss ? "Not checked" : props.ue4ss.extraLoaders.length ? props.ue4ss.extraLoaders.join(", ") : "None found"}</dd></div></dl><div className="settings-actions"><button onClick={props.onOpenLogs}><FolderOpen size={16} />Manager logs</button><button disabled={!props.ue4ss?.logFound} onClick={props.onOpenUe4ssLog}><FileText size={16} />UE4SS log</button></div></section><section className="panel compatibility-overview"><h2>Compatibility</h2><p>{props.compatibility?.catalogState ?? "Check unavailable"}</p><strong>{props.compatibility ? props.compatibility.issues.length ? `${props.compatibility.issues.length} reported issues` : "No issues reported by available checks" : "Run checks to see reported issues"}</strong><div>{props.compatibility?.issues.slice(0, 5).map(issue => <span key={issue.id}>{issue.title}</span>)}</div></section>{props.diagnostics && <section className="diagnostic-list compact">{props.diagnostics.items.map(item => <article key={item.label}><b>{item.label}</b><div><strong>{item.value}</strong>{item.action && <p>{item.action}</p>}</div></article>)}</section>}</div>}
    {tab === "isolation" && <div className="isolation-layout">{!props.isolation ? <section className="panel isolation-intro"><Stethoscope size={32} /><h2>Find a failing mod set without guessing</h2><p>The assistant saves a checkpoint, then tests with managed mods off before trying smaller mod groups. Runtime and unmanaged files remain. You report what happened. Save files stay untouched.</p><p className="muted">Use the main menu or a disposable test save for each launch.</p><button className="primary" onClick={props.onStartIsolation}>Start guided isolation</button></section> : <><section className="isolation-status"><div><span className={`op-state ${props.isolation.status === "completed" ? "ready" : "warning"}`}>{props.isolation.status}</span><h2>{props.isolation.phase === "runtime" ? "Repeat baseline" : props.isolation.phase === "runtime-suspect" ? "Inconsistent baseline" : props.isolation.phase.replaceAll("-", " ")}</h2><p>{props.isolation.instruction}</p></div><dl><div><dt>Candidate groups</dt><dd>{props.isolation.candidateGroups.length}</dd></div><div><dt>Next subset</dt><dd>{props.isolation.currentModIds.length} mod entries</dd></div><div><dt>Observations</dt><dd>{props.isolation.observations.length}</dd></div></dl></section>{props.isolation.status === "active" ? <section className="isolation-actions"><button className="primary" disabled={props.loading} onClick={() => props.onRunIsolationStep(props.isolation!)}>Launch this step</button><span>After returning from the game, record only what you observed:</span><div><button onClick={() => props.onRecordIsolation(props.isolation!, "worked")}>Worked</button><button onClick={() => props.onRecordIsolation(props.isolation!, "not-loaded")}>Mods did not load</button><button onClick={() => props.onRecordIsolation(props.isolation!, "crashed")}>Crashed</button><button onClick={() => props.onRecordIsolation(props.isolation!, "performance-issue")}>Performance issue</button></div><button className="danger-text" onClick={() => props.onCancelIsolation(props.isolation!.id)}>Cancel session</button></section> : <section className="isolation-result"><h2>Evidence summary</h2>{props.isolation.suspectedModIds.length ? <><p>The issue was narrowed to this inseparable dependency group. This is evidence, not automatic blame:</p><ul>{props.isolation.suspectedModIds.map(id => <li key={id}><code>{id}</code></li>)}</ul></> : <p>{props.isolation.instruction}</p>}<ol>{props.isolation.observations.map((item, index) => <li key={`${item.phase}-${index}`}><b>{item.phase}</b><span>{item.enabledModIds.length} mod entries → {outcomeLabel(item.outcome)}</span></li>)}</ol></section>}</>}</div>}
    {tab === "config" && <div className="config-workbench"><aside>{props.configs.map(config => <button key={config.path} className={selected?.path === config.path ? "selected" : ""} onClick={() => setSelectedPath(config.path)}><b>{config.path.split(/[\\/]/).at(-1)}</b><small>{config.format.toUpperCase()} · {config.writable ? "Editable" : "Read only"}</small></button>)}</aside>{selected ? <section><header><div><h2>{selected.path.split(/[\\/]/).at(-1)}</h2><code>{selected.path}</code></div><span className={`op-state ${selected.writable ? "ready" : "unverified"}`}>{selected.writable ? "editable" : "read only"}</span></header><textarea spellCheck={false} aria-label="Configuration content" readOnly={!selected.writable || props.loading} value={content} onChange={event => setContent(event.target.value)} /><div className="config-actions"><button disabled={props.loading || !selected.writable || content === selected.content} onClick={() => props.onPreviewConfig(selected.path, content)}>Preview changes</button><button className="primary" disabled={props.loading || !selected.writable || !props.configPreview?.valid || !previewCurrent || content === selected.content} onClick={() => props.onApplyConfig(selected.path, content, selected.sha256)}><Save size={16} />Apply with backup</button></div>{props.configPreview?.path === selected.path && <div className={`config-diff ${props.configPreview.valid ? "" : "invalid"}`}><b>{hashUnavailable ? "Preview verification is unavailable in this window. Reopen the app before applying changes." : !previewCurrent ? "Preview is out of date. Preview changes again before applying." : props.configPreview.valid ? `Preview: ${props.configPreview.diff.filter(line => line.startsWith("+")).length} added, ${props.configPreview.diff.filter(line => line.startsWith("-")).length} removed lines` : props.configPreview.problem}</b><pre>{previewCurrent ? props.configPreview.diff.join("\n") || "No changes" : ""}</pre></div>}</section> : <section className="empty-state"><FileText size={32} /><h2>No supported configuration found</h2><p>INI, JSON, and TOML are editable. Lua remains read only.</p></section>}<div className="config-history"><h2>Rollback history</h2>{props.configHistory.map(item => <article key={item.id}><div><b>{item.path.split(/[\\/]/).at(-1)}</b><small>{displayDate(item.createdAt)}</small></div><button onClick={() => props.onRollbackConfig(item.id)}><Undo2 size={15} />Restore</button></article>)}</div></div>}
    {tab === "activity" && <div className="activity-timeline">{props.activity.map(item => <article key={item.id}><Activity size={16} /><div><b>{item.summary}</b><small>{item.kind} · {displayDate(item.startedAt)}</small></div><span className={`op-state ${item.status === "completed" ? "ready" : item.status === "failed" ? "blocked" : item.status === "rolled-back" ? "unverified" : "warning"}`}>{item.status}</span></article>)}{!props.activity.length && <div className="quiet-state"><Activity size={23} /><span>No recorded operations yet.</span></div>}<h2>Recent launch sessions</h2>{props.sessions.map(session => <article className="launch-session" key={session.id}><Stethoscope size={16} /><div><b>{session.mode} launch via {session.launcher}</b><small>{outcomeLabel(session.outcome)}</small></div>{session.outcome && session.outcome !== "unknown" ? <span>{displayDate(session.startedAt)}</span> : <div className="session-outcomes" aria-label="Label launch result"><button onClick={() => props.onCompleteSession(session.id, "worked")}>Worked</button><button onClick={() => props.onCompleteSession(session.id, "not-loaded")}>Not loaded</button><button onClick={() => props.onCompleteSession(session.id, "crashed")}>Crash</button><button onClick={() => props.onCompleteSession(session.id, "performance-issue")}>Performance</button></div>}</article>)}</div>}
    {tab === "support" && <div className="support-layout"><section className="panel"><ShieldAlert size={30} /><h2>Support bundle</h2><p>Review what will be written locally. Nothing is uploaded automatically.</p><ul>{props.supportPreview?.sections.map(section => <li key={section}>{section}</li>)}</ul><button className="primary" disabled={!props.supportPreview || props.loading} onClick={props.onCreateSupportBundle}><FileCheck2 size={17} />Save support bundle</button></section><section className="panel"><h2>Automatic redactions</h2><ul>{props.supportPreview?.redactions.map(item => <li key={item}>{item}</li>)}</ul>{props.supportPreview ? <p className={props.supportPreview.includesSaveData ? "status-error" : "muted"}>Save data included: {props.supportPreview.includesSaveData ? "Yes" : "No"}</p> : <p className="muted">Bundle preview unavailable. Refresh checks before saving.</p>}<button disabled={!props.diagnostics} onClick={props.onCopyDiagnostics}><Clipboard size={16} />Copy diagnostics only</button></section></div>}
  </div>;
}
