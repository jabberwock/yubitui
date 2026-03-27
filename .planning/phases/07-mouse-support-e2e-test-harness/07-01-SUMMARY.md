---
phase: 07-mouse-support-e2e-test-harness
plan: 01
subsystem: ui
tags: [mouse, click-region, ratatui, crossterm, appstate, conpty, windows]

# Dependency graph
requires:
  - phase: 06-tech-debt-infrastructure
    provides: per-screen action enums (DashboardAction, KeyAction, etc.) and AppState structure
provides:
  - Region, ClickAction, ClickRegion types in src/model/click_region.rs (zero ratatui imports)
  - Clone+Debug derives on all 7 action enums
  - click_regions: Vec<ClickRegion> field on AppState with serde(skip)
  - yubikey_state() and yubikey_count() convenience methods on AppState
  - From<ratatui::layout::Rect> for Region impl in src/tui/mod.rs
  - Windows ConPTY graceful degradation for EnableMouseCapture/DisableMouseCapture
affects:
  - 07-02 (wires click regions into screen render functions)
  - 07-03 (E2E test harness that uses ClickRegion types)
  - 07-04 (mouse event dispatch uses ClickAction)

# Tech tracking
tech-stack:
  added: []
  patterns: [ClickRegion dispatch pattern, model-layer region types with tui-layer Rect conversion boundary]

key-files:
  created:
    - src/model/click_region.rs
  modified:
    - src/model/mod.rs
    - src/model/app_state.rs
    - src/tui/mod.rs
    - src/tui/dashboard.rs
    - src/tui/keys.rs
    - src/tui/pin.rs
    - src/tui/piv.rs
    - src/tui/ssh.rs
    - src/tui/diagnostics.rs
    - src/tui/help.rs
    - src/app.rs

key-decisions:
  - "ClickAction placed in src/model/click_region.rs referencing tui action enums — cross-layer reference is valid within a single Rust crate; CI lint only forbids ratatui imports in model, not action enum imports"
  - "Added Debug derive alongside Clone to all action enums — required because ClickAction derives Debug and wraps them"
  - "From<Rect> for Region placed in src/tui/mod.rs as the sole ratatui conversion boundary (per D-08)"
  - "EnableMouseCapture wrapped in if-let-Err so Windows ConPTY failures are logged at debug level and do not crash startup"
  - "DisableMouseCapture cleanup uses let _ = to silently ignore failure on ConPTY environments"

patterns-established:
  - "Region type is ratatui-free; conversion from Rect happens only in the tui layer"
  - "ClickRegion stores (Region, ClickAction) pairs; AppState.click_regions holds current frame's hit regions"

requirements-completed: [MOUSE-03, MOUSE-04]

# Metrics
duration: 4min
completed: 2026-03-26
---

# Phase 7 Plan 01: ClickRegion Type Infrastructure Summary

**Region/ClickRegion/ClickAction types in model layer with zero ratatui imports, Clone+Debug on all action enums, From<Rect> conversion boundary in tui layer, and Windows ConPTY graceful degradation**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-26T05:59:40Z
- **Completed:** 2026-03-26T06:02:51Z
- **Tasks:** 2
- **Files modified:** 11 (1 created, 10 modified)

## Accomplishments
- Created `src/model/click_region.rs` with `Region`, `ClickAction`, and `ClickRegion` types — zero ratatui imports
- Added `#[derive(Clone, Debug)]` to all 7 action enums across tui screens
- Added `click_regions: Vec<ClickRegion>` with `#[serde(skip)]` to `AppState`, plus convenience methods
- Added `From<ratatui::layout::Rect> for Region` in `src/tui/mod.rs` as the sole Rect conversion boundary
- Split `EnableMouseCapture` into a gracefully-failing call for Windows ConPTY compatibility

## Task Commits

Each task was committed atomically:

1. **Task 1: ClickRegion types + Clone on action enums + AppState field** - `74e78aa` (feat)
2. **Task 2: From<Rect> conversion + ConPTY graceful degradation** - `dc46d2b` (feat)

## Files Created/Modified
- `src/model/click_region.rs` - New: Region (contains/hit-test), ClickAction (wraps all screen actions), ClickRegion (region+action pair)
- `src/model/mod.rs` - Added `pub mod click_region`
- `src/model/app_state.rs` - Added `click_regions` field, `yubikey_state()` and `yubikey_count()` methods
- `src/tui/mod.rs` - Added `From<ratatui::layout::Rect> for crate::model::click_region::Region`
- `src/tui/dashboard.rs` - Added `#[derive(Clone, Debug)]` to `DashboardAction`
- `src/tui/keys.rs` - Added `#[derive(Clone, Debug)]` to `KeyAction`
- `src/tui/pin.rs` - Added `#[derive(Clone, Debug)]` to `PinAction`
- `src/tui/piv.rs` - Added `#[derive(Clone, Debug)]` to `PivAction`
- `src/tui/ssh.rs` - Added `#[derive(Clone, Debug)]` to `SshAction`
- `src/tui/diagnostics.rs` - Added `#[derive(Clone, Debug)]` to `DiagnosticsAction`
- `src/tui/help.rs` - Added `#[derive(Clone, Debug)]` to `HelpAction`
- `src/app.rs` - Separated EnableMouseCapture with error handling; separated DisableMouseCapture cleanup

## Decisions Made
- Added `Debug` derive alongside `Clone` to all action enums: required because `ClickAction` derives `Debug` and wraps the variant types; the plan only specified `Clone` but `Debug` was required for compilation (auto-fixed under Rule 1)
- `ClickAction` placed in `src/model/` referencing `src/tui/` types: within a single Rust crate, this cross-layer reference compiles without circular dependency issues; the CI lint only restricts ratatui imports in model, not action enum imports from tui

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added Debug derive to all action enums**
- **Found during:** Task 1 (ClickRegion types creation)
- **Issue:** `ClickAction` derives `Debug`, which requires all wrapped types to also implement `Debug`. The plan only specified adding `Clone` to action enums.
- **Fix:** Added `Debug` alongside `Clone` in `#[derive(Clone, Debug)]` for all 7 action enums.
- **Files modified:** src/tui/dashboard.rs, keys.rs, pin.rs, piv.rs, ssh.rs, diagnostics.rs, help.rs
- **Verification:** `cargo check` passed with zero errors
- **Committed in:** `74e78aa` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug fix)
**Impact on plan:** Essential for compilation. No scope creep.

## Issues Encountered
- Worktree branch was 60 commits behind main (at phase 5 wip commit). Reset with `git reset --hard main` before starting work, since all phase 5/6 work had landed on main.

## Next Phase Readiness
- All prerequisite types for mouse hit-testing are in place
- Plan 02 can wire `From::<Rect>::from(rect)` to produce `ClickRegion` entries during render
- Plan 03 (E2E test harness) can reference `ClickRegion` and `Region` types for test utilities
- No blockers

---
*Phase: 07-mouse-support-e2e-test-harness*
*Completed: 2026-03-26*
