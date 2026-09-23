import { Clipboard, FileText, FolderOpen, RefreshCw, Stethoscope } from "lucide-react";
import { StatusBadge } from "../components/StatusBadge";
import type { DiagnosticReport, Ue4ssInfo } from "../types";

interface Props {
  report: DiagnosticReport | null;
  loading: boolean;
  ue4ss: Ue4ssInfo | null;
  onRun: () => void;
  onCopy: () => void;
  onOpenUe4ssLog: () => void;
  onOpenLogs: () => void;
}

export function DiagnosticsPage({ report, loading, ue4ss, onRun, onCopy, onOpenUe4ssLog, onOpenLogs }: Props) {
  const hasIssue = report?.items.some(item => item.status === "error" || item.status === "warning") ?? false;
  return <div className="page"><header className="page-header"><div><p className="eyebrow">MOD DOCTOR</p><h1>Diagnostics</h1><p className="muted">Check the game, mod deployment, UE4SS, archive tools, and Proton configuration.</p></div><button className="primary" disabled={loading} onClick={onRun}><RefreshCw className={loading ? "spin" : ""} size={18} />{loading ? "Checking…" : "Run diagnostics"}</button></header>
    <section className="panel"><h2>Logs</h2><p>Manager logs contain recorded operations and errors. UE4SS logs are separate and may be from an earlier session.</p><div className="settings-actions"><button onClick={onOpenLogs}><FolderOpen aria-hidden size={16} />Open manager logs</button><button disabled={!ue4ss?.logFound} onClick={onOpenUe4ssLog}><FileText aria-hidden size={16} />Open UE4SS log</button></div>{ue4ss?.installed && !ue4ss.logFound && <small className="warn-text">No UE4SS log was found at the expected location. This does not confirm whether the runtime loaded.</small>}</section>
    {!report ? <section className="empty-state"><Stethoscope aria-hidden size={34} /><h2>Ready for a health check</h2><p>Diagnostics are read-only. They do not change your game or Steam settings.</p><button className="primary" onClick={onRun}>Start check</button></section> : <><section className={`doctor-summary ${hasIssue ? "warning" : "good"}`}><Stethoscope aria-hidden size={30} /><div><span>Diagnostic checks</span><strong>{hasIssue ? report.overall : "No issues found by these checks"}</strong></div><button onClick={onCopy}><Clipboard size={17} />Copy report</button></section><section className="diagnostic-list">{report.items.map(item => <article key={item.label}><StatusBadge status={item.status}>{item.label}</StatusBadge><div><b>{item.value}</b>{item.action && <p>{item.action}</p>}</div></article>)}</section></>}
  </div>;
}
