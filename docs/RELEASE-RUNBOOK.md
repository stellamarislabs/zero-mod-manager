# Release preparation and manual publication

## Repository ownership

The continuation repository is https://github.com/stellamarislabs/zero-mod-manager.
Its issue tracker is https://github.com/stellamarislabs/zero-mod-manager/issues.
The working tree originates from arctco/zcom-mod-manager. Do not push
continuation commits or tags there. Create/select the owner's continuation repository
first; preserve the original remote as upstream and configure origin only after
the destination is verified. Never use an invented Nexus mod ID or repository URL.
Enable Issues and publish the modified source, license, attribution and build scripts.

The release workflow derives project/update URLs from github.repository, refuses
the original upstream repository, and creates a **draft** release. Set the optional
ZERO_MOD_MANAGER_NEXUS_URL repository variable only after the real page exists.
No tag should be pushed until the source tree has been reviewed for credentials,
personal paths and unintended assets. Do not use git add . on this working tree:
output/, artifacts/, game fixtures and generated evidence are not source commits.

## Local handoff

Run scripts/prepare-release.ps1 from the repository with PowerShell 7. It records
an allowlisted source snapshot (including uncommitted source), builds the Windows
packages, checks that source did not change during the build, packages documentation,
and generates SHA256SUMS plus release provenance. It never pushes or publishes.
It excludes .git, game files, credentials, node_modules, target, outputs and old releases.
Build dependencies are pinned by package-lock.json and Cargo.lock; source-build
instructions are in CONTRIBUTING.md. Do not edit source while preparation is running.

Run scripts/verify-release.ps1 against the prepared directory. A checksum pass
means integrity against that manifest, not authenticated publisher identity.
Keep the working tree and the generated source snapshot used for every distributed binary.
Do not combine a new binary with an older source archive or checksum manifest.

## RC publication gates

1. Review current automated results and user-provided game evidence in QA-MATRIX.md.
2. Verify repository ownership, source download, support URL and application links.
3. Review Nexus copy, migration guide, limitations, attribution and unsigned warning.
4. Verify every artifact hash and source/provenance correspondence.
5. Attach matching source and documentation next to the binary downloads.
6. The owner reviews and manually publishes the draft; preparation does not authorize publication.

## Stable gates

Do not promote RC solely because it compiles or passes unit tests. Complete the
claimed platform/game matrix, audit open P0/P1 issues, and provision release signing.
If reducing the original feature/platform scope, explicitly approve that scope
change and update the release claims; do not silently declare unfinished gates passed.

## Rollback

Keep previous release assets and their source available. If a defect is found,
mark the affected release as withdrawn and explain the impact; do not overwrite
published artifacts under the same filename/version with different bytes.
Do not instruct users to delete their library or saves. Follow MIGRATION-ROLLBACK.md.
