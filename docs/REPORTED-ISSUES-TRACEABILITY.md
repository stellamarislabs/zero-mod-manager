# Reported issue traceability

This ledger converts public support history into a release gate. A code change
is not considered closed until its automated regression and, where the behavior
depends on the game, its real-game evidence are attached to the release record.

| Source / symptom | Root cause or risk | 0.7 control | Automated evidence | Real-game evidence required |
| --- | --- | --- | --- | --- |
| Manager reports healthy but the EA launch is vanilla | Install path and file presence were treated as runtime proof | EA metadata discovery plus UE4SS log-backed preflight | launcher and preflight tests | EA App launch with loader initialization in log and visible mod behavior |
| UE4SS performance complaint | Process exit and file layout cannot attribute frame loss | Runtime-only and controlled troubleshoot sessions; outcome is user-labelled | session persistence and restoration tests | baseline, runtime-only, and mod subset comparison |
| Moved Steam library | Persisted path becomes stale | library-folder discovery and explicit invalid-path state | Steam library parser regression | moved-library upgrade scenario |
| 7-Zip installed but not found | installer does not add CLI to PATH | registry, standard-folder, NanaZip, PATH, and manual resolver | resolver tests | Windows clean-machine smoke test |
| ModKit/plugin payload incomplete | generic file picking loses plugin structure | dedicated plugin family preserves `.uplugin`, registry, and content | archive fixture lifecycle | full plugin visible in game |
| IoStore triplet or load order fails | family and order invariants not enforced | pair/triplet validation, retoc evidence, previewable order | lifecycle matrix | two-direction winner test |
| Linux launch inherits AppImage libraries | child environment contamination | sanitized host `xdg-open` environment | environment unit test | AppImage/Proton launch |
| Managed file changed by the mod | checksum guard prevented all lifecycle actions | explicit force path with warning and preserved transaction history | changed-file lifecycle tests | writable UE4SS settings mod update |
| Interrupted install/profile/temporary launch | partial filesystem state | snapshots, operation records, pending restore, and rollback path | interruption/restart tests | forced termination during RC smoke test |
| Malicious or pathological archive | extraction before sufficient limits | isolated staging plus link/path/device/count/size/ratio/depth limits | archive adversarial suite | not applicable |

The detailed 179-comment and bug-report inventory must be preserved with source
URLs in the release issue tracker. This repository intentionally stores the
deduplicated engineering cases rather than personal Nexus usernames or copied
comments.
