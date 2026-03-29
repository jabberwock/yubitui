---
phase: 13-ui-polish
plan: "02"
subsystem: tui
tags: [ui-polish, datatable, buttons, keys-screen, piv-screen]
dependency_graph:
  requires: []
  provides: [polished-keys-screen, polished-piv-screen]
  affects: [src/tui/keys.rs, src/tui/piv.rs]
tech_stack:
  added: []
  patterns: [DataTable-for-tabular-slots, Button-for-all-actions]
key_files:
  created: []
  modified:
    - src/tui/keys.rs
    - src/tui/piv.rs
    - src/tui/snapshots/yubitui__tui__keys__tests__keys_default_state.snap
    - src/tui/snapshots/yubitui__tui__keys__tests__keys_no_yubikey.snap
    - src/tui/snapshots/yubitui__tui__piv__tests__piv_default_state.snap
    - src/tui/snapshots/yubitui__tui__piv__tests__piv_no_yubikey.snap
decisions:
  - "DataTable::new() takes only Vec<ColumnDef>; rows added via add_row() — no rows-at-construction API"
  - "ButtonVariant not imported in keys.rs (unused — no variant styling needed for these actions)"
  - "Pre-existing dashboard snapshot failures from other agents are out of scope (deferred)"
metrics:
  duration: "25 minutes"
  completed: "2026-03-29T19:28:34Z"
  tasks_completed: 2
  files_modified: 6
requirements: [POLISH-03, POLISH-04]
---

# Phase 13 Plan 02: KeysScreen and PivScreen Polish Summary

KeysScreen and PivScreen polished with DataTable for slot lists and Buttons for all actions, matching PIN Management visual standard.

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | Polish KeysScreen — DataTable for slots, Buttons for actions | 5d87329c | src/tui/keys.rs, 2 snapshots |
| 2 | Polish PivScreen — DataTable for slots, Buttons for actions | defe7e6a | src/tui/piv.rs, 2 snapshots |

## Changes Made

### KeysScreen (src/tui/keys.rs)

**Slot summary — replaced 3 Labels with DataTable:**
- 3-column table: Slot (18), Status (7), Fingerprint (40)
- [SET] badge for occupied slots, [EMPTY] for empty
- Fingerprint shows first 16 chars with trailing "—" for empty

**Touch policies — kept as Labels with bracket notation:**
- Changed from `Signature: On` to `Signature: [On]` format
- Secondary info, not tabular enough for DataTable

**Action items — replaced 8 Labels with Buttons:**
- [G] Generate Key on Card
- [I] Import Existing Key
- [D] Delete Key Slot
- [V] View Full Key Details
- [E] Export SSH Public Key
- [K] Key Attributes
- [T] Touch Policy
- [A] Attestation

**No-YubiKey state:**
- Changed message to "No YubiKey detected." (consistent with other screens)
- Added [R] Refresh button

### PivScreen (src/tui/piv.rs)

**Slot list — replaced 4 Labels with DataTable:**
- 4-column table: cursor (2), Status (7), Slot (30), Occupancy (9)
- [OK] for occupied, [EMPTY] for empty
- > cursor indicator for selected row (maps to existing up/down keybindings)

**Action items — replaced instruction Label with Buttons:**
- [V] View Slot
- [D] Delete Slot
- [R] Refresh

**No-YubiKey state:**
- Added [R] Refresh button

### Layout

Both screens follow: Header -> DataTable (slots) -> spacer -> Buttons -> Footer

## Verification

```
cargo check: PASSED (1 pre-existing warning unrelated to this plan)
tui::keys::tests: 6/6 PASSED
tui::piv::tests: 2/2 PASSED
```

## Deviations from Plan

### Auto-fixed Issues

None.

### Scope-boundary items (not fixed, not caused by this plan)

**[Pre-existing] dashboard snapshot failures**
- Found during: full test suite run
- Issue: `dashboard.rs`, `oath.rs`, `otp.rs` are modified by other agents in this worktree; their snapshots are stale
- Action: logged as out-of-scope, not touched
- Files: src/tui/dashboard.rs (modified by other agent)

## Known Stubs

None. Both screens display real data from `YubiKeyState` model. DataTable rows are built from live `openpgp` and `piv` state.

## Self-Check: PASSED

- src/tui/keys.rs: FOUND
- src/tui/piv.rs: FOUND
- Commit 5d87329c: FOUND (git log)
- Commit defe7e6a: FOUND (git log)
- All 8 tests pass: VERIFIED
