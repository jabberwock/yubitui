---
phase: 02-ux-menus-wizards-fixes
plan: "01"
subsystem: ui-foundation
tags: [popup-widgets, mouse-support, gnupg-path, cross-platform, ratatui]
dependency_graph:
  requires: []
  provides: [popup-widgets, mouse-capture, gnupg-home-authoritative]
  affects: [src/ui/widgets, src/app.rs, src/utils/config.rs, src/diagnostics/ssh_agent.rs, src/yubikey/ssh_operations.rs, src/diagnostics/scdaemon.rs]
tech_stack:
  added: []
  patterns: [Layout-based centering for popups, gpgconf-based path resolution, MouseEvent dispatch]
key_files:
  created:
    - src/ui/widgets/mod.rs
    - src/ui/widgets/popup.rs
  modified:
    - src/ui/mod.rs
    - src/app.rs
    - src/utils/config.rs
    - src/diagnostics/ssh_agent.rs
    - src/yubikey/ssh_operations.rs
    - src/diagnostics/scdaemon.rs
decisions:
  - "Used Layout-based centering instead of area.centered() -- ratatui 0.29 does not have centered() on Rect (only on text/paragraph types)"
  - "Added #![allow(dead_code)] to popup.rs module level -- public API functions unused until downstream plans wire them in"
  - "gpgconf --list-dirs homedir is authoritative source for all gnupg path resolution"
metrics:
  duration_minutes: 15
  completed_date: "2026-03-24T19:18:44Z"
  tasks_completed: 3
  tasks_total: 3
  files_changed: 7
---

# Phase 2 Plan 01: Foundation Infrastructure Summary

**One-liner:** Popup/menu overlay widgets, mouse capture with scroll navigation, and gpgconf-authoritative gnupg path resolution replacing three duplicated hardcoded implementations.

## What Was Built

### Task 1: Popup/Menu Widget Module

Created `src/ui/widgets/popup.rs` with three reusable rendering functions:
- `render_popup` — generic centered popup with title and body text
- `render_confirm_dialog` — [Y]/[N] confirmation with optional WARNING/destructive styling
- `render_context_menu` — floating list with yellow-bold selection highlight

All functions use `Clear` before rendering to avoid visual artifacts. Centered using a two-pass `Layout` approach (vertical + horizontal) since `Rect::centered()` does not exist in ratatui 0.29.

Module declared as `pub mod widgets` in `src/ui/mod.rs`.

### Task 2: Mouse Capture and Event Handling

In `src/app.rs`:
- Added `EnableMouseCapture` to terminal setup `execute!` call
- Added `DisableMouseCapture` to terminal restore `execute!` call
- Extended `handle_events()` to dispatch `Event::Mouse` to new `handle_mouse_event()`
- `handle_mouse_event()` handles `ScrollUp`/`ScrollDown` for ImportKey screen list navigation, left-click placeholder for Plan 02-03

### Task 3: gnupg_home() Unification

Replaced three separate gnupg path implementations with a single authoritative `gnupg_home()` in `src/utils/config.rs`:
- Priority 1: `$GNUPGHOME` environment variable
- Priority 2: `gpgconf --list-dirs homedir` (correct on all platforms including GPG4Win)
- Priority 3: Windows `%APPDATA%/gnupg` fallback
- Priority 4: Unix `~/.gnupg` fallback

Added `gpg_agent_conf()` and `scdaemon_conf()` convenience helpers. Removed all `#[allow(dead_code)]` attributes.

Fixed all three callers:
- `src/diagnostics/ssh_agent.rs`: now calls `config::gpg_agent_conf()`
- `src/yubikey/ssh_operations.rs`: `get_gpg_agent_conf_path()` delegates to `config::gnupg_home()`
- `src/diagnostics/scdaemon.rs`: now calls `config::scdaemon_conf()`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - API Mismatch] area.centered() does not exist on Rect in ratatui 0.29**
- **Found during:** Task 1 implementation
- **Issue:** Plan specified `area.centered(Constraint::Percentage(width_pct), Constraint::Length(height))` but this method exists only on `Text`/`Line`/`Paragraph` types, not on `Rect`
- **Fix:** Used two-pass `Layout::default()` centering (vertical constraints then horizontal constraints on the vertical mid-band)
- **Files modified:** `src/ui/widgets/popup.rs`
- **Commit:** 076b6c0c

**2. [Rule 2 - Clippy compliance] Single-match patterns in handle_mouse_event**
- **Found during:** Task 2 clippy check
- **Issue:** `match self.current_screen { Screen::Keys => ... _ => {} }` triggered `clippy::single-match`
- **Fix:** Replaced with `if self.current_screen == Screen::Keys && ...` guards
- **Files modified:** `src/app.rs`
- **Commit:** 11bfecc6

## Known Stubs

None. All functions are fully implemented and compilable. The left-click handler in `handle_mouse_event` has a `// no-op` comment with explicit documentation that it will be wired in Plan 02-03 — this is intentional and documented in the plan.

## Verification Results

- `cargo check`: PASSED
- `cargo clippy -- -D warnings`: PASSED
- `cargo test`: PASSED (0 regressions)
- `grep -rn "home_dir.*\.gnupg" src/`: 0 matches

## Self-Check: PASSED
