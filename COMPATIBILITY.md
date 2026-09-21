# Zero Mod Manager Compatibility Matrix

Last verified: 2026-09-21 against the local `0.7.0` continuation branch.

The matrix uses the real release artifacts listed below. Archives are staged as untrusted input, IoStore containers are verified with `retoc 0.1.5`, and the test fails when any preview is rejected or invalid. It does not launch the game.

| Mod artifact | Detected payloads | Result |
| --- | --- | --- |
| BackpackCustomization candidate 6 | UE4SS native mod + Unreal plugin (manifest, registry, PAK/UTOC/UCAS) | Pass |
| Expanded Customization 2.3.0 Full | UE4SS native mod + `ECNamedVisuals` Unreal plugin | Pass |
| ExpandedWardrobe 3.0.0 beta Safe Installer | Standalone and ZCUnlocked-compatible UE4SS alternatives | Pass; alternatives are separate install choices |
| HubHelmetHide 1.0.1 | UE4SS native mod | Pass |
| SquadExpansion 1.0.0 Beta | Mixed UE4SS Lua/native component + verified IoStore triplet | Pass |
| UnlockVisibleOutfits loose folder | UE4SS Lua folder | Pass |
| ZCUnlocked Steam 1.4.5 | UE4SS native mod and settings | Pass |

## Re-run the local matrix

The ignored integration test deliberately needs user-supplied local artifacts:

```powershell
$env:ZERO_MOD_MANAGER_ARCHIVES = 'C:\path\to\selected-archives'
$env:ZERO_MOD_MANAGER_LOOSE_MODS = 'C:\path\to\loose-mod-folder'
$env:ZERO_MOD_MANAGER_RETOC = 'C:\path\to\retoc.exe'
cargo test describes_locally_downloaded_archives -- --ignored --nocapture
```

The public test suite uses synthetic fixtures and does not redistribute these mods.
