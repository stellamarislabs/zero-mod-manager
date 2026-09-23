import { AlertTriangle, Check, Loader2, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { AdoptionGroup, AdoptionReport, ExistingModCandidate, ExistingModScan } from "../types";

export interface DraftAdoptionGroup extends AdoptionGroup {
  selected: boolean;
}

export function initialAdoptionGroups(candidates: ExistingModCandidate[]): DraftAdoptionGroup[] {
  return candidates.map(candidate => ({
    candidateIds: [candidate.id],
    name: candidate.name,
    selected: candidate.selectedByDefault && candidate.adoptable
  }));
}

interface Props {
  scan: ExistingModScan;
  busy: boolean;
  onClose: () => void;
  onAdopt: (groups: AdoptionGroup[]) => Promise<AdoptionReport>;
}

export function AdoptionDialog({ scan, busy, onClose, onAdopt }: Props) {
  const [groups, setGroups] = useState<DraftAdoptionGroup[]>(() => initialAdoptionGroups(scan.candidates));
  const [errors, setErrors] = useState<Map<string, string>>(new Map());
  const byId = useMemo(() => new Map(scan.candidates.map(candidate => [candidate.id, candidate])), [scan.candidates]);
  useEffect(() => {
    setGroups(initialAdoptionGroups(scan.candidates));
    setErrors(new Map());
  }, [scan.scanId]);

  const selected = groups.filter(group => group.selected && group.candidateIds.every(id => byId.get(id)?.adoptable));


  async function adopt() {
    const report = await onAdopt(selected.map(({ candidateIds, name }) => ({
      candidateIds, name,
    })));
    const failures = report.outcomes.filter(outcome => outcome.error);
    setErrors(new Map(failures.flatMap(outcome => outcome.candidateIds.map(id => [id, outcome.error!] as const))));
    if (failures.length === 0) {
      onClose();
      return;
    }
    const failed = new Set(failures.flatMap(outcome => outcome.candidateIds));
    setGroups(current => current
      .filter(group => group.candidateIds.some(id => failed.has(id)))
      .map(group => ({ ...group, candidateIds: group.candidateIds.filter(id => failed.has(id)), selected: true })));
  }

  return <div className="dialog-backdrop" role="presentation">
    <section className="adoption-dialog panel" role="dialog" aria-modal="true" aria-labelledby="adoption-title">
      <header className="adoption-heading">
        <div><p className="eyebrow">MIGRATION</p><h1 id="adoption-title">Adopt existing mods</h1><p className="muted">Each selected mod stays separate. Files are copied into the managed library without changing the game folder.</p></div>
        <button className="detail-close" aria-label="Close existing mod review" onClick={onClose} disabled={busy}><X size={18} /></button>
      </header>

      {scan.warnings.map(warning => <div className="inline-warning" key={warning}><AlertTriangle size={17} /><span>{warning}</span></div>)}
      {groups.length === 0 && <div className="adoption-empty"><Check size={26} /><b>No unmanaged supported mods were found.</b></div>}

      <div className="adoption-list" aria-label="Existing mod candidates">
        {groups.map((group, index) => {
          const members = group.candidateIds.map(id => byId.get(id)).filter((item): item is ExistingModCandidate => !!item);
          if (members.length === 0) return null;
          const adoptable = members.every(member => member.adoptable);
          const runtime = members.some(member => member.likelyRuntimeComponent);
          const failure = members.map(member => errors.get(member.id)).find(Boolean);
          return <article className={`adoption-row ${adoptable ? "" : "blocked"}`} key={group.candidateIds.join("|")}>
            <label className="adoption-select">
              <input type="checkbox" checked={group.selected} disabled={!adoptable || busy} onChange={event => setGroups(current => current.map((item, currentIndex) => currentIndex === index ? { ...item, selected: event.target.checked } : item))} />
              <span>{group.candidateIds.length > 1 ? `${group.candidateIds.length} components` : members[0].modType === "ue4ss" ? "UE4SS mod" : members[0].modType === "plugin" ? "Plugin mod" : members[0].modType === "gamedir" ? "LogicMod" : members[0].modType === "iostore" ? "IoStore mod" : "PAK mod"}</span>
            </label>
            <input className="adoption-name" aria-label={`Name for ${members[0].name}`} value={group.name} maxLength={120} disabled={!adoptable || busy} onChange={event => setGroups(current => current.map((item, currentIndex) => currentIndex === index ? { ...item, name: event.target.value } : item))} />
            <div className="adoption-meta"><span>{members.reduce((count, member) => count + member.files.length, 0)} file{members.reduce((count, member) => count + member.files.length, 0) === 1 ? "" : "s"}</span><span>{members.every(member => member.enabled) ? "Enabled" : members.some(member => member.enabled) ? "Partly enabled" : "Disabled"}</span>{members.some(member => member.packageCount) && <span>{members.reduce((count, member) => count + member.packageCount, 0)} packages</span>}</div>
            {runtime && <small className="warn-text">Likely part of the UE4SS runtime. It is unchecked by default because uninstalling it may damage the runtime.</small>}
            {members.flatMap(member => member.warnings).map(warning => <small className="warn-text" key={warning}>{warning}</small>)}
            {!adoptable && <small className="error-text">{members.find(member => member.blockedReason)?.blockedReason}</small>}
            {failure && <small className="error-text" role="alert">{failure}</small>}
            <details><summary>Files</summary><ul className="file-list">{members.flatMap(member => member.files).map(file => <li key={file}>{file}</li>)}</ul></details>
          </article>;
        })}
      </div>

      {scan.unsupported.length > 0 && <section className="adoption-unsupported"><h2>Detected but not safely adoptable</h2><p>Zero Mod Manager cannot restore files that were replaced before it started managing them.</p><ul>{scan.unsupported.map(item => <li key={item}>{item}</li>)}</ul></section>}

      <p className="muted">A game-folder scan cannot prove that separate mods belong to one bundle. To install a bundle as one entry, use its original archive.</p>
      <footer className="dialog-actions">
        <button onClick={onClose} disabled={busy}>Close</button>
        <button className="primary" onClick={() => void adopt()} disabled={busy || selected.length === 0 || selected.some(group => !group.name.trim())}>{busy ? <><Loader2 className="spin" size={16} />Adopting {selected.length}…</> : `Adopt ${selected.length} selected`}</button>
      </footer>
    </section>
  </div>;
}
