---
phase: 10-fido2-screen
plan: "01"
subsystem: model
tags: [fido2, ctap2, model, types, mock]
dependency_graph:
  requires: []
  provides: [fido2-model-layer]
  affects: [src/model/mod.rs, src/model/app_state.rs, src/model/mock.rs]
tech_stack:
  added: [ctap-hid-fido2 = "3.5.9"]
  patterns: [fresh-connection-per-call, model-layer-zero-ratatui]
key_files:
  created:
    - src/model/fido2.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/model/mod.rs
    - src/model/app_state.rs
    - src/model/mock.rs
    - src/model/detection.rs
decisions:
  - "Fido2State.firmware_version stored as Option<String> (formatted '5.4.3') — model layer handles conversion from packed u32 so TUI just displays the string"
  - "algorithms extracted as Vec<String> (alg names only) from Info.algorithms: Vec<(String,String)> — TUI only needs algorithm names, not type strings"
  - "Fido2State.credentials: None = locked (PIN not provided); Some(vec![]) = no credentials; Some(creds) = populated — three-state distinction drives TUI rendering"
metrics:
  duration_minutes: 15
  completed_date: "2026-03-27"
  tasks_completed: 2
  files_changed: 6
---

# Phase 10 Plan 01: FIDO2 Model Layer Summary

FIDO2 model layer with ctap-hid-fido2 crate integration: Fido2State/Fido2Credential types, CTAP2 operations (get_info, enumerate_credentials, delete_credential, set_pin, change_pin), mock fixture with 2 credentials, and Screen::Fido2 variant.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Add ctap-hid-fido2 dependency, create Fido2 model types, update mod.rs/app_state.rs | 9c29ab3 |
| 2 | Extend mock fixture with FIDO2 data, fix all YubiKeyState constructors | a96f209 |

## What Was Built

### src/model/fido2.rs (new)

Zero ratatui imports (INFRA-03/04 boundary). Exports:
- `Fido2State` — device info struct with firmware_version, algorithms, pin_is_set, pin_retry_count, credentials, supports_cred_mgmt
- `Fido2Credential` — credential struct with rp_id, rp_name, user_name, credential_id
- `get_fido2_device()` — fresh FidoKeyHid connection with Windows admin privilege hint on access errors
- `get_fido2_info()` — device info without credentials (no PIN required)
- `enumerate_credentials(pin)` — two-step enumerate_rps → enumerate_credentials CTAP 2.1 protocol
- `delete_credential(pin, credential_id)` — delete by raw credential ID bytes
- `set_pin(new_pin)` — set first FIDO2 PIN
- `change_pin(current_pin, new_pin)` — change existing PIN

### Key API Corrections vs. Plan (Rule 1 auto-fixes)

The ctap-hid-fido2 3.5.9 actual types differed from the research documentation:
1. `Info.algorithms` is `Vec<(String, String)>` not `Vec<String>` — extracted alg names as second element
2. `Info.firmware_version` is `u32` (not `Option<u32>`) — 0 maps to None
3. `PublicKeyCredentialUserEntity.name` and `.display_name` are `String` (not `Option<String>`)
4. `PublicKeyCredentialRpEntity.name` is `String` (not `Option<String>`)

These were corrected by inspecting the downloaded crate source at `~/.cargo/registry/src/`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] ctap-hid-fido2 type mismatches in research documentation**
- **Found during:** Task 1 (first cargo check)
- **Issue:** Research documented `Info.algorithms: Vec<String>` but actual type is `Vec<(String, String)>`; `firmware_version: Option<u32>` but actual type is `u32`; user/rp entity name fields are `String` not `Option<String>`
- **Fix:** Inspected crate source at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ctap-hid-fido2-3.5.9/src/` and corrected all field accesses accordingly
- **Files modified:** src/model/fido2.rs
- **Commit:** 9c29ab3

## Known Stubs

None. The model layer is fully wired. The `fido2: None` in detection.rs is intentional — FIDO2 state is fetched on-demand (same pattern as `oath: None`), not during the initial PC/SC reader scan.

## Self-Check: PASSED

- src/model/fido2.rs: FOUND
- Commit 9c29ab3: FOUND
- Commit a96f209: FOUND
- cargo check: PASSED (68 pre-existing warnings, zero new errors)
- cargo test: 127 passed, 0 failed
