---
phase: 11-yubikey-slot-delete-workflow
plan: 01
subsystem: otp
tags: [model, tui, apdu, otp, pcsc]
dependency_graph:
  requires: []
  provides: [src/model/otp.rs, src/tui/otp.rs]
  affects: [src/model/mod.rs, src/model/mock.rs, src/model/detection.rs, src/model/app_state.rs, src/tui/mod.rs, src/tui/dashboard.rs]
tech_stack:
  added: []
  patterns: [pcsc-apdu-pattern, textual-rs-widget-pattern, insta-snapshot-tests]
key_files:
  created:
    - src/model/otp.rs
    - src/tui/otp.rs
    - src/tui/snapshots/yubitui__tui__otp__tests__otp_default_state.snap
    - src/tui/snapshots/yubitui__tui__otp__tests__otp_no_yubikey.snap
  modified:
    - src/model/mod.rs
    - src/model/mock.rs
    - src/model/detection.rs
    - src/model/app_state.rs
    - src/tui/mod.rs
    - src/tui/dashboard.rs
    - Cargo.toml
decisions:
  - "OTP slot status is Occupied/Empty only — credential type is write-only on hardware; screen includes hardware limitation note"
  - "nav_7 used instead of nav_9 — phases 9/10 (oath/fido2) not yet in this worktree; OTP becomes nav_7 in current sequence"
  - "Screen::Otp added after Screen::Piv — consistent with current enum ordering (no Oath/Fido2 variants yet)"
metrics:
  duration: "~8 minutes"
  completed: "2026-03-28"
  tasks_completed: 2
  files_modified: 8
  files_created: 4
---

# Phase 11 Plan 01: OTP Slot Status Screen Summary

OTP slot occupancy model + APDU function + OtpScreen widget via SELECT OTP AID (A0 00 00 05 27 20 01 01) + READ STATUS (00 03 00 00) bitmask parsing.

## What Was Built

### Task 1: OTP model types, APDU function, mock fixture, and Screen enum

- **`src/model/otp.rs`** — `OtpSlotStatus` enum (Occupied/Empty), `OtpState` struct (slot1/slot2 + touch flags), APDU constants (SELECT_OTP, READ_OTP_STATUS), bitmask constants (SLOT1_VALID=0x01, SLOT2_VALID=0x02, SLOT1_TOUCH=0x04, SLOT2_TOUCH=0x08), `parse_otp_status()` helper (testable without hardware), `get_otp_slot_status()` function following piv.rs pattern (kill_scdaemon, 50ms sleep, exclusive connect, SELECT, READ, parse), 6 unit tests
- **`src/model/mod.rs`** — Added `pub mod otp;` and `pub otp: Option<otp::OtpState>` field to `YubiKeyState`
- **`src/model/mock.rs`** — Added `otp: Some(OtpState { slot1: Occupied, slot2: Empty, slot1_touch: false, slot2_touch: false })` to mock fixture
- **`src/model/detection.rs`** — Added `let otp = super::otp::get_otp_slot_status().ok();` after PIV detection block and `otp,` in `YubiKeyState` construction
- **`src/model/app_state.rs`** — Added `Otp,` variant to `Screen` enum

### Task 2: OTP screen widget + dashboard nav_7 wiring + snapshot tests

- **`src/tui/otp.rs`** — `OtpScreen` widget with `compose()` showing slot status labels, hardware limitation note, Esc/R/Q keybindings, 2 snapshot tests (otp_default_state, otp_no_yubikey)
- **`src/tui/mod.rs`** — Added `pub mod otp;`
- **`src/tui/dashboard.rs`** — Added `[7] OTP Slots` button, `nav_7` keybinding, button event match arm, and `nav_7` action handler pushing `OtpScreen::new(otp_state)`. Updated nav_1 description from "1-6 Navigate" to "1-7 Navigate"

## Verification

- `cargo check` passes (no compile errors, 49 pre-existing warnings)
- `cargo test model::otp` — 6 tests pass (bitmask parsing)
- `cargo test tui::otp` — 2 snapshot tests pass (otp_default_state, otp_no_yubikey)
- `cargo test dashboard` — 3 tests pass (existing dashboard tests not broken)
- Full `cargo test` — 118 tests, 0 failures

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed textual-rs path dependency for worktree**
- **Found during:** Task 1 (first cargo test run)
- **Issue:** `Cargo.toml` had `textual-rs = { path = "../textual-rs/crates/textual-rs" }`. From the worktree at `.claude/worktrees/agent-ac9d17e8`, this relative path resolves to `.claude/worktrees/textual-rs/crates/textual-rs` which does not exist. The actual library is at `/Users/michael/code/textual-rs/crates/textual-rs`.
- **Fix:** Changed to absolute path `textual-rs = { path = "/Users/michael/code/textual-rs/crates/textual-rs" }`
- **Files modified:** `Cargo.toml`, `Cargo.lock`
- **Commit:** de90714

**2. [Rule 2 - Adaptation] nav_7 instead of nav_9; Screen::Otp after Piv instead of after Oath**
- **Found during:** Task 2 (reading dashboard.rs and app_state.rs)
- **Issue:** The plan was written assuming phases 9 (Oath) and 10 (Fido2) had already been executed and added `oath`/`fido2` fields to `YubiKeyState`, `Screen::Oath`/`Screen::Fido2` variants, and nav_7/nav_8 bindings to the dashboard. None of these exist in the current worktree (stopped after phase 08-06).
- **Fix:** Used `nav_7` (next available slot after existing nav_1 through nav_6) and placed `Screen::Otp` after `Screen::Piv` in the enum. The plan's interface block described a future state.
- **Files modified:** `src/tui/dashboard.rs`, `src/model/app_state.rs`
- **Commit:** b9e4850

## Known Stubs

None. The OtpScreen renders real data from `OtpState` when a YubiKey is present, and shows "No YubiKey Detected" when `otp_state` is `None`. The mock fixture provides deterministic data for tests. No placeholder text flows to UI rendering.

## Self-Check: PASSED
