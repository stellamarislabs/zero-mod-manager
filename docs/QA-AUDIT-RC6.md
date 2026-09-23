# RC6 targeted release audit — 2026-09-22

## Scope and verdict

Reviewed install/replace/uninstall planning, explicit bundle identity, native consent,
profile-reference retention, preview lifetime, event subscriptions and Library layout.
Existing archive, transaction, launcher and recovery suites were rerun. Real user game
files and application records were not modified for testing. This is beta/RC evidence,
not Stable certification, a penetration test or a leak-free guarantee.

## Confirmed findings and remediation

| Severity | Finding / evidence | Remediation / regression |
| --- | --- | --- |
| P1 | User logs contained `plugin:dialog\|confirm not allowed by ACL`; synchronous `window.confirm` call sites did not await native consent. | Explicit, awaited native confirmation; reject/cancel fails closed; confirm/save permissions narrowly enabled. Tests exercise pending, consent, cancellation and rejection. |
| P2 | Standalone replacement previews offered complete-bundle Update; App rejected owners with no bundle ID. This error also appears in user logs. | Shared bundle planner requires one explicit bundle. Individual replacement remains the valid action. Mixed independent entries never auto-merge. |
| P2 | Open previews retained replacement claims after the owner was deleted. | Refresh reconciles deleted replacement/conflict IDs and package targets. Frontend regression plus backend deletion/matching assertions. This is a verified stale-state path, not proof of every reported reinstall instance. |
| P2 | Single replacement removed the old profile foreign key and did not preserve disabled/hidden state. | Transfer profile selections before removal; preserve state. Integration test covers replacement, stored version, uninstall and fresh reinstall. |
| P2 | A second inspection or cancellation could release deployment input. | Busy guard for dropped archives and disabled Cancel during deployment. Runtime-completion cleanup remains explicitly allowed. |
| P2 | Native browser prompt was used for rename. | In-app, labelled rename dialog. |
| P2 | Bundle versions were absent; long titles could exceed their flex track. | Recorded version, mixed/unknown states, constrained title track, original title tooltip and component details. |
| P3 | Subscription registration rejection was unhandled; toast timer lacked unmount cleanup. | Rejection handling, subscription lifetime tests and unmount cleanup. No long-duration memory profile was performed. |

## Checks

- TypeScript typecheck: passed.
- React/Vitest: 151 passed across 23 files.
- Rust library tests: 185 passed, 3 ignored (188 discovered).
- Rust Clippy: all targets/features, warnings denied, passed.
- NPM production dependency audit: zero known vulnerabilities at execution time.
- Shared SVG, four ICO frames, installer/uninstaller and content-hashed icon: passed.
- Version contract: all files match 0.7.0-rc.6.
- Chromium fixture: production Library components with synthetic known/unknown/mixed
  versions at 1440×900 and 760×600. Maintenance buttons share a row; no observed
  button overlap. Small layout retains Components/Verify/Uninstall/Details.
- Original ZCOM 0.6.5 CSS reviewed locally: preserved table hierarchy, enable control,
  search/filter and compact row actions; retained Zero's own blue/cyan branding.
- Installer/portable startup and checksum results are supplied by the prepared
  release pipeline; do not treat source tests alone as packaged-binary validation.

## UI assessment (manual, limited scope; not certification)

| Dimension | /20 | Qualification |
| --- | ---: | --- |
| Accessibility | 15 | Labelled enable controls, keyboard dialogs and focus indication; full screen-reader/contrast audit still pending. |
| Performance | 13 | Compact rows and bounded operations; large-library profiling/virtualization not qualified. |
| Theme consistency | 18 | Shared shell, tokens and original Zero icon; no per-page redesign. |
| Responsive layout | 17 | Two tested viewport sizes; arbitrary DPI/device matrix not exhausted. |
| UX/state correctness | 17 | Consent and update routing fixed; unknown metadata explicitly remains unknown. |

Positive controls: persistent package rollback, exact ownership checks, separate
adoption records, no save manipulation, subscription cleanup and scoped status labels.

## Remaining qualification / recommendations

- User acceptance: install v1 → replace with v2 → confirm version → uninstall →
  install v2 fresh; repeat with a genuine bundle and test Cancel on confirmation.
- Game/platform matrix, long-run heap/handle and large-library stress tests remain
  pending. No current Rust advisory scan or independent security review was run.
- Modified bundle payloads deliberately stop whole-package updates; they are not
  silently overwritten. Review the affected file before retrying.
- Old adopted files without version metadata remain “Version not recorded”; no
  filename-based retroactive rewrite of the user's existing library was performed.
- Existing known limitations and Stable gates remain in `KNOWN_LIMITATIONS.md`.
- No public release was created or updated as part of this audit.
