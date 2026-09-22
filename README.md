<div align="center">
  <img src="src/assets/icon.svg" alt="" width="160" height="160">
  <h1>Zero Mod Manager</h1>
</div>

A dedicated open-source mod operations console for **Star Wars: Zero Company**.

> **Portable release note:** extract the ZIP before launching it. Release
> executables contain the frontend and never require a localhost development
> server. See [PORTABLE-DATA.md](PORTABLE-DATA.md) for the optional
> self-contained data mode. Windows installer releases use the same production
> executable as the portable archive.

Zero Mod Manager understands Zero Company mod payloads instead of
treating them as arbitrary files. It discovers Steam installations, validates
IoStore containers with retoc, manages UE4SS script and DLL mods, installs
game-folder mods such as ReShade, detects package overlap,
records SHA-256 ownership, and treats Linux/Proton as a first-class platform.

> Zero Mod Manager is an independent continuation based on ZCOM Mod Manager
> 0.6.5. It is not an official arctco release. This project is not affiliated with or
> endorsed by Electronic Arts, Lucasfilm, Disney, Bit Reactor, or Nexus Mods.
> Star Wars and related names are trademarks of their respective owners.

## Continuation links and release status

- [Source and development](https://github.com/stellamarislabs/zero-mod-manager)
- [Bug reports](https://github.com/stellamarislabs/zero-mod-manager/issues)
- [Application releases](https://github.com/stellamarislabs/zero-mod-manager/releases)

0.7.0-rc.1 is a test candidate, not Stable. Read [download verification and
Windows warnings](docs/RELEASE-SECURITY.md), [known limitations](KNOWN_LIMITATIONS.md),
and [migration/rollback](docs/MIGRATION-ROLLBACK.md) before installing.
The current local Windows packages are unsigned. Game acceptance is pending.

## Features

- Command Center readiness with explicit Ready, Warning, Blocked, and
  Unverified evidence states
- Named profiles, previewable switching, Profile Lock import/export, automatic
  snapshots, and Last Known Good checkpoints
- Modded, temporary vanilla, and controlled troubleshoot launch sessions with
  crash-safe profile restoration and user-labelled outcomes
- Signed community compatibility catalog with Ed25519 verification, expiry,
  rollback protection, evidence provenance, and offline last-known-good cache
- Config Workbench for validated INI, JSON, and TOML changes, diff, backup, and
  rollback; Lua remains read-only
- Privacy-safe local support bundles with preview and automatic redaction
- Steam and EA App installation metadata discovery, plus manual launch profiles

- Steam AppID `2075800` discovery across default and additional libraries
- Steam-aware game launch from the Home page, with an optional custom executable
- Dynamic Steam build-ID detection and update warnings
- ZIP, 7z, direct PAK/UTOC/UCAS, folder, picker, and drag-and-drop input
- Nested archive payload discovery without copying documentation or random data
- Guided FOMOD installers: an archive that scripts its own options is installed
  by answering the author's questions, with images, descriptions, and
  recommended answers
- Separate, labeled choices for packaged variants bundled in sibling folders
- IoStore pair/triplet validation and retoc 0.1.5 verification
- PAK-only and UE4SS Lua/DLL mod support
- Unreal plugin mods with their `.uplugin`, `AssetRegistry.bin`, and complete
  packaged content preserved under `SWZeroCompany/Mods`
- Whole-file configuration mods for `Saved/Config/Windows`, with backup,
  restore, checksum guards, and a running-game safety check
- Opt-in import from ZCOM Mod Manager 0.6.5 with verified library copying,
  database backup; Nexus credentials are not imported
- Existing-mod discovery and non-destructive adoption for packaged, UE4SS, and
  additive `LogicMods` installations
- Relocatable manager-owned source library plus checksum-guarded deployment records
- Enable, disable, hide, verify, and safe uninstall operations
- Filename and hashed IoStore package conflict detection
- Conflict-aware packaged-mod load order with preview and rollback
- Optional spoiler-sensitive package paths, disabled by default
- UE4SS layout checks and formatting-preserving `mods.txt` updates
- Guided UE4SS runtime installation from a package you downloaded yourself

- On-demand update checking, with MD5 identification for mods installed before
  the manager tracked provenance and an opt-in throttled check at start-up
- Linux compatdata and Proton DLL-override diagnostics
- Sanitized structured logs and a copyable Mod Doctor report
- Release checking that remains disabled until the continuation repository and
  Nexus page are explicitly configured at build time
- No account or always-on network requirement, telemetry, analytics, or advertisements

## Screenshots

The application ships a restrained tactical dark interface with seven primary
areas: Command Center, Library, Install, Profiles, Health, Settings,
and About. Release
screenshots are kept in `docs/screenshots/` when captured from a tagged build.

The interface contains no extracted game assets. The application icon is the
project's own zero-ring mark, drawn as plain SVG in `src/assets/icon.svg`.

## Supported Platforms

| Platform | Architecture | Packages |
| --- | --- | --- |
| Linux | x86_64 | AppImage and `.deb` |
| Windows 10/11 | x86_64 | NSIS `.exe` installer and portable `.zip` |

Steam Deck/SteamOS should work through the x86_64 Linux AppImage. Add it as a
non-Steam application if desired. See [Known Limitations](KNOWN_LIMITATIONS.md)
for the current boundaries.

## Supported Mod Types

### IoStore packaged mods

The common payload is one logical container:

```text
Example_P.pak
Example_P.utoc
Example_P.ucas
```

UTOC and UCAS must share a basename and both must be present. A companion PAK
is installed when supplied. retoc verification is optional. If it is unavailable, explicit confirmation allows installation with an Unverified warning. Failed verification cannot be bypassed.

When an archive contains packaged alternatives in separate folders, the install
review presents each folder as a labeled option instead of combining every
variant. Choose only the version or component you want; selected alternatives
remain separate library entries.

Required components can also ship together in one archive. Put a
`zcom-mod.json` beside each component's payload; the review applies the nearest
manifest to that component and offers **Install all components**. Fresh bundles
are committed as one operation and are completely removed if any component
fails. On upgrade, each component is matched to the installed payload it
replaces, even if the old components originally came from separate archives;
those replacement components are currently confirmed one at a time so each old
version retains its existing rollback guarantee.

### PAK-only mods

A conventional `Example_P.pak` payload is supported and deployed to `~mods`.

### UE4SS mods

A UE4SS mod is a folder the runtime loads by name, holding either payload:

```text
MyMod/Scripts/main.lua
MyMod/dlls/main.dll
```

Both are recognized, in any capitalization, at any nesting depth. An archive
that ships several mod folders installs each as its own entry, so they can be
enabled, ordered, and removed separately. UE4SS starts mods in the order
`mods.txt` lists them, and that order is editable on the Load order tab; the
managed block is kept directly before the runtime's `Keybinds` entry, while
other runtime entries and comments keep their relative order. UE4SS must already
have a healthy Zero Company layout. Zero Mod Manager does not redistribute
UE4SS; drop a downloaded runtime package on the installer and it is recognized
as the runtime rather than as a mod.

### Game-folder mods

Mods that the game reads from its own folders are installed from three
recognized layouts:

```text
AnyFolder/SWZeroCompany/Content/Movies/Intro.mp4   deployed relative to the game
LogicMods/Blueprint_P.pak                          deployed to Content/Paks/LogicMods
ReShade/dxgi.dll + ReShade.ini + shaders           deployed to Binaries/Win64
```

This is the only mod type that replaces an existing file. The original is kept
in the managed library and restored when the mod is disabled or uninstalled,
and a file another installed mod owns is never overwritten.

### Updating a mod

Installing a newer build of a mod you already have is recognized at inspection:
the preview offers to replace the installed version rather than reporting a
deployment conflict. The previous version's files are moved aside, not deleted,
so a failed upgrade puts them back and leaves the old version installed. The
replacement keeps its predecessor's position in the load order.

### Naming

A mod is named after the download it came from, with Nexus publishing metadata
removed, so `ZCUnlocked 34 1.3 2026-08-30T07-32Z i9WZfkaQ7.zip` installs as
"ZC Unlocked" at version 1.3. Names are editable before installing and can be
changed later from the library; renaming never touches deployed file names.

### Migrating existing mods

When Zero Mod Manager first connects to a game installation, it checks the controlled mod
folders for packages installed by hand or by another manager. The same scan is
always available from **Library → Scan game folder**.

PAK/IoStore container families, UE4SS mod folders, and additive `LogicMods` can
be adopted. Review the candidates, optionally merge container families that
belong to one download, edit their names, and select what Zero Mod Manager should manage.
Adoption copies each payload into the managed library and records checksums;
the live files, filenames, load order, and `mods.txt` are not changed.

Known UE4SS runtime components are shown but unchecked. Replacement-style
game-folder mods such as ReShade are reported but cannot be adopted safely,
because Zero Mod Manager did not see and back up the original file they replaced.

## Installation

Download the package for your platform from the GitHub release:

- Linux: make the AppImage executable and run it, or install the `.deb`.
- Windows: run the NSIS installer, or extract the portable `.zip` anywhere and
  run `Zero Mod Manager.exe`. Select a trusted retoc executable in Settings if
  you install IoStore mods; third-party tools are not silently bundled. The portable
  build also assumes the Microsoft Edge WebView2 runtime is already present,
  which it is on Windows 11 and on Windows 10 machines with current Edge; the
  installer downloads it when missing. Community builds are unsigned, so
  Windows SmartScreen may show a warning. Verify the release checksum and
  repository before choosing **Run anyway**.

Release packages do not include retoc or an archive extractor. Select a trusted,
host-installed retoc executable for IoStore verification. ZIP is built in; 7z
and RAR archives use a host-installed 7-Zip/NanaZip-compatible command.

## Quick Start

1. Start Zero Mod Manager.
2. Confirm the automatically detected game path, or choose **Locate game**.
3. Open **Install** and drop a downloaded mod archive onto the window.
4. Review detected files, verification, compatibility, and conflicts.
5. Choose **Install**.

By default, **Launch game** uses Steam. To use a different executable or
launcher, select it under **Settings → Game installation**, save the setting,
and launch from Home. **Use Steam** clears the override.

The manager makes one lightweight request to the project’s latest GitHub
release endpoint when it opens. If a newer manager release exists, an update
icon appears beside **About**. Offline failures do not block startup, mod
management, or game launch.

Instead of manually extracting `.pak`, `.ucas`, and `.utoc` into
`SWZeroCompany/Content/Paks/~mods`, drop the downloaded archive into the app.
The `~mods` directory is created automatically.

## Installing a Mod

Archives are extracted into a unique cache sandbox. Absolute paths, `..` path
traversal, and symbolic links are rejected. Executables, scripts, and DLLs are
never run. Only recognized packaged-mod or Lua payload files are copied into
the managed library and then deployed.

Installation follows: extract → recognize → validate → stage → deploy → commit
database ownership. If deployment or the database commit fails, newly deployed
files are removed.

## Guided Installers (FOMOD)

Larger mods increasingly ship as one download holding every variant the author
offers, with a `fomod/ModuleConfig.xml` describing the questions to ask and the
files each answer installs. When a download carries one, the installer reads it
and asks those questions instead of presenting the folders inside the archive.

Each step shows its option groups with the author's own descriptions and
images, and the option under the cursor is illustrated beside the list. The
answer the author recommends is selected to begin with. An option the script
marks as required cannot be unticked, and one an earlier answer rules out
cannot be chosen.

Answers drive the script the way its author wrote it. Choosing an option sets
the flags the script declares; later steps appear or are skipped according to
those flags; an option's own availability can depend on an earlier answer; and
the final set of flags decides both the files attached to each option and the
script's conditional installs. **Back** takes an answer away along with
everything it decided, and returns that step exactly as it was left.

Only the files the answers selected are installed. They are written into a
sandbox of their own and then read exactly like an ordinary download, so the
result reaches the same review screen, with the same container verification,
conflict detection, compatibility check, and naming, as any other mod. An
archive holding sixteen mutually exclusive variants therefore becomes one mod
entry rather than sixteen options to compare by hand, and the download it came
from is still recorded, so update checking keeps working.

After a guided install is confirmed, the manager retains the complete FOMOD
source tree and the answers used—not only the selected payload. Its library row
then offers **Reconfigure FOMOD**, which reopens the installer with those choices
preselected. Finishing produces the normal replacement preview, so changed
options are validated and deployed transactionally over the existing mod.

The script is untrusted archive data and is never executed. Every source and
destination path is checked against the same rules the extractor applies, once
when the script is parsed and again immediately before each file is written; a
path that would leave the package is refused. Group rules such as "choose
exactly one" are enforced where the files are decided rather than only in the
interface.

Conditions on other game plugins or on tool and game versions have no meaning
for Zero Company, which has no such plugins. They are read, reported as a
visible warning, and treated as unmet, so an option gated behind one is shown
as unavailable rather than installed silently. An archive whose script cannot
be read is installed the previous way, with its folders offered as labeled
options.

## Enabling / Disabling Mods

Use the switch on **Mods**. Disabling a packaged mod removes only destinations
recorded for that mod and retains the managed source copy. Enabling redeploys
from that copy. UE4SS toggles also edit only the matching `mods.txt` entry;
comments, unrelated entries, indentation, and line endings are retained.

## Uninstalling Mods

Before deletion, the manager recalculates every deployed SHA-256 checksum. If a
file changed since deployment, uninstall stops and keeps it. The normal UI does
not offer a casual force-delete path; the safe default is always to preserve
unexpected user data.

## Conflict Detection

Two levels are tracked:

1. **Filesystem collision:** two payloads target the same destination. A new
   install never overwrites an existing file.
2. **Package collision:** retoc package identifiers are hashed and stored. Mods
   that override the same identifiers are reported as overlapping packages,
   even when container filenames differ.

Normal UI and logs show only overlap counts. Raw asset paths are exposed only
after the user enables advanced package names in Settings and opens Advanced
Details; those names can contain spoilers.

## Load Order

**Mods → Load order** lists supported packaged mods from highest to lowest
priority. Move a mod toward the top to make it win known package overlaps, then
review the exact deployment filenames before applying. Newly installed
runtime-supported packaged mods start at the highest priority.

The manager keeps original filenames in its source library and applies a
numeric patch rank only to deployed companions. For example,
`Example_P.pak/.utoc/.ucas` at priority 3 becomes
`Example_0003_P.pak/.utoc/.ucas`. Every current file is checksum-verified
before a rename. A failed filesystem or database step rolls back, and an
interrupted operation is recovered at the next startup.

IoStore triplets are orderable because both priority directions were
demonstrated against Zero Company's runtime and re-verified with retoc after
each rename. Pure UTOC/UCAS pairs and PAK-only mods remain visible but
non-orderable. The pair layout is untested; the PAK-only capability fixture did
not pass the runtime gate. PAK-only package contents also remain opaque, so
their overlap winners cannot be identified automatically.

## Container Verification

Zero Mod Manager invokes a user-selected or host-installed **retoc** using
`retoc verify <container.utoc>` and collects package identifiers with `retoc
list --package --path`. A failed verifier prevents IoStore installation. An unavailable verifier allows
installation only after explicit confirmation; integrity and package overlaps remain unverified. Tool output shown in normal mode has home-directory prefixes
replaced with `~` and is truncated to avoid accidental data disclosure.

## UE4SS Mods

The expected runtime is:

```text
SWZeroCompany/Binaries/Win64/
├── dwmapi.dll
└── ue4ss/
    ├── UE4SS.dll
    ├── UE4SS-settings.ini
    └── Mods/
```

UE4SS is only needed by mods that require it. Many Zero Company mods are plain
IoStore or PAK payloads and work without it.

### Getting UE4SS

**Home → Runtime readiness → UE4SS runtime** offers two actions:

1. **Get the tested build on Nexus Mods** opens
   <https://www.nexusmods.com/starwarszerocompany/mods/9> in your browser. That
   page hosts the UE4SS build tested against Zero Company.
2. **Install from downloaded package…** takes the ZIP or 7z you downloaded and
   unpacks it into `SWZeroCompany/Binaries/Win64`.

The archive goes through the same sandbox as mod archives: absolute paths, `..`
traversal, and symbolic links are rejected, and nothing is executed. On a
reinstall or upgrade, your configuration is preserved rather than overwritten:

- `ue4ss/UE4SS-settings.ini`
- `ue4ss/Mods/mods.txt` and `ue4ss/Mods/mods.json`
- every `load_order.txt` under `ue4ss/Mods/`

Lua mods you installed yourself need no rule: a package does not contain them,
and a file the package does not contain is never touched. Lua mods the package
*does* ship (`BPModLoaderMod`, `ConsoleCommandsMod`, and friends) belong to the
runtime and are updated with it, so an upgraded `UE4SS.dll` is never left
paired with stale scripts.

The manager reports how many files it wrote and which it kept. To adopt a
shipped `UE4SS-settings.ini`, rename or delete your copy and install again.

Download the archive in your browser and open it in Install.

## Linux / Proton

Steam libraries under `~/.local/share/Steam`, `~/.steam/steam`, and every path
in `steamapps/libraryfolders.vdf` are scanned. The manager also checks for
`steamapps/compatdata/2075800`.

When UE4SS is present but no matching launch option can be detected, add this
to the game's Steam launch options:

```text
WINEDLLOVERRIDES="dwmapi=n,b" %command%
```

The application never edits Steam launch options. Flatpak Steam libraries may
require manual game selection and appropriate filesystem permissions.

When **Launch game** is used from the AppImage, the same Steam URI is opened
through the host system with AppImage library, Python, and toolkit paths removed
from the child environment. This prevents a closed Steam client from being
started against libraries inside the temporary AppImage mount; Steam's own game
launch options, including MangoHud and `WINEDLLOVERRIDES`, still apply normally.

## Windows

Steam is not assumed to be on `C:`. Common install roots and configured library
folders are scanned. The selected directory must contain both
`SWZeroCompany/Binaries/Win64/SWZeroCompany.exe` and
`SWZeroCompany/Content/Paks/`.

Managed payloads and backups default to the application's Local AppData folder,
not roaming AppData. **Settings → Managed mod library** can move them to an
empty folder on any drive; the manager verifies the copy before switching and
does not move the files already deployed into the game.

Unsigned builds can trigger SmartScreen. Code signing can be added later
through standard Tauri signing secrets without changing application behavior.

## Diagnostics

**Mod Doctor** checks the game layout, manifest/build, `~mods`, owned mods,
package conflicts, retoc, UE4SS, compatdata, and the Proton DLL override. The
report is copyable and home-directory paths are sanitized. Structured JSONL
logs are available from Settings → **Open logs folder**.

## Manual downloads

Download archives in your browser, then open them in **Install**. Discover,
Nexus account/API access, mod update queries, and `nxm://` downloads are no longer
part of Zero Mod Manager. Ordinary external Nexus links remain available.
If an earlier version owned `nxm://` links, reselect Vortex as the handler in
Vortex settings. This update does not overwrite another application's handler.

## Optional `zcom-mod.json` Manifest

Existing mods do not need a manifest. Authors can provide metadata:

```json
{
  "schemaVersion": 1,
  "id": "community.example.cheaper-actions",
  "name": "Cheaper Actions",
  "version": "1.0.0",
  "author": "Example Author",
  "description": "Adjusts action economy values.",
  "game": {
    "appId": 2075800,
    "testedBuilds": ["24874058"]
  },
  "type": ["iostore"],
  "requires": { "ue4ss": false }
}
```

The versioned JSON Schema is [schema/zcom-mod.schema.json](schema/zcom-mod.schema.json).
Unknown properties are allowed for forward-compatible community extensions.

## Building From Source

Requirements:

- Node.js 22+
- Rust stable
- Tauri 2 Linux prerequisites (`webkit2gtk-4.1`, GTK 3, librsvg, patchelf)
- `7z` for 7z archive installation/tests

```bash
git clone <continuation-repository-url>
cd ZeroModManager
npm ci
npm run tauri build
```

No third-party runtime or modding tool is fetched as part of the application
build. IoStore verification tests use a retoc executable explicitly supplied by
the developer or discovered on the host.

## Development

```bash
npm run typecheck
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

Three tests are ignored by default because they need legal local fixtures that
CI cannot download: an IoStore triplet, representative mod archives, and a real
UE4SS distribution package. To exercise the UE4SS installer end to end,
download a package from the mod page above and run:

```bash
ZERO_MOD_MANAGER_UE4SS_ARCHIVE=/path/to/ue4ss-package.zip \
  cargo test --manifest-path src-tauri/Cargo.toml -- --ignored
```

It performs a fresh install, then an upgrade over edited configuration and a
user-supplied Lua mod, and asserts what is kept and what is replaced.

The application is offline-first. Use synthetic fixtures only—never add game
assets, package dumps, credentials, SDK dumps, or personal logs.

## Project Structure

```text
src/                         React/TypeScript UI
src-tauri/src/steam/         Steam and game discovery
src-tauri/src/archives/      sandboxed ZIP/7z staging
src-tauri/src/mods/          payload and manifest recognition
src-tauri/src/fomod/         FOMOD installer scripts and guided selection
src-tauri/src/adoption.rs    existing-mod discovery and adoption
src-tauri/src/deployment/    ownership-safe lifecycle
src-tauri/src/retoc/         verifier abstraction
src-tauri/src/ue4ss/         runtime and mods.txt handling
src-tauri/src/database/      SQLite schema and queries
src-tauri/src/diagnostics/   Mod Doctor
schema/                      optional community manifest schema
```

## Release Builds

CI builds the production executable on every main-branch push and pull request.
Tags matching `v*` publish a GitHub release immediately and attach Linux
AppImage/deb and Windows NSIS installer/portable zip artifacts. The release is
not a draft, so bump the version in `package.json`, `package-lock.json`,
`src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`,
then smoke-test both platforms before tagging. The release workflow rejects a
tag that does not match those files.

```bash
npm run check:release-version -- v0.4.1
git tag -s v0.7.0 -m "Zero Mod Manager 0.7.0"
git push origin v0.7.0
```

Confirm checksums after the run finishes. See
[KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) before release.

## Roadmap

- **0.2:** conflict-aware packaged-mod load order, Steam launch, and automatic
  manager release notices shipped; profiles and dependency metadata remain
- **0.7:** Local-first installation; retired Nexus API and download handoff.
- **0.4:** existing-mod migration, bundled packaged-variant selection, and a
  custom game executable shipped
- **0.6:** guided FOMOD installers for archives that script their own options
- **Future / separate project:** Zero Mod Studio for asset inspection and authoring

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) for setup,
architecture, tests, privacy expectations, and adding a mod format. Security
issues involving archive handling or deletion safety should not be disclosed
with real user paths or game data.

## Credits

Thanks to the Zero Company modding community and to the maintainers of retoc, Tauri, React, rusqlite, zip-rs, and the wider open-source ecosystem.

## Third-Party Software

retoc is MIT-licensed third-party software. Zero Mod Manager can use a copy the
user explicitly selects or installs on the host; it is not included silently in
release packages. Exact copyright and license notices are in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
No code was copied from Vortex or another mod manager.

## License

Zero Mod Manager is licensed under the
[GNU General Public License version 3 only (`GPL-3.0-only`)](LICENSE).
Continuation copyright © 2026 Zero Mod Manager contributors. Original work
copyright © 2026 Victor Hugo (arctco).

The license permits forks and redistribution, but it does not grant permission
to present a modified build as an official ZCOM Mod Manager release. Zero Mod
Manager therefore uses a separate name, identifier, and logo. See the upstream
[trademark and branding policy](TRADEMARKS.md) for use of the project name and
logo.

## Disclaimer

Modding can make saves or game installations unstable. Back up important data,
read each mod's documentation, and review compatibility after every game
update. This project does not provide game files, UE SDK data, or copyrighted
assets and does not bypass ownership or platform protections.
