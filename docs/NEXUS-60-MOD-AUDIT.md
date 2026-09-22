# Nexus top-60 compatibility audit

Research snapshot: 2026-09-21. The first 60 results in the all-time
endorsement ordering were reviewed across their Files pages. Archive previews
were available for 57 entries; Expanded Wardrobe, Unique Talents, and Aftermath
were classified from their published installation instructions.

No mod archive was downloaded or redistributed for this audit. These findings
are structural metadata evidence, not a claim that the mod was exercised in
the game.

## Reviewed Nexus mod IDs

9, 34, 44, 20, 29, 30, 21, 23, 72, 3, 37, 55, 47, 73, 27, 160, 18, 14,
61, 33, 118, 58, 77, 2, 62, 38, 140, 109, 63, 110, 60, 40, 103, 133, 17,
137, 162, 105, 39, 48, 102, 174, 64, 154, 157, 135, 186, 153, 188, 129,
206, 10, 50, 201, 36, 53, 218, 100, 193, and 11.

## Format findings

| Package family | Representative IDs | 0.7 status |
| --- | --- | --- |
| PAK/IoStore | 3, 14, 18, 33, 39, 40, 47, 50, 61, 62, 63, 64, 72, 73, 77, 100, 102, 103, 105, 109, 110, 135, 137, 140, 154 | Recognized; companion-file checks only, no external content verifier. |
| UE4SS runtime | 9 | Classified as runtime rather than a library mod. |
| UE4SS Lua/native | 38 and other runtime mods | Recognized when the standard `Mods/<name>/Scripts` or `dlls` layout is present. |
| Unreal plugin/ModKit | 160, 162, 174, 186, 188, 193, 201, 206, 218 | Full plugin folders and sidecars are retained. |
| FOMOD/multiple variants | 21, 23, 27, 30, 37, 48, 133 | FOMOD choices are supported; ambiguous non-FOMOD siblings remain Unverified. |
| Hybrid package | 2, 10, 11, 34, 44, 55, 58, 129, 133 | Components are detected; fresh bundles install atomically with persistent identity. Multi-component replacement rollback remains a release gate. |
| Root config preset | 20 and an optional file from 2 | Not installed yet. Config presets require semantic INI merge rather than whole-file replacement. |
| External installer | 118 | Blocked and explained; setup executables are never run. |
| External authoring tool | 157 | Blocked as an application; nested sample PAK files are not offered as mods. |
| Previous mod manager | 29 | Classified as an application, not a game mod. |

## Implemented from this audit

- External EXE/MSI and scripted installers receive a non-installable package
  assessment before nested payload discovery.
- Native inspection includes ASI files and the common `dsound`, `winhttp`, and
  XInput proxy DLL families.
- Production packages now prove that the embedded frontend reached the Rust
  command layer without a localhost server.
- The exact Unreal Engine minor version is not asserted without runtime
  evidence.

## Remaining qualification

- Add copyright-free structural fixtures for every row above.
- Extend the backend bundle journal from fresh installs to all-component updates.
- Add semantic root-INI preset import.
- Exercise at least one real archive from each family against the Steam build.
- Keep EA, Epic, and Proton results Unverified until tested on those launchers.
