---
phase: 04-programmatic-subprocess-control
plan: "02"
subsystem: yubikey-pin, ui-pin, app
tags: [gpg, pinentry-mode-loopback, status-fd, command-fd, tui-pin-input, no-terminal-escape]
dependency_graph:
  requires: [04-01]
  provides: [programmatic_pin_operations, in_tui_pin_flow]
  affects: [04-03, 04-04]
tech_stack:
  added: []
  patterns: [stdin-stderr-thread, mpsc-channel-io, dead_code-for-legacy-compat]
key_files:
  created: []
  modified:
    - src/yubikey/pin_operations.rs
    - src/ui/pin.rs
    - src/app.rs
decisions:
  - "Used TODO comment instead of #[deprecated] on old interactive fns — clippy -D warnings treats deprecated-fn usage as error and app.rs still called them until Task 2 removed the calls; after Task 2 the old fns got #[allow(dead_code)]"
  - "Background thread + mpsc channel for stderr reading: main thread writes card-edit commands then PINs to stdin; stderr thread forwards status lines; avoids deadlock from trying to read stderr and write stdin sequentially in a single thread"
  - "OperationRunning renders synchronously (blocking call to run_gpg_pin_operation) — no actual async; progress popup is shown but spinner does not animate during the blocking call; acceptable for v1 since gpg operations are fast"
  - "OperationResult is a new PinScreen variant; any keypress returns to Main and clears the result — simpler than a timed dismiss"
  - "render_popup called with width_pct=60, height=10 for the result overlay — consistent with other result popups in the codebase"
metrics:
  duration_seconds: 420
  completed_date: "2026-03-25"
  tasks_completed: 2
  files_created: 0
  files_modified: 3
---

# Phase 04 Plan 02: In-TUI PIN Operations Summary

Replaced all interactive terminal-escape PIN operations with non-interactive gpg subprocess calls. PINs are now collected via the TUI PIN input widget, passed to gpg via `--command-fd 0`, and gpg feedback is parsed via `--status-fd 2` and shown in-TUI. The terminal never escapes to an external process for any PIN operation.

## What Was Built

**Task 1: Programmatic gpg PIN operation functions (`src/yubikey/pin_operations.rs`)**

Added `PinOperationResult { success: bool, messages: Vec<String> }` and four public programmatic functions:
- `change_user_pin_programmatic(current_pin, new_pin)` — commands `admin/passwd/1/q`
- `change_admin_pin_programmatic(current_pin, new_pin)` — commands `admin/passwd/3/q`
- `set_reset_code_programmatic(admin_pin, reset_code)` — commands `admin/passwd/4/q`
- `unblock_user_pin_programmatic(reset_code_or_admin, new_pin)` — commands `admin/passwd/2/q`

Core helper `run_gpg_pin_operation`: spawns `gpg --card-edit --pinentry-mode loopback --status-fd 2 --command-fd 0` with all three streams piped. A background thread reads stderr line-by-line and forwards lines via `mpsc::channel`. The main thread writes card-edit commands to stdin, then iterates the channel responding to `GET_HIDDEN` prompts with PINs. Error/success status derived from `GpgStatus` enum via `gpg_status::parse_status_line`.

Old interactive functions kept with `#[allow(dead_code)]` and TODO comments noting Plan 04-04 removal.

**Task 2: Wire PIN UI and app.rs (`src/ui/pin.rs`, `src/app.rs`)**

`pin.rs`:
- Added `PinInputActive`, `OperationRunning`, `OperationResult` to `PinScreen` enum
- Added `pin_input: Option<PinInputState>`, `operation_running: bool`, `operation_status: Option<String>`, `progress_tick: usize`, `pending_operation: Option<PinScreen>` to `PinState`
- `render()` dispatches new variants: `PinInputActive` → `render_pin_input`; `OperationRunning` → `render_progress_popup` overlay on main; `OperationResult` → `render_popup` overlay on main

`app.rs`:
- `c`/`a`/`r` keys in `PinScreen::Main` now set `pending_operation` and `pin_input` then transition to `PinInputActive` (no `LeaveAlternateScreen`)
- `UnblockWizardWithReset`/`UnblockWizardWithAdmin` Enter now transitions to `PinInputActive` (no `LeaveAlternateScreen`)
- `PinInputActive` routes all key events to `PinInputState::handle_key`; `Submit` calls `execute_pin_operation_programmatic`
- `OperationResult` — any key returns to `Main`
- Replaced `execute_pin_operation` (which had `disable_raw_mode` / `LeaveAlternateScreen`) with `execute_pin_operation_programmatic` — pure in-TUI, dispatches to the four `_programmatic` functions

## Verification

- `cargo build` — succeeds
- `cargo clippy -- -D warnings` — clean
- `cargo test` — 57 tests, all pass (no regressions)
- `grep`: `LeaveAlternateScreen` absent from `execute_pin_operation_programmatic`
- `grep`: `--pinentry-mode loopback` present in `pin_operations.rs`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `#[deprecated]` + `-D warnings` causes compilation failure**
- **Found during:** Task 1 verification
- **Issue:** Plan instructed marking old functions with `#[deprecated]`, but clippy `-D warnings` treats calls to deprecated functions as errors. The old `execute_pin_operation` in app.rs still called the deprecated functions, causing `cargo clippy -- -D warnings` to fail.
- **Fix:** Replaced `#[deprecated]` attributes with `// TODO(04-04): Remove` comments on the four legacy interactive functions. This keeps the same intent (document they are superseded) without breaking the build. After Task 2 wired the programmatic functions and removed the old calls, the old functions were given `#[allow(dead_code)]`.
- **Files modified:** `src/yubikey/pin_operations.rs`
- **Commit:** 721d5e9

**2. [Rule 1 - Bug] `render_popup` signature mismatch**
- **Found during:** Task 2 verification
- **Issue:** Called `render_popup(frame, area, "Result", msg)` but the function requires `width_pct: u16` and `height: u16` parameters.
- **Fix:** Added `60, 10` as width/height arguments.
- **Files modified:** `src/ui/pin.rs`
- **Commit:** edf6b07

**3. [Rule 1 - Bug] Clippy `single_match` and `get_first` lints**
- **Found during:** Task 2 verification
- **Issue:** `match key.code { Esc => {...} _ => {} }` (single-arm match) and `values.get(0)` (use `values.first()` instead) triggered `-D warnings` errors.
- **Fix:** Converted single-arm match to `if` statement; replaced all `values.get(0)` with `values.first()`.
- **Files modified:** `src/app.rs`
- **Commit:** edf6b07

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1    | 721d5e9 | feat(04-02): programmatic gpg PIN operation functions |
| 2    | edf6b07 | feat(04-02): wire PIN UI and app.rs to use programmatic operations |

## Known Stubs

None — all PIN operations are wired end-to-end. The `OperationRunning` spinner does not animate during the blocking gpg call (animation requires async), but the overlay is shown and the status message is correct. This is a cosmetic limitation, not a functional stub.

## Self-Check: PASSED
