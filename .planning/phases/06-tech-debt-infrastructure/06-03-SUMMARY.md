---
phase: 06-tech-debt-infrastructure
plan: "03"
subsystem: mock-mode
tags: [mock, testing, sleep-audit, status-bar, hardware-free]
dependency_graph:
  requires: ["06-01"]
  provides: ["--mock CLI flag", "hardware-free fixture mode", "50ms sleep at all APDU entry points"]
  affects: ["src/main.rs", "src/app.rs", "src/model/mock.rs", "src/tui/mod.rs", "src/model/detection.rs", "src/model/pin_operations.rs", "src/diagnostics/mod.rs"]
tech_stack:
  added: []
  patterns: ["Mock fixture as hardcoded Rust struct (no file I/O)", "Diagnostics::default() for hardware-free mode", "mock_mode guard on all detect_all() call sites"]
key_files:
  created:
    - src/model/mock.rs
  modified:
    - src/main.rs
    - src/app.rs
    - src/model/mod.rs
    - src/tui/mod.rs
    - src/model/detection.rs
    - src/model/pin_operations.rs
    - src/diagnostics/mod.rs
decisions:
  - "Mock fixture is a hardcoded Rust struct — no JSON/TOML, no file I/O; deterministic for CI"
  - "Diagnostics::default() returns all-healthy state for mock mode (no hardware queries)"
  - "50ms sleep added to detection.rs and pin_operations factory_reset — both were missing the grace period after kill_scdaemon()"
metrics:
  duration: "~25 minutes"
  completed: "2026-03-26T20:18:15Z"
  tasks_completed: 2
  files_modified: 7
---

# Phase 06 Plan 03: Mock Mode + 50ms Sleep Audit Summary

Mock mode (`--mock` CLI flag) with fixture YubiKey state for hardware-free operation, plus verified 50ms sleep coverage at all PC/SC card access entry points.

## What Was Built

**Task 1: --mock flag and fixture**

- Added `--mock` (`-m`) CLI flag to `Args` struct in `main.rs`
- `App::new(mock: bool)` skips hardware detection and Diagnostics::run() when mock=true
- Created `src/model/mock.rs` with `mock_yubikey_states()` returning a fully-configured YubiKey 5 NFC:
  - Serial: 12345678, Firmware: 5.4.3
  - All three OpenPGP slots occupied (SIG: EdDSA/Ed25519, ENC: ECDH/Cv25519, AUT: EdDSA/Ed25519)
  - PIV slot 9a occupied
  - PINs at 3/3 retries, none blocked
  - Touch policies: sig=On, enc=Off, aut=On, att=Off
- Added `Diagnostics::default()` impl (all-healthy, mock version strings)
- All `YubiKeyState::detect_all()` call sites in `app.rs` guarded with `!self.state.mock_mode`
- `DashboardAction::Refresh` reloads mock fixture instead of calling hardware when mock=true
- Added `pub fn is_mock(&self) -> bool` accessor to `App`

**Task 2: Mock status bar + 50ms sleep audit**

- `render_status_bar()` detects `app.is_mock()` and renders yellow background with `[MOCK] YubiTUI — Hardware simulation active | ...` prefix
- Mock status bar uses `Color::Yellow` background and `Color::Black` foreground for readability

**50ms Sleep Audit Results:**

| Entry Point | File | Status |
|-------------|------|--------|
| `connect_to_openpgp_card()` | `src/model/card.rs:63-64` | Already had sleep |
| `get_piv_state()` | `src/model/piv.rs:40-41` | Already had sleep |
| `detect_all_yubikey_states()` | `src/model/detection.rs:37-41` | **FIXED — sleep added** |
| `factory_reset_openpgp()` | `src/model/pin_operations.rs:244-247` | **FIXED — sleep added** |
| `pin_operations.rs` other funcs | Use `connect_to_openpgp_card()` | OK (sleep inherited) |
| `openpgp.rs`, `attestation.rs`, `key_operations.rs`, `touch_policy.rs`, `ssh_operations.rs` | Use `connect_to_openpgp_card()` | OK (sleep inherited) |

Two gaps were found and fixed: `detection.rs` and `pin_operations.rs` factory reset both called `kill_scdaemon()` but immediately proceeded to `ctx.connect()` without the 50ms grace period.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Added Diagnostics::default() impl**
- **Found during:** Task 1
- **Issue:** `App::new(mock: bool)` needed `Diagnostics::default()` but none existed; all sub-types lacked Default too
- **Fix:** Added `impl Default for Diagnostics` in `src/diagnostics/mod.rs` returning all-healthy mock state
- **Files modified:** `src/diagnostics/mod.rs`
- **Commit:** 008bffd

**2. [Rule 1 - Bug] 50ms sleep missing in factory_reset_openpgp()**
- **Found during:** Task 2 sleep audit
- **Issue:** `factory_reset_openpgp()` called inline `kill_scdaemon` (not via `card::kill_scdaemon()`) and had no sleep before `ctx.connect()`
- **Fix:** Added `std::thread::sleep(Duration::from_millis(50))` after the kill
- **Files modified:** `src/model/pin_operations.rs`
- **Commit:** 6e6783f

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 008bffd | feat(06-03): add --mock flag and hardcoded YubiKey fixture |
| 2 | 6e6783f | feat(06-03): mock status bar + 50ms sleep at all card entry points |

## Known Stubs

None. The mock fixture provides real data values that exercise all screens.

## Self-Check: PASSED
