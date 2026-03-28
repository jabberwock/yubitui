---
phase: 09-oath-totp-screen
plan: "03"
subsystem: tui/oath
tags: [oath, totp, hotp, wizard, textual-rs, add-account, delete-account]
dependency_graph:
  requires: [09-01, 09-02]
  provides: [add-account-wizard, delete-account-confirm, password-protected-message]
  affects: [src/tui/oath.rs]
tech_stack:
  added: []
  patterns: [push_screen_deferred, on_event-keyevent, ConfirmScreen, PopupScreen]
key_files:
  created: []
  modified: [src/tui/oath.rs]
decisions:
  - "Used on_event() with downcast_ref::<KeyEvent>() for character-level input in AddAccountScreen — on_action only fires for registered bindings, not raw chars"
  - "DeleteConfirmScreen delegates compose()/key_bindings() to inner ConfirmScreen and overrides on_action to call delete_credential()"
  - "OATH password-protected branch uses 3-line informational message per OATH-05 scope (password management deferred to v2)"
  - "AddAccountState.oath_type defaults to OathType::Totp (manually implemented Default since OathType has no derive Default)"
metrics:
  duration: "~30min"
  completed: "2026-03-27"
  tasks_completed: 1
  files_modified: 1
---

# Phase 09 Plan 03: OATH Add Account Wizard and Delete Flow Summary

Add Account wizard (5-step push_screen sequence) and Delete confirmation flow for the OATH screen.

## What Was Built

**AddAccountScreen** — a 5-step sequential wizard pushed via `push_screen_deferred`:

1. **Issuer** — freeform text input (optional, can be blank)
2. **Account Name** — required; shows error if empty
3. **Secret Key** — required; masked as asterisks during input
4. **Type** — T for TOTP (default), H for HOTP; Enter confirms selection
5. **Confirm** — review summary; Enter calls `put_credential()`, Esc cancels

Character input handled via `on_event()` with `downcast_ref::<KeyEvent>()` — this captures raw keystrokes before the keybinding system fires, enabling per-character typing, backspace, and TypeSelect shortcuts.

**DeleteConfirmScreen** — a thin wrapper around `ConfirmScreen` with a `"cannot be undone"` warning body. On confirm, calls `delete_credential()` and shows success/error popup. On cancel, pops itself.

**Password-protected branch** — `OathScreen.compose()` now shows a 3-line informational message when `oath_state.password_required == true`, guiding the user to use `ykman oath access change` to remove the password.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] OathType missing Default derive**
- **Found during:** Task 1 (writing AddAccountState which has `pub oath_type: OathType`)
- **Issue:** `OathType` in model/oath.rs has no `#[derive(Default)]`, making AddAccountState::default() fail
- **Fix:** Manually implemented `Default for OathType` returning `OathType::Totp` in tui/oath.rs (model layer not modified per UI/data separation rule)
- **Files modified:** src/tui/oath.rs

**2. [Rule 3 - Blocking] on_key_event not in Widget trait**
- **Found during:** Task 1 compilation
- **Issue:** Plan specified `on_key_event()` but textual-rs Widget trait uses `on_event(&dyn Any, ctx) -> EventPropagation`
- **Fix:** Implemented `on_event()` with `event.downcast_ref::<KeyEvent>()` — same pattern as masked_input.rs
- **Files modified:** src/tui/oath.rs

**3. [Rule 3 - Blocking] Worktree missing OATH files from main**
- **Found during:** Initial setup
- **Issue:** This worktree branched before 09-01/09-02 commits (model and OathScreen widget)
- **Fix:** Merged main into worktree branch (`git merge main --no-ff`) to bring in oath model + OathScreen
- **Commit:** b91ce96

**4. [Rule 3 - Blocking] textual-rs path incorrect for worktree depth**
- **Found during:** Initial cargo check
- **Issue:** Cargo.toml had `path = "../textual-rs/..."` which is correct for the main repo but the worktree is 4 levels deeper
- **Fix:** Updated to `path = "../../../../textual-rs/..."` matching the actual filesystem layout

## Known Stubs

None — all flows are wired. `add_account` pushes `AddAccountScreen` which calls `put_credential()`. `delete_account` pushes `DeleteConfirmScreen` which calls `delete_credential()`. Both make real APDU calls via the model layer.

## Test Results

- 123 tests total; 0 failures
- 2 new snapshot tests added: `add_account_screen_initial`, `add_account_screen_step_navigation`

## Self-Check: PASSED

- src/tui/oath.rs contains `AddAccountScreen` ✓
- src/tui/oath.rs contains `AddAccountStep` with all 5 variants ✓
- src/tui/oath.rs contains `AddAccountState` with issuer, account_name, secret_b32, oath_type ✓
- `add_account` action pushes `AddAccountScreen::new()` ✓
- `delete_account` action pushes `DeleteConfirmScreen` ✓
- `put_credential(` called in AddAccountScreen confirm flow ✓
- `delete_credential(` called in DeleteConfirmScreen confirm action ✓
- `"password-protected"` text present ✓
- `"cannot be undone"` text present ✓
- `cargo check` passes with zero errors ✓
- All 123 tests pass ✓
