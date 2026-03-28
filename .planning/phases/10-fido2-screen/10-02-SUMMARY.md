---
phase: 10-fido2-screen
plan: "02"
subsystem: tui
tags: [fido2, tui, widget, pin-management, credential-management]
dependency_graph:
  requires: [fido2-model-layer]
  provides: [fido2-tui-widget]
  affects: [src/tui/fido2.rs, src/tui/mod.rs]
tech_stack:
  added: []
  patterns: [textual-rs-widget, reactive-state, push-screen-deferred, insta-snapshots]
key_files:
  created:
    - src/tui/fido2.rs
    - src/tui/snapshots/yubitui__tui__fido2__tests__fido2_credentials_locked.snap
    - src/tui/snapshots/yubitui__tui__fido2__tests__fido2_default_state.snap
    - src/tui/snapshots/yubitui__tui__fido2__tests__fido2_navigate_down.snap
    - src/tui/snapshots/yubitui__tui__fido2__tests__fido2_no_pin.snap
    - src/tui/snapshots/yubitui__tui__fido2__tests__fido2_no_yubikey.snap
  modified:
    - src/tui/mod.rs
decisions:
  - "PinAuthScreen pops parent Fido2Screen on success and pushes new Fido2Screen with credentials — cleanest way to propagate credential list without cross-screen state mutation"
  - "DeleteCredentialScreen falls back to PinAuthScreen if cached_pin is None — handles case where user navigates to delete without prior PIN auth"
  - "ModalScreen wrapper used for all pushed sub-screens (PinSetScreen, PinChangeScreen, PinAuthScreen, DeleteCredentialScreen) — consistent with existing pattern in oath.rs and pin.rs"
metrics:
  duration_minutes: 4
  completed_date: "2026-03-27"
  tasks_completed: 1
  files_changed: 7
---

# Phase 10 Plan 02: Fido2Screen Widget Summary

Fido2Screen textual-rs widget with device info display (firmware, algorithms, PIN status), three-state credential list rendering, PIN set/change/auth sub-screens with masked input, and DeleteCredentialScreen wrapping ConfirmScreen.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Create Fido2Screen widget with info display, credential list, and PIN/delete flows | 79b6156 |

## What Was Built

### src/tui/fido2.rs (new, 981 lines)

Zero ratatui model boundary violations (only `ratatui::buffer::Buffer` and `ratatui::layout::Rect` imported — required by Widget trait). Exports:

**Fido2Screen** — main widget following OathScreen pattern exactly:
- Header "FIDO2 / Security Key"
- Info section (always visible, no PIN required per D-03): firmware version, algorithms, PIN status with retry count
- Passkeys section (conditional):
  - No PIN: "No PIN configured -- press S to set one."
  - Locked: "Credentials locked -- press P to authenticate"
  - No CTAP 2.1: "Passkey management requires CTAP 2.1 (not supported by this device)"
  - Empty: "No passkeys stored on this device."
  - Populated: numbered list with `>` selection marker, `rp_id` and `user_name` per row
- Keybindings: Esc (back), Up/Down/j/k (navigate), S (set/change PIN), D (delete), R (reset no-op), P (unlock)

**PinSetScreen** — 2-step wizard (EnterNew -> ConfirmNew):
- Masked `*` input, min 4 chars validation
- On success: pops self, pushes PopupScreen "PIN set successfully."

**PinChangeScreen** — 3-step wizard (EnterCurrent -> EnterNew -> ConfirmNew):
- Same masked input pattern, min 4 chars on new PIN
- On success: pops self, pushes PopupScreen "PIN changed successfully."

**PinAuthScreen** — single PIN entry for credential unlock:
- On success: pops self + parent Fido2Screen, pushes new Fido2Screen with credentials from `enumerate_credentials()`
- On error: shows "Invalid PIN: {error}" inline, clears input buffer

**DeleteCredentialScreen** — wraps ConfirmScreen (destructive=true):
- Delegates compose()/key_bindings() to inner ConfirmScreen
- "confirm" with cached_pin: calls `delete_credential(pin, &credential_id)`, pops to PopupScreen
- "confirm" without cached_pin: pops self, pushes PinAuthScreen (user must authenticate first)

### src/tui/mod.rs (modified)

Added `pub mod fido2;` after existing module declarations.

## Deviations from Plan

None — plan executed exactly as written. All interfaces from Plan 01 matched the actual model layer. The on_event() dispatch pattern followed the oath.rs/AddAccountScreen pattern (downcast KeyEvent, match key bindings, call on_action).

## Known Stubs

- `"reset"` action in on_action(): no-op in this plan — wired in Plan 03 as documented in plan spec.

## Self-Check: PASSED

- src/tui/fido2.rs: FOUND (981 lines, well above 200 minimum)
- src/tui/mod.rs contains `pub mod fido2`: FOUND
- Commit 79b6156: FOUND
- All acceptance criteria verified:
  - Fido2Screen struct: FOUND
  - Widget impl for Fido2Screen: FOUND
  - Header "FIDO2 / Security Key": FOUND
  - "Firmware:" label: FOUND
  - "Algorithms:" label: FOUND
  - "PIN:" label with status: FOUND
  - "Passkeys" section: FOUND
  - "No PIN configured -- press S to set one.": FOUND
  - "Credentials locked -- press P to authenticate": FOUND
  - PinSetScreen, PinChangeScreen, PinAuthScreen, DeleteCredentialScreen: ALL FOUND
  - Keybindings s, d, r, p, Esc, Up, Down, j, k: ALL FOUND
  - on_action arms set_pin, delete_credential, authenticate_pin, back, up, down, reset: ALL FOUND
  - 5 snapshot tests: ALL FOUND and PASSING
  - cargo test: 132 passed, 0 failed
  - ratatui imports only buffer::Buffer and layout::Rect: VERIFIED
