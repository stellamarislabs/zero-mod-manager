import { useState } from "react";
import type { ModSummary } from "../types";
import { groupMods } from "../utils/modGroups";
import { ConfirmDialog } from "./ConfirmDialog";

export function LibraryGrouping({ mods, busy, onUngroup, onRestore }: {
  mods: ModSummary[]; busy: boolean;
  onUngroup: (id: string) => Promise<void>;
  onRestore: (id: string) => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [separating, setSeparating] = useState<ReturnType<typeof groupMods>[number] | null>(null);
  const [restoring, setRestoring] = useState<ModSummary | null>(null);
  const bundles = groupMods(mods).filter(group => group.members.length > 1 && group.members[0].bundleId);
  const legacy = mods.filter(mod => !mod.bundleId && ["pak", "iostore"].includes(mod.modType) && new Set(mod.files.map(file => file.name.replace(/\.(pak|utoc|ucas)$/i, "").toLowerCase())).size > 1);
  return <section className="library-cleanup" aria-label="Organize existing mods">
    <button disabled={busy} aria-expanded={open} onClick={() => setOpen(!open)}>Organize mods</button>
    {open && <div className="panel cleanup-panel">
      <h2>Separate incorrectly grouped mods</h2>
      <p className="muted">If different mods share one Library row, separate them here. Files, enabled states and load order stay unchanged.</p>
      {bundles.length === 0 && <p className="muted">No grouped mods to separate.</p>}
      {bundles.map(group => <div className="header-actions" key={group.id}>
        <span>{group.name} <small>{group.members.length} entries{group.members.every(mod => mod.hidden) ? ", hidden" : ""}</small></span>
        <button disabled={busy} onClick={() => setSeparating(group)}>Separate mods</button>
      </div>)}
      {legacy.length > 0 && <details>
        <summary>Recover older merged records</summary>
        <p className="muted">For entries created by the old Adopt → Merge action. Each complete PAK or IoStore file set becomes a separate mod. Ambiguous or modified records stay unchanged.</p>
        {legacy.map(mod => <div className="header-actions" key={mod.id}>
          <span>{mod.name}</span><button disabled={busy} onClick={() => setRestoring(mod)}>Recover separate mods</button>
        </div>)}
      </details>}
    </div>}
    {separating && <ConfirmDialog title={`Separate mods in ${separating.name}?`} confirmLabel="Separate mods" onCancel={() => setSeparating(null)} onConfirm={() => {
      const id = separating.members[0].id; setSeparating(null); void onUngroup(id).catch(() => {});
    }}>
      <p>Each entry below will have its own Library row. No mod is uninstalled and no game files change. A real bundle will lose its shared update and remove actions.</p>
      <ul>{separating.members.map(mod => <li key={mod.id}>{mod.name}</li>)}</ul>
    </ConfirmDialog>}
    {restoring && <ConfirmDialog title={`Recover separate mods from ${restoring.name}?`} confirmLabel="Check and recover" onCancel={() => setRestoring(null)} onConfirm={() => {
      const id = restoring.id; setRestoring(null); void onRestore(id).catch(() => {});
    }}>
      <p>Checks file ownership and copies, then restores a separate Library row for each complete file set. Profiles and compatible in-app checkpoints are preserved. Game files are not changed.</p>
      <p>Previously exported profile files are not rewritten. Export them again after recovery. Changed files, unclear ownership or incompatible checkpoints stop the operation without keeping changes.</p>
    </ConfirmDialog>}
  </section>;
}
