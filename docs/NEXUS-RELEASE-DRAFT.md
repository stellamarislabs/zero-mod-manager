# Zero Mod Manager — 0.7.0-rc.7

**Release candidate / opt-in testing. Windows x64. Not a Stable release.**

An independent continuation of ZCOM Mod Manager, originally created by
Victor Hugo (arctco). New name and branding; not an official arctco release.
Not affiliated with or endorsed by Electronic Arts, Lucasfilm Games, Disney,
Bit Reactor, or Respawn Entertainment.

## What it does

- Installs supported local mod archives and manages enabled state.
- Provides profiles, deployment previews, snapshots, and recovery workflows.
- Offers modded, temporary vanilla, and troubleshooting launch modes.
- Shows local compatibility findings and produces previewable local support bundles.
- Uses a compact, consistent Command Center and persistent launch controls.

Download mods in your browser and open the archive in Install. There is no Nexus
login, API integration, Discover browser, automatic mod download, or mod update
query. External Nexus page links remain. The manager's own application update
check is separate. Do not let Vortex and this manager deploy the same files at
the same time; disable one manager's deployment before handing control to another.

## Installation

Choose **one**: the Windows installer or the portable ZIP. Extract the complete
ZIP into a writable folder before running Zero Mod Manager.exe. No localhost
server or development tools are needed. See PORTABLE-DATA.md for optional
self-contained storage. Keep backups of an existing managed library before migration.

ZIP extraction is built in. 7z/RAR archives require a compatible host extractor.
UE4SS-based mods need their appropriate runtime. IoStore companion files are checked,
but container contents and asset-level overlaps are not inspected. No separate container tool is needed.

## Testing and limitations

Automated checks and startup smoke tests are not proof that every mod works.
Only list game/launcher/mod combinations with completed evidence from QA-MATRIX.md
at publication time. Do not advertise Linux, Steam Deck, or EA App as qualified
until their tests pass. The current public compatibility catalog is not provisioned.
See KNOWN_LIMITATIONS.md for ordering, replacement mods, config, and bundle-update limits.

Save files are not managed. Use the main menu or a disposable test save for
troubleshooting. Do not install or change mods while the game is running.

## Download safety

These local RC packages are unsigned. Windows may show an unknown-publisher or
SmartScreen warning. Follow RELEASE-SECURITY.md and compare SHA-256 checksums;
do not disable Windows security or ignore malware detections.

## Source, credits, and reports

GPL-3.0-only. Preserve the original credits, LICENSE and THIRD_PARTY_NOTICES.md.
Publish the version-matched source ZIP alongside the binaries and checksum list;
do not link only to the upstream source or a moving development branch.
The corresponding-source principle is explained in the [GNU GPL FAQ](https://www.gnu.org/licenses/gpl-faq.html.en#SourceAndBinaryOnDifferentSites).

[Source repository](https://github.com/stellamarislabs/zero-mod-manager) ·
[Report an issue](https://github.com/stellamarislabs/zero-mod-manager/issues)

Report the application version, launcher, game build, mod filenames/versions,
reproduction steps, and expected/actual outcome. Preview and redact a local
support bundle before sharing. Never upload saves, credentials, or game assets.

## Maintainer-only publication checklist (remove this section from Nexus copy)

- Add the verified continuation repository, issue tracker and exact source download URLs.
- Add actual successful game-test combinations; retain remaining limitations.
- Upload only the artifacts from the matching prepared release set, including source.
- Keep this file's RC/unsigned warnings visible on the published page.
- Obtain the owner's final publication approval; this draft is not publication.
