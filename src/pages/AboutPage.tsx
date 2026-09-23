import { ExternalLink, Github, RefreshCw, ShieldCheck } from "lucide-react";
import type { UpdateInfo } from "../types";

export function AboutPage({ projectUrl, nexusUrl, onOpenLink, update, checking, error, onCheckUpdates }: { projectUrl: string; nexusUrl: string; onOpenLink: (url: string) => void; update: UpdateInfo | null; checking: boolean; error: string | null; onCheckUpdates: () => void }) {
  const releasesConfigured = projectUrl.length > 0;
  return <div className="page about-page">
    <header className="page-header"><div><p className="eyebrow">ZERO MOD MANAGER</p><h1>About</h1><p className="muted">A focused, open-source mod manager for Star Wars: Zero Company.</p></div></header>
    <section className="about-grid">
      <article className="panel about-intro"><ShieldCheck aria-hidden size={32} /><div><h2>Safer mod management</h2><p>Install, organize and check your Zero Company mods in one place.</p></div></article>
      <article className="panel version-card"><p className="eyebrow">INSTALLED VERSION</p><strong>v{__APP_VERSION__}</strong><p className="muted">{releasesConfigured ? "Updates are checked when the app opens." : "Update checks are disabled until the continuation repository is published."}</p><button className="primary" disabled={checking || !releasesConfigured} onClick={onCheckUpdates}><RefreshCw className={checking ? "spin" : ""} aria-hidden size={17} />{checking ? "Checking GitHub…" : "Check again"}</button>
        <div className="update-result" aria-live="polite">
          {!checking && !error && update && (update.releaseAvailable === false ? <p className="muted">No public release is available yet.</p> : update.updateAvailable
            ? <><b className="warn-text">Version {update.latestVersion} is available.</b><button className="link-button" onClick={() => onOpenLink(update.releaseUrl)}>Open release page <ExternalLink aria-hidden size={14} /></button>{nexusUrl && <button className="link-button" onClick={() => onOpenLink(nexusUrl)}>View Nexus Mods page <ExternalLink aria-hidden size={14} /></button>}</>
            : <b className="success-text">No newer release found. Latest checked GitHub release: v{update.latestVersion}.</b>)}
          {!checking && !error && !update && releasesConfigured && <p className="muted">Update status not checked.</p>}
          {error && <><p className="muted">Updates could not be checked. You can retry or open All releases.</p><details><summary>Technical details</summary><small className="status-error">{error}</small></details></>}
        </div>
      </article>
      <article className="panel about-project"><h2>Independent continuation</h2><p>Zero Mod Manager is an independent continuation based on ZCOM Mod Manager 0.6.5. It is not an official arctco release and is not affiliated with or endorsed by Lucasfilm Games, Electronic Arts, Bit Reactor, or Respawn Entertainment.</p><div className="settings-actions">{projectUrl && <><button onClick={() => onOpenLink(projectUrl)}><Github aria-hidden size={17} />View continuation source</button><button onClick={() => onOpenLink(`${projectUrl}/releases`)}><ExternalLink aria-hidden size={17} />All releases</button></>}{nexusUrl && <button onClick={() => onOpenLink(nexusUrl)}><ExternalLink aria-hidden size={17} />Nexus Mods page</button>}<button onClick={() => onOpenLink("https://github.com/arctco/zcom-mod-manager")}><Github aria-hidden size={17} />Original ZCOM source</button></div><small>GNU GPLv3-only · Continuation © 2026 Zero Mod Manager contributors · Original work © 2026 Victor Hugo (arctco)</small></article>
    </section>
  </div>;
}
