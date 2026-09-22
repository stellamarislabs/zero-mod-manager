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
