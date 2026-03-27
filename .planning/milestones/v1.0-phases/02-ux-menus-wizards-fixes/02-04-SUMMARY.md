---
phase: 02-ux-menus-wizards-fixes
plan: "04"
subsystem: ui/dashboard
tags: [context-menu, popup, mouse, navigation, ux]
dependency_graph:
  requires: [02-01, 02-02, 02-03]
  provides: [dashboard-context-menu, popup-widget-module]
  affects: [src/app.rs, src/ui/dashboard.rs, src/ui/help.rs, src/ui/widgets/popup.rs]
tech_stack:
  added: [ratatui Clear widget, EnableMouseCapture/DisableMouseCapture]
  patterns: [popup-overlay, DashboardState, handle_mouse_event]
key_files:
  created:
    - src/ui/widgets/mod.rs
    - src/ui/widgets/popup.rs
  modified:
    - src/ui/dashboard.rs
    - src/app.rs
    - src/ui/help.rs
    - src/ui/mod.rs
decisions:
  - "Used centered_rect helper with Layout::Fill constraints since ratatui 0.29 Rect does not have a .centered() method"
  - "render_popup and render_confirm_dialog marked #[allow(dead_code)] as public API surface for Plans 02-02/02-03"
  - "DashboardState uses #[derive(Default)] per clippy derivable-impls guidance"
  - "Mouse click on context menu closes menu (full hit-testing deferred)"
metrics:
  duration_minutes: 15
  completed_date: "2026-03-24"
  tasks_completed: 1
  tasks_total: 2
  files_created: 2
  files_modified: 4
---

# Phase 2 Plan 4: Dashboard Context Menu Summary

Dashboard context menu with popup overlay widget system. Users can now press `m` or `Enter` on the Dashboard to open a navigable context menu with all 5 screens listed — no keybinding memorization required.

## What Was Built

### Dashboard Context Menu (Task 1)

Added the repeatedly-requested context menu to the Dashboard. The menu appears as a floating popup overlay (using ratatui's `Clear` widget + `List`) when users press `m` or `Enter` from the Dashboard.

**Navigation:**
- `m` or `Enter` on Dashboard — opens the context menu
- `Up` / `Down` arrows — move selection (yellow bold highlight)
- Mouse scroll — moves selection
- `Enter` — navigates to selected screen
- `Esc` or mouse click — closes menu without navigating

**Menu items:**
1. Diagnostics → Screen::Diagnostics
2. Key Management → Screen::Keys
3. PIN Management → Screen::PinManagement (also resets PinState)
4. SSH Setup Wizard → Screen::SshWizard
5. Help → Screen::Help

### Popup Widget Module (deviation: created as blocking dependency)

Plan 02-01 was the designated plan to create `src/ui/widgets/popup.rs`, but this plan ran in parallel before 02-01 committed. Created the widget module here as a Rule 3 auto-fix (blocking dependency).

Three public functions in `src/ui/widgets/popup.rs`:
- `render_popup` — generic centered popup with title and body text
- `render_confirm_dialog` — confirmation dialog with [Y]es/[N]o prompt, destructive styling in red
- `render_context_menu` — floating list menu with selection highlight

### Mouse Support

Added `EnableMouseCapture` / `DisableMouseCapture` to `App::run()` and a new `handle_mouse_event` method. Scroll events navigate both the dashboard context menu and the Keys import list.

### Help Screen Update

Added "m / Enter — Open navigation menu (Dashboard)" to the Global section of the help screen.

## Verification

- `cargo check` — passed
- `cargo clippy -- -D warnings` — passed
- `cargo test` — 0 tests, no failures
- `cargo fmt -- --check` — passed

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Created popup widget module (02-01 dependency not yet committed)**
- **Found during:** Task 1 setup — `src/ui/widgets/popup.rs` did not exist
- **Issue:** Plan 02-04 depends on 02-01 which creates the popup widget, but parallel execution means 02-01 had not committed when this plan ran
- **Fix:** Created `src/ui/widgets/mod.rs` and `src/ui/widgets/popup.rs` with all three exported functions matching the interface specified in the 02-01 plan
- **Files modified:** src/ui/widgets/mod.rs (new), src/ui/widgets/popup.rs (new), src/ui/mod.rs
- **Commit:** 4b3ca983

**2. [Rule 1 - Bug] ratatui 0.29 does not have Rect::centered()**
- **Found during:** Implementation — plan referenced `area.centered()` method
- **Fix:** Implemented `centered_rect()` helper using `Layout::Fill` constraints (ratatui 0.29 pattern)
- **Files modified:** src/ui/widgets/popup.rs

**3. [Rule 1 - Bug] clippy derivable-impls on DashboardState**
- **Found during:** `cargo clippy -- -D warnings`
- **Fix:** Replaced manual `impl Default` with `#[derive(Default)]`
- **Files modified:** src/ui/dashboard.rs

## Checkpoint: Task 2 Visual Verification

Task 2 was a `checkpoint:human-verify`. Config `auto_advance: true` — auto-approved.

Built features available for manual verification:
1. Dashboard context menu — press `m` or `Enter`, navigate with Up/Down, activate with Enter, close with Esc
2. Mouse scroll on context menu moves selection
3. Help screen — `?` shows "m / Enter  Open navigation menu (Dashboard)"

## Self-Check: PASSED
