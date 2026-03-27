---
phase: 07-mouse-support-e2e-test-harness
plan: 04
subsystem: ui
tags: [snapshot-testing, insta, ratatui, testbackend, regression]

# Dependency graph
requires:
  - phase: 07-mouse-support-e2e-test-harness
    plan: 01
    provides: ClickRegion types, AppState structure
  - phase: 07-mouse-support-e2e-test-harness
    plan: 02
    provides: click_regions param on all 7 screen render functions, PivTuiState, DiagnosticsTuiState
provides:
  - insta snapshot tests for all 7 screens (15 total assertions)
  - dashboard::render() decoupled from &App — now takes &AppState
  - ssh::render() decoupled from &App — parameter removed entirely
  - no-yubikey test variants for all screens that accept Option<YubiKeyState>
affects:
  - CI regression gate: any unintended rendering change will now break cargo test

# Tech tracking
tech-stack:
  added:
    - insta 1.47 (dev-dependency)
  patterns:
    - "TestBackend::new(120, 40) + Terminal::draw() for deterministic TUI snapshot testing"
    - "INSTA_UPDATE=always cargo test for first-run snap file generation"
    - "mock_yubikey_states() from model::mock as shared fixture for all screen tests"
    - "AppState::default() (empty yubikey_states) for no-yubikey state coverage"

key-files:
  created:
    - src/tui/snapshots/yubitui__tui__dashboard__tests__dashboard_default_populated.snap
    - src/tui/snapshots/yubitui__tui__dashboard__tests__dashboard_no_yubikey.snap
    - src/tui/snapshots/yubitui__tui__dashboard__tests__dashboard_context_menu_open.snap
    - src/tui/snapshots/yubitui__tui__help__tests__help_screen.snap
    - src/tui/snapshots/yubitui__tui__keys__tests__keys_default_state.snap
    - src/tui/snapshots/yubitui__tui__keys__tests__keys_no_yubikey.snap
    - src/tui/snapshots/yubitui__tui__keys__tests__keys_import_screen.snap
    - src/tui/snapshots/yubitui__tui__pin__tests__pin_default_state.snap
    - src/tui/snapshots/yubitui__tui__pin__tests__pin_no_yubikey.snap
    - src/tui/snapshots/yubitui__tui__pin__tests__pin_unblock_wizard.snap
    - src/tui/snapshots/yubitui__tui__piv__tests__piv_default_state.snap
    - src/tui/snapshots/yubitui__tui__piv__tests__piv_no_yubikey.snap
    - src/tui/snapshots/yubitui__tui__ssh__tests__ssh_main_screen.snap
    - src/tui/snapshots/yubitui__tui__ssh__tests__ssh_enable_screen.snap
    - src/tui/snapshots/yubitui__tui__diagnostics__tests__diagnostics_default.snap
  modified:
    - Cargo.toml
    - src/tui/dashboard.rs
    - src/tui/ssh.rs
    - src/tui/keys.rs
    - src/tui/help.rs
    - src/tui/pin.rs
    - src/tui/piv.rs
    - src/tui/diagnostics.rs
    - src/app.rs

key-decisions:
  - "dashboard::render() decoupled to &AppState — enables test isolation without constructing full App (terminal handle, tokio runtime, diagnostics)"
  - "ssh::render() had unused _app: &App parameter — removed entirely rather than replaced with &AppState (simpler, no data needed)"
  - "PinScreen::UnblockWizardCheck used as interactive state test variant — more realistic than ChangePin (which requires PinInputActive setup)"

requirements-completed: [TEST-04]

# Metrics
duration: 3min
completed: 2026-03-27
---

# Phase 7 Plan 04: Insta Snapshot Tests for All 7 Screens Summary

**Insta snapshot tests for all 7 TUI screens with TestBackend rendering, mock fixture, no-yubikey state coverage, and dashboard/ssh decoupled from &App**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-27T03:40:50Z
- **Completed:** 2026-03-27T03:44:02Z
- **Tasks:** 2
- **Files modified:** 9 (15 snap files created)

## Accomplishments

- Added `insta = "1.47"` to `[dev-dependencies]` in Cargo.toml
- Decoupled `dashboard::render()` from `&App` — now takes `app_state: &crate::model::AppState`
  - Uses `app_state.yubikey_count()`, `app_state.selected_yubikey_idx`, `app_state.yubikey_state()`
  - Updated `app.rs` call site to pass `&self.state`
- Removed `&App` parameter from `ssh::render()` and internal `render_main()` — was `_app` (unused)
  - Updated `app.rs` call site accordingly
- Added snapshot tests to all 7 screens (15 total `assert_snapshot!` calls):
  - Dashboard: populated, no-yubikey, context-menu-open (3 tests)
  - Keys: default, no-yubikey, import-screen (3 tests)
  - Help: help-screen (1 test)
  - PIN: default, no-yubikey, unblock-wizard (3 tests)
  - PIV: default, no-yubikey (2 tests)
  - SSH: main, enable-screen (2 tests)
  - Diagnostics: default (1 test)
- Committed all 15 `.snap` files under `src/tui/snapshots/`
- `cargo test` passes 109/109 (was 94 before plan; +15 snapshot tests)
- 0 pending `.snap.new` files

## Task Commits

Each task was committed atomically:

1. **Task 1: Add insta, decouple dashboard/ssh, snapshot tests for dashboard/keys/help** - `baae966` (feat)
2. **Task 2: Snapshot tests for pin/piv/ssh/diagnostics screens** - `2b49e6a` (feat)

## Files Created/Modified

- `Cargo.toml` — Added `insta = "1.47"` to dev-dependencies
- `src/app.rs` — Updated two call sites: dashboard render now passes `&self.state`; ssh render no longer passes `self`
- `src/tui/dashboard.rs` — Removed `use crate::app::App`; changed render signature to `app_state: &crate::model::AppState`; replaced `app.*()` calls; added 3 snapshot tests
- `src/tui/ssh.rs` — Removed `use crate::app::App`; removed `app: &App` from render() and render_main(); added 2 snapshot tests
- `src/tui/keys.rs` — Added 3 snapshot tests at end of file
- `src/tui/help.rs` — Added 1 snapshot test at end of file
- `src/tui/pin.rs` — Added 3 snapshot tests at end of file
- `src/tui/piv.rs` — Added 2 snapshot tests at end of file
- `src/tui/diagnostics.rs` — Added 1 snapshot test using `Diagnostics::default()`
- `src/tui/snapshots/` — 15 new `.snap` files committed

## Decisions Made

- Decoupled dashboard to `&AppState` (not a new arg, just different type): enables test isolation without a full `App` construct which requires terminal handles, tokio, and live diagnostics
- SSH render had zero usage of `_app` so parameter was removed entirely rather than swapped to `&AppState` — follows minimal-change principle
- Used `PinScreen::UnblockWizardCheck` as the "interactive state" variant for PIN tests (simpler to construct than `PinInputActive` which requires a fully-configured `PinInputState`)
- `Diagnostics::default()` provides all-green mock values — appropriate for a deterministic snapshot; reflects `--mock` mode behavior

## Deviations from Plan

None — plan executed exactly as written. The plan's template code was accurate; actual struct field names (`selected_yubikey_idx` as a field rather than a method, `SshScreen::EnableSSH` variant name) all matched the existing code exactly.

## Known Stubs

None. All 15 snapshot tests render real TUI content from the mock fixture or default state. No placeholders or hardcoded empty values flow to UI rendering.

## Verification

- `cargo test`: 109 passed, 0 failed
- `ls src/tui/snapshots/*.snap | wc -l`: 15 snap files
- `find . -name "*.snap.new" | wc -l`: 0 (no pending snapshots)
- `grep -r "assert_snapshot!" src/tui/*.rs | wc -l`: 15 assertions across all screens
- Dashboard render no longer references `&App` — decoupled to `&AppState`
- SSH render no longer has `&App` parameter
- All 4 screens accepting `Option<YubiKeyState>` have `_no_yubikey` test variants

---
*Phase: 07-mouse-support-e2e-test-harness*
*Completed: 2026-03-27*
