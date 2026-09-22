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
