---
phase: 05-native-card-protocol
plan: 01
subsystem: yubikey
tags: [pcsc, apdu, openpgp, rust, smartcard, tlv]

requires:
  - phase: 03-advanced-yubikey-features
    provides: "existing detection/pin/openpgp/key_operations modules being replaced"

provides:
  - "src/yubikey/card.rs: PC/SC primitives (connect_to_openpgp_card, get_data, apdu_sw, apdu_error_message, tlv_find, serial_from_aid, kill_scdaemon)"
  - "detection.rs: PC/SC reader enumeration replacing ykman list --serials"
  - "pin.rs: GET DATA 0xC4 binary PIN status read replacing gpg --card-status"
  - "openpgp.rs: GET DATA 0x6E + 0x65 card state read replacing gpg --card-status"
  - "key_operations.rs: GET DATA 0x6E algorithm attributes replacing ykman openpgp info"

affects:
  - 05-02-PLAN
  - 05-03-PLAN

tech-stack:
  added: []
  patterns:
    - "card.rs as single PC/SC APDU primitive module; all operations call card::connect_to_openpgp_card then card::get_data"
    - "kill_scdaemon() before every native PC/SC operation to release card channel"
    - "tlv_find() BER-TLV walker for DO 0x6E nested data objects"
    - "apdu_error_message(sw, context) maps SW codes to plain English; SW goes to tracing::debug! only"
    - "serial_from_aid() extracts BCD-encoded serial from AID select response bytes 10-13"

key-files:
  created:
    - src/yubikey/card.rs
  modified:
    - src/yubikey/mod.rs
    - src/yubikey/pin_operations.rs
    - src/yubikey/detection.rs
    - src/yubikey/pin.rs
    - src/yubikey/openpgp.rs
    - src/yubikey/key_operations.rs
    - src/main.rs

key-decisions:
  - "card.rs #[allow(dead_code)] on all pub functions until Plan 02 wires touch policy and PIV — annotations to be removed as callers are added"
  - "detection.rs contains format_fingerprint() and parse_algorithm_attributes() as pub helpers shared by openpgp.rs and key_operations.rs"
  - "detect_all_yubikey_states() builds full YubiKeyState from a single card connection per reader — no multiple subprocess calls"
  - "Touch policies read via native GET DATA 0xD6/0xD7/0xD8 in detection.rs (Plan 2 will also wire set_touch_policy)"
  - "parse_pin_status, parse_card_status, parse_ykman_openpgp_info kept with #[allow(dead_code)] — unit test regression suites remain valid"

patterns-established:
  - "PC/SC operation: kill_scdaemon -> Context::establish -> list_readers -> connect(Exclusive, T0|T1) -> SELECT OpenPGP AID -> get_data -> interpret SW"
  - "TLV navigation: tlv_find(&app_data, 0x73) for Discretionary Data, then tlv_find(disc, 0xC7/C8/C9) for fingerprints, 0xC1/C2/C3 for algorithm attributes"

requirements-completed: [NATIVE-PCSC-01, NO-GPG-BIN-01, NO-YKMAN-BIN-01]

duration: 7min
completed: 2026-03-25
---

# Phase 05 Plan 01: Native Card Protocol Foundation Summary

**PC/SC APDU primitives module (card.rs) added; detection, pin, openpgp, and key_operations rewritten to read card data via raw APDUs instead of gpg --card-status and ykman subprocess calls**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-03-25T00:06:41Z
- **Completed:** 2026-03-25T00:13:41Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- New `src/yubikey/card.rs` with 8 public functions: `kill_scdaemon`, `connect_to_openpgp_card`, `get_data`, `get_data_2byte_tag`, `apdu_sw`, `apdu_error_message`, `serial_from_aid`, `tlv_find`; 20 unit tests covering all pure functions
- `detection.rs` completely rewritten: PC/SC reader enumeration replaces `ykman list --serials`; full `YubiKeyState` built from a single card connection per reader (PIN status, OpenPGP state, touch policies, all from APDUs)
- `pin.rs`, `openpgp.rs`, `key_operations.rs` rewritten to use native GET DATA APDUs (0xC4, 0x6E, 0x65, 0x5F50); `gpg --card-status` subprocess calls eliminated from all three
- `apdu_sw` helper migrated from `pin_operations.rs` (private) to `card.rs` (public); pin_operations delegates to `super::card::apdu_sw`
- 80 total unit tests, all pass; clippy -D warnings clean

## Task Commits

1. **Task 1: Create src/yubikey/card.rs — PC/SC primitives module** - `993976b` (feat)
2. **Task 2: Replace detection.rs, pin.rs, openpgp.rs, key_operations.rs** - `f66a01e` (feat)

## Files Created/Modified

- `src/yubikey/card.rs` - New: PC/SC primitives (connect, get_data, apdu_sw, apdu_error_message, tlv_find, serial_from_aid, kill_scdaemon); 20 unit tests
- `src/yubikey/mod.rs` - Added `pub mod card;`
- `src/yubikey/pin_operations.rs` - Replaced private `fn apdu_sw` body with delegation to `super::card::apdu_sw`
- `src/yubikey/detection.rs` - Rewritten: PC/SC reader enumeration; removed ykman/gpg subprocess calls; added format_fingerprint, parse_algorithm_attributes pub helpers
- `src/yubikey/pin.rs` - Rewritten `get_pin_status()` using GET DATA 0xC4; removed `use std::process::Command`
- `src/yubikey/openpgp.rs` - Rewritten `get_openpgp_state()` using GET DATA 0x6E + 0x65 + 0x5F50; removed `use std::process::Command`
- `src/yubikey/key_operations.rs` - Rewritten `get_key_attributes()` using GET DATA 0x6E TLV parsing; removed ykman subprocess call
- `src/main.rs` - Updated `--list` flag to use `detect_all_yubikey_states()` (was `detect_yubikeys`)

## Decisions Made

- `card.rs` functions carry `#[allow(dead_code)]` for now since detection.rs/pin.rs/openpgp.rs do not call them via `super::card::` prefix everywhere yet (they have their own internal inlined versions). Plans 02 and 03 will consolidate.
- `detect_all_yubikey_states()` in detection.rs builds the full YubiKeyState inline without calling `pin::get_pin_status()` or `openpgp::get_openpgp_state()` separately — avoids multiple card connections.
- Touch policy read wired in detection.rs using GET DATA 0xD6/0xD7/0xD8; set_touch_policy (PUT DATA) stays in touch_policy.rs for Plan 02.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed pre-existing `unused variable: state` clippy warning in src/ui/pin.rs**
- **Found during:** Task 1 verification
- **Issue:** `fn render_unblock_wizard_check` parameter `state: &PinState` was unused; clippy -D warnings was already failing before my changes
- **Fix:** Renamed parameter to `_state` per Rust convention for intentionally unused params
- **Files modified:** src/ui/pin.rs
- **Verification:** `cargo clippy -- -D warnings` passes
- **Committed in:** `993976b` (Task 1 commit)

**2. [Rule 1 - Bug] Updated `--list` CLI flag in main.rs to use new detection API**
- **Found during:** Task 2 verification
- **Issue:** `main.rs` imported `detect_yubikeys` which was removed from detection.rs; compile error
- **Fix:** Updated import to `detect_all_yubikey_states()` and display to use `key.info` field
- **Files modified:** src/main.rs
- **Verification:** Compiles cleanly
- **Committed in:** `f66a01e` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 pre-existing bug, 1 blocking compile error from API change)
**Impact on plan:** Both fixes necessary for compilation. No scope creep.

## Issues Encountered

- `dead_code` warnings on new `pub fn` in card.rs since binary crate and callers not yet wired — resolved with `#[allow(dead_code)]` following established pattern in this codebase.
- `tlv_find` had explicit lifetime `'a` flagged as needless by clippy — removed, Rust infers correctly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- card.rs primitives ready for Plan 02 (touch policy PUT DATA, PIV native reads)
- All existing unit tests preserved and passing (80 total)
- No gpg --card-status calls remain in detection.rs, pin.rs, or openpgp.rs
- No ykman calls remain in detection.rs or key_operations.rs for read operations
- Plan 02 can build directly on card::connect_to_openpgp_card and card::get_data

## Self-Check: PASSED

- FOUND: src/yubikey/card.rs
- FOUND: src/yubikey/detection.rs
- FOUND: .planning/phases/05-native-card-protocol/05-01-SUMMARY.md
- FOUND commit 993976b: feat(05-01): add card.rs PC/SC primitives module
- FOUND commit f66a01e: feat(05-01): replace gpg/ykman card reads with native PC/SC APDUs
- All 80 unit tests pass
- cargo clippy -D warnings passes

---
*Phase: 05-native-card-protocol*
*Completed: 2026-03-25*
