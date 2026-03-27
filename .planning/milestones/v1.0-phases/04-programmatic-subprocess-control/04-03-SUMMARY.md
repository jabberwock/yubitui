---
phase: 04-programmatic-subprocess-control
plan: "03"
subsystem: yubikey-key-operations, ui-keygen-wizard
tags: [gpg, batch-keygen, keytocard, tui-wizard, no-terminal-escape]
dependency_graph:
  requires: [04-01, 04-02]
  provides: [keygen_wizard, import_key_programmatic, generate_key_batch]
  affects: [04-04]
tech_stack:
  added: []
  patterns: [7-step-wizard-ui, programmatic-gpg-batch, auto-map-subkeys-by-capability, current_date_ymd-without-chrono]
key_files:
  created: []
  modified:
    - src/yubikey/key_operations.rs
    - src/ui/keys.rs
    - src/app.rs
decisions:
  - "KeyAlgorithm, KeyGenParams, KeyOperationResult, ImportResult all marked #[allow(dead_code)] in Task 1 — wired in Task 2"
  - "GenerateKey KeyScreen variant removed (was never constructed after wizard added); render_generate_key kept with #[allow(dead_code)] as safety fallback"
  - "generate_key_on_card / import_key_to_card marked #[deprecated] + #[allow(dead_code)] — kept for reference, not deleted"
  - "current_date_ymd() uses std::time + Gregorian day-of-epoch algorithm to avoid chrono dependency"
  - "import_key_programmatic writes all edit-key commands upfront then responds to GET_HIDDEN PIN prompts via mpsc channel — same pattern as pin_operations.rs"
  - "Backup path in wizard defaults to ~/yubikey-backup-YYYY-MM-DD.gpg; user can edit inline"
  - "execute_key_operation no longer has LeaveAlternateScreen for GenerateKey/ImportKey paths; only ViewStatus and ExportSSH remain (neither escapes the TUI)"
metrics:
  duration_seconds: 1140
  completed_date: "2026-03-25"
  tasks_completed: 2
  files_created: 0
  files_modified: 3
---

# Phase 04 Plan 03: Key Generation Wizard and Programmatic Import Summary

Non-interactive TUI-driven key generation (7-step wizard) and key import (admin PIN collection + auto-map by capability) — both operations run entirely inside the TUI with no terminal escape.

## What Was Built

**Task 1: Non-interactive key generation and import backend (`src/yubikey/key_operations.rs`)**

- `KeyAlgorithm` enum: Ed25519/Cv25519, RSA2048, RSA4096 with Display impl
- `KeyGenParams` struct: algorithm, expire_date, name, email, backup, backup_path
- `KeyOperationResult` struct: success, messages, fingerprint
- `ImportResult` struct: sig_filled, enc_filled, aut_filled, messages, `format_slots()` → "SIG ✓  ENC ✓  AUT —"
- `generate_key_batch()`: creates temp param file, spawns `gpg --batch --status-fd 2 --gen-key`, reads KEY_CREATED fingerprint via mpsc channel, optionally exports backup via `gpg --export-secret-keys`
- `import_key_programmatic()`: discovers subkeys via `gpg --list-keys --with-colons` field-12 capability parsing, spawns `gpg --edit-key --pinentry-mode loopback --status-fd 2 --command-fd 0`, writes key/keytocard/slot commands upfront, responds to GET_HIDDEN with admin PIN
- `parse_subkey_capabilities()`: parses sub/ssb colon-format lines, extracts capability flags (s=SIG, e=ENC, a=AUT)
- Old `generate_key_on_card()` and `import_key_to_card()` marked `#[deprecated]` + `#[allow(dead_code)]`

**Task 2: Key generation wizard UI and import wiring (`src/ui/keys.rs`, `src/app.rs`)**

- `KeyGenStep` enum: 7 steps (Algorithm, Expiry, Identity, Backup, Confirm, Running, Result)
- `KeyGenWizard` struct: all form fields, active_field, editing_path, editing_custom_expiry
- 4 new `KeyScreen` variants: `KeyGenWizardActive`, `KeyImportPinInput`, `KeyImportRunning`, `KeyOperationResult`
- 5 new fields on `KeyState`: keygen_wizard, pin_input, operation_status, progress_tick, import_result
- Render functions: `render_keygen_algorithm` (step list with description), `render_keygen_expiry` (with inline custom date input), `render_keygen_identity` (two text fields, cursor block), `render_keygen_backup` (Y/N toggle + path editor), `render_keygen_confirm` (summary table), `render_key_operation_running` (progress popup), `render_key_operation_result` (slot fill display)
- `handle_keygen_wizard_key()` in `app.rs`: routes key events through all 7 wizard steps
- `execute_keygen_batch()`: builds `KeyGenParams` from wizard state, calls `generate_key_batch`, shows result in `KeyOperationResult` screen
- `execute_key_import()`: extracts admin PIN from `PinInputState`, calls `import_key_programmatic`, shows `format_slots()` result
- `execute_key_operation` no longer contains `LeaveAlternateScreen` for GenerateKey/ImportKey — only ViewStatus and ExportSSH remain
- `current_date_ymd()` helper: Gregorian day-of-epoch arithmetic using only `std::time`

## Verification

- `cargo build` — succeeds
- `cargo clippy -- -D warnings` — clean (0 errors)
- `cargo test` — 57 tests pass (36 pre-existing + 21 from Phase 4 Plan 01)
- `grep` confirms: no `LeaveAlternateScreen` in `execute_key_operation` body

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] import_key_programmatic initial design had stdin/PIN interleaving issue**
- **Found during:** Task 1 implementation
- **Issue:** First draft closed stdin before reading GET_HIDDEN status; GPG PIN prompts would not be answered
- **Fix:** Rewrote to keep stdin alive, write commands upfront, then respond to GET_HIDDEN via channel (same pattern as pin_operations.rs)
- **Files modified:** src/yubikey/key_operations.rs
- **Commit:** 6966a9e

**2. [Rule 1 - Bug] GenerateKey KeyScreen variant caused "never constructed" clippy error**
- **Found during:** Task 2 clippy pass
- **Issue:** After wiring 'g' to `KeyGenWizardActive`, the `GenerateKey` variant became never-constructed; clippy -D warnings failed
- **Fix:** Removed `GenerateKey` from `KeyScreen` enum; kept `render_generate_key` with `#[allow(dead_code)]` as defensive fallback; removed its render arm from the match
- **Files modified:** src/ui/keys.rs
- **Commit:** c2c9056

**3. [Rule 3 - Blocking] Worktree branch missing Phase 4 foundational work**
- **Found during:** Initial setup
- **Issue:** Worktree branch `worktree-agent-adb7d433` was at Phase 3 state — missing `gpg_status.rs`, `pin_input.rs`, `progress.rs` which Plan 04-03 depends on
- **Fix:** `git merge main` to fast-forward the worktree branch to include Phase 4 Plans 01 and 02
- **Files modified:** N/A (merge)
- **Commit:** a1e227d (merge commit from main)

## Commits

| Task | Commit  | Description                                                          |
|------|---------|----------------------------------------------------------------------|
| 1    | 6966a9e | feat(04-03): non-interactive key generation and import backend       |
| 2    | c2c9056 | feat(04-03): key generation wizard UI and import wiring in app.rs   |

## Known Stubs

None — all wizard steps are fully rendered and wired to real backend functions. The wizard is synchronous (no async); GPG operations run inline on Enter.

## Self-Check: PASSED

All created/modified files confirmed present. Both task commits (6966a9e, c2c9056) confirmed in git log.
