import { ArchiveRestore, Loader2, X } from "lucide-react";
import type { LegacyImportStatus } from "../types";

export function LegacyImportDialog({ status, busy, onImport, onClose }: {
  status: LegacyImportStatus;
  busy: boolean;
  onImport: () => void;
  onClose: () => void;
}) {
  return <div className="dialog-backdrop" role="presentation">
    <section className="panel legacy-import-dialog" role="dialog" aria-modal="true" aria-labelledby="legacy-import-title">
      <button className="detail-close" aria-label="Not now" onClick={onClose} disabled={busy}><X aria-hidden size={18} /></button>
      <ArchiveRestore aria-hidden size={30} />
      <div><p className="eyebrow">LEGACY DATA FOUND</p><h1 id="legacy-import-title">Continue with your ZCOM library</h1></div>
      <p>Zero Mod Manager found {status.modCount} managed mod{status.modCount === 1 ? "" : "s"} and {status.fileCount} library file{status.fileCount === 1 ? "" : "s"} from ZCOM Mod Manager 0.6.5.</p>
      <div className="inline-note"><b>Safe import</b><span>The library is copied and checksum-verified first. The original data is left unchanged, and a backup of this new database is retained.</span></div>
      <small className="legacy-source">Source: {status.dataDirectory}</small>
      <div className="dialog-actions"><button disabled={busy} onClick={onClose}>Not now</button><button className="primary" disabled={busy} onClick={() => onImport()}>{busy ? <Loader2 className="spin" aria-hidden size={17} /> : <ArchiveRestore aria-hidden size={17} />}{busy ? "Importing and verifying…" : "Import library"}</button></div>
    </section>
  </div>;
}
