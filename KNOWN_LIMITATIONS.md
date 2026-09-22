# Known Limitations — 0.7.0-rc.2

- Nexus integration is removed. Download archives in a browser and use Install.
  Reassign old nxm associations in Vortex if needed.
- Container content/asset verification is not performed. IoStore companion-file checks,
  archive safety, file checksums and destination ownership checks remain enabled.
- UE4SS, 7-Zip, and NanaZip are never silently bundled or downloaded.
  0.7.0-rc.2 supports explicit local tool selection and verified local package
  installation; the signed tool-source catalog and consent-driven official
  downloader are a remaining Stable gate.
- The compatibility engine accepts and verifies signed Catalog v1 envelopes,
  but a public catalog repository, production Ed25519 key, release URL, and
  maintainer signing procedure still need to be provisioned. Until then the
  application reports catalog-backed compatibility as **Unverified** and uses
  local/author-manifest evidence only.
- English interface copy is not yet fully routed through the future
  localization message catalog. Localization is therefore not enabled in this
  release candidate.
- This release candidate has automated coverage but is not a substitute for the
  real-game qualification matrix. Steam and EA App on Windows, Linux/Proton,
  SteamOS, migration from 0.6.5, and every supported payload family must record
  visible game behavior or runtime/log evidence before Stable.
- The top-60 Nexus review in `docs/NEXUS-60-MOD-AUDIT.md` is based on published
  file trees and installation instructions. It does not grant a Verified badge
  without the corresponding archive and a real-game result.
- Package installation, full replacement and removal use a persistent file/SQLite
  rollback journal. Interrupted operations restore automatically on startup, not
  through a user-selectable recovery wizard. Recovery is blocked while the game
  runs or when backup integrity cannot be established.
- Complete replacement requires confirmation; omitted components are removed.
  The explicit package target handles renamed archives. Component-only updates
  retain their package identity but do not retire other components.
- Package enable/disable and multi-selection cleanup currently run component
  operations sequentially. Each component has rollback protection, but the whole
  selection is not a single atomic transaction.
- Recovery history is retained under `package-recovery` in application data and
  can consume significant disk space. There is not yet an automatic retention
  policy or cleanup UI. Do not manually change a pending `package-operation`.
- Loose root `Engine.ini` presets remain blocked. Installing them as whole-file
  replacements would erase unrelated settings, so they will enter support only
  through semantic INI diff/merge and profile rollback.
- Compatibility catalog absence never becomes a positive compatibility claim.
  Unknown combinations remain **Unverified** and local conflict analysis still
  runs from the installed payloads.
- Guided Isolation keeps hard dependencies together and uses binary subdivision,
  but a fault that appears only when members from two different halves interact
  may require a second manual run with a user-chosen combined subset. Results
  are evidence levels, never an automatic accusation against a mod author.

- 7z and RAR installation uses the open-source 7-Zip command-line program
  available on the host. ZIP support is built in. The tool is looked for on
  `PATH`, in the standard 7-Zip and NanaZip install folders, and in the
  directory 7-Zip's installer registered; Settings takes a path for anything
  else. A missing tool produces setup guidance.
- RAR still needs a 7-Zip build carrying the RAR codec, which many omit.
- UE4SS can be installed from a package the user downloaded, but it is never
  downloaded automatically. Download its archive in a browser.
- UE4SS installation preserves `UE4SS-settings.ini`, `mods.txt`, `mods.json`,
  and every `load_order.txt`. A package shipping newer defaults for those will
  not replace an existing copy; remove yours first to adopt them.
- Applying a UE4SS start order writes the managed entries as one block directly
  before the runtime's `Keybinds` block, which Zero Company requires to remain
  last. Other comments, blank lines, and runtime-owned entries keep their
  relative order, but a managed mod that was hand-placed elsewhere moves into
  that block. Order among managed mods is preserved, and mods installed before
  this release keep the order the file already has.
- UE4SS starts DLL mods and Lua mods in two separate passes: every DLL mod runs
  as the runtime initializes, and the Lua mods only once the scripting runtime
  exists. Order is therefore settable within each pass, never across them, and
  a request to interleave them is normalized into what the runtime will do.
- UE4SS start order covers `mods.txt` only. BPModLoader keeps its own list in
  `BPModLoaderMod/load_order.txt` for blueprint mods, which the manager still
  preserves rather than writes, so blueprint load order stays manual.
- Recovery mechanisms differ between package, profile, runtime and config
  operations; do not assume every application action uses the package journal.
  Avoid editing the managed library while an operation or recovery is pending.
- Game-folder mods are recognized from three layouts: a tree containing
  `SWZeroCompany`, a `LogicMods` blueprint pack, and a loader shim named after
  the system library it replaces (`dxgi.dll`, `dinput8.dll`, and similar) with
  the files beside it. Anything else in the archive is listed and left alone.
- A game-folder mod is the only kind that replaces an existing file. The
  original is kept in the managed library and restored on disable or removal,
  but a file another mod already owns is never overwritten.
- Existing-mod discovery adopts additive PAK/IoStore, UE4SS, and LogicMods.
  It reports but does not adopt ReShade and other replacement-style game-folder
  mods because their pre-mod originals are no longer available to back up.
- Lua mods bundled inside a UE4SS package are treated as part of the runtime
  and are overwritten on upgrade. Edits to a shipped mod's scripts are lost;
  copy it under a new folder name to keep changes.
- Steam launch options are inspected heuristically and never edited. On Linux,
  confirm `WINEDLLOVERRIDES="dwmapi=n,b" %command%` manually.
- The optional custom game launcher starts the selected file with its containing
  folder as the working directory and does not add command-line arguments. On
  Linux, select a native launcher or wrapper rather than a Windows executable
  that the host cannot run directly.
- Encrypted or future container formats are not validated by a content parser.
- PAK-only mods cannot provide package-level overlap metadata; only destination
  filename collision is available for them.
- Load-order management is enabled for IoStore triplets with a companion PAK,
  which passed the two-direction Zero Company runtime test. Pure UTOC/UCAS
  pairs remain visible but non-orderable because they are not independently
  verified.
- PAK-only mods remain visible but non-orderable: the local capability fixture
  did not pass the runtime gate. Their contents are also opaque, so the manager
  cannot identify which assets a PAK-only mod wins or loses.
- A FOMOD installer script is read for its options and file rules, not run. The
  format also allows conditions on other game plugins and on tool or game
  versions; Zero Company has no such plugins, so those conditions are reported
  as a visible warning and treated as unmet, and an option gated behind one is
  shown as unavailable. Version-numbered dependencies on the game or on the
  manager are treated the same way.
- An installer script the manager cannot parse is not a refusal: the archive is
  read the previous way instead, with its folders offered as labeled options.
  The same applies to a file the script names that the archive does not
  actually contain, which is skipped with a warning rather than failing the
  install.
- Option images are shown for PNG, JPEG, GIF, WebP, and BMP up to 4 MB each. A
  larger or differently encoded image is left out and its option still lists
  its name and description.
- A scripted install produces mod entries from the files it selected, not one
  entry per answer. Where those files form a single flat payload they become
  one mod, so re-running the installer with different answers replaces that mod
  rather than adding to it.
- Hiding a mod affects the library list only. A hidden mod is still installed,
  still deployed, still counted on Home, and still listed in the load-order and
  UE4SS start-order editors, because it still loads and its position still
  matters.
- Removing or disabling a mod prunes the empty folders its own payload created,
  but only below that mod type's deployment base and only while they are empty.
  A folder still holding a settings file, a log, or anything else the manager
  does not own is kept, and a game-folder mod is never pruned because its base
  is the game installation itself.
- Profiles record enabled state, package/UE4SS priority, runtime requirement,
  notes, and lockfile identity. Save files are intentionally neither read nor
  copied, and BPModLoader-specific ordering remains external until its public
  on-disk contract is verified.
- RC release builds may be unsigned and SmartScreen may warn on Windows. Release
  CI always publishes `SHA256SUMS`; Stable is blocked unless a detached minisign
  signature can also be produced. Authenticode remains conditional on obtaining
  a Windows signing certificate.
- Flatpak/Snap sandbox permissions and uncommon portable Steam installations
  may require manual game-path selection.
- Windows artifacts are generated by GitHub Actions; they cannot be produced on
  a Linux host without a complete Windows MSVC/WiX/NSIS toolchain.
- AppImage builds on rolling distributions whose libraries use modern RELR ELF
  sections set `NO_STRIP=1`; the linuxdeploy binary embedded by Tauri otherwise
  uses an older `strip` that cannot parse those sections. Release CI also sets
  this compatibility flag.
