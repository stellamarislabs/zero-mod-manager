import { FolderOpen, Play, Wrench } from "lucide-react";
import type { LaunchMode, OperationalStatus } from "../types";

export function LaunchControls({ status, canLaunch, launching, onHealth, onLaunchGame, onLaunchMode, onProfiles }: {
  status: OperationalStatus;
  canLaunch: boolean;
  launching: boolean;
  onHealth: () => void;
  onLaunchGame: () => void;
  onLaunchMode?: (mode: LaunchMode) => void;
  onProfiles?: () => void;
}) {
  const label = status[0].toUpperCase() + status.slice(1);
  return <section className="command-bar" aria-label="Launch and readiness controls">
    <button className={`command-readiness ${status}`} onClick={onHealth} aria-label={`Readiness: ${label}. Open Health`}><span className={`op-state ${status}`}>{label}</span></button>
    <button className="primary" aria-label={onLaunchMode ? "Launch modded" : "Launch game"} onClick={() => onLaunchMode ? onLaunchMode("modded") : onLaunchGame()} disabled={!canLaunch || launching || status === "blocked"}>
      <Play aria-hidden size={17} />{launching ? "Launching…" : onLaunchMode ? "Launch modded" : "Launch game"}
    </button>
    <button onClick={() => onLaunchMode ? onLaunchMode("vanilla") : onLaunchGame()} disabled={!canLaunch || launching}><Play aria-hidden size={17} />Launch vanilla</button>
    <button onClick={() => onLaunchMode?.("troubleshoot")} disabled={!onLaunchMode || !canLaunch || launching}><Wrench aria-hidden size={17} />Troubleshoot</button>
    <button onClick={onProfiles} disabled={!onProfiles}><FolderOpen aria-hidden size={17} />Profiles</button>
  </section>;
}
