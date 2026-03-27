---
phase: 05-native-card-protocol
plan: "06"
subsystem: ui
tags: [piv, ui, screen, navigation]
dependency_graph:
  requires: [05-01, 05-02]
  provides: [piv-screen-ui]
  affects: [app.rs, ui/mod.rs, ui/piv.rs, ui/dashboard.rs]
tech_stack:
  added: []
  patterns: [ratatui-screen-pattern, yubikey-state-cloned]
key_files:
  created:
    - src/ui/piv.rs
  modified:
    - src/app.rs
    - src/ui/mod.rs
    - src/ui/dashboard.rs
    - src/yubikey/piv.rs
decisions:
  - "#[allow(dead_code)] retained on SlotInfo.algorithm and SlotInfo.subject — populated by backend but not yet displayed in UI; future plan can add algorithm column"
  - "Context menu Help shifts to index 5 (was 4) to insert PIV Certificates at index 4 — preserves existing items in order"
  - "Plain ASCII [OK]/[  ] used instead of emoji for slot markers — cross-platform safe, no encoding issues"
metrics:
  duration_minutes: 20
  completed_date: "2026-03-26"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 5
---

# Phase 05 Plan 06: PIV Screen UI Summary

Wire the existing PIV backend (get_piv_state, PivState, SlotInfo) to a new Screen::Piv in the TUI, accessible via key '6' and dashboard menu, showing 9a/9c/9d/9e slot occupancy from native PC/SC data.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Create src/ui/piv.rs PIV screen renderer | 99273c9 | src/ui/piv.rs, src/ui/mod.rs |
| 2 | Wire Screen::Piv into app.rs and navigation | 17f2d8e | src/app.rs, src/ui/dashboard.rs, src/yubikey/piv.rs |

## What Was Built

### src/ui/piv.rs (new)
Read-only PIV certificates screen. Renders four standard PIV slots (9a Authentication, 9c Digital Signature, 9d Key Management, 9e Card Authentication) as occupied or empty based on `YubiKeyState.piv` data. Three states handled: no YubiKey detected, PIV data unavailable, and full slot listing.

### app.rs changes
- Added `Screen::Piv` enum variant
- Added render dispatch arm: `Screen::Piv => { let yk = self.yubikey_state().cloned(); ui::piv::render(frame, chunks[0], &yk) }`
- Added keybind `'6'` => `Screen::Piv` in regular navigation
- Context menu Down/ScrollDown limit raised from 4 to 5 (now 6 items: Diagnostics, Keys, PIN, SSH, PIV, Help)
- Context menu Enter match: index 4 => Screen::Piv, index 5 => Screen::Help

### ui/mod.rs changes
- Added `pub mod piv;`
- Added `Screen::Piv => "PIV Certificates"` arm in `render_status_bar`

### dashboard.rs changes
- Added `[6] PIV Certificates` to static navigation menu list
- Added `"PIV Certificates"` to context menu items slice

### yubikey/piv.rs changes
- Removed `#[allow(dead_code)]` from `PivState.slots` (now consumed by piv.rs renderer)
- Retained `#[allow(dead_code)]` on `SlotInfo.algorithm` and `SlotInfo.subject` (populated by backend, not yet displayed)

## Verification

- `cargo build --release` passes
- `cargo clippy -- -D warnings` passes (no warnings)
- `cargo test` passes: 85/85 tests pass
- Key '6' navigates to PIV screen from any screen (via global navigation handler)
- Dashboard menu shows `[6] PIV Certificates`
- Dashboard context menu ('m') includes `PIV Certificates` as option 4
- Esc returns to Dashboard (via existing global `else { self.current_screen = Screen::Dashboard }` handler)
- PIV screen shows slot status from `YubiKeyState.piv` (native PC/SC)

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None — piv.rs renders from real `YubiKeyState.piv` data populated by `get_piv_state()` native PC/SC calls. No hardcoded or placeholder data.

## Self-Check: PASSED

Files verified:
- src/ui/piv.rs: FOUND
- src/app.rs: FOUND (Screen::Piv variant, keybind '6', render dispatch)
- src/ui/mod.rs: FOUND (pub mod piv, Screen::Piv status bar arm)
- src/ui/dashboard.rs: FOUND ([6] PIV item, context menu entry)

Commits verified:
- 99273c9: FOUND (Task 1 - piv.rs renderer)
- 17f2d8e: FOUND (Task 2 - wiring)
