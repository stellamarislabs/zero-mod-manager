# Changelog

## 0.7.0-rc.7 — Changed-file confirmation and safe retry

- Fixed update and uninstall flows when a managed file changed and the transaction rolled back. The app now offers an explicit decision showing the affected path.
- The decision names whether the changed file will be replaced or deleted. Cancelling keeps the installed mod and its files.
- Removed the unsupported suggestion that every changed file is a mod-generated settings file; native DLLs can change too.
- Added application and transaction regression tests for refusal, confirmation, retry, and complete removal.

## 0.7.0-rc.6 — Reliable replacements and clearer Library

- Standalone replacements no longer invoke complete-bundle updates. Only explicit bundle ownership enables that action; independent overlapping entries stay separate.
- Deleted Library records are removed from pending replacement/conflict claims and obsolete package selections.
- Native confirmations are asynchronous, awaited and fail closed. Confirm/save capabilities are explicitly enabled; rename uses an in-app dialog.
- Single-entry upgrades preserve saved profile references, disabled state and Library visibility.
- Deployment blocks cancelling or replacing its staged input. Subscription failures and toast cleanup are handled.
- Library shows recorded installed versions, mixed bundle versions and missing-version labels. Explicit dotted archive-version suffixes are recognized.
- Compact maintenance actions, contained long titles, clearer action spacing and accessible enable labels preserve the familiar table layout.

## 0.7.0-rc.5 — Evidence-based status and consistent Windows branding

- Optional load-order filename normalization no longer looks like an unresolved conflict or a pending user edit. Missing checks are not displayed as zero problems.
- Runtime file detection, historical log presence, saved profile selections, and actual deployment are presented as different evidence. No log-file-existence check claims in-game success.
- Failed refreshes invalidate stale status. Unreadable core data pauses actions, with a retry path. Tool, build, catalog and support results remain unknown when unavailable.
- Profile totals group only explicit bundle identities. Saved profile changes can be reviewed even for the active profile; editing does not silently deploy.
- Config changes cannot be applied using an outdated diff, and invalid runtime packages cannot be installed.
- Window, taskbar, installer and existing application shortcuts use the same circular cyan logo. Installer updates preserve shortcut arguments and pin preferences.
- Profile preview and applied order now agree; current-deployment snapshots preserve pending profile drafts. Profile switches enroll in the persistent rollback journal.
- Session/support records use actual deployed selections, not requested runtime versions or profile drafts. Temporary launch guard prevents restoring a different unapplied saved state.
- Vanilla/isolation wording explicitly states that runtime and unmanaged files remain; a repeated baseline cannot identify a runtime fault.
- Added UI/backend regressions, SVG-to-ICO checks, actual Windows taskbar-property checks and isolated shortcut-upgrade fixtures.

## 0.7.0-rc.4 — Independent mod identity and factory reset

- Removed Merge/Group from existing-mod adoption. Every detected mod remains independent, even when all entries are selected. The backend rejects attempts to merge scan candidates.
- Library identity no longer guesses bundles from a shared archive path. Explicitly installed bundles retain their Components drawer. Bundle installation requires one inspection and a reviewed grouping confirmation.
- Organize mods can separate wrongly grouped entries without moving game files. Old flattened PAK/IoStore adoption records recover as separate mods, preserving applicable profile references and checkpoints.
- Settings > Factory reset requires an acknowledgement and typed RESET. On next launch, app-owned data moves into recovery folders; game mods, saves and external custom library copies remain untouched. The manager closes before resetting, and interrupted resets can resume.
- Startup creates the WebView only after reset and database initialization, so browser cache handles cannot block reset. Pending installations, package recovery and temporary launch states block reset until resolved.

## 0.7.0-rc.3 — Existing mod adoption and bundle recovery

- Group as one mod preserves independent component records, names, enabled states and load order. Mixed supported mod families can share a Components drawer.
- Library > Organize mods groups already-managed entries without changing game files. Runtime-owned UE4SS components are excluded.
- Explicit recovery for old Adopt/Merge PAK/IoStore records verifies ownership and checksums, restores component records and preserves profiles and compatible in-app checkpoints. Ambiguous records are rejected. Previously exported profile locks must be exported again.
- Grouped adoption and legacy recovery use the persistent rollback journal. Scan results are rejected after changing game folders or during temporary launch recovery.
- Discover full Unreal plugin folders in SWZeroCompany/Mods. UE4SS enabled.txt is recognized even when mods.txt disables or omits the mod, matching loader behavior.
- Bundle titles survive component and whole-package updates; single-component bundles still expose their drawer.


## 0.7.0-rc.2 — Package recovery and release hardening

- Complete package updates now show replacement confirmation and retire components omitted from the new archive. An explicit target selector supports renamed packages.
- Package installation, updates and removal use a persistent file/SQLite rollback journal. Managed single-mod installs, replacement, enable/disable and removal also use it. Startup restores an interrupted operation before normal database initialization.
- Recovery copies are checksum-validated before restoration; interrupted files are retained locally for investigation. No game saves are collected.
- Existing profile memberships and disabled state survive matched component upgrades.
- GitHub update checks distinguish no public release from network errors, compare RC/stable versions correctly and let RC builds see published prereleases.
- Local Clippy, formatting and production dependency checks are now part of the handoff.

### Historical implementation notes (superseded where noted above)

### Package-level library actions

- Bundle rows keep the Library action layout, with package-wide enable/disable, verification, visibility and one-confirmation removal. Components remain available in a right-side drawer.
- Removal preflights every component for changed active files before deleting any. RC2 adds persistent rollback for complete-package removal.
- Individual component upgrades preserve package identity. RC2 additionally supports confirmed complete-package replacement and retirement of omitted components.

### Custom executable administrator approval

- Custom game launches request the standard Windows UAC prompt only when Windows requires elevation (error 740). Only the selected executable is elevated; the manager stays unchanged.
- Declining approval cancels the launch without retries and returns through the existing temporary-profile recovery flow.
- Other launch errors do not request administrator access.

### Container-tool removal and bundled library

- Removed external container verification, its tool settings, install/adoption prompts and health warnings. Archive safety, companion-file checks and managed-file checksum protection remain.
- Library groups components by bundle identity or the exact original archive; bundle drawers expose individual component controls. Mod counts now count groups, not components.
- Asset-level conflicts inside newly installed containers are no longer inspected.

### Installation review noise reduction

- Documentation, checksum lists and license text excluded from deployment are now neutral advanced details, not warnings.
- Unknown payloads, native code, missing runtime, failed validation and compatibility warnings remain visible.
- Moved non-actionable container/package metadata into advanced details.

### Library clarity and compact layouts

- Separated cleanup controls from navigation and removed negative tab spacing.
- Added a default-on, view-only filter for known bundled UE4SS component folders.
- Clarified mod totals, reduced repeated technical metrics, and collapsed secondary overview details.
- Adapted the library table for narrow windows; secondary file actions remain available in mod details.
- Shortened update error messages while retaining diagnostic detail in About.

### Library and launch safety polish

- Added selected-mod removal and temporary installation cleanup with explicit, destructive confirmation and acknowledgement.
- Added a short Continue/Cancel explanation before Troubleshoot launches.
- Made profile deletion discoverable; active and launch-recovery profiles cannot be deleted.
- Lightened shared blue surfaces across pages while preserving the fixed navigation and launch bar.


### Troubleshoot launch hotfix

- Hide the Windows tasklist console during game-process polling.
- Reject missing/empty executables before requesting launch or changing a temporary profile.
- Preserve the pending profile recovery record if restoration fails.

### Local-first workflow update

- Removed Discover, Nexus API/account integration, mod update queries and nxm download handling. Manual archives and external Nexus links remain.
- Made unavailable retoc verification optional with explicit consent; failed verification stays blocked. Unverified containers remain visible in Library and Health.
- Reduced repetitive screen introductions and moved technical evidence into expandable details.
- Retired this application's stored Nexus credentials; no Vortex credentials are touched.

All notable changes are documented here.

## 0.7.0-rc.1 — Operational Readiness

### Added

- Production Windows packaging now emits the installer and portable ZIP from
  Tauri's release pipeline. A release smoke test launches an isolated copy and
  requires the embedded frontend to report ready without a localhost server.
- Portable builds use normal per-user data by default and support an explicit
  self-contained mode through `portable-data.flag` beside the executable.
- Archive inspection now distinguishes mod bundles, runtimes, external tools,
  and external installers. Standalone/setup executables are never run and no
  longer cause nested sample PAK files to be offered as mods.
- Native payload discovery now includes ASI files plus `dsound`, `winhttp`, and
  common XInput proxy DLLs.
- Fresh multi-component downloads now install through one backend transaction.
  Components share a persistent bundle identity; a failure in deployment,
  metadata, ordering, or profile capture removes the complete new bundle and
  leaves its staged archive available for retry.

- A Command Center with four explicit readiness states: Ready, Warning,
  Blocked, and Unverified. Launch decisions now include the selected profile,
  launcher, game build, compatibility findings, and UE4SS runtime evidence.
- Named profiles, previewable atomic profile switches, portable Profile Lock
  files, automatic snapshots, and user-created Last Known Good checkpoints.
- Modded, temporary vanilla, and controlled troubleshoot launch modes. The
  previous profile is restored after temporary sessions and after an
  interrupted manager restart; session outcomes are labelled by the user rather
  than inferred from a process exit.
- A provenance-aware compatibility engine combining deployed-file ownership,
  IoStore package overlap, manifest relationships, and signed community rules.
  Ed25519 verification, expiry, schema checks, rollback protection, and a last
  verified local cache are enforced before catalog data is trusted.
- Config Workbench support for validated INI, JSON, and TOML edits with preview,
  backups, optimistic concurrency checks, and rollback. Lua is deliberately
  read-only.
- Privacy-previewed local support bundles containing operational, profile,
  compatibility, diagnostic, and bounded log evidence. Credentials, personal
  paths, and save contents are excluded or redacted; bundles are never uploaded.
- Steam, EA App metadata, and manual launcher profiles; activity history;
  local-first manual archive installation; and public Manifest v2,
  Profile Lock v1, and Compatibility Catalog v1 contracts.

### Security

- All deployment mutations are refused while the game is running.
- Archive staging now enforces traversal/link rejection, Windows device-name,
  path depth, component length, file count, expanded-size, single-file size,
  and compression-ratio limits before deployment.
- Native payloads remain inert archive data: installers and scripts from a mod
  are never executed.

### Changed

- The product is now branded Zero Mod Manager. ZCOM Mod Manager remains visible
  only as the GPL-3.0 upstream project and migration source.
- Navigation is organized around Command Center, Library, Discover, Install,
  Profiles, Health, Settings, and About.
- A file merely being in the expected folder is no longer sufficient evidence
  that UE4SS loaded. Runtime health requires its game-session log and reports
  proxy/VC runtime/Proton problems separately.
- The application no longer presents an unverified Unreal Engine minor version
  as fact. Runtime and build evidence will supply an exact fingerprint when it
  is available.

### Release qualification

- RC status does not claim universal Nexus compatibility. Stable promotion
  still requires the real-game/platform matrix in `docs/QA-MATRIX.md`, zero open
  P0/P1 issues, release signatures/checksums, and publication of matching GPL
  source.

## 0.6.5

### Fixed

- A mod that writes its own settings or data next to its deployed files can now
  be updated, disabled, and removed. Such a mod changes a managed file every
  time the game runs, and the guard that protects a file the user edited by
  hand refused all three actions on that evidence alone, leaving an entry in the
  library that nothing worked on. The refusal now names the file, explains that
  a mod may have written it, and offers to go ahead: the changed file is
  replaced by the new version on an update and removed with the payload on a
  disable or an uninstall. Declining still keeps the file and changes nothing.
- 7-Zip is found where it is actually installed. Its Windows installer does not
  add itself to `PATH`, and only `PATH` was searched, so a machine that plainly
  had 7-Zip was told that "archive support requires the 7z command-line tool".
  The standard 7-Zip and NanaZip folders and the directory 7-Zip's installer
  registered are now searched too, `7za` and `7zr` are accepted, and
  **Settings → Archive tool** takes a path to any other build. The failure
  message says where to set it.
- A failure now stays on screen until it is dismissed, with buttons to copy it
  and to open the logs. Error messages disappeared after four and a half
  seconds, which was not long enough to read an install failure, let alone
  quote it. Every failure the interface reports is also appended to
  `application.jsonl`, so one that has already been closed can still be found.
- UE4SS is no longer called healthy on the strength of its files alone. That
  verdict was reached from file presence, so a user whose game never loaded the
  runtime was told nothing was wrong. Diagnostics now reports whether UE4SS has
  written its log — the only evidence in the game folder that it ever loaded —
  and offers to open it. It also names a second proxy DLL beside the game
  executable, since UE4SS loads only as `dwmapi.dll` and a copy renamed to
  `version.dll` or similar lets the game start while the runtime never loads,
  and it checks for the Visual C++ 2015-2022 x64 runtime UE4SS links against,
  whose absence hangs the game at start-up or leaves UE4SS silently unloaded.
- Child processes no longer flash a console window over the game or the
  manager on Windows.

## 0.6.2

### Added

- Newly installed FOMOD mods now offer **Reconfigure FOMOD** in the library. The
  complete installer source and its last answers are retained with the managed
  mod, so the wizard can be reopened with those choices preselected even when
  the original download has been moved or deleted. The result returns to the
  normal validation and in-place replacement review before deployment.

## 0.6.1

### Fixed

- Option images and the installer's header image now appear in released builds.
  They are handed to the interface as `data:` URLs, which the application's
  content security policy did not list as an image source, so a released 0.6.0
  showed every guided installer without its pictures while option names,
  descriptions, group rules, conditional steps, and the install itself worked
  normally. `img-src` now allows `data:`, which permits images only and grants
  no script execution.
- The policy is injected into the frontend the application embeds, and a
  development run loads the frontend from the dev server instead, so it is not
  applied there. That is why the images were present while developing and
  absent in the released package. A release build is the only place this class
  of difference can be seen; `devCsp` does not reproduce it while an external
  `devUrl` is configured.

## 0.6.0

### Added

- Mods that ship a FOMOD installer script are now installed by answering the
  author's own questions instead of by picking through the folders the archive
  contains. Reading a download looks for `fomod/ModuleConfig.xml`, and when one
  is present the installer presents its steps: option groups with the author's
  descriptions and images, the recommended answer already selected, and the
  choice illustrated beside the list. Required options cannot be unticked, and
  an option an earlier answer rules out cannot be chosen.
- Answers carry through the script the way its author wrote it. Selecting an
  option sets the flags the script declares, later steps appear or are skipped
  according to them, an option's own availability can depend on an earlier
  answer, and the final set of flags decides both the per-option files and the
  script's conditional installs. Going back takes an answer away along with
  everything it decided, and returns the step exactly as it was left.
- Only the files the answers selected are installed. They are written into a
  sandbox of their own and then read exactly like an ordinary download, so a
  scripted install arrives at the same review screen with the same container
  verification, conflict detection, compatibility check, and naming as any
  other mod. Sixteen mutually exclusive variants in one archive therefore
  become one mod entry rather than sixteen options to choose between by hand.
- A scripted install records the download it came from, so update checking,
  Nexus matching, and in-place replacement keep working for it.
- `fomod/info.xml` supplies the mod's published title, author, version, and
  description when the selected files carry no `zcom-mod.json` of their own,
  replacing the name derived from the download's file name.

### Notes

- Installer scripts are read as untrusted archive data. Every source and
  destination path is checked against the same rules the extractor applies,
  once when the script is parsed and again immediately before each file is
  written, and a path that would leave the package is refused. Group rules
  such as "choose exactly one" are enforced where the files are decided, not
  only in the interface. No part of a script is executed.
- Conditions on other game plugins or on tool and game versions have no meaning
  for Zero Company, which has no such plugins. They are read, reported as a
  visible warning, and treated as unmet, so an option gated behind one is shown
  as unavailable rather than silently installed.
- An archive whose script cannot be read is still installed the previous way,
  with its folders offered as labeled options.

## 0.5.3

### Fixed

- Nexus update checks now follow the exact file variant that was installed
  instead of comparing it with one page-wide newest file. Mods offering several
  mutually exclusive main or optional files therefore neither report another
  choice as an update nor miss an update published for an optional variant.
  Cached results are stored per installed Nexus file and existing false results
  are replaced on the next check.
- UE4SS mods are now inserted and ordered before the runtime's `Keybinds` entry.
  Existing managed entries found below it are moved above it on the next toggle
  or load-order apply, while the runtime comment attached to `Keybinds` remains
  attached.
- Dragging a mod into the installer now starts native inspection before any
  previous preview is discarded. A failed or transient drop therefore keeps
  the usable preview on screen, and successful inspection swaps previews only
  after the new source is staged. Windows shell drops receive a short bounded
  retry, and drops from inside 7-Zip or WinRAR explain that the archive itself
  should be dropped or extracted first. Received paths and failures are written
  to the sanitized application log for diagnosis.
- Launching Steam from the Linux AppImage no longer passes the mounted image's
  libraries, Python runtime, or GTK/GIO/Qt plugin paths into Steam. The manager
  opens the same `steam://run/2075800` address through the host `xdg-open` with
  a rebuilt environment, so Steam still applies the game's configured launch
  options without inheriting `/tmp/.mount_*` dependencies.
- Reduced the WebView2 composition work implicated in Windows app windows turning
  dark, stale, or partially displaced after a mod action. Page and toast
  transform animations, transformed switches, and backdrop blur are gone, and
  the native webview has an explicit dark background. The application shell is
  now pinned to the WebView edges instead of relying on `100vh`, fixing the
  reported state where the sidebar and mod list stopped partway down the window
  even though a toast still rendered at the real bottom. A post-render guard
  repairs and logs any remaining height mismatch. JavaScript render errors,
  uncaught errors, the WebView user agent, viewport details, and repaired layout
  dimensions are persisted to `application.jsonl` instead of existing only in
  DevTools.
- Managed payloads and backups are no longer forced into roaming AppData. New
  installations use Local AppData, an existing populated 0.5.0 library remains
  recognized in place, and Settings can move the library to any empty folder.
  Moves work across drives, hash-verify every copied file before switching,
  serialize against mod operations, and keep the original until the new path is
  saved successfully.

### Notes

- Steam launching still uses `steam://run/2075800`, so Steam applies the launch
  options configured for the game. The Linux AppImage now opens that URI with
  a sanitized environment. The separate Windows Custom Launch loop report did
  not identify a manager-side argument or executable fault, so 0.5.3 does not
  replace the Windows launch mechanism without a reproducible manager-only case.

## 0.5.0

### Fixed

- Uninstalling a packaged mod no longer takes the whole interface down. The
  load order is drafted in local state, and the refresh that follows a removal
  arrived while the draft still named the removed mod; rendering that row looked
  the name up in a list it had just left, threw during render, and React
  unmounted everything — leaving a dark, unresponsive window that only a restart
  fixed. Both order drafts are now reconciled against the live lists before
  anything reads them. Reported against 0.4.0 and 0.4.1 with Squad Six, whose
  Core and Runtime components are removed one after the other.
- Whatever else goes wrong, the window no longer goes blank: an interface error
  now shows what failed, with **Reload the interface** and **Try to continue**,
  instead of unmounting into an empty window.
- Removing or disabling a UE4SS mod now takes its folders with it. The payload
  was deleted but `ue4ss/Mods/<Name>/Scripts` was left standing, so the runtime
  and the user both still saw a mod that was no longer installed and the tree
  had to be cleared by hand. Only empty directories below the deployment base
  are removed: a folder holding anything else, and the shared `Mods` base
  itself, are left alone, and game-folder mods are not pruned at all.

- Installed mods are now checked for newer files on Nexus Mods. A download
  through the handoff records which mod and file it came from, installation
  attaches that to the mod, and **Check for updates** on the Mods page asks
  Nexus what each of them now offers.
- A library that predates any of this is not left out. A check first offers the
  MD5 of every unmatched mod's archive to Nexus, which recognises the file it
  was uploaded as and identifies the mod and file exactly. An archive Nexus does
  not know is remembered as such, so an automatic check does not ask again.
- A mod whose archive is gone, or that was adopted from disk and never had one,
  can be pointed at its Nexus page by hand from **More details**. The file
  recorded as installed is the one carrying the installed version, so linking
  never invents an update.
- A mod linked to a Nexus page can be opened there from its row in the library
  and from **More details**. A mod with no page shows no button rather than a
  dead one.
- Any mod can be taken out of update checking from **More details**, whether or
  not it is linked to a Nexus page. A mod that never came from Nexus was
  otherwise offered to the archive lookup on every check the user asked for,
  which spends a request per check on a mod Nexus will never recognise. An
  excluded mod is left out of both the checks and the lookup, so unlinking now
  sticks rather than being matched and linked again by the next check, and
  **Check this mod again** puts it back.
- An update is the newest file Nexus still offers in the `MAIN` or `UPDATE`
  categories. Superseded, archived, and deleted files are ignored, an optional
  extra is never mistaken for an upgrade, and file ids decide what is newer
  because Nexus issues them in upload order.
- A premium account can fetch the update in place; the file then goes through
  the same inspection and replacement path as a website handoff. A free account
  is sent to the mod's files tab, because only the website can mint the key its
  download link needs.
- Added an opt-in **Check installed mods for updates when the application
  starts** setting, off by default. It is saved the moment it is set, like the
  key and the handler beside it. The stored result stands for six hours, so
  reopening the manager does not spend the Nexus rate limit, and a check cut
  short by a rate limit is retried rather than waiting the interval out.
- Settings remembers which Nexus account a stored key belongs to, so the
  connection is described again after a restart without asking Nexus on launch.
- A download started from the website now has a screen of its own: the file
  name, a progress bar, and the transferred size appear as soon as the link is
  resolved, instead of an inspection spinner that sat there until a payload
  appeared. Progress is also reported at most ten times a second rather than
  once per chunk, which was flooding the interface it was meant to keep alive.
- Mods can be hidden from the library list. A hidden mod stays installed,
  deployed, and ordered — this is for the UE4SS runtime's own bundled mods,
  which existing-mod discovery adopts and which then crowd the list. The
  **Hidden only** filter brings them back, and the count line says how many
  are out of view.
- The Mods, Settings, and About pages now use the whole window width instead of
  stopping at a fixed centred column.
- A manager update now offers **Get it on Nexus Mods** beside the GitHub release
  link, and the About page links the manager's Nexus page whether or not an
  update is waiting. A release is published in both places, so the notice no
  longer assumes where this copy came from.
- A `zcom-mod.json` manifest is no longer reported as a file that is "not part
  of a recognized mod layout". The manager reads that file; saying it did not
  told authors their own metadata was a problem.

## 0.4.1

- Multi-component archives can carry a separate `zcom-mod.json` beside each
  component. The install review labels and checks each payload with its own
  metadata and offers one **Install all components** action for bundles that
  contain no mutually exclusive options.
- A bundle upgrade matches and replaces each previously installed component,
  including components that originally came from separate downloads. Failed
  installations keep their validated preview available for retry.

## 0.4.0

- Archives containing packaged variants in separate folders now present each
  folder as a labeled installation option. This fixes downloads such as
  Blackmarket Discounts, whose four strengths were previously flattened into
  one mod and would all be installed together.
- Added an optional custom game executable or launcher in Settings. Home uses
  it instead of the Steam URI when configured, and **Use Steam** clears the
  override.
- Added existing-mod discovery and adoption for PAK/IoStore containers, UE4SS
  folders, and additive LogicMods. A first-connection prompt and a permanent
  Mods-page action open a review wizard. Adoption copies payloads into the
  managed library without moving, renaming, or rewriting live files, and each
  selected group succeeds or fails independently.
- Migration preserves UE4SS enabled state and order from `mods.txt`, supports
  merging packaged container families, excludes already managed destinations,
  blocks incomplete or unverifiable IoStore sets, and reports replacement-style
  injectors that cannot be adopted without an original-file backup.

## 0.3.0

- Added UE4SS start-order management. The Load order tab lists UE4SS mods in
  start order and writes the managed block of `mods.txt` back. The runtime uses
  two passes — every DLL mod starts as UE4SS initializes, the Lua mods only once
  the scripting runtime exists — so the editor sets order within each pass and
  normalizes any request to interleave them. Comments, blank lines, and the
  runtime's own entries keep their position, and mods installed before this
  release keep the order the file already has.
- Added in-place upgrades. A newer build of an installed mod is recognized at
  inspection and offered as a replacement instead of a deployment conflict. The
  previous version's files are moved aside rather than deleted, so a failure
  anywhere in the new installation puts them back and leaves the old version
  installed. The replacement keeps its predecessor's position in the load order,
  and inherits the original files a game-folder mod displaced.
- Every UE4SS mod folder in an archive now installs as its own library entry,
  so mods that shipped together can be enabled, ordered, and removed separately.
- UE4SS DLL mods install. A mod folder is recognized by `Scripts/main.lua` or
  `dlls/*.dll`, so native mods such as Unique Talents for All and ZCUnlocked
  are no longer rejected as unrecognized payloads.
- Every UE4SS mod folder inside one archive is installed, instead of only the
  first, each with its own `mods.txt` line.
- Archives written on Windows with `\` separators extract correctly on Linux.
  Their entries previously became single files with backslashes in the name, so
  the mod layout never appeared and detection failed.
- Mods are named after the download rather than after the first file inside it.
  Nexus publishing metadata (mod id, version, upload stamp, and the random
  suffix) is stripped, and the version it carries is kept.
- Mod names can be edited before installing and renamed afterwards from the
  library. Renaming changes the label only; deployed files keep their names.
- Added game-folder mods: ReShade and other loader shims, replacement movies
  and audio, and `LogicMods` blueprint packs. A file the mod replaces is kept
  in the managed library and restored when the mod is disabled or removed.
- A UE4SS runtime package dropped on the installer is now recognized as the
  runtime and offered as a runtime install, instead of being taken apart into
  the mods it ships.
- An archive holding several mods is previewed as several mods, each named and
  installed on its own.
- Files an archive contains that are not part of a recognized layout are listed
  before installing, and native code inside a mod is called out as such rather
  than reported as ignored.
- The UE4SS mod count on Home and in diagnostics counts DLL mods too.
- The load-order tab now says where the mods it does not list are ordered
  instead, so a library of UE4SS mods no longer looks like the editor lost them.
- Fixed doubled event subscriptions. The unsubscribe handle arrives after the
  effect is cleaned up, so every listener was registered twice: one dropped
  archive was inspected twice and one `nxm://` link would download twice. A
  superseded inspection now also releases its own extraction sandbox, and stale
  sandboxes are cleared at startup.
- Library row actions are laid out as two rows of three; six buttons never fit
  the single row they were given.
- An upgrade of a mod installed on a different drive from the managed library
  no longer fails before it starts. Moving the previous version's files aside
  used a rename, which cannot cross a filesystem boundary, and the game is
  regularly on a second drive.
- Payload paths shown in the install preview use one separator on every
  platform instead of mixing both on Windows.

## 0.2.0

- Fixed Home and Settings folder actions by moving trusted path opening behind
  validated native commands and surfacing failures in the interface.
- Added a Home-page game launch button that opens Zero Company through Steam,
  preserving the user’s Steam and Proton launch configuration.
- GitHub release checks now run once at startup. When a newer release exists,
  a compact update icon appears beside About; the manual retry remains there.
- Added a conflict-aware load-order editor for runtime-verified IoStore
  triplets. PAK-only mods and pure IoStore pairs remain visible but gated.
- Added deterministic numeric `_P` deployment ranks, with the highest row
  winning known package overlaps.
- Added active and potential conflict states plus winner previews that keep raw
  package paths private.
- Added review-before-apply, SHA-256 preflight checks, rollback-safe renames,
  and startup recovery for interrupted load-order operations.
- Newly installed runtime-supported packaged mods default to the highest
  priority and normalize the existing managed order in the same confirmed
  installation flow.
- Archives containing overlapping IoStore container variants are now rejected
  with guidance to install only one variant.
- Added a real SQLite v2 migration while preserving existing mod ownership and
  enabled states.

## 0.1.5

- Restored the original 0.1.0 blue, steel, and gold interface palette while
  preserving the newer application layout and features.
- Added an About page with the installed version, project information, license,
  source links, and release links.
- Added an on-demand GitHub update check that compares the installed version
  with the latest published release and links directly to available updates.
- Corrected repository, release API, documentation, and external-link targets
  to `arctco/zcom-mod-manager`.

## 0.1.4

- Added a portable Windows zip to the release artifacts. Extract it anywhere and
  run `ZCOM Mod Manager.exe`. The bundled `retoc.exe` ships beside it and is
  detected automatically, so IoStore verification works without configuration.
  The portable build expects the Microsoft Edge WebView2 runtime to already be
  installed; the NSIS installer still fetches it when missing.
- Fixed the release step that packages that zip. It looked for the executable
  under the product name, but `tauri build` leaves the cargo package name in
  `target/release`, so 0.1.3 published without the zip its notes announced.

## 0.1.3

- Removed the MSI bundle. NSIS is now the only Windows installer.

## 0.1.2

- Added the Nexus Mods `nxm://` handoff. **Mod Manager Download** on the website
  hands the link to the manager, which fetches the file and routes it through
  the existing review and validation path.
- Added Nexus API key storage in the OS secret store, with a plain-text database
  fallback that the interface discloses.
- Added an opt-in `nxm://` protocol registration toggle; the association is
  never claimed automatically.
- Links for other games are refused rather than downloaded.
- Settings names the application currently holding `nxm://` when registration
  does not take effect.
- Registered the Linux `nxm://` handler directly instead of through
  `tauri-plugin-deep-link`, whose quoted `Exec` line `xdg-mime` can never
  resolve, and claimed the scheme in `<desktop>-mimeapps.list`, which
  `xdg-mime query` reads before the file `xdg-mime default` writes.

- Restored the original Z application icon, drawn as plain SVG in
  `src-tauri/icons/app-icon.svg`. The clone trooper helmet artwork used for the
  0.1.1 icon was withdrawn at the artist's request and is no longer distributed.

- Fixed UE4SS upgrades leaving runtime-supplied Lua mods at their old version.
  Only `UE4SS-settings.ini`, `mods.txt`, `mods.json`, and `load_order.txt` are
  preserved now; mods a package ships move with the runtime, and mods the user
  installed are untouched because a package never contains them.
- Added an opt-in test that runs the installer against a real published UE4SS
  package via `ZCOM_UE4SS_ARCHIVE`.

## 0.1.1

- Added a guided UE4SS runtime installer that unpacks a user-downloaded package
  into `Binaries/Win64` while preserving `ue4ss/Mods/` and `UE4SS-settings.ini`.
- Added links to the tested Zero Company UE4SS build and to the game's Nexus
  Mods page, scoped through the opener capability allowlist.
- Added search and status filtering to the Mods page.
- Reworked the application icon and interface palette around the clone trooper
  helmet artwork.

## 0.1.0

- Added cross-platform Steam library and build discovery with manual fallback.
- Added ZIP, 7z, direct packaged-file, folder, and drag-and-drop intake.
- Added IoStore companion validation and retoc 0.1.5 verification.
- Added PAK-only and existing-runtime UE4SS Lua mod management.
- Added an SQLite managed library with SHA-256 ownership records.
- Added transactional deployment, safe enable/disable, verify, and uninstall.
- Added filename/package overlap detection with spoiler-safe defaults.
- Added manifest compatibility warnings and game-update detection.
- Added Mod Doctor, UE4SS layout checks, and Linux/Proton guidance.
- Added Linux and Windows CI/release workflows, documentation, and licensing.
