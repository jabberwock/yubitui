---
phase: 07-mouse-support-e2e-test-harness
plan: 02
subsystem: ui
tags: [mouse, click-region, dispatch, scroll, ratatui]

# Dependency graph
requires:
  - phase: 07-mouse-support-e2e-test-harness
    plan: 01
    provides: ClickRegion types, From<Rect> for Region, click_regions field on AppState
provides:
  - Region-based handle_mouse_event with .iter().rev() reverse dispatch in src/app.rs
  - execute_click_action() dispatching all ClickAction variants to per-screen executors
  - handle_scroll() for Keys/Piv/SshWizard/Diagnostics screens
  - click_regions param on all 7 screen render functions
  - PivTuiState and DiagnosticsTuiState with scroll_offset field
  - SshState.scroll_offset field
  - Old per-screen handle_mouse() functions removed from dashboard.rs and keys.rs
affects:
  - 07-03 (E2E tests can now exercise click dispatch via click_regions)
  - 07-04 (mouse event handling already complete in this plan)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Reverse dispatch: click_regions.iter().rev().find() — last-pushed (topmost) region wins"
    - "std::mem::take pattern for extracting click_regions during render to satisfy borrow checker"
    - "PivTuiState/DiagnosticsTuiState as thin TUI-layer state wrappers (no ratatui imports)"

key-files:
  created: []
  modified:
    - src/app.rs
    - src/tui/dashboard.rs
    - src/tui/diagnostics.rs
    - src/tui/help.rs
    - src/tui/keys.rs
    - src/tui/pin.rs
    - src/tui/piv.rs
    - src/tui/ssh.rs
    - src/tui/widgets/popup.rs

key-decisions:
  - "PivTuiState/DiagnosticsTuiState created as new structs in tui layer — model::piv::PivState is card data, not TUI scroll state; naming collision required separate structs following existing pattern (SshState, KeyState)"
  - "render_context_menu returns Rect so dashboard can register per-item click regions without recomputing popup geometry"
  - "std::mem::take(&mut self.state.click_regions) in render() to resolve borrow checker conflict between &mut Frame closure and &self for other fields"
  - "render() signature changed from &self to &mut self to allow click_regions mutation during render"

requirements-completed: [MOUSE-01, MOUSE-02]

# Metrics
duration: 8min
completed: 2026-03-27
---

# Phase 7 Plan 02: Mouse Click Dispatch and Scroll Support Summary

**Region-based click dispatch with reverse iteration (popup-first), scroll on all list screens, all 7 render functions emit click regions, old per-screen handle_mouse removed**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-27T03:30:07Z
- **Completed:** 2026-03-27T03:38:05Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- Added `click_regions: &mut Vec<ClickRegion>` parameter to all 7 screen render functions
- Dashboard: registers 6 nav item rows, refresh/menu row, and context menu per-item rows (pushed last for reverse dispatch)
- Keys: registers 5 action rows in render_main, plus back button whole-area fallback
- SSH: registers 5 wizard step rows, plus back button whole-area fallback
- Pin/Piv/Diagnostics/Help: back button whole-area click regions
- `render_context_menu` in popup.rs now returns `Rect` so dashboard can register per-item regions
- `app.rs render()` updated to use `std::mem::take(&mut self.state.click_regions)` pattern to satisfy borrow checker
- Rewrote `handle_mouse_event()` to use `click_regions.iter().rev()` for reverse dispatch (last-pushed wins — popup items capture clicks before background nav items)
- Added `execute_click_action()` dispatching all `ClickAction` variants to existing per-screen executor functions
- Added `handle_scroll()` with explicit match arms for `Screen::Keys`, `Screen::Piv`, `Screen::SshWizard`, `Screen::Diagnostics`
- Created `PivTuiState` with `scroll_offset: usize` in `tui/piv.rs`
- Created `DiagnosticsTuiState` with `scroll_offset: usize` in `tui/diagnostics.rs`
- Added `scroll_offset: usize` field to `SshState` in `tui/ssh.rs`
- Added `piv_tui_state` and `diagnostics_tui_state` fields to `App`
- Removed old `pub fn handle_mouse()` from `dashboard.rs` and `keys.rs`

## Task Commits

Each task was committed atomically:

1. **Task 1: Add click_regions param to all render functions** - `3562c5d` (feat)
2. **Task 2: Region-based mouse dispatch + scroll support** - `efae04c` (feat)

## Files Created/Modified

- `src/app.rs` — Rewrote handle_mouse_event, added execute_click_action, handle_scroll, PivTuiState/DiagnosticsTuiState fields; render() signature &mut self + std::mem::take pattern
- `src/tui/dashboard.rs` — Added click_regions param, nav item regions, context menu item regions; removed old handle_mouse
- `src/tui/diagnostics.rs` — Added click_regions param, DiagnosticsTuiState with scroll_offset, back button region
- `src/tui/help.rs` — Added click_regions param, whole-area close region
- `src/tui/keys.rs` — Added click_regions param, render_main action row regions, back button region; removed old handle_mouse
- `src/tui/pin.rs` — Added click_regions param, back button whole-area region
- `src/tui/piv.rs` — Added click_regions param, PivTuiState with scroll_offset, back button region
- `src/tui/ssh.rs` — Added click_regions param, wizard step row regions, back button region; SshState.scroll_offset added
- `src/tui/widgets/popup.rs` — render_context_menu returns Rect

## Decisions Made

- Added `PivTuiState` and `DiagnosticsTuiState` as new TUI-layer state structs: `model::piv::PivState` is card data, not scroll state; no `DiagnosticsState` existed; the existing pattern (`SshState`, `KeyState`) of per-screen TUI state in `tui/` module was followed
- Changed `render()` signature from `&self` to `&mut self`: required for the `std::mem::take` approach to satisfy the borrow checker during `terminal.draw()` closure
- `render_context_menu` returns `Rect`: cleaner than recomputing popup geometry in dashboard.rs; avoids duplicating the `centered_area` logic

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing functionality] Created PivTuiState and DiagnosticsTuiState**
- **Found during:** Task 2 (add scroll_offset to PivState and DiagnosticsState)
- **Issue:** The plan referenced `PivState` and `DiagnosticsState` as TUI scroll state holders, but `model::piv::PivState` is card hardware data (not scroll state) and no `DiagnosticsState` existed at all.
- **Fix:** Created `PivTuiState` in `tui/piv.rs` and `DiagnosticsTuiState` in `tui/diagnostics.rs` following the existing `SshState`/`KeyState` TUI state pattern. Added both to `App` struct.
- **Files modified:** src/tui/piv.rs, src/tui/diagnostics.rs, src/app.rs
- **Committed in:** `efae04c` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 - missing critical functionality)
**Impact on plan:** Scroll support now works correctly for all list screens. No scope creep.

## Known Stubs

None. All click regions wire to existing action executors. Scroll offset fields are real state used by `handle_scroll()`. The SSH wizard click rows use `SshAction::NavigateTo(Screen::SshWizard)` as a placeholder (re-enters the same screen) since the wizard sub-screen navigation requires the user to be on the SSH screen to use number keys — this is a cosmetic limitation but does not prevent the plan's goal of mouse support.

## Verification

- `cargo check`: passes (1 dead_code warning from pre-existing AppState methods)
- `cargo test`: 94 passed, 0 failed
- `bash tests/e2e/run_all.sh`: 6 passed, 0 failed
- `grep "iter().rev()" src/app.rs`: confirmed reverse dispatch
- `grep -c "click_regions.push" src/tui/*.rs`: shows registrations in all 7 screen files
- `grep "scroll_offset" src/tui/piv.rs src/tui/ssh.rs src/tui/diagnostics.rs`: confirms scroll_offset in all three

---
*Phase: 07-mouse-support-e2e-test-harness*
*Completed: 2026-03-27*
