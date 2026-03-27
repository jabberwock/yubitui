---
phase: 08-textual-rs-migration
plan: 03
subsystem: tui-screens
tags: [textual-rs, widget-migration, diagnostics, piv, ssh, footer, key-bindings]

dependency_graph:
  requires:
    - phase: 08-02
      provides: "textual-rs App runner, HelpScreen Widget pattern, theme/config infrastructure"
  provides:
    - "DiagnosticsScreen as textual-rs Widget with Header/Footer/keybindings"
    - "PivScreen as textual-rs Widget with slot status display and Footer"
    - "SshWizardScreen as textual-rs Widget with all 6 sub-screens as Reactive state"
  affects: [08-04, 08-05, 08-06, all-subsequent-screen-migrations]

tech-stack:
  added: []
  patterns:
    - "Widget compose() with Header + Label content + Footer — established for all info/status screens"
    - "Reactive<*State> for internal sub-screen navigation in SshWizardScreen"
    - "on_action() back/step navigation via ctx.pop_screen_deferred() and state.update()"

key-files:
  modified:
    - src/tui/diagnostics.rs
    - src/tui/piv.rs
    - src/tui/ssh.rs
  deleted:
    - src/tui/snapshots/yubitui__tui__diagnostics__tests__diagnostics_default.snap
    - src/tui/snapshots/yubitui__tui__piv__tests__piv_default_state.snap
    - src/tui/snapshots/yubitui__tui__piv__tests__piv_no_yubikey.snap
    - src/tui/snapshots/yubitui__tui__ssh__tests__ssh_enable_screen.snap
    - src/tui/snapshots/yubitui__tui__ssh__tests__ssh_main_screen.snap

key-decisions:
  - "SshWizardScreen retains all 6 sub-screens (Main, EnableSSH, ConfigureShell, RestartAgent, ExportKey, TestConnection) as Reactive<SshState>.screen — no push_screen_deferred used for sub-screens; keeps migration minimal and SshState serializable (D-04)"
  - "DiagnosticsScreen uses full-width layout (no sidebar) per plan guidance — all diagnostic items in one list is more readable than category sidebar for this small screen"
  - "PivTuiState, DiagnosticsTuiState, SshState, PivAction, DiagnosticsAction, SshAction enums all preserved intact (D-04)"
  - "Old insta snapshots deleted for replaced tests; new tests use TestApp::new pattern (D-09)"

requirements-completed: [INFRA-03]

duration: ~30min
completed: 2026-03-27
---

# Phase 8 Plan 03: Diagnostics, PIV, and SSH Screen Migration Summary

**Diagnostics, PIV, and SSH screens migrated to textual-rs Widgets with Header, Footer, and visible keybindings — 4 of 7 screens now migrated**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-03-27T13:35:00Z
- **Completed:** 2026-03-27T13:40:49Z
- **Tasks:** 2
- **Files modified:** 3 (all rewrites), 5 snapshot files deleted

## Accomplishments

- `DiagnosticsScreen` implements `Widget` with `compose()` building Header + diagnostic status Labels + Footer; key_bindings declares Esc=back and R=run_diagnostics
- `PivScreen` implements `Widget` with PIV slot occupancy status Labels; key_bindings declares Esc=back, V=view_slot, R=refresh per UI-SPEC
- `SshWizardScreen` implements `Widget` with `Reactive<SshState>` driving all 6 sub-screen variants; key_bindings declares Esc=back, A=add_to_agent, R=refresh, and number keys 1-5 for wizard steps
- All three files: zero hardcoded `Color::` values, no `fn render(frame: &mut Frame, ...)` functions remaining
- 5 old insta snapshots deleted (old ratatui-direct tests replaced by TestApp::new tests)
- 110 tests pass (up from 109 in Plan 02 — 2 new tests added per screen = +6 new tests, but some old snapshot tests removed = net +1)

## Task Commits

1. **Task 1: Migrate Diagnostics screen to textual-rs Widget** - `fc1c515` (feat)
2. **Task 2: Migrate PIV and SSH screens to textual-rs Widgets** - `a8dd739` (feat)

## Files Created/Modified

- `src/tui/diagnostics.rs` — rewritten as `DiagnosticsScreen` implementing `Widget`
- `src/tui/piv.rs` — rewritten as `PivScreen` implementing `Widget`
- `src/tui/ssh.rs` — rewritten as `SshWizardScreen` implementing `Widget`
- 5 old insta snapshot files deleted

## Decisions Made

- `SshWizardScreen` retains all 6 sub-screens as `Reactive<SshState>.screen` rather than using `push_screen_deferred` for each step. The SSH wizard's sub-screens are lightweight info+confirm screens — internal reactive state is simpler and keeps `SshState` fully serializable per D-04. `push_screen_deferred` is reserved for heavier modal flows (key generation wizard, PIN input dialogs) in Plans 05-06.
- `DiagnosticsScreen` uses full-width content (no sidebar) per plan guidance: "If no sidebar is natural for diagnostics (all items in one list), use full-width like Help." The 4 diagnostic categories (PC/SC, GPG, SSH, scdaemon) flow naturally in a sequential list.
- `PivTuiState` needed `#[derive(PartialEq)]` in addition to `Default, Clone` — `Reactive<T>` requires `T: PartialEq`. Added as a Rule 1 auto-fix (same issue was present for `DiagnosticsTuiState`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Reactive<T> requires T: PartialEq — missing from *TuiState derives**
- **Found during:** Task 1 (DiagnosticsScreen), same applied in Task 2 (PivTuiState)
- **Issue:** `Reactive::<T>::new()` requires `T: Clone + PartialEq + Send + Sync + 'static`. The existing `DiagnosticsTuiState` and `PivTuiState` only had `#[derive(Default, Clone)]`.
- **Fix:** Added `PartialEq` to both derives. `SshState` also needed `PartialEq` added (it previously had no derive at all).
- **Files modified:** src/tui/diagnostics.rs, src/tui/piv.rs, src/tui/ssh.rs
- **Committed in:** fc1c515 (Task 1), a8dd739 (Task 2)

---

**Total deviations:** 1 auto-fixed (Rule 1 bug — missing trait bound)
**Impact on plan:** Required for correctness. No scope creep.

## Known Stubs

- `DiagnosticsScreen.on_action("run_diagnostics")`: calls `ctx.pop_screen_deferred()` as placeholder — full async re-run of diagnostics will be wired in a subsequent plan when the RootScreen is built and can re-construct `DiagnosticsScreen` with fresh data.
- `PivScreen.on_action("refresh")`: calls `ctx.pop_screen_deferred()` as placeholder — same pattern as above.
- `SshWizardScreen.on_action("add_to_agent" | "execute" | "refresh")`: calls `ctx.pop_screen_deferred()` — SSH operations (gpg-agent manipulation) are app-level side effects that need wiring through the app runner in subsequent plans.

These stubs do not prevent the plan's goal from being achieved — the plan's goal is screen migration (Widget trait implementation with Header/Footer/keybindings), not full SSH/PIV operation wiring. The stubs correctly document what future plans (08-04 through 08-06) will implement.

## Next Phase Readiness

- Pattern now established for 4 of 7 screens (Help, Diagnostics, PIV, SSH)
- 110 tests passing, no regressions
- Plans 08-04 (Pin screen migration) and 08-05 (Dashboard migration) can proceed

---
*Phase: 08-textual-rs-migration*
*Completed: 2026-03-27*

## Self-Check: PASSED

| Check | Result |
|-------|--------|
| src/tui/diagnostics.rs exists | FOUND |
| src/tui/piv.rs exists | FOUND |
| src/tui/ssh.rs exists | FOUND |
| 08-03-SUMMARY.md exists | FOUND |
| Commit fc1c515 (Task 1) | FOUND |
| Commit a8dd739 (Task 2) | FOUND |
| impl Widget for DiagnosticsScreen | FOUND |
| impl Widget for PivScreen | FOUND |
| impl Widget for SshWizardScreen | FOUND |
| cargo check | PASSES (0 errors) |
| cargo test | 110 passed, 0 failed |
