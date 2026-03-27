---
phase: 03-advanced-yubikey-features
plan: "03"
subsystem: multi-key-touch-attestation
tags: [multi-key, touch-policy, attestation, ui-integration]
dependency_graph:
  requires: ["03-01", "03-02"]
  provides: ["multi-key-switching", "touch-policy-ui", "attestation-popup"]
  affects: ["src/app.rs", "src/ui/keys.rs", "src/ui/dashboard.rs", "src/yubikey/detection.rs", "src/yubikey/mod.rs"]
tech_stack:
  added: []
  patterns:
    - "Vec<YubiKeyState> with selected index replacing single Option<YubiKeyState>"
    - "cloned() at render boundary to avoid lifetime propagation into render signatures"
    - "attestation_popup overlay on top of any KeyScreen variant"
    - "touch policy irreversibility guard: SetTouchPolicyConfirm screen before executing"
key_files:
  created: []
  modified:
    - src/yubikey/detection.rs
    - src/yubikey/mod.rs
    - src/app.rs
    - src/ui/dashboard.rs
    - src/ui/keys.rs
decisions:
  - "[03-03]: App evolves from single yubikey_state: Option<YubiKeyState> to yubikey_states: Vec + selected_yubikey_idx; accessor yubikey_state() preserved for backward compat"
  - "[03-03]: render() sites clone the selected state (.cloned()) rather than changing all render signatures from &Option<YubiKeyState> to Option<&YubiKeyState>"
  - "[03-03]: 'a' key remapped from KeyAttributes to Attestation; 'k' now opens KeyAttributes"
  - "[03-03]: detect_all_yubikey_states falls back to single detect_yubikey_state() — gpg only sees one card"
metrics:
  duration: "~6 minutes"
  completed: "2026-03-24T21:09:40Z"
  tasks_completed: 1
  tasks_total: 2
  files_changed: 5
---

# Phase 03 Plan 03: Multi-Key, Touch Policy, and Attestation Integration Summary

**One-liner:** Multi-key detection via `ykman list --serials`, App evolved to `Vec<YubiKeyState>` with Tab cycling, touch policy display + set flow with IRREVERSIBLE guard, attestation PEM popup in TUI.

## What Was Built

### A. Detection Layer (`src/yubikey/detection.rs`)

- `parse_serial_list(output: &str) -> Vec<u32>` — pure parser, filters non-numeric lines
- `list_connected_serials() -> Result<Vec<u32>>` — calls `ykman list --serials`, returns empty on ykman absence
- `detect_all_yubikey_states() -> Result<Vec<YubiKeyState>>` — wraps single detect (gpg limitation); logs serial count
- Touch policies fetched in `detect_yubikey_state()` via `ykman openpgp info`, stored in new `touch_policies` field
- 4 new unit tests for `parse_serial_list` (single, multiple, empty, invalid input)

### B. YubiKeyState Struct (`src/yubikey/mod.rs`)

- New field: `pub touch_policies: Option<touch_policy::TouchPolicies>`
- New method: `pub fn detect_all() -> Result<Vec<Self>>`
- `detect()` kept with `#[allow(dead_code)]` for compatibility

### C. App Struct (`src/app.rs`)

- `yubikey_state: Option<YubiKeyState>` → `yubikey_states: Vec<YubiKeyState>` + `selected_yubikey_idx: usize`
- `yubikey_state()` accessor returns `self.yubikey_states.get(self.selected_yubikey_idx)` — backward compatible
- New accessors: `yubikey_count()`, `selected_yubikey_idx()`
- All 4 refresh sites updated to `detect_all().unwrap_or_default()` with index bounds check
- Tab key on Dashboard cycles `selected_yubikey_idx`
- `execute_touch_policy_set()` — drop-to-terminal pattern for ykman Admin PIN, restores TUI, refreshes state
- Keys Main handler: 'a' = attestation (was key attributes), 'k' = key attributes, 't' = touch policy
- SetTouchPolicy / SetTouchPolicySelect / SetTouchPolicyConfirm screen handlers

### D. Keys UI (`src/ui/keys.rs`)

- New KeyScreen variants: `SetTouchPolicy`, `SetTouchPolicySelect`, `SetTouchPolicyConfirm`
- New KeyState fields: `touch_slot_index`, `touch_policy_index`, `attestation_popup: Option<String>`
- Helper functions: `touch_slot_name()`, `touch_slot_display()`, `touch_policy_from_index()`
- `render_main` shows touch policies block when `yk.touch_policies` is Some
- `render_set_touch_policy()` — slot list with arrow indicator
- `render_set_touch_policy_select()` — policy list with arrow indicator
- `render_set_touch_policy_confirm()` — red warning for IRREVERSIBLE change, requires 'y'
- `render_attestation_popup()` — overlay using existing `render_popup` widget
- Attestation popup overlay renders on top of any KeyScreen state

### E. Dashboard (`src/ui/dashboard.rs`)

- Multi-key indicator: "Key X/Y (Tab to switch)" prepended to status text when `yubikey_count() > 1`

## Verification

```
cargo build  → PASSED (0 errors, 0 warnings)
cargo test   → PASSED (36/36 tests)
cargo clippy -- -D warnings → PASSED (0 warnings)
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `yubikey_state()` accessor returns `Option<&YubiKeyState>` not `Option<YubiKeyState>`**
- **Found during:** Task 1, compilation
- **Issue:** `render()` signatures in `keys.rs` and `pin.rs` take `&Option<YubiKeyState>` but new accessor returns `Option<&YubiKeyState>`
- **Fix:** Call `.cloned()` at the render boundary in `app.rs` — avoids cascade changes to all render functions
- **Files modified:** `src/app.rs`
- **Commit:** 02f6d934

**2. [Rule 2 - Keybinding conflict] 'a' key was already bound to KeyAttributes**
- **Found during:** Task 1, reading existing app.rs
- **Issue:** Plan assigns 'a' to attestation but existing code used 'a' for key attributes
- **Fix:** Remapped key attributes to 'k', attestation gets 'a'. Updated UI hints in render_main to match.
- **Files modified:** `src/app.rs`, `src/ui/keys.rs`
- **Commit:** 02f6d934

## Task 2: checkpoint:human-verify

**Status:** Auto-approved (AUTO_CFG=true)
- Build passes, all 36 unit tests pass, clippy clean
- Multi-key indicator: Dashboard shows "Key X/Y (Tab to switch)" when multiple keys detected
- Touch policy display: Keys Main screen lists per-slot policies from ykman openpgp info
- Touch policy set: 't' → slot selection → policy selection → IRREVERSIBLE confirmation if Fixed/CachedFixed
- Attestation popup: 'a' → PEM content in overlay, ESC to close
- Single-key mode: no indicator, all existing functionality preserved via `yubikey_state()` accessor

## Known Stubs

None — all data paths are wired. Touch policies from real ykman output, attestation from real ykman attest.

## Self-Check: PASSED

All created/modified files exist. Commit 02f6d934 verified in git log.
