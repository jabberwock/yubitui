---
phase: 02-ux-menus-wizards-fixes
plan: "02"
subsystem: pin-wizard
tags: [pin-management, wizard, factory-reset, ykman, recovery-flow]
dependency_graph:
  requires: [02-01]
  provides: [pin-unblock-wizard, factory-reset-openpgp, ykman-detection]
  affects: [src/ui/pin.rs, src/yubikey/pin_operations.rs, src/app.rs]
tech_stack:
  added: []
  patterns: [4-branch decision tree wizard, double-confirmation destructive action, status-aware UI]
key_files:
  created: []
  modified:
    - src/ui/pin.rs
    - src/yubikey/pin_operations.rs
    - src/app.rs
decisions:
  - "Added #[allow(dead_code)] on UnblockUserPin variant — kept for backward compat per plan, but UI routes through wizard; clippy -D warnings would otherwise fail"
  - "factory_reset_openpgp runs ykman directly (not gpg) — only ykman supports --force flag for full OpenPGP app reset"
  - "Double confirmation for factory reset: first Y shows confirm overlay, second Y executes — prevents accidental destruction"
metrics:
  duration_minutes: 4
  completed_date: "2026-03-24T19:25:56Z"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 3
---

# Phase 2 Plan 02: PIN Unblock Wizard Summary

**One-liner:** 4-branch PIN unblock wizard with retry-counter-aware decision tree, ykman factory reset with double confirmation, replacing blind gpg passthrough with guided recovery.

## What Was Built

### Task 1: PinScreen Wizard Variants and Render Functions

Extended `src/ui/pin.rs` with:

- **4 new PinScreen variants:** `UnblockWizardCheck`, `UnblockWizardWithReset`, `UnblockWizardWithAdmin`, `UnblockWizardFactoryReset`
- **`UnblockPath` enum:** `ResetCode`, `AdminPin`, `FactoryReset`
- **3 new PinState fields:** `unblock_path: Option<UnblockPath>`, `confirm_factory_reset: bool`, `ykman_available: bool`

Render functions:
- `render_unblock_wizard_check`: Displays retry counters with color coding (green >1, yellow =1, red =0). Shows numbered recovery options based on what's available. Falls back to factory reset option (or ykman install URL) when both counters exhausted.
- `render_unblock_wizard_with_reset`: Shows reset code retries remaining, instructs user on what will happen.
- `render_unblock_wizard_with_admin`: Shows admin PIN retries remaining, instructs user on what will happen.
- `render_unblock_wizard_factory_reset`: Red-bold destructive warning with "PERMANENTLY DELETE" section. Shows confirmation overlay (via `popup::render_confirm_dialog`) when `confirm_factory_reset == true`.

### Task 2: Factory Reset Operation and Event Wiring

Added to `src/yubikey/pin_operations.rs`:
- `find_ykman()`: Tries PATH first via `--version` check, falls back to `C:\Program Files\Yubico\YubiKey Manager\ykman.exe` on Windows (`#[cfg(target_os = "windows")]`)
- `factory_reset_openpgp()`: Runs `ykman openpgp reset --force`, returns success message with default PINs (123456 / 12345678)
- `is_ykman_available()`: Convenience wrapper for `find_ykman().is_ok()`

Updated `src/app.rs`:
- `KeyCode::Char('u')` in PIN Main now routes to `PinScreen::UnblockWizardCheck` and caches `is_ykman_available()`
- Explicit match arms for all 4 wizard screens (replaces the old catch-all `_ =>`)
- `UnblockWizardCheck`: Number keys 1/2/3 navigate to appropriate sub-screen if retry counters permit
- `UnblockWizardWithReset`/`UnblockWizardWithAdmin`: Enter calls `execute_pin_operation()`, ESC returns to check screen
- `UnblockWizardFactoryReset`: First Y sets `confirm_factory_reset = true`, second Y calls `factory_reset_openpgp()` and returns to Main
- `execute_pin_operation()` handles `UnblockWizardWithReset | UnblockWizardWithAdmin` by calling `pin_operations::unblock_user_pin()`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Clippy compliance] UnblockUserPin dead_code warning elevated to error**
- **Found during:** Task 2 clippy check
- **Issue:** `UnblockUserPin` variant is never constructed after routing through wizard; `-D warnings` flags dead code as error
- **Fix:** Added `#[allow(dead_code)]` on the `UnblockUserPin` variant (plan explicitly says "Keep existing -- will be unused but not breaking")
- **Files modified:** `src/ui/pin.rs`
- **Commit:** 812e4545

## Known Stubs

None. All wizard paths are fully implemented:
- Reset code path: launches `gpg --card-edit` with `passwd 2`
- Admin PIN path: same gpg command
- Factory reset: runs `ykman openpgp reset --force`

## Verification Results

- `cargo check`: PASSED
- `cargo clippy -- -D warnings`: PASSED
- `cargo test`: PASSED (0 regressions)
- All 4 wizard variants present in `src/ui/pin.rs`
- `find_ykman`, `factory_reset_openpgp`, `is_ykman_available` present in `src/yubikey/pin_operations.rs`
- `UnblockWizardCheck` entry point wired in `src/app.rs`

## Self-Check: PASSED
