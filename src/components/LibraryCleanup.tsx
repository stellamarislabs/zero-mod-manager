import { useState } from "react";
import type { ModSummary } from "../types";
import { ConfirmDialog } from "./ConfirmDialog";

export function LibraryCleanup({ mods, busy, temporaryCount, onRemove, onClearTemporary }: {
  mods: ModSummary[]; busy: boolean; temporaryCount: number;
  onRemove: (mods: ModSummary[]) => void; onClearTemporary: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const [confirm, setConfirm] = useState<"mods" | "temporary" | null>(null);
  const chosen = mods.filter(mod => selected.includes(mod.id));
  return <section className="library-cleanup" aria-label="Library cleanup">
    <button disabled={busy} aria-expanded={open} onClick={() => setOpen(!open)}>Clean library</button>
    {open && <div className="panel cleanup-panel">
      <h2>Library cleanup</h2>
      <p className="muted">Choose mods to remove, or discard unfinished installation previews.</p>
      <fieldset disabled={busy}>
        <legend>Select mods to remove</legend>
        <div className="cleanup-list">{mods.map(mod => <label key={mod.id}>
          <input type="checkbox" aria-label={`Select ${mod.name} for removal`} checked={selected.includes(mod.id)} onChange={event => setSelected(current => event.target.checked ? [...current, mod.id] : current.filter(id => id !== mod.id))} />
          <span>{mod.name}{mod.hidden ? " (hidden)" : ""}<small>{mod.enabled ? "Enabled" : "Disabled"}</small></span>
        </label>)}</div>
        {mods.length === 0 && <p>No installed mods.</p>}
        <div className="header-actions">
          <button className="danger" disabled={chosen.length === 0} onClick={() => setConfirm("mods")}>Remove selected ({chosen.length})</button>
          <button disabled={temporaryCount === 0} onClick={() => setConfirm("temporary")}>Clear temporary files ({temporaryCount})</button>
        </div>
      </fieldset>
    </div>}
    {confirm === "mods" && <ConfirmDialog title="Permanently remove selected mods?" danger confirmLabel={`Remove ${chosen.length} ${chosen.length === 1 ? "mod" : "mods"}`}
      acknowledgement="I understand these mods must be reinstalled to use them again."
      onCancel={() => setConfirm(null)} onConfirm={() => { setConfirm(null); if (!busy && chosen.length) { onRemove(chosen); setSelected([]); } }}>
      <p>Deletes managed copies and unchanged deployed files. These mods will no longer be available to profiles. Keep your original downloads; snapshots do not contain mod archives.</p>
      <ul className="cleanup-list">{chosen.map(mod => <li key={mod.id}>{mod.name}</li>)}</ul>
      <small>Changed files stop removal. If a disk error interrupts cleanup, inspect the failed mod. Earlier removals are not undone. Saves and original downloads are not touched.</small>
    </ConfirmDialog>}
    {confirm === "temporary" && <ConfirmDialog title="Discard unfinished installations?" danger confirmLabel="Clear temporary files"
      acknowledgement="I understand that unfinished installation choices will be lost."
      onCancel={() => setConfirm(null)} onConfirm={() => { setConfirm(null); if (!busy) onClearTemporary(); }}>
      <p>Clears {temporaryCount} installation previews or installer sessions. You will need to open their archives again.</p>
      <small>Installed mods, original downloads, profiles and backups are kept.</small>
    </ConfirmDialog>}
  </section>;
}
