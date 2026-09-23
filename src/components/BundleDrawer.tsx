import { useEffect, useRef } from "react";
import type { ModSummary } from "../types";

export function BundleDrawer({ name, members, busy, onClose, onToggle, onVerify, onUninstall, onOpenFiles, onRename, onOpenSource, onReconfigure, onOpenModPage }: {
  onRename: (mod: ModSummary) => void; onOpenSource: (mod: ModSummary) => void; onReconfigure: (mod: ModSummary) => void; onOpenModPage: (mod: ModSummary) => void;
  name: string; members: ModSummary[]; busy: boolean; onClose: () => void;
  onToggle: (mod: ModSummary) => void; onVerify: (mod: ModSummary) => void;
  onUninstall: (mod: ModSummary) => void; onOpenFiles: (mod: ModSummary) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const close = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const el = dialog.current!;
    if (el.showModal) el.showModal(); else el.setAttribute("open", "");
    close.current?.focus();
    return () => { el.close?.(); previous?.focus(); };
  }, []);
  return <dialog ref={dialog} className="bundle-drawer" aria-label={`${name} components`} onCancel={event => { event.preventDefault(); onClose(); }}>
    <header><div><h2>{name}</h2><p>{members.length} components in this bundle</p></div><button ref={close} onClick={onClose}>Close</button></header>
    {members.map(mod => <article key={mod.id}>
      <h3>{mod.name}</h3><p>{mod.modType === "ue4ss" ? "UE4SS" : mod.modType}{mod.version ? ` · ${mod.version}` : ""}</p>
      <label className="component-enabled"><input type="checkbox" checked={mod.enabled} disabled={busy} onChange={() => onToggle(mod)} />Enabled: {mod.name}</label>
      <div className="header-actions"><button onClick={() => onOpenFiles(mod)}>Open files</button><button disabled={busy} onClick={() => onVerify(mod)}>Verify files</button><button className="danger" disabled={busy} onClick={() => onUninstall(mod)}>Remove component</button></div>
      <div className="header-actions"><button disabled={busy} onClick={() => onRename(mod)}>Rename component</button><button onClick={() => onOpenSource(mod)}>Open library copy</button>{mod.fomod && <button disabled={busy} onClick={() => onReconfigure(mod)}>Reconfigure</button>}{mod.nexusUrl && <button onClick={() => onOpenModPage(mod)}>Nexus page</button>}</div>
      <details><summary>Files ({mod.files.length})</summary><ul>{mod.files.map(file => <li key={file.destination}>{file.name}</li>)}</ul></details>
    </article>)}
  </dialog>;
}
