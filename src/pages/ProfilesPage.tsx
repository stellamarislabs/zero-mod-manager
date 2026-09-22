import { Check, Download, FilePlus2, Save, ShieldCheck, Trash2, Upload } from "lucide-react";
import { useEffect, useState } from "react";
import type { ProfileDetail, ProfileSummary, ProfileSwitchPreview, SnapshotSummary } from "../types";

interface Props {
  profiles: ProfileSummary[];
  selected: ProfileDetail | null;
  preview: ProfileSwitchPreview | null;
  snapshots: SnapshotSummary[];
  busy: boolean;
  onSelect: (id: string) => void;
  onCreate: (name: string, notes: string) => void;
  onSave: (profile: ProfileDetail) => void;
  onDelete: (profile: ProfileSummary) => void;
  onSetMod: (modId: string, enabled: boolean, priority: number | null) => void;
  onPreview: (id: string) => void;
  onActivate: (id: string) => void;
  onExport: (id: string) => void;
  onImport: () => void;
  onSnapshot: (label: string, lastKnownGood: boolean) => void;
  onRestoreSnapshot: (snapshot: SnapshotSummary) => void;
}

export function ProfilesPage({ profiles, selected, preview, snapshots, busy, onSelect, onCreate, onSave, onDelete, onSetMod, onPreview, onActivate, onExport, onImport, onSnapshot, onRestoreSnapshot }: Props) {
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [notes, setNotes] = useState("");
  const [draft, setDraft] = useState<ProfileDetail | null>(selected);
  const [checkpoint, setCheckpoint] = useState("Manual checkpoint");
  useEffect(() => { setDraft(selected); }, [selected]);

  return <div className="page profiles-page">
    <header className="page-header"><div><h1>Profiles</h1></div><div className="header-actions"><button onClick={onImport}><Upload size={16} />Import lockfile</button><button className="primary" onClick={() => setCreating(true)}><FilePlus2 size={17} />New profile</button></div></header>
    {creating && <section className="inline-editor" aria-label="Create profile"><label>Name<input autoFocus value={name} onChange={event => setName(event.target.value)} /></label><label>Notes<input value={notes} onChange={event => setNotes(event.target.value)} placeholder="Campaign, test purpose, or save name" /></label><button onClick={() => { setCreating(false); setName(""); setNotes(""); }}>Cancel</button><button className="primary" disabled={!name.trim()} onClick={() => { onCreate(name, notes); setCreating(false); setName(""); setNotes(""); }}>Create</button></section>}
    <div className="profile-layout">
      <aside className="profile-list" aria-label="Profiles">{profiles.map(profile => <button key={profile.id} className={selected?.id === profile.id ? "selected" : ""} onClick={() => onSelect(profile.id)}><span>{profile.name}{profile.active && <small><Check size={13} />Active</small>}</span><b>{profile.enabledMods}/{profile.totalMods}</b></button>)}</aside>
      <div className="profile-workspace">
        {!draft ? <section className="empty-state"><ShieldCheck size={34} /><h2>Select a profile</h2><p>Choose a profile or create one to manage a mod set.</p></section> : <>
          <section className="profile-heading"><div><input aria-label="Profile name" value={draft.name} onChange={event => setDraft({ ...draft, name: event.target.value })} /><textarea aria-label="Profile notes" value={draft.notes} onChange={event => setDraft({ ...draft, notes: event.target.value })} placeholder="Campaign or test notes" /></div><div className="profile-actions"><button onClick={() => onExport(draft.id)}><Download size={16} />Export</button>{!draft.active && <button className="danger-text" onClick={() => onDelete(draft)}><Trash2 size={16} />Delete</button>}<button onClick={() => onSave(draft)}><Save size={16} />Save details</button>{!draft.active && <button className="primary" disabled={busy} onClick={() => onPreview(draft.id)}>Review switch</button>}</div></section>
          <section className="profile-mods"><div className="table-heading"><span>Enabled</span><span>Mod</span><span>Type</span><span>Priority</span></div>{draft.mods.map(mod => <div className="profile-mod-row" key={mod.modId}><label className="switch"><input type="checkbox" checked={mod.enabled} onChange={event => { onSetMod(mod.modId, event.target.checked, mod.loadPriority); setDraft({ ...draft, mods: draft.mods.map(item => item.modId === mod.modId ? { ...item, enabled: event.target.checked } : item) }); }} /><span /><em>{mod.enabled ? "On" : "Off"}</em></label><b>{mod.name}</b><span className="type-chip">{mod.modType}</span><input aria-label={`${mod.name} priority`} type="number" value={mod.loadPriority ?? ""} onChange={event => onSetMod(mod.modId, mod.enabled, event.target.value ? Number(event.target.value) : null)} /></div>)}</section>
          {preview?.profileId === draft.id && <section className={`switch-review ${preview.blocked ? "blocked" : ""}`}><div><h2>{preview.blocked ? "Profile switch blocked" : `${preview.changes.length} changes ready`}</h2><p>{preview.blocked ? preview.reasons.join(" ") : "The current deployment will be snapshotted before these changes are applied."}</p></div><ul>{preview.changes.slice(0, 8).map(change => <li key={change.modId}><b>{change.name}</b><span>{change.fromEnabled ? "Enabled" : "Disabled"} → {change.toEnabled ? "Enabled" : "Disabled"}</span></li>)}</ul><button className="primary" disabled={preview.blocked || busy} onClick={() => onActivate(draft.id)}>Apply profile</button></section>}
        </>}
      </div>
    </div>
    <section className="checkpoint-bar"><label>Checkpoint name<input value={checkpoint} onChange={event => setCheckpoint(event.target.value)} /></label><button onClick={() => onSnapshot(checkpoint, false)}>Create checkpoint</button><button onClick={() => onSnapshot(checkpoint, true)}>Mark last known good</button><span>{snapshots.length} retained checkpoints</span></section>
    {!!snapshots.length && <section className="checkpoint-list" aria-label="Retained checkpoints">{snapshots.slice(0, 12).map(snapshot => <article key={snapshot.id}><div><b>{snapshot.label}{snapshot.lastKnownGood && <span>Last known good</span>}</b><small>{snapshot.kind} · {new Date(snapshot.createdAt).toLocaleString()}</small></div><button disabled={busy} onClick={() => onRestoreSnapshot(snapshot)}>Restore as profile</button></article>)}</section>}
  </div>;
}
