# Zero Mod Manager visual system

## Current direction
The user retired the holographic concept on 2026-09-22. Consistency with Library, Profiles and Settings now takes priority. Historical concept images are not acceptance targets.

Players use this desktop utility alongside their game to inspect mod state and make safe changes. Retain the existing dark, restrained application palette and familiar system typography; no home-only visual language.

## Shared frame
- Shell owns the sidebar and persistent launch toolbar. Their DOM nodes, dimensions and positions remain unchanged when navigating at the same viewport size.
- Sidebar width is 224px on every page. Branding stays in the sidebar.
- The toolbar contains readiness, Launch Modded, Launch Vanilla, Troubleshoot and Profiles. It uses normal shared buttons and wraps only with viewport width.
- Only main content scrolls. Reserve its scrollbar gutter to avoid horizontal shifts.
- Do not introduce route-dependent shell classes, fixed page-owned mastheads, skewed controls or display fonts.

## Command Center
Use the shared page heading, Readiness / Operations / Activity tabs, a system list and evidence detail. Preserve real metrics, missing-data states, navigation callbacks, profile/snapshot/activity summary and launch guards. No holographic map, projection or decorative background.

Colors come from styles.css tokens: surface, surface-soft, outline, text, muted and interactive. Semantic colors indicate actual status, never decoration. Use the same 7px button corners, system font and thin borders as other pages.

## Implementation
- src/components/Shell.tsx: persistent frame.
- src/components/LaunchControls.tsx: shared launch actions.
- src/pages/HomePage.tsx: system selection, evidence and operations.
- src/command-center.css: restrained home content layout and shared toolbar.
- src/styles.css: common application tokens and controls.

## Verification
Test navigation without replacing sidebar/toolbar nodes, and compare their bounding boxes between Home, Settings and About in a real browser. Check 1440 x 900 and 760 x 600, scrolling, keyboard selection, launch guards and unknown-data states. QA fixture data is synthetic and must not be presented as real game evidence.
