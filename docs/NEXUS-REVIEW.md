# Nexus and Upstream Review

Reviewed on 2026-09-20/21:

- all 179 Nexus comments across 6 comment pages;
- all 3 Nexus bug reports;
- the upstream 0.6.5 source/tag;
- upstream GitHub issue 1 and pull request 3.

## Findings mapped to 0.7.0

| Report | 0.7.0 response |
| --- | --- |
| Startup remains on “Preparing your mod library” after a Steam library move | A stale saved path is now recoverable state; the app opens, shows the old path, and offers **Locate game**. Other startup requests use `Promise.allSettled`, and a dashboard failure gets a persistent recovery screen with retry and log actions. |
| Linux extraction creates names containing backslashes | ZIP member names and external-extractor output normalize both slash styles into real path components; traversal and drive-prefix forms remain rejected. |
| Linux buttons for game/mod/log folders appear to do nothing | AppImage launches `xdg-open` with the mounted runtime environment removed, matching the existing safe Steam-launch behavior. |
| No manager log is created for startup failures | The log directory and `application.jsonl` are created at the beginning of native setup, before the database and library are initialized. |
| Unreal ModKit plugin archives install only their PAK triplet | Upstream pull request 3 was integrated. `.uplugin`, `AssetRegistry.bin`, and the full plugin tree deploy together under `SWZeroCompany/Mods/<Plugin>`. |
| Configuration/INI mod support requested | A complete `SWZeroCompany/Saved/Config/Windows` layout is recognized. Whole files are backed up, checksum-guarded, and restored. Loose INIs are rejected, and writes are blocked while the game is running. Proton targets the matching Steam library’s compatdata prefix. |
| Nexus update/API behavior is unclear | Existing per-file variant tracking, cache, rate-limit, authentication, and free-vs-premium handoff behavior are retained. Continuation self-update checks stay disabled until its own repository is configured, preventing an accidental check against the upstream project. |
| EA App + UE4SS behavior is uncertain | Manual/EA installations are explicitly marked **Experimental** in Mod Doctor. File deployment is supported; a generated UE4SS log is required before runtime support is presented as working. |
| Proton DLL override is missed for games in an additional Steam library | Detection now checks both the game library and the primary Steam userdata locations, with case-insensitive filenames. |

## Regression-only reports

- The modified-file disable/removal problem reported against 0.6.2 was fixed in 0.6.5 and remains covered by lifecycle tests.
- A startup freeze reporter confirmed that the latest upstream version fixed their case; it remains a regression test area rather than a new defect.

## References

- Nexus Mods page: <https://www.nexusmods.com/starwarszerocompany/mods/29>
- Upstream source: <https://github.com/arctco/zcom-mod-manager>
- Plugin mod pull request: <https://github.com/arctco/zcom-mod-manager/pull/3>
- Config mod issue: <https://github.com/arctco/zcom-mod-manager/issues/1>
