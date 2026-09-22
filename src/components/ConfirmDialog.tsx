import { useEffect, useId, useRef, useState, type ReactNode } from "react";

export function ConfirmDialog({ title, children, confirmLabel = "Continue", danger = false, acknowledgement, onConfirm, onCancel }: {
  title: string; children: ReactNode; confirmLabel?: string; danger?: boolean; acknowledgement?: string;
  onConfirm: () => void; onCancel: () => void;
}) {
  const [acknowledged, setAcknowledged] = useState(false);
  const ref = useRef<HTMLDialogElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);
  const titleId = useId();
  const descriptionId = useId();
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const dialog = ref.current!;
    if (dialog.showModal) dialog.showModal();
    else dialog.setAttribute("open", "");
    cancel.current?.focus();
    return () => { if (dialog.close) dialog.close(); previous?.focus(); };
  }, []);
  return <dialog ref={ref} className={`confirm-dialog${danger ? " confirm-danger" : ""}`} aria-labelledby={titleId} aria-describedby={descriptionId}
    onCancel={event => { event.preventDefault(); onCancel(); }}>
    <h2 id={titleId}>{title}</h2>
    <div id={descriptionId} className="confirm-copy">{children}</div>
    {acknowledgement && <label className="cleanup-ack"><input type="checkbox" checked={acknowledged} onChange={event => setAcknowledged(event.target.checked)} />{acknowledgement}</label>}
    <div className="dialog-actions">
      <button ref={cancel} onClick={onCancel}>Cancel</button>
      <button className={danger ? "danger" : "primary"} disabled={!!acknowledgement && !acknowledged} onClick={onConfirm}>{confirmLabel}</button>
    </div>
  </dialog>;
}
