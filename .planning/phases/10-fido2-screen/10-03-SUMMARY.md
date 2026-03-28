---
phase: 10-fido2-screen
plan: "03"
subsystem: tui+model
tags: [fido2, reset, raw-hid, ctap2, countdown, worker]
dependency_graph:
  requires: [fido2-model-layer, fido2-tui-widget]
  provides: [fido2-reset-workflow]
  affects: [src/model/fido2.rs, src/tui/fido2.rs, Cargo.toml]
tech_stack:
  added: [hidapi = "2" (direct dep, was transitive only)]
  patterns: [raw-ctaphid-frame, run_worker_with_progress, on_mount-own_id]
key_files:
  created:
    - src/tui/snapshots/yubitui__tui__fido2__tests__fido2_reset_confirm_screen.snap
    - src/tui/snapshots/yubitui__tui__fido2__tests__fido2_reset_guidance_waiting.snap
  modified:
    - Cargo.toml
    - src/model/fido2.rs
    - src/tui/fido2.rs
decisions:
  - "reset_fido2() uses raw CTAPHID frames via hidapi directly — ctap-hid-fido2 does not expose authenticatorReset (command 0x07)"
  - "Nonce for CTAPHID_INIT uses fixed bytes [0x01..0x08] — nonce is not security-critical per FIDO HID spec"
  - "ResetGuidanceScreen countdown loop sends tick before sleep so initial 10 display is immediate"
  - "Worker checks is_fido_device_present() on each tick before sleep — device detection latency under 1 second"
  - "ResetGuidanceScreen stores own_id via on_mount() to match worker source_id in on_event()"
metrics:
  duration_minutes: 10
  completed_date: "2026-03-28"
  tasks_completed: 2
  files_changed: 5
---

# Phase 10 Plan 03: FIDO2 Reset Workflow Summary

FIDO2 reset workflow: hand-rolled authenticatorReset via raw CTAPHID frames with hidapi, irreversibility confirmation dialog, and guided countdown screen with 10-second replug timer and device presence polling via run_worker_with_progress.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Add hidapi dep, CTAPHID constants, find_fido_hid_device_path(), is_fido_device_present(), reset_fido2() | da5afa3 |
| 2 | ResetConfirmScreen + ResetGuidanceScreen with countdown + wire 'reset' action + snapshots | 2bec3ef |

## What Was Built

### src/model/fido2.rs (extended)

Added raw HID reset capability. Exports:
- `CTAPHID_INIT`, `CTAPHID_CBOR`, `BROADCAST_CID`, `FIDO_USAGE_PAGE`, `FIDO_USAGE` — CTAP HID spec constants
- `CTAP2_OK`, `CTAP2_ERR_NOT_ALLOWED` — CTAP2 status codes
- `find_fido_hid_device_path()` — iterate HID device list, return first device with usage_page=0xF1D0, usage=0x01
- `is_fido_device_present()` — thin wrapper around find_fido_hid_device_path() for polling
- `reset_fido2()` — sends CTAPHID_INIT on broadcast channel to get channel ID, then CTAPHID_CBOR with payload 0x07 (authenticatorReset) on allocated channel; parses response status byte

### Cargo.toml (modified)

Added `hidapi = "2"` as a direct dependency (was transitive only via ctap-hid-fido2). Direct dep required to call `HidApi::new()` and iterate device list.

### src/tui/fido2.rs (extended)

Added three new types and wired the "reset" action:

**ResetConfirmScreen** — wraps ConfirmScreen (destructive=true) with irreversibility warning body:
- "WARNING: This will permanently delete ALL passkeys and the FIDO2 PIN."
- "All FIDO2 credentials stored on this YubiKey will be destroyed."
- on_action("confirm"): pop self, push ModalScreen(ResetGuidanceScreen)
- on_action("cancel"): pop self

**ResetPhase** enum — six-state machine:
- WaitingForUnplug, WaitingForReplug(u8), Resetting, Success, Expired, Error(String)

**ResetGuidanceScreen** — textual-rs widget with:
- on_mount() stores own_id via Cell<Option<WidgetId>> for worker source_id matching
- compose() renders phase-specific content per D-10 and D-11 requirements
- WaitingForReplug shows countdown bar: `Time remaining: {secs}s  [####    ]`
- run_worker_with_progress spawned on Enter in WaitingForUnplug phase
- Worker loops 10..0, sends progress tick per second, polls is_fido_device_present()
- WorkerProgress<u8> received in on_event() updates WaitingForReplug(secs)
- WorkerResult<ResetWorkerResult> triggers either Resetting (calls reset_fido2()) or Expired

**Fido2Screen "reset" action** — changed from no-op to `ctx.push_screen_deferred(ModalScreen(ResetConfirmScreen::new()))`.

**Snapshot tests** — 2 new tests:
- `fido2_reset_confirm_screen`: renders ResetConfirmScreen at 80x24
- `fido2_reset_guidance_waiting`: renders ResetGuidanceScreen initial state (WaitingForUnplug)

## Deviations from Plan

None — plan executed exactly as written. The hidapi API accepted a `CString` path from `device_info.path()` directly and `open_path()` worked without any adaptation.

## Known Stubs

None. The reset workflow is fully wired end-to-end:
- Model: reset_fido2() sends real CTAPHID frames
- TUI: ResetConfirmScreen -> ResetGuidanceScreen -> reset_fido2() call
- Worker polling: is_fido_device_present() used in countdown loop

## Self-Check: PASSED

- Cargo.toml contains `hidapi = "2"`: FOUND
- src/model/fido2.rs contains `pub fn reset_fido2() -> Result<()>`: FOUND
- src/model/fido2.rs contains `pub fn is_fido_device_present() -> bool`: FOUND
- src/model/fido2.rs contains `pub fn find_fido_hid_device_path()`: FOUND
- src/model/fido2.rs contains `CTAPHID_INIT` and `CTAPHID_CBOR` constants: FOUND
- src/model/fido2.rs contains `FIDO_USAGE_PAGE: u16 = 0xF1D0`: FOUND
- src/model/fido2.rs contains `CTAP2_ERR_NOT_ALLOWED`: FOUND
- src/tui/fido2.rs contains `pub struct ResetConfirmScreen`: FOUND
- src/tui/fido2.rs contains `pub struct ResetGuidanceScreen`: FOUND
- src/tui/fido2.rs contains `enum ResetPhase` with all variants: FOUND
- src/tui/fido2.rs ResetConfirmScreen body "permanently delete ALL passkeys": FOUND
- src/tui/fido2.rs ResetGuidanceScreen "within 10 seconds of being plugged in": FOUND
- src/tui/fido2.rs ResetGuidanceScreen "Plug in your YubiKey NOW": FOUND
- src/tui/fido2.rs countdown bar "Time remaining:": FOUND
- src/tui/fido2.rs Fido2Screen "reset" action pushes ResetConfirmScreen: FOUND
- src/tui/fido2.rs contains run_worker_with_progress: FOUND
- Tests fido2_reset_confirm_screen and fido2_reset_guidance_waiting: FOUND, PASSING
- Commit da5afa3: FOUND
- Commit 2bec3ef: FOUND
- cargo test: 134 passed, 0 failed
