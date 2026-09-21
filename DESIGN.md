---
name: Zero Mod Manager
description: A restrained tactical desktop interface for safe Zero Company mod management.
colors:
  void: "#090d12"
  surface: "#10161e"
  surface-raised: "#151d27"
  surface-soft: "#1a2430"
  outline: "#263342"
  outline-strong: "#39495b"
  text: "#e8edf3"
  muted: "#8f9baa"
  command-gold: "#e4b85c"
  command-gold-hover: "#f2c86d"
  success: "#60d394"
  warning: "#f0b95b"
  error: "#f17373"
  focus: "#8bc5ff"
typography:
  headline:
    fontFamily: "Inter, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.75rem"
    fontWeight: 700
    lineHeight: 1.15
    letterSpacing: "-0.025em"
  title:
    fontFamily: "Inter, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.2rem"
    fontWeight: 700
  body:
    fontFamily: "Inter, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "Inter, ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.78rem"
    fontWeight: 650
rounded:
  control: "7px"
  panel: "9px"
  feature: "12px"
  pill: "20px"
spacing:
  xs: "5px"
  sm: "9px"
  md: "14px"
  lg: "20px"
  xl: "24px"
components:
  button-primary:
    backgroundColor: "{colors.command-gold}"
    textColor: "#17120a"
    rounded: "{rounded.control}"
    padding: "0 14px"
    height: "38px"
  button-secondary:
    backgroundColor: "{colors.surface-soft}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "0 14px"
    height: "38px"
  panel:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.panel}"
    padding: "24px"
  input:
    backgroundColor: "#0a1016"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "0 11px"
    height: "40px"
---

# Design System: Zero Mod Manager

## 1. Overview

**Creative North Star: "The Calm Operations Console"**

The interface is a compact desktop work surface for users who need to understand and reverse filesystem changes. Deep blue-black tonal layers separate navigation, content, and inspection surfaces; restrained command gold marks primary intent. Familiar controls and dense, legible lists take priority over decorative novelty.

The 0.6.5 information architecture is the continuity baseline. New recovery and diagnostic states must look native to that system, not like a redesign. The system rejects neon sci-fi decoration, glass effects, indefinite loading screens, and motion without operational meaning.

**Key Characteristics:**

- Restrained dark tonal hierarchy
- Gold reserved for commands and current selection
- Explicit semantic health states with text and icons
- Dense but predictable desktop layouts
- Reversible actions and visible recovery paths

## 2. Colors

The palette uses cold near-black surfaces and one warm command accent, with semantic colors reserved for real status.

### Primary

- **Command Gold** (`#e4b85c`): primary actions, active navigation cues, install progress, and actionable links.
- **Command Gold Hover** (`#f2c86d`): hover feedback for gold actions only.

### Neutral

- **Void** (`#090d12`): application background.
- **Operations Surface** (`#10161e`): primary panels and lists.
- **Raised Console** (`#151d27`): raised panels and tool regions.
- **Soft Control** (`#1a2430`): secondary controls and selected rows.
- **Steel Outline** (`#263342`): standard dividers and borders.
- **Signal Text** (`#e8edf3`): primary content.
- **Muted Telemetry** (`#8f9baa`): secondary content and metadata.

### Named Rules

**The Command Accent Rule.** Gold marks something the user can do or the currently selected context. It is not decoration.

**The Verifiable State Rule.** Success, warning, and error colors appear with a text label or icon and only when that state is known.

## 3. Typography

**Display Font:** Inter with system sans-serif fallbacks  
**Body Font:** Inter with system sans-serif fallbacks  
**Label/Mono Font:** `ui-monospace` for paths, filenames, hashes, and diagnostic payloads

**Character:** Neutral, compact, and native to a desktop utility. Weight and size establish hierarchy; labels do not rely on decorative type.

### Hierarchy

- **Headline** (700, `1.75rem`, 1.15): page titles and recovery-state titles.
- **Title** (700, `1.2rem`): panel and dialog titles.
- **Body** (400, `1rem`, 1.5): instructions and explanations, normally capped near 70 characters.
- **Label** (650, `0.78rem`): control labels, status text, and compact metadata.
- **Eyebrow** (800, `0.68rem`, `0.17em`): short uppercase section context only.

### Named Rules

**The Scan First Rule.** Tables and status rows use concise labels; supporting explanation follows only where a decision needs it.

## 4. Elevation

The system is flat by default. Depth comes from tonal layering and one-pixel outlines. Shadows are reserved for transient overlays, toasts, sticky change bars, and dialogs that must visibly sit above the work surface.

### Shadow Vocabulary

- **Transient Overlay** (`0 14px 38px #0009`): toasts and temporary feedback.
- **Blocking Dialog** (`0 28px 80px #000c`): modal workflows that prevent interaction with the underlying page.

### Named Rules

**The Flat Until Necessary Rule.** Persistent panels do not use shadows. Elevation must communicate stacking or temporary state.

## 5. Components

### Buttons

- **Shape:** compact rounded rectangle (`7px`) with a minimum height of `38px`.
- **Primary:** Command Gold background, dark text, `0 14px` padding, weight 700.
- **Hover / Focus:** gold lightens on hover; every control uses the `#8bc5ff` two-pixel focus ring.
- **Secondary:** soft surface with steel outline; destructive intent appears only on hover or confirmation.

### Chips

- **Style:** muted text, soft blue-black surface, one-pixel border, `20px` pill radius.
- **State:** chips describe type or state; they are not used as decorative badges.

### Cards / Containers

- **Corner Style:** `9px` for panels, `12px` only for large drop zones.
- **Background:** Operations Surface or Raised Console.
- **Shadow Strategy:** none at rest.
- **Border:** one-pixel Steel Outline.
- **Internal Padding:** typically `20px` to `24px`.

### Inputs / Fields

- **Style:** near-black input surface, Steel Outline, `6px` to `7px` radius, `40px` minimum height.
- **Focus:** two-pixel blue focus ring and, where useful, Command Gold border.
- **Error / Disabled:** semantic text plus state icon; disabled opacity never replaces an accessible label.

### Navigation

The left rail remains `224px` on the desktop layout. Items use transparent backgrounds at rest, a soft tonal fill when active, and a narrow Command Gold inset marker. Navigation labels remain visible; icons never stand alone.

### Recovery State

Startup recovery replaces an indefinite splash with a centered, bounded explanation, one primary repair action, secondary retry/log actions, and a sanitized technical detail disclosure.

## 6. Do's and Don'ts

### Do:

- **Do** preserve the 0.6.5 navigation order, spacing rhythm, and familiar control vocabulary.
- **Do** use `#e4b85c` only for primary commands, selection, and actionable links.
- **Do** pair every health color with an icon and descriptive text.
- **Do** expose retry, repair, logs, and rollback actions adjacent to the failure they address.
- **Do** respect reduced-motion preferences and maintain visible keyboard focus.

### Don't:

- **Don't** turn the utility into a decorative game launcher or marketing surface.
- **Don't** redesign familiar 0.6.5 navigation and workflows for novelty.
- **Don't** hide errors behind an indefinite loader, vague toast, or silent fallback.
- **Don't** use neon sci-fi decoration, glassmorphism, excessive gradients, gradient text, or gratuitous motion.
- **Don't** use colored side-stripe borders on cards or callouts; the existing active navigation inset is the sole navigation-state exception.
- **Don't** imply official upstream endorsement or reuse the original logo without written permission.
