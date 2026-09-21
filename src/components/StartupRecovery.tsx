import { FolderOpen, RefreshCw, TriangleAlert } from "lucide-react";
import brandMark from "../assets/icon.svg";

export function StartupRecovery({ message, retrying, onRetry, onOpenLogs }: {
  message: string;
  retrying: boolean;
  onRetry: () => void;
  onOpenLogs: () => void;
}) {
  return <main className="startup-recovery" role="alert">
    <img className="brand-mark" src={brandMark} alt="" width={72} height={72} />
    <TriangleAlert aria-hidden size={28} />
    <div>
      <p className="eyebrow">STARTUP NEEDS ATTENTION</p>
      <h1>Your mod library could not be prepared</h1>
      <p className="muted">Nothing was installed, removed, or changed. Retry the checks or open the application logs for the recorded cause.</p>
    </div>
    <pre>{message}</pre>
    <div className="startup-actions">
      <button className="primary" disabled={retrying} onClick={onRetry}><RefreshCw className={retrying ? "spin" : ""} aria-hidden size={17} />{retrying ? "Retrying…" : "Retry startup"}</button>
      <button onClick={onOpenLogs}><FolderOpen aria-hidden size={17} />Open logs</button>
    </div>
  </main>;
}
