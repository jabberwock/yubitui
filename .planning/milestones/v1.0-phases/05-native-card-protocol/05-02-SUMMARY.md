---
phase: 05-native-card-protocol
plan: 02
subsystem: yubikey
tags: [pcsc, apdu, openpgp, piv, attestation, touch-policy, rust, smartcard]

requires:
  - phase: 05-native-card-protocol
    plan: 01
    provides: "card.rs PC/SC primitives (connect_to_openpgp_card, get_data, apdu_sw, apdu_error_message)"

provides:
  - "touch_policy.rs: native GET DATA 0xD6-0xD9 read and VERIFY+PUT DATA set; from_byte/to_byte; KDF detection"
  - "piv.rs: native SELECT PIV AID + GET DATA per slot; parse_piv_slot_presence helper"
  - "attestation.rs: YubiKey ATTEST APDU (INS=0xFB) replaces ykman openpgp keys attest"
  - "detection.rs: wired to touch_policy::get_touch_policies_native (no local duplicate)"
  - "card.rs: PIV_AID and SELECT_PIV constants added"

affects:
  - 05-03-PLAN

tech-stack:
  added:
    - "base64 = 0.22 — PEM encoding for DER attestation cert"
  patterns:
    - "touch policy set: kill_scdaemon -> connect_to_openpgp_card -> GET DATA 0xF9 (KDF check) -> VERIFY Admin PIN [0x00,0x20,0x00,0x83] -> PUT DATA [0x00,0xDA,0x00,DO]"
    - "attestation: connect_to_openpgp_card -> ATTEST [0x00,0xFB,CRT_TAG,0x00,0x00] -> DER -> PEM via base64"
    - "PIV native: kill_scdaemon -> pcsc Context -> SELECT PIV AID -> GET DATA per slot [0x00,0xCB,0x3F,0xFF]"
    - "Admin PIN collection in TUI: SetTouchPolicyPinInput screen reuses PinInputState widget"

key-files:
  created: []
  modified:
    - src/yubikey/touch_policy.rs
    - src/yubikey/piv.rs
    - src/yubikey/attestation.rs
    - src/yubikey/detection.rs
    - src/yubikey/card.rs
    - src/yubikey/pin_operations.rs
    - src/app.rs
    - src/ui/keys.rs
    - Cargo.toml

key-decisions:
  - "set_touch_policy now takes admin_pin: &str parameter — native VERIFY Admin PIN APDU replaces ykman --force"
  - "SetTouchPolicyPinInput KeyScreen added to collect Admin PIN in TUI before executing native set"
  - "KDF check via GET DATA 0xF9 before PUT DATA — bail with clear message if KDF enabled (VERIFY APDU gives wrong PIN on KDF cards)"
  - "base64 crate added for DER-to-PEM; wraps at 64 chars per line per PEM convention"
  - "get_touch_policies_native moved to touch_policy.rs (not detection.rs); detection delegates via super::touch_policy::get_touch_policies_native"
  - "local read_touch_policies_from_card and policy_byte_to_touch_policy removed from detection.rs — replaced by touch_policy::get_touch_policies_native"
  - "find_ykman marked #[allow(dead_code)] in pin_operations.rs — fully removed in Plan 03"

requirements-completed: [NATIVE-PCSC-01, NO-YKMAN-BIN-01]

duration: 12min
completed: 2026-03-25
---

# Phase 05 Plan 02: Native Touch Policy, PIV, and Attestation Summary

**Native PC/SC APDU operations replace all remaining ykman CLI calls — touch policy read/set via GET/PUT DATA, PIV detection via SELECT PIV + GET DATA per slot, attestation via YubiKey ATTEST (INS=0xFB)**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-03-25T00:20:00Z
- **Completed:** 2026-03-25T00:32:00Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- `touch_policy.rs` rewritten: `from_byte`/`to_byte` methods on `TouchPolicy`; `get_touch_policies_native(card: &pcsc::Card)` reads DOs 0xD6-0xD9; `set_touch_policy` rewritten to VERIFY Admin PIN + PUT DATA with KDF detection via GET DATA 0xF9; `parse_touch_policies` kept with `#[allow(dead_code)]`
- `piv.rs` rewritten: `get_piv_state()` uses native SELECT PIV AID + GET DATA per slot (9a/9c/9d/9e); `parse_piv_slot_presence` helper; `parse_piv_info` kept with `#[allow(dead_code)]`; `PIV_AID`/`SELECT_PIV` constants added
- `attestation.rs` rewritten: `get_attestation_cert` uses YubiKey-proprietary ATTEST APDU (INS=0xFB, P1=CRT_TAG); DER response base64-encoded to PEM; 0x6A88 mapped to "Key not generated on-device" message; `parse_attestation_result` kept with `#[allow(dead_code)]`
- `detection.rs` updated: replaces local `read_touch_policies_from_card`/`policy_byte_to_touch_policy` with `super::touch_policy::get_touch_policies_native(&card)`
- `card.rs`: PIV_AID and SELECT_PIV constants added
- TUI: `SetTouchPolicyPinInput` KeyScreen variant added; Admin PIN collected before native `set_touch_policy` call
- 5 new unit tests: `from_byte` variants, `to_byte` roundtrip, PIV slot presence (occupied + empty), `slot_to_crt_tag`
- 85 total tests, all pass; clippy -D warnings clean

## Task Commits

1. **Task 1: Native touch policy read/set and PIV detection** - `7846f57` (feat)
2. **Task 2: Native attestation and wire detection.rs** - `a509c51` (feat)

## Files Created/Modified

- `src/yubikey/touch_policy.rs` - Added from_byte/to_byte/get_touch_policies_native; set_touch_policy rewritten to VERIFY+PUT DATA APDUs
- `src/yubikey/piv.rs` - Rewritten to SELECT PIV + GET DATA; parse_piv_slot_presence added; Command::new("ykman") removed
- `src/yubikey/attestation.rs` - Rewritten to ATTEST APDU (0xFB); DER→PEM via base64; ykman removed
- `src/yubikey/detection.rs` - Removed local touch policy read; delegates to touch_policy::get_touch_policies_native
- `src/yubikey/card.rs` - PIV_AID and SELECT_PIV constants added
- `src/yubikey/pin_operations.rs` - find_ykman marked #[allow(dead_code)]
- `src/app.rs` - execute_touch_policy_set takes admin_pin; SetTouchPolicyPinInput flow added
- `src/ui/keys.rs` - SetTouchPolicyPinInput KeyScreen variant added
- `Cargo.toml` - base64 = "0.22" added

## Decisions Made

- `set_touch_policy` signature changed to add `admin_pin: &str` — native VERIFY Admin PIN APDU requires PIN from caller; ykman previously used `--force` to bypass this
- KDF detection before PIN verify prevents silent wrong-PIN failure on KDF-enabled YubiKeys (GET DATA 0xF9 non-empty and non-zero byte = KDF active)
- `SetTouchPolicyPinInput` reuses existing `PinInputState` widget — no new UI components needed
- `get_touch_policies_native` belongs in `touch_policy.rs` (not inlined in detection.rs) for single-responsibility and testability

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing] Added #[allow(dead_code)] to find_ykman in pin_operations.rs**
- **Found during:** Task 2 verification (clippy -D warnings)
- **Issue:** attestation.rs no longer calls find_ykman; clippy -D warnings flagged it as unused
- **Fix:** Added #[allow(dead_code)] with doc comment noting Plan 03 will remove it
- **Files modified:** src/yubikey/pin_operations.rs
- **Committed in:** `a509c51` (Task 2 commit)

**2. [Rule 1 - Bug] Admin PIN collection added to touch policy set flow**
- **Found during:** Task 1 implementation
- **Issue:** Plan said "wire admin_pin through" but the old ykman --force flow never collected Admin PIN; the TUI had no PIN collection in the touch policy path
- **Fix:** Added SetTouchPolicyPinInput KeyScreen, transitioned through it from SetTouchPolicySelect/SetTouchPolicyConfirm before calling execute_touch_policy_set
- **Files modified:** src/app.rs, src/ui/keys.rs
- **Committed in:** `7846f57` (Task 1 commit)

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 03 can delete find_ykman entirely (no callers remain)
- All ykman calls eliminated from touch_policy.rs, piv.rs, attestation.rs, detection.rs
- card.rs primitives fully wired; #[allow(dead_code)] annotations on card.rs functions can be removed in Plan 03
- 85 total unit tests pass; no hardware required for any test

## Self-Check: PASSED

- FOUND: src/yubikey/touch_policy.rs contains fn from_byte, fn to_byte, fn get_touch_policies_native
- FOUND: src/yubikey/touch_policy.rs set_touch_policy contains 0x20, 0x00, 0x83 (VERIFY Admin PIN APDU)
- FOUND: src/yubikey/touch_policy.rs set_touch_policy contains 0xDA (PUT DATA INS)
- FOUND: src/yubikey/piv.rs does NOT contain Command::new("ykman")
- FOUND: src/yubikey/piv.rs contains SELECT_PIV and 0xCB, 0x3F, 0xFF
- FOUND: src/yubikey/attestation.rs contains 0xFB (YubiKey ATTEST INS)
- FOUND: src/yubikey/attestation.rs contains card::connect_to_openpgp_card
- FOUND: src/yubikey/detection.rs does NOT contain find_ykman
- FOUND: src/yubikey/detection.rs contains get_touch_policies_native
- FOUND commit 7846f57: feat(05-02): native touch policy read/set and PIV detection
- FOUND commit a509c51: feat(05-02): native attestation cert fetch and wire native touch policies to detection
- cargo clippy -- -D warnings: PASSES
- cargo test: 85 passed, 0 failed

---
*Phase: 05-native-card-protocol*
*Completed: 2026-03-25*
