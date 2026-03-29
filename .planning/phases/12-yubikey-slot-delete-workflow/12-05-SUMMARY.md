---
phase: 12-yubikey-slot-delete-workflow
plan: 05
subsystem: tui
tags: [refresh, on-demand-fetch, oath, fido2, piv, post-delete]
dependency_graph:
  requires: []
  provides: [SLOTDEL-04]
  affects: [src/tui/oath.rs, src/tui/fido2.rs, src/tui/piv.rs]
tech_stack:
  added: []
  patterns: [pop_screen_deferred + push_screen_deferred for screen replacement, detect_all() for full YubiKeyState refresh]
key_files:
  created: []
  modified:
    - src/tui/oath.rs
    - src/tui/fido2.rs
    - src/tui/piv.rs
decisions:
  - "Use F5 for FIDO2 refresh since 'r' is already bound to 'reset' — avoids key conflict"
  - "Use YubiKeyState::detect_all() in PIV post-delete rather than constructing a minimal state from get_piv_state(), since PivScreen takes Option<YubiKeyState> not Option<PivState>"
metrics:
  duration: "~15 minutes"
  completed: "2026-03-29"
  tasks: 2
  files: 3
---

# Phase 12 Plan 05: Wire OATH/FIDO2 On-Demand Refresh and Fix PIV Post-Delete State Summary

**One-liner:** Wired get_oath_state() and get_fido2_info() behind R/F5 refresh keys, and replaced the discarded fresh_piv_state no-op with detect_all() so PIV shows updated slots after delete.

## What Was Done

### Task 1: Wire OATH and FIDO2 refresh handlers

**oath.rs** — The `"refresh"` action handler had a `let _ = ctx;` no-op. Replaced with:
- Calls `crate::model::oath::get_oath_state().ok()` to fetch live credentials from the card
- Pops the current screen and pushes a new `OathScreen::new(fresh_oath)` with the fetched state
- Follows the pop+push pattern used elsewhere in the codebase for screen replacement

**fido2.rs** — The `"r"` key was already bound to `"reset"` (FIDO2 factory reset). Added:
- New `F5` keybinding in `FIDO2_BINDINGS` with action `"refresh"` and description `"F5 Refresh"`
- New `"refresh"` arm in `on_action`: calls `crate::model::fido2::get_fido2_info().ok()`, pops + pushes `Fido2Screen::new(fresh_fido2)`

### Task 2: Fix PIV post-delete state

**piv.rs** — `DeletePivConfirmScreen::on_action("confirm")` called `get_piv_state()` then immediately discarded the result with `let _ = fresh_piv_state;`, then pushed `PivScreen::new(None)`. The user saw "No YubiKey Detected" after every successful delete.

Replaced with:
```rust
let fresh_yk = crate::model::YubiKeyState::detect_all()
    .ok()
    .and_then(|mut v| if v.is_empty() { None } else { Some(v.remove(0)) });
ctx.push_screen_deferred(Box::new(PivScreen::new(fresh_yk)));
```

This calls the full detection pipeline so PivScreen gets an actual `YubiKeyState` with updated slot data.

## Deviations from Plan

**1. [Rule 1 - Constraint] Used F5 instead of R for FIDO2 refresh**
- **Found during:** Task 1 — reading fido2.rs bindings
- **Issue:** `"r"` was already bound to `"reset"` (FIDO2 factory reset, a destructive op). Plan acknowledged this conflict and said to use F5 or another key.
- **Fix:** Added `KeyCode::F(5)` binding for `"refresh"` action
- **Files modified:** src/tui/fido2.rs

No other deviations — plan executed exactly as written for oath.rs and piv.rs.

## Verification

- `cargo check` — clean (0 errors)
- `cargo clippy -- -D warnings` — clean (0 warnings)
- `cargo test` — 160 passed, 0 failed
- Manual: not applicable (hardware required; logic is correct by inspection)

## Known Stubs

None — all three refresh/post-delete handlers now call real model functions. No placeholder patterns remain.

## Self-Check: PASSED

- src/tui/oath.rs: modified (refresh handler wired)
- src/tui/fido2.rs: modified (F5 binding + refresh handler added)
- src/tui/piv.rs: modified (detect_all() replaces discarded state)
- Commit 6c58db73 exists and contains all three files
