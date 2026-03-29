---
phase: 13-ui-polish
plan: "03"
subsystem: tui
tags: [ui-polish, datatable, progressbar, button, oath, fido2]
dependency_graph:
  requires: []
  provides: [polished-oath-screen, polished-fido2-screen]
  affects: [src/tui/oath.rs, src/tui/fido2.rs]
tech_stack:
  added: []
  patterns: [DataTable with add_row, ProgressBar::new(f64), Button with ButtonVariant, conditional button display]
key_files:
  created: []
  modified:
    - src/tui/oath.rs
    - src/tui/fido2.rs
    - src/tui/snapshots/yubitui__tui__oath__tests__oath_default_state.snap
    - src/tui/snapshots/yubitui__tui__oath__tests__oath_navigate_down.snap
    - src/tui/snapshots/yubitui__tui__oath__tests__oath_no_credentials.snap
    - src/tui/snapshots/yubitui__tui__oath__tests__oath_password_protected.snap
    - src/tui/snapshots/yubitui__tui__oath__tests__oath_screen_empty_credentials.snap
    - src/tui/snapshots/yubitui__tui__oath__tests__oath_screen_no_yubikey.snap
    - src/tui/snapshots/yubitui__tui__oath__tests__oath_screen_password_required.snap
    - src/tui/snapshots/yubitui__tui__oath__tests__oath_screen_with_credentials.snap
    - src/tui/snapshots/yubitui__tui__fido2__tests__fido2_credentials_locked.snap
    - src/tui/snapshots/yubitui__tui__fido2__tests__fido2_default_state.snap
    - src/tui/snapshots/yubitui__tui__fido2__tests__fido2_from_mock.snap
    - src/tui/snapshots/yubitui__tui__fido2__tests__fido2_navigate_down.snap
    - src/tui/snapshots/yubitui__tui__fido2__tests__fido2_no_pin.snap
decisions:
  - "DataTable::new(columns) takes only column defs; rows added via add_row(&mut self) before boxing"
  - "ProgressBar::new(f64) accepts 0.0-1.0; TOTP countdown = secs_remaining / 30.0"
  - "stable_snapshot helper updated to skip ProgressBar render line after countdown label (skip_next flag)"
  - "Fido2Screen Reset FIDO2 uses ButtonVariant::Error for visual warning; always shown"
  - "Conditional buttons: Unlock only when credentials=None+PIN set; Delete only when creds loaded+non-empty"
metrics:
  duration: "~20 minutes"
  completed: "2026-03-29T19:32:00Z"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 15
---

# Phase 13 Plan 03: OathScreen + Fido2Screen Polish Summary

OathScreen and Fido2Screen upgraded from flat Label lists to DataTable credential display, ProgressBar TOTP countdown, and action Buttons matching PIN Management visual standard.

## What Was Built

### Task 1: OathScreen Polish (src/tui/oath.rs)

**Credential list** — replaced manual format-string header/separator/row Labels with a 4-column DataTable:
- Columns: cursor (2), Name (30), Code (14), Type (8)
- Cursor column shows ">" for selected row, " " otherwise
- Code column shows TOTP code or "------", HOTP code or "[Enter]"
- Type column shows "[TOTP]" or "[HOTP]" badge

**TOTP countdown** — replaced ASCII `[====    ]` bar with:
- `Label::new("TOTP refreshes in {}s")` for the text
- `ProgressBar::new(secs_remaining as f64 / 30.0)` for the visual bar

**Action Buttons** — added to all states:
- With credentials: Add Account (A), Delete Account (D), Refresh (R)
- Empty credentials: Add Account (A)
- No YubiKey: Refresh (R)

**stable_snapshot helper** — updated to normalize both the countdown label line and the following ProgressBar render line using a `skip_next` flag pattern.

### Task 2: Fido2Screen Polish (src/tui/fido2.rs)

**PIN status badges** — changed from plain text to bracket notation:
- `PIN: [SET] (N retries remaining)` instead of `PIN: Set (N retries remaining)`
- `PIN: [NOT SET]` instead of `PIN: Not set`

**Passkey list** — replaced per-credential Label lines with a 3-column DataTable:
- Columns: cursor (2), Relying Party (32), User (30)

**Conditional action Buttons** — smart display based on state:
- "Set PIN (S)" when no PIN configured, "Change PIN (S)" when PIN set
- "Unlock Credentials (P)" only when credentials are locked (None) and PIN is set
- "Delete Credential (D)" only when credentials are loaded and non-empty
- "Reset FIDO2 (R)" always shown, uses `ButtonVariant::Error` for red visual warning

## Test Results

- 10 oath tests: all pass (8 snapshot tests updated, 2 non-snapshot tests unchanged)
- 8 fido2 tests: all pass (5 snapshot tests updated, 3 non-snapshot tests unchanged)
- `cargo check`: clean

## Deviations from Plan

None — plan executed exactly as written.

Note: OathScreen (Task 1) changes were committed to HEAD as part of commit `1db7bbf7` by another concurrent agent running `cargo insta accept`. The implementation is identical to what was planned; only the commit attribution differs.

## Known Stubs

None — both screens wire real data from their state objects.

## Self-Check: PASSED

- src/tui/oath.rs: present in HEAD (1db7bbf7)
- src/tui/fido2.rs: present in HEAD (4984db9d)
- All snapshot files updated and accepted
- cargo check: passes
- 18 tests: all pass
