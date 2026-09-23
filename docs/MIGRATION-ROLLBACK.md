# Migration and rollback

On first 0.7 start, Zero Mod Manager upgrades its local database in place only
after opening the existing data successfully. The managed source library remains
the authoritative copy; deployed game files are not reinstalled merely because
the database schema changed.

Before release qualification, an upgrade smoke test must keep a copy of the
0.6.5 data directory and managed library. If migration fails, close 0.7, retain
its logs, restore that copy, and reopen 0.6.5. Do not point two manager versions
at the same live database.

Profile switches and temporary vanilla/troubleshoot launches create a snapshot
before changing deployment state. A failed switch attempts to restore its
previous profile. A pending temporary state is detected at the next manager
start and restored after confirming the game is no longer running.

Config Workbench writes only validated INI, JSON, or TOML. It copies the prior
file to the manager backup area, swaps the new file into place, and records the
checkpoint in Health → Config Workbench. Restore uses that exact backup.

Save files are never part of a profile, migration, support bundle, or rollback.
## RC2 package recovery

Install, complete-package replacement and removal snapshot affected files and the
database before mutation. On failure the journal restores the prior state. After
interruption, startup attempts recovery before opening the normal database. This
is automatic, not a choice dialog. Close the game before retrying recovery. If
backup validation fails, stop and retain the application data for investigation;
do not delete the pending package-operation folder.

Completed recovery history is kept locally in package-recovery, including copies
of interrupted files where applicable. It can be large. There is no retention UI
yet. Package toggles and selected-library cleanup remain sequential component
operations rather than one selection-wide transaction.
## RC4 existing-mod workflow (supersedes RC3 grouping)

For an unmanaged installation, scan the game folder and select mods to adopt.
Every detected entry keeps its own identity, files, enabled state and order.
There is no Merge action. Similar names and shared archive paths do not create
bundles. A cross-loader bundle cannot reliably be reconstructed from loose game
files; use its original package and review the bundle installation if needed.

Library > Organize mods > Separate mods separates wrongly grouped records. Older
flattened Adopt/Merge records have an explicit Check and recover action there.
It never changes game files. It verifies stored copies and ownership, preserves
profile memberships and compatible in-app checkpoints, and rolls back on failure.
Unclear layouts, changed files, author metadata or incompatible historical
checkpoints stop recovery. Re-export any external profile lockfiles afterward;
previously exported files and historical session logs are not rewritten.

## Factory reset without uninstalling

Settings > Factory reset > Reset app data requires the acknowledgement and exact
text RESET. Cancel pending installations and finish temporary launch recovery
first. The app closes and completes the reset before opening its database or
WebView on the next launch. It never uninstalls game mods or touches saves.

App-owned data, profiles, settings, history, library copies and browser cache are
retired to sibling folders named `app.zeromodmanager.desktop.reset-recovery-<id>`
(portable mode: `data.reset-recovery-<id>`). Externally relocated library copies
are deliberately retained and their location is disclosed in the confirmation.
These recovery folders are not loaded automatically. Keep them until the fresh
setup is validated; they may contain original-file backups needed for replacement
mods, which cannot safely be recaptured by rescanning the already-modded game.

If interrupted, startup resumes the recorded reset. Unrecognized/new files or
linked paths stop reset without deleting that data. Do not remove its journal or
recovery folders to force startup. Manual restoration requires closing the app
and restoring each complete matching data root together; do not mix an old
database with a new library. Original downloaded archives and upstream ZCOM data
are outside the reset scope.
