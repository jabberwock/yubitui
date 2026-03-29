---
phase: 12-yubikey-slot-delete-workflow
plan: "04"
subsystem: tui
tags: [refresh, detect_all, dashboard, keys, piv, hardware-redetection]
dependency_graph:
  requires: []
  provides: [working-refresh-on-dashboard, working-refresh-on-keys, working-refresh-on-piv]
  affects: [src/tui/dashboard.rs, src/tui/keys.rs, src/tui/piv.rs]
tech_stack:
  added: []
  patterns: [pop-then-push-fresh-screen, detect_all-on-refresh]
key_files:
  created: []
  modified:
    - src/tui/dashboard.rs
    - src/tui/keys.rs
    - src/tui/piv.rs
decisions:
  - "Use pop+push-fresh pattern (same as delete confirm screens) rather than in-place mutation"
  - "KeysScreen and PivScreen take first detected YubiKey (v.remove(0)) since screens are per-device"
metrics:
  duration: "~10 minutes"
  completed: "2026-03-29"
  tasks_completed: 2
  files_modified: 3
---

# Phase 12 Plan 04: Wire Refresh Actions Summary

One-liner: Replaced three no-op refresh stubs with `detect_all()` + pop/push-fresh-screen pattern across Dashboard, Keys, and PIV screens.

## What Was Done

All three TUI screens had placeholder "refresh" action handlers that either did nothing or only popped the current screen. This plan replaced them with real hardware re-detection using `crate::model::YubiKeyState::detect_all()`, followed by popping the current screen and pushing a fresh instance with the newly detected state.

### Changes Per File

**src/tui/dashboard.rs** (line ~323)
- Replaced: empty no-op comment block
- With: `detect_all()` call, clone + update `app_state.yubikey_states`, `pop_screen_deferred()` + `push_screen_deferred(DashboardScreen::new(fresh_app_state, diagnostics))`

**src/tui/keys.rs** (line ~572)
- Replaced: empty no-op comment block
- With: `detect_all()` call taking first result via `v.remove(0)`, `pop_screen_deferred()` + `push_screen_deferred(KeysScreen::new(fresh_yk))`

**src/tui/piv.rs** (line ~303)
- Replaced: pop-only stub (only called `ctx.pop_screen_deferred()` with no push)
- With: `detect_all()` call taking first result, `pop_screen_deferred()` + `push_screen_deferred(PivScreen::new(fresh_yk))`

## Verification

- `cargo check`: pass
- `cargo clippy -- -D warnings`: pass (zero warnings)
- `cargo test`: 160 passed, 0 failed

## Deviations from Plan

None - plan executed exactly as written.

## Commits

| Task | Description | Hash |
|------|-------------|------|
| 1+2  | Wire refresh in dashboard, keys, piv | 313bace7 |

## Known Stubs

None. All three refresh handlers are now fully wired to hardware re-detection.

## Self-Check: PASSED

- src/tui/dashboard.rs contains `detect_all`: confirmed
- src/tui/keys.rs contains `detect_all`: confirmed
- src/tui/piv.rs contains `detect_all`: confirmed
- Commit 313bace7 exists: confirmed
- All 160 tests pass: confirmed
