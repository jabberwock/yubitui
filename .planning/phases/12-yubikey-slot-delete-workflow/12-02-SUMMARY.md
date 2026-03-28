---
phase: 12-yubikey-slot-delete-workflow
plan: "02"
subsystem: piv
tags: [piv, delete, 3des, management-key, apdu]
dependency_graph:
  requires: []
  provides: [piv-delete-model, piv-delete-tui-flow]
  affects: [src/model/piv_delete.rs, src/tui/piv.rs]
tech_stack:
  added: [des = 0.9.0-rc.3, cipher = 0.5]
  patterns: [3des-challenge-response, PUT-DATA-delete, MOVE-KEY-delete]
key_files:
  created:
    - src/model/piv_delete.rs
  modified:
    - src/tui/piv.rs
    - src/model/mod.rs
    - Cargo.toml
    - src/tui/snapshots/yubitui__tui__piv__tests__piv_default_state.snap
decisions:
  - "des 0.9.0-rc.3 with cipher 0.5 (not cipher 0.4 — version mismatch with des crate)"
  - "cipher::Array TryFrom API used (from_slice deprecated in cipher 0.5)"
  - "DeletePivConfirmScreen pushes PivScreen(None) after delete — user sees empty screen with R-to-refresh hint; full YubiKeyState not available in modal context"
metrics:
  duration_seconds: 360
  completed_date: "2026-03-28"
  tasks_completed: 2
  tasks_total: 2
  files_created: 1
  files_modified: 4
---

# Phase 12 Plan 02: PIV Delete Model and TUI Flow Summary

PIV certificate and key deletion via 3DES management key challenge-response (des crate TdesEde3), PUT DATA empty 0x53 for certificate removal, MOVE KEY (INS=0xF6) for key deletion gated on firmware >= 5.7.0, with slot navigation and management key input flow in PivScreen.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Add des crate + create PIV delete model module | 536f4cef | src/model/piv_delete.rs, Cargo.toml, src/model/mod.rs |
| 2 | Add delete flow to PIV screen with management key input and firmware gate | 203d55d3 | src/tui/piv.rs, snapshot |

## What Was Built

### src/model/piv_delete.rs

New model module providing:

- `PivSlot` enum with `Authentication`, `Signature`, `KeyManagement`, `CardAuth` variants
  - `object_id_bytes()` — 3-byte TLV object tag per NIST SP 800-73-4
  - `slot_id()` — 1-byte slot id (0x9A/0x9C/0x9D/0x9E)
  - `display_name()` — human-readable label
  - `from_slot_str()` — parser for "9a"/"9c"/"9d"/"9e"
- `PIV_DEFAULT_MGMT_KEY_3DES: &[u8; 24]` — well-known default key (01..08 × 3)
- `authenticate_piv_mgmt_key_3des(card, key)` — 3DES-EDE challenge-response via des::TdesEde3 + cipher::BlockCipherEncrypt
- `delete_piv_certificate(card, slot)` — PUT DATA (INS=0xDB) with empty BER-TLV 0x53
- `delete_piv_key(card, slot, firmware)` — MOVE KEY (INS=0xF6, P1=0xFF) gated on firmware >= 5.7.0
- `delete_piv_slot(slot, mgmt_key, firmware)` — high-level function: kill_scdaemon + connect + SELECT PIV + authenticate + delete cert + attempt key delete

No ratatui imports (model/view boundary enforced).

### src/tui/piv.rs

Updated PivScreen with:

- `selected_slot: usize` field in `PivTuiState` (default 0)
- Up/Down/j/k navigation with selection cursor (`>`) in compose()
- `D` keybinding (`delete_slot` action, shown in footer)
- `delete_slot` action checks slot occupancy; empty slot shows info popup; occupied slot pushes `MgmtKeyThenDeleteScreen`
- `MgmtKeyThenDeleteScreen`: hex management key input (48 chars = 24 bytes), empty Enter uses default key, validates hex before proceeding, pushes `DeletePivConfirmScreen`
- `DeletePivConfirmScreen`: wraps `ConfirmScreen`, firmware-gated body text, on confirm calls `delete_piv_slot`, pops screens, pushes fresh PivScreen + success popup; on error shows error popup

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] cipher 0.4 vs cipher 0.5 version conflict**
- **Found during:** Task 1 (first cargo check)
- **Issue:** Plan specified `cipher = "0.4"` but des 0.9.0-rc.3 depends on cipher 0.5, causing `KeyInit` trait mismatch (`crypto-common 0.1.7` vs `0.2.1`)
- **Fix:** Changed `cipher = "0.4"` to `cipher = "0.5"` in Cargo.toml
- **Files modified:** Cargo.toml

**2. [Rule 3 - Blocking] cipher 0.5 deprecated Array::from_slice / clone_from_slice**
- **Found during:** Task 1 (clippy check)
- **Issue:** `cipher::Array::from_slice()` and `clone_from_slice()` are deprecated in cipher 0.5; should use `TryFrom` instead
- **Fix:** Changed to `.try_into().map_err(...)` for both key and challenge block construction
- **Files modified:** src/model/piv_delete.rs

**3. [Rule 2 - Missing] Insta snapshot update required**
- **Found during:** Task 2 (cargo test)
- **Issue:** Adding `selected_slot` cursor marker `>` changed the PIV screen compose() output, breaking the existing insta snapshot
- **Fix:** Ran `cargo insta accept` to update the snapshot to reflect the new UI (slot navigation cursor + updated hint text)
- **Files modified:** src/tui/snapshots/yubitui__tui__piv__tests__piv_default_state.snap

### Design Adjustment

**DeletePivConfirmScreen screen refresh:** The plan specified pushing a fresh PivScreen "constructed with the updated state" after delete. The confirm screen doesn't have access to the full YubiKeyState (only the slot and firmware). Rather than making a partial YubiKeyState that might show stale data for other sections, the implementation pushes `PivScreen::new(None)` which shows "No YubiKey Detected" and the user can press R to refresh. This is safe and correct; the user will see the success popup before the refreshed screen.

## Known Stubs

None — all operations are wired. `delete_piv_slot` calls real APDUs; management key auth uses real 3DES.

## Self-Check

### Files created/modified

- [x] src/model/piv_delete.rs — created (432 lines)
- [x] src/tui/piv.rs — modified (delete flow added)
- [x] src/model/mod.rs — `pub mod piv_delete;` added
- [x] Cargo.toml — des + cipher dependencies added

### Commits

- 536f4cef: feat(12-02): add des crate + PIV delete model module
- 203d55d3: feat(12-02): add delete flow to PIV screen with management key input and firmware gate

### Verification

- cargo check: PASSED
- cargo test: PASSED (160 tests)
- cargo clippy -D warnings: PASSED
- No ratatui imports in src/model/piv_delete.rs: CONFIRMED
- 3DES uses des::TdesEde3: CONFIRMED
- PIV key delete gated on firmware >= 5.7.0: CONFIRMED

## Self-Check: PASSED
