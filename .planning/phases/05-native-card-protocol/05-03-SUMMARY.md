---
phase: 05-native-card-protocol
plan: 03
status: complete
completed: 2026-03-26
commits:
  - 3aae84d  # cleanup (find_ykman + unused crates already done)
  - 894a7c1  # import state machine rework (SIG working, ENC blocker noted)
  - e0e1c33  # fix fingerprint detection (T=0 GET RESPONSE + 0x6E parsing)
---

# Plan 05-03 Summary: Cleanup + Import + Fingerprint Detection Fix

## What Was Built

### Task 1: Cleanup (already complete by session start — 3aae84d)
- `find_ykman()` deleted from `pin_operations.rs`
- Unused crates (`openpgp-card`, `card-backend-pcsc`, `yubikey`) removed from `Cargo.toml`
- Zero `Command::new("ykman")` calls in `src/`
- Zero `"--card-status"` subprocess calls in `src/`

### Task 1 scope expansion: GPG keytocard state machine (894a7c1)
- Replaced pre-buffered stdin with prompt-driven state machine for `gpg --edit-key keytocard`
- DELETE_KEY before each slot session (clears stale agent stubs)
- Two-field import form: key passphrase (optional) + admin PIN
- Handles: `cardedit.genkeys.storekeytype`, `keytocard.where`, `passphrase.enter`, `cardedit.genkeys.replace_key`, `keyedit.save.okay`
- SIG slot import: working. ENC slot: "already stored on card" from prior session.

### Bug fix: Fingerprint detection (e0e1c33)
Root cause of "Keys: ❌ Sign ❌ Encrypt ❌ Auth" despite keys on card:

**T=0 GET RESPONSE chaining not implemented.** GET DATA `0x6E` (Application Related Data) returns SW `0x613B` on YubiKey 5 firmware 5.4.x — meaning 59 bytes of response data are pending and must be fetched via GET RESPONSE (`00 C0 00 00 3B`). Without issuing GET RESPONSE, the card enters a bad state where subsequent GET DATA calls for `0xC5`, `0xC1-C3` return SW `0x6B00` (Wrong P1-P2).

**Fixes applied:**
- `card::get_data`: loops GET RESPONSE on SW `0x61xx` until `0x9000`, assembling full data
- `detection::read_openpgp_state_from_card`: rewrote to read `0x6E` container and parse `C5`/`C1-C3` via `tlv_find` (with outer-tag strip + `0x73` fallback) rather than direct DO reads
- `detection`: deferred GET DATA `0x4F` to after management AID query (secondary contributor)

**Result:** Dashboard now correctly shows `Keys: ✅ Sign ✅ Encrypt ❌ Auth` (matching `gpg --card-status`).

## Verification
- `cargo build --release` → clean
- `cargo clippy -- -D warnings` → clean
- `cargo test` → 85/85 passed
- `grep -rn "find_ykman" src/` → 0 results
- `grep -rn 'Command::new("ykman")' src/` → 0 results
- `grep -rn '"--card-status"' src/` → 0 results
- TUI dashboard: `Keys: ✅ Sign ✅ Encrypt ❌ Auth` (correct — AUT not on card)
- Firmware/serial correct: `YubiKey 5C (SN: 26928089, FW: 5.4.3)`

## Key Decisions
- Read fingerprints via `0x6E` container (not direct `C5`) — required for T=0 YubiKey firmware
- GET RESPONSE loop in `card::get_data` is the correct fix (not a workaround) per ISO 7816
- `0x4F` deferred to fallback: management AID provides better data (firmware, form factor, serial)

## Human Verification Pending
Task 2 gate: user to confirm app works on real YubiKey without ykman installed.
See 05-03-PLAN.md Task 2 checklist. Fingerprint display is now fixed — remaining
verification is user acceptance.
