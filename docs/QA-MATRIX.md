# 0.7 release qualification matrix

`0.7.0-rc.7` is a release candidate, not a certification of all planned features.
Game acceptance is pending. Evidence below distinguishes automated checks from
manual platform qualification. Last local test record: 2026-09-22.

## RC7 changed-file regression, 2026-09-23

- A managed DLL with a changed checksum blocks the first update or removal attempt.
  The transaction restores the previous files and library entry.
- The app recognizes the rollback reason, identifies the affected path and asks
  before retrying with the changed-file override. Cancelling keeps the old mod.
- Automated application tests exercise the update and uninstall retries, including
  the resulting Library state. The package transaction test checks both refusal
  and explicit removal. Real-game confirmation remains pending.

## RC5 evidence corrections

- Optional naming normalization does not produce a conflict warning.
- Missing or failed data checks do not display successful/zero-result data.
- Profile previews match normalized priorities; snapshots/session evidence use
  actual deployed selections rather than pending profile drafts.
- Windows branding is checked from the SVG through all ICO frames, PE resources,
  hidden-window taskbar property readback and isolated NSIS shortcut fixtures.
- Current runtime logs are not parsed for session success; only file presence
  is reported. The real-game matrix remains separate.

## Automated gates

- Rust unit/integration suite and React component suite pass. Current local RC
  evidence: 183 Rust tests passed, 3 fixture-dependent tests ignored; 140 React
  tests passed; TypeScript/Vite production build and native Rust release build
  passed on Windows 11.
- Manifest v1/v2, Profile Lock v1, and Catalog v1 schema and round-trip tests pass.
- PAK, IoStore, UE4SS Lua, UE4SS DLL, plugin, game-directory, config, and FOMOD
  lifecycle tests cover install, enable, order, update, disable, uninstall, and rollback.
- Adversarial archive, locked-file, interrupted-operation, catalog signature,
  catalog rollback and profile restoration have automated tests. Nexus API/lineage
  tests were retired with the feature, not silently skipped.
- Component accessibility checks and browser layout checks at 760×600 and
  1440×900 passed. Full screen-reader, high-DPI and keyboard audits remain pending.
- Release requirement: no open P0/P1. Formal issue triage remains pending;
  known implementation limits are documented in `KNOWN_LIMITATIONS.md`.

The default suite skips three external-fixture tests. Two were separately run
and passed on 2026-09-22: five local mod ZIPs (including both Operations Tweaks
variants and the four-component Ship Paint package), and a copied IoStore
triplet rename/byte-preservation check. These do not prove in-game compatibility.
The UE4SS distribution fixture test remains pending. Clippy with all targets,
all features and warnings denied passed locally. Production npm audit reported
zero vulnerabilities; this is not an audit of every dependency or native payload.
Hosted CI and Linux builds are not signed off. WSL is not installed on this host.

## Stable blockers outside the local automated suite

- Execute and retain evidence for every row in the real-game matrix below.
- Provision the public signed compatibility catalog, production key, and URLs.
- Finish the consent-driven, hash-pinned official runtime/tool downloader.
- Route all user-visible copy through the English message catalog.
- Product repository/issue/update URLs use stellamarislabs/zero-mod-manager.
  Public catalog and continuation Nexus listing still need configuration.
- Configure detached release signing and validate signature verification. Local RC EXE/installer: NotSigned.
  Unsigned distribution instructions: `RELEASE-SECURITY.md`.

## Real game and package matrix

| Host | Launcher / package | Clean | 0.6.5 upgrade | Existing modded install | Runtime/log evidence | Visible behavior |
| --- | --- | --- | --- | --- | --- | --- |
| Windows 10 x64 | Steam / installer | Pending | Pending | Pending | Pending | Pending |
| Windows 11 x64 | Steam / portable | Pending | Pending | Pending | Pending | Pending |
| Windows 11 x64 | EA App / installer | Pending | Pending | Pending | Pending | Pending |
| Supported Linux x64 | Steam/Proton / AppImage | Pending | Pending | Pending | Pending | Pending |
| Supported Linux x64 | Steam/Proton / `.deb` | Pending | Pending | Pending | Pending | Pending |
| SteamOS / Deck Desktop Mode | Steam/Proton / AppImage | Pending | n/a | Pending | Pending | Pending |

Every host must include one real PAK, IoStore triplet, UE4SS Lua, UE4SS DLL,
full ModKit plugin, config package, FOMOD, and mixed profile. File placement alone
is not a pass.

## Release evidence

- Version-matched source archive and per-file snapshot manifest, linked to binary hashes.
- SHA-256 list and detached release signature.
- Authenticode status stated explicitly; if unsigned, SmartScreen guidance is visible.
- Own repository, issue tracker, catalog, update, and Nexus URLs configured.
- Changelog, limitations, migration/rollback, support, and attribution reviewed.
