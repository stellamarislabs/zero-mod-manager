import { useEffect, useId, useRef, useState } from "react";

interface Props {
  onReset: () => Promise<void>;
  disabled?: boolean;
  resetting?: boolean;
  retainedLibraryPath?: string | null;
}

function ResetConfirmation({ onReset, onCancel, resetting = false, disabled = false, retainedLibraryPath }: Props & { onCancel: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);
  const titleId = useId();
  const descriptionId = useId();
  const [acknowledged, setAcknowledged] = useState(false);
  const [confirmation, setConfirmation] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const busy = resetting || submitting;
  const ready = acknowledged && confirmation === "RESET" && !busy && !disabled;

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const element = dialog.current!;
    if (element.showModal) element.showModal();
    else element.setAttribute("open", "");
    cancel.current?.focus();
    return () => { if (element.close) element.close(); previous?.focus(); };
  }, []);

  async function confirm() {
    if (!ready) return;
    setSubmitting(true);
    setError(null);
    try {
      await onReset();
      // The app closes after scheduling the reset. Keep the action locked until then.
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      setSubmitting(false);
    }
  }

  return <dialog ref={dialog} className="confirm-dialog confirm-danger factory-reset-dialog" aria-labelledby={titleId} aria-describedby={descriptionId}
    aria-busy={busy} onCancel={event => { event.preventDefault(); if (!busy) onCancel(); }}>
    <h2 id={titleId}>Reset Zero Mod Manager?</h2>
    <div id={descriptionId} className="confirm-copy">
      <p>Clears this app’s mod records, app-owned library copies, profiles, settings, history and cache on the next launch. The app will close first.</p>
      <p><strong>Your installed game mods and saves stay untouched.</strong> Existing mods become unmanaged. Scan the game folder to add them again.</p>
      <p>Original-file backups will no longer be available in the reset app. Mods that replaced game files may not be safe to adopt again.</p>
      <small>A local recovery backup is kept outside the app’s data folder.</small>
      {retainedLibraryPath && <p className="factory-reset-retained">Custom library copies at <code>{retainedLibraryPath}</code> will remain. App records and settings still reset.</p>}
    </div>
    <label className="cleanup-ack"><input type="checkbox" checked={acknowledged} disabled={busy}
      onChange={event => setAcknowledged(event.target.checked)} />I understand that the manager will forget my mods and profiles.</label>
    <label className="factory-reset-confirmation">Type RESET to confirm
      <input value={confirmation} onChange={event => setConfirmation(event.target.value)} disabled={busy}
        autoComplete="off" spellCheck={false} autoCapitalize="off" />
    </label>
    {error && <div role="alert" className="factory-reset-error"><strong>Reset could not be scheduled.</strong><p>{error}</p><small>Review the issue, then retry or cancel.</small></div>}
    {busy && <p role="status">Preparing the reset. The app will close when ready.</p>}
    <div className="dialog-actions">
      <button ref={cancel} disabled={busy} onClick={onCancel}>Cancel</button>
      <button className="danger" disabled={!ready} onClick={() => { void confirm(); }}>{busy ? "Preparing reset…" : "Reset app and close"}</button>
    </div>
  </dialog>;
}

export function FactoryResetPanel({ onReset, disabled = false, resetting = false, retainedLibraryPath }: Props) {
  const [confirm, setConfirm] = useState(false);
  return <article className="panel factory-reset-panel" aria-labelledby="factory-reset-heading">
    <h2 id="factory-reset-heading">Factory reset</h2>
    <p>Start over with an empty manager. Installed game mods and saves are kept.</p>
    <button className="danger" disabled={disabled || resetting} onClick={() => setConfirm(true)}>Reset app data…</button>
    {confirm && <ResetConfirmation onReset={onReset} resetting={resetting} disabled={disabled} retainedLibraryPath={retainedLibraryPath} onCancel={() => setConfirm(false)} />}
  </article>;
}
