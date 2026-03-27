---
phase: 08-textual-rs-migration
plan: 05
subsystem: tui-screens
tags: [textual-rs, dashboard, keys, widget-migration, keygen-wizard, push-screen, root-screen]

dependency_graph:
  requires:
    - phase: 08-03
      provides: "DiagnosticsScreen, PivScreen, SshWizardScreen as textual-rs Widgets"
    - phase: 08-04
      provides: "PinManagementScreen Widget, PopupScreen/ConfirmScreen widgets, push_screen_deferred patterns"
  provides:
    - "DashboardScreen as textual-rs Widget — root screen with 6 navigation Buttons"
    - "KeysScreen as textual-rs Widget with key slot display and action Buttons"
    - "KeyGenWizardScreen — 7-step wizard as pushed screen (not inline state)"
    - "ImportKeyScreen — import flow as pushed screen"
    - "TouchPolicyScreen — slot selector as pushed screen"
    - "KeyDetailScreen — generic operation info screen"
    - "ProgressLabel — spinner widget ported to textual-rs"
    - "src/app.rs: DashboardScreen::new() wired as root (replaces HelpScreen)"
  affects: [08-06, all-7-screens-complete]

tech-stack:
  added: []
  patterns:
    - "Root screen wiring: App::new(|| Box::new(DashboardScreen::new(app_state, diagnostics)))"
    - "Navigation: on_action() push_screen_deferred for all 6 screen transitions from Dashboard"
    - "Multi-step wizard as pushed screen: KeyGenWizardScreen with internal RefCell<KeyGenWizard> state machine"
    - "Global quit/theme handled by textual-rs App runner (not by on_action) — no ctx.exit()/set_theme needed"

key-files:
  created: []
  modified:
    - src/tui/dashboard.rs
    - src/tui/keys.rs
    - src/tui/widgets/progress.rs
    - src/app.rs
  deleted:
    - src/tui/snapshots/yubitui__tui__dashboard__tests__dashboard_context_menu_open.snap
    - src/tui/snapshots/yubitui__tui__dashboard__tests__dashboard_default_populated.snap
    - src/tui/snapshots/yubitui__tui__dashboard__tests__dashboard_no_yubikey.snap
    - src/tui/snapshots/yubitui__tui__keys__tests__keys_default_state.snap
    - src/tui/snapshots/yubitui__tui__keys__tests__keys_import_screen.snap
    - src/tui/snapshots/yubitui__tui__keys__tests__keys_no_yubikey.snap

key-decisions:
  - "textual-rs App runner handles 'q' quit and Ctrl+T theme cycling globally — DashboardScreen on_action does NOT implement 'quit' or 'cycle_theme' actions; would require ctx.set_theme(&mut self) which conflicts with on_action(&AppContext)"
  - "KeyState.pin_input removed from struct — not needed in new textual-rs model where PIN input is a pushed screen; avoids PinInputState: Clone bound violation without touching model code"
  - "KeyGenWizardScreen uses RefCell<KeyGenWizard> for internal step state — not Reactive<T> (which would require PartialEq on KeyGenWizard containing nested Option<T>s)"
  - "Old insta snapshot tests deleted — tested old ratatui render() functions which no longer exist; replaced by TestApp pilot tests using app.buffer() assertion"
  - "DashboardAction/DashboardState/KeyAction/KeyState/KeyScreen all preserved per D-04 — Tauri serialization compatibility maintained"

requirements-completed: [INFRA-03]

duration: ~10min
completed: 2026-03-27
---

# Phase 8 Plan 05: Dashboard and Keys Screen Migration Summary

**Dashboard wired as root screen with 6 push_screen_deferred navigation buttons; Keys screen and all 7 sub-flows (KeyGenWizard, Import, Delete, TouchPolicy) migrated to textual-rs Widgets — all 7 screens now migrated**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-03-27T14:00:00Z
- **Completed:** 2026-03-27T14:10:00Z
- **Tasks:** 2
- **Files modified:** 4 modified, 6 deleted

## Accomplishments

- `DashboardScreen` implements `Widget`: Header("yubitui -- YubiKey Management"), device status Labels, 6 navigation Buttons, Footer with keybindings declared per UI-SPEC
- `src/app.rs` updated: `DashboardScreen::new(app_state, diagnostics)` replaces `HelpScreen::new()` as root screen — app now boots to Dashboard
- All 7 screens migrated: Help (08-01), Diagnostics/PIV/SSH (08-03), PIN (08-04), Dashboard/Keys (08-05)
- `KeysScreen` implements `Widget` with key slot status Labels, 7 action Buttons, Footer
- `KeyGenWizardScreen` — 7-step key generation wizard as a pushed screen with internal state machine
- `ImportKeyScreen`, `TouchPolicyScreen`, `KeyDetailScreen` — operation sub-flows as pushed screens
- `ProgressLabel` widget ported to textual-rs; old `render_progress_popup` kept as dead-code legacy stub
- No hardcoded `Color::` values in any migrated file
- Old render() and handle_key() free functions deleted from both files
- 6 old insta snapshot files deleted; replaced by TestApp pilot tests
- 108 tests pass, 0 failures

## Task Commits

1. **Task 1: Migrate Dashboard screen and wire as root** — `5c37225` (feat)
2. **Task 2: Migrate Keys screen to textual-rs Widget** — `9bf6438` (feat)

## Files Created/Modified

- `src/tui/dashboard.rs` — rewritten as `DashboardScreen` textual-rs Widget; old render/handle_key deleted
- `src/tui/keys.rs` — rewritten as `KeysScreen` + `KeyGenWizardScreen` + `ImportKeyScreen` + `TouchPolicyScreen` + `KeyDetailScreen` + `ProgressLabel`
- `src/tui/widgets/progress.rs` — legacy `render_progress_popup` retained as dead-code stub; no StatefulWidget impl
- `src/app.rs` — DashboardScreen::new() as root; _app_state prefix removed (now used)

## Decisions Made

- **textual-rs App runner handles quit/theme globally:** `AppContext.set_theme()` takes `&mut self` but `on_action` receives `&AppContext` (shared reference). Dashboard does not re-implement Ctrl+T or q in `on_action` — textual-rs handles them at the App runner level. This matches the anti-pattern note in RESEARCH.md.

- **KeyState.pin_input removed:** The old `KeyState` included `pub pin_input: Option<PinInputState>` from the imperative event loop design. In the textual-rs model, PIN input is a pushed `PinInputWidget` screen — not inline state. Removing `pin_input` eliminates the `PinInputState: Clone` bound violation without touching model code (D-03 compliant).

- **RefCell vs Reactive for wizard state:** `KeyGenWizardScreen` uses `RefCell<KeyGenWizard>` rather than `Reactive<KeyGenWizard>`. `Reactive<T>` requires `T: PartialEq` — `KeyGenWizard` contains `Option<T>` fields that would require PartialEq cascading into model types. `RefCell` avoids the constraint since wizard screen doesn't need reactive subscriptions.

- **Old insta snapshots deleted:** 6 snapshot files for dashboard and keys tested old ratatui `render()` functions that no longer exist. Following the same pattern established in Plans 03 and 04.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] KeyState.pin_input: PinInputState does not implement Clone**
- **Found during:** Task 2 (cargo check)
- **Issue:** New `KeyState` derives `Clone`, but `PinInputState` has no `Clone` impl. PinInputState is in TUI layer but implementing Clone would be wasted effort since pin_input is not needed in the new textual-rs model.
- **Fix:** Removed `pub pin_input` field from `KeyState`. PIN input is now always a pushed `PinInputWidget` screen, consistent with how PinManagementScreen (Plan 04) handles it.
- **Files modified:** src/tui/keys.rs
- **Committed in:** 9bf6438 (Task 2 commit)

**2. [Rule 1 - Bug] ctx.set_theme() requires &mut AppContext, unavailable in on_action(&AppContext)**
- **Found during:** Task 1 (cargo check investigation before writing)
- **Issue:** Plan specified Ctrl+T should call `ctx.set_theme()` in `on_action()`. But `AppContext.set_theme()` takes `&mut self`, incompatible with `on_action(&AppContext)` signature.
- **Fix:** Removed `cycle_theme` action entirely from Dashboard. textual-rs App runner already handles Ctrl+T globally (confirmed in app.rs source at line 382). No user-facing regression.
- **Files modified:** src/tui/dashboard.rs
- **Committed in:** 5c37225 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 bugs)
**Impact on plan:** Both fixes required for correctness. No scope creep. Ctrl+T theme cycling still works via textual-rs built-in handler.

## Known Stubs

- `DashboardScreen.on_action("refresh")`: no-op stub — app-level YubiKey re-detection wired in 08-06.
- `DashboardScreen.on_action("open_menu")`: pushes Help screen as placeholder — full context menu in 08-06.
- `KeysScreen.on_action("refresh")`: no-op stub — app-level refresh in 08-06.
- `KeyDetailScreen.on_action("execute")`: pops screen — actual model operation wiring in 08-06.
- `ImportKeyScreen`: available_keys list is always empty at construct time — operation wiring in 08-06.

These stubs do not prevent the plan's goal from being achieved — the plan's goal is screen migration (all 7 screens as Widget impls with Header/Footer/keybindings/push_screen_deferred navigation), not full operation wiring (08-06's scope).

## Next Phase Readiness

- All 7 screens are now textual-rs Widgets: Help, Diagnostics, PIV, SSH, PIN, Dashboard, Keys
- App boots to Dashboard and navigates to all screens via push_screen_deferred
- 108 tests passing, no regressions
- Plan 08-06 can wire model operations (refresh, key gen execution, etc.) into the screen stack

---
*Phase: 08-textual-rs-migration*
*Completed: 2026-03-27*

## Self-Check: PASSED

| Check | Result |
|-------|--------|
| src/tui/dashboard.rs exists | FOUND |
| src/tui/keys.rs exists | FOUND |
| src/app.rs exists | FOUND |
| 08-05-SUMMARY.md exists | FOUND |
| Commit 5c37225 (Task 1) | FOUND |
| Commit 9bf6438 (Task 2) | FOUND |
| impl Widget for DashboardScreen | FOUND |
| impl Widget for KeysScreen | FOUND |
| push_screen_deferred in dashboard.rs | FOUND |
| push_screen_deferred in keys.rs | FOUND |
| DashboardScreen::new( in app.rs | FOUND |
| No old render(frame: &mut Frame) in dashboard.rs | CONFIRMED |
| No old render(frame: &mut Frame) in keys.rs | CONFIRMED |
| No hardcoded Color:: values in keys.rs code | CONFIRMED (Color:: only in comment) |
| cargo check | PASSES (0 errors) |
| cargo test | 108 passed, 0 failed |
