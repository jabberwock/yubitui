---
phase: 12-yubikey-slot-delete-workflow
plan: 01
subsystem: ui
tags: [openpgp, apdu, pcsc, yubikey, pin]

# Dependency graph
requires:
  - phase: 12-yubikey-slot-delete-workflow
    provides: Research — attribute-change trick APDU sequence, Admin PIN VERIFY
provides:
  - OpenPGP slot delete model function (VERIFY Admin PIN + PUT DATA x2 attribute-change trick)
  - PinThenDeleteScreen — Admin PIN collection screen
  - DeleteKeyScreen — ConfirmScreen wrapper that executes delete APDU
  - delete_key action on KeysScreen wired to new two-step flow
affects: [keys-screen, openpgp-model]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "PIN collection screen via on_event char input (same as PinAuthScreen in fido2.rs)"
    - "ConfirmScreen wrapper pattern for destructive operations (same as DeleteCredentialScreen)"

key-files:
  created:
    - src/model/openpgp_delete.rs
  modified:
    - src/model/mod.rs
    - src/tui/keys.rs

key-decisions:
  - "OpenPGP slot delete uses PUT DATA attribute-change trick: RSA4096 then RSA2048 — no DELETE KEY APDU exists"
  - "Admin PIN verified via VERIFY APDU (0x20 P2=0x83) before PUT DATA — returns retry count from SW 0x63Cx"
  - "PinThenDeleteScreen -> DeleteKeyScreen two-step flow matches fido2 PinAuthScreen -> DeleteCredentialScreen pattern"
  - "Only occupied slots allow delete; empty slot shows informational popup instead"

patterns-established:
  - "Two-step delete pattern: PIN collection screen followed by ConfirmScreen wrapper"
  - "Model layer uses super::card functions directly — no ratatui imports"

requirements-completed: [SLOTDEL-01]

# Metrics
duration: 3min
completed: 2026-03-28
---

# Phase 12 Plan 01: OpenPGP Slot Delete Summary

**OpenPGP individual key slot deletion via Admin PIN + RSA attribute-change trick (PUT DATA RSA4096 -> RSA2048), with two-step TUI flow (PIN collection -> confirmation) wired into KeysScreen**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-28T08:17:35Z
- **Completed:** 2026-03-28T08:20:12Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- `src/model/openpgp_delete.rs` — standalone delete module with `OpenPgpKeySlot` enum, RSA attribute constants, and `delete_openpgp_key()` using VERIFY + two PUT DATA APDUs
- `PinThenDeleteScreen` — Admin PIN collection screen following `PinAuthScreen` pattern; on submit pushes `DeleteKeyScreen`
- `DeleteKeyScreen` — `ConfirmScreen` wrapper following `DeleteCredentialScreen` pattern; calls `delete_openpgp_key()` on confirm, shows success or error popup
- `delete_key` action updated: maps `selected_key_index` to `OpenPgpKeySlot`, guards empty slots with informational popup

## Task Commits

1. **Task 1: Create OpenPGP slot delete model module** - `6063ea91` (feat)
2. **Task 2: Wire DeleteKeyScreen into keys.rs** - `96182ecd` (feat)

## Files Created/Modified

- `src/model/openpgp_delete.rs` — OpenPgpKeySlot enum, RSA4096/2048 constants, delete_openpgp_key() function
- `src/model/mod.rs` — added `pub mod openpgp_delete;`
- `src/tui/keys.rs` — PinThenDeleteScreen, DeleteKeyScreen, updated delete_key action handler

## Decisions Made

- No new crate dependencies needed — uses existing `pcsc::Card` and `super::card` helpers
- Admin PIN VERIFY SW 0x63Cx matching uses `sw & 0xFF00 == 0x6300` (consistent with existing `apdu_error_message` pattern)
- Removed stale `ConfirmScreen` import at top of keys.rs to satisfy CI `-D warnings`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Header::new() requires &str not String**
- **Found during:** Task 2 (compile check)
- **Issue:** `Header::new(format!(...))` fails because `Header::new` takes `&str`, not `String`
- **Fix:** Used `.as_str()` on the format result for the Header call
- **Files modified:** src/tui/keys.rs
- **Verification:** `cargo check` passes with no errors
- **Committed in:** 96182ecd (Task 2 commit)

**2. [Rule 1 - Bug] Removed unused ConfirmScreen import causing clippy -D warnings failure**
- **Found during:** Task 2 (compile check)
- **Issue:** Previous import `use crate::tui::widgets::popup::{PopupScreen, ConfirmScreen}` left `ConfirmScreen` unused after switching to full-path references in `DeleteKeyScreen`
- **Fix:** Removed `ConfirmScreen` from the import — `PopupScreen` retained
- **Files modified:** src/tui/keys.rs
- **Verification:** `cargo check` clean, `cargo test` 152/152 pass
- **Committed in:** 96182ecd (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — compile errors)
**Impact on plan:** Both were minor compile-time fixes required for correctness. No scope creep.

## Issues Encountered

None beyond the two auto-fixed compile errors above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- OpenPGP delete model and TUI flow complete; plan 12-02 (PIV slot delete) can proceed independently
- `delete_openpgp_key()` is exported and tested at the compile level — hardware integration test requires a physical YubiKey
- All 152 existing tests continue to pass

---
*Phase: 12-yubikey-slot-delete-workflow*
*Completed: 2026-03-28*
