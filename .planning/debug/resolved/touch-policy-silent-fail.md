---
status: resolved
trigger: "Setting touch policy completes without error but the policy doesn't change — gpg --card-status still shows UIF=off after the wizard finishes."
created: 2026-03-26T00:00:00Z
updated: 2026-03-26T05:00:00Z
---

## Current Focus

hypothesis: CONFIRMED — KDF (Iterated S2K) is enabled on this YubiKey. VERIFY is sent with raw PIN bytes, but card requires SHA-256 hashed PIN per DO 0xF9. Code bails with "requires ykman" error instead of hashing natively.
test: Implement native KDF hashing: parse DO 0xF9 BER-TLV, extract salt_pw3 (tag 0x86) + iteration count (tag 0x83) + hash algo (tag 0x82), compute S2K hash, use hashed bytes in VERIFY APDU.
expecting: VERIFY returns 0x9000 and touch policy write succeeds end-to-end.
next_action: Add sha2 crate, implement kdf_hash_pin(), replace bail with native KDF VERIFY, remove debug eprintln! from app.rs

## Symptoms

expected: After confirming touch policy change in the wizard, the selected slot's UIF setting changes (e.g. Sign=on)
actual: Wizard completes with no error, but touch policy remains unchanged (UIF still off)
errors: No explicit error messages shown
reproduction: Navigate to touch policy section, select a slot, select a policy (e.g. On), confirm with Admin PIN 12345678, wizard completes — but gpg --card-status shows UIF unchanged
started: Unknown — may never have worked
user_pin: 123456
admin_pin: 12345678

## Eliminated

- hypothesis: Wrong APDU bytes (incorrect DO tag, policy byte, or second byte 0x20)
  evidence: APDU [0x00, 0xDA, 0x00, do_tag, 0x02, policy.to_byte(), 0x20] is correct per OpenPGP spec. DO tags 0xD6-0xD9 correct. policy.to_byte() returns correct values (0x00-0x04). 0x20 is the correct general button flag byte.
  timestamp: 2026-03-26

- hypothesis: SW not checked (silent failure ignored)
  evidence: Both VERIFY and PUT DATA SWs are checked with if sw != 0x9000 { bail! }. Errors are set in key_state.message and shown in Main screen render.
  timestamp: 2026-03-26

- hypothesis: Admin PIN extracted incorrectly from PinInputState
  evidence: PinInputState with single "Admin PIN" field. values().into_iter().next() correctly extracts first field value. all_filled() prevents Submit when field is empty. PIN bytes sent as ASCII.
  timestamp: 2026-03-26

- hypothesis: has_key guard bypassed (policy set on empty slot)
  evidence: The has_key guard checks yubikey_state.openpgp.signature_key.is_some() before navigating to SetTouchPolicySelect. If has_key is False, user sees "No key in X slot" and cannot proceed. User reports wizard completes, so has_key must be True.
  timestamp: 2026-03-26

- hypothesis: Reconnect verification false positives
  evidence: Commits 46e6e22, 036bbbf, 996db59 show: (1) same-session readback correctly detected failure for empty slots; (2) reconnect readback then showed failure for OCCUPIED slots; (3) reconnect verification was removed as "false failure." The reconnect readback showing old value for occupied slots is CONSISTENT with card being reset (RESET_CARD) between PUT DATA and reconnect readback — not a false failure.
  timestamp: 2026-03-26

- hypothesis: SCARD_RESET_CARD clears write-back buffer before EEPROM commit (LeaveCard fix)
  evidence: LeaveCard fix was applied (card.disconnect(LeaveCard) before return). User reports touch policy STILL shows Off after wizard completes. LeaveCard either didn't solve the problem or the write-back model assumption is incorrect for this YubiKey firmware.
  timestamp: 2026-03-26T02

## Evidence

- timestamp: 2026-03-26T01
  checked: pcsc-2.9.0/src/lib.rs Card::drop implementation
  found: impl Drop for Card { fn drop(&mut self) { let _err = ffi::SCardDisconnect(self.handle, Disposition::ResetCard.into_raw()); } }
  implication: Every time a pcsc::Card goes out of scope (including at end of set_touch_policy), SCardDisconnect(SCARD_RESET_CARD) is called. This resets the YubiKey. If YubiKey uses write-back caching for EEPROM writes, the reset clears the buffer before the UIF value is committed.

- timestamp: 2026-03-26T02
  checked: git log for src/model/touch_policy.rs — commits 46e6e22, 036bbbf, 996db59
  found: Three commits show iterative attempts to verify touch policy persistence. 46e6e22 added same-session readback (detected failure for empty slots). 036bbbf switched to reconnect readback (detected failure for OCCUPIED slots too). 996db59 removed all verification ("false failures for occupied slots"). The reconnect failure for occupied slots is the smoking gun — same-session showed new value (RAM buffer), reconnect showed old value (EEPROM, post-reset).
  implication: Pattern matches SCARD_RESET_CARD clearing write buffer. PUT DATA writes to session-local buffer, returns SW 9000. RESET_CARD clears buffer. EEPROM unchanged.

- timestamp: 2026-03-26T03
  checked: pcsc::Card::disconnect method signature
  found: pub fn disconnect(mut self, disposition: Disposition) -> Result<(), (Card, Error)>. If called with Disposition::LeaveCard, disconnects without resetting. If error, returns card in Err tuple which gets dropped with RESET_CARD.
  implication: Fix: call card.disconnect(pcsc::Disposition::LeaveCard).ok() before returning Ok from set_touch_policy. This prevents RESET_CARD from clearing the write buffer, allowing YubiKey EEPROM commit to complete.

- timestamp: 2026-03-26T04
  checked: Full code path from UI input to set_touch_policy call
  found: touch_slot_name(index) → "sig"/"enc"/"aut"/"att" (correct). touch_policy_from_index(index) → TouchPolicy::On etc (correct). PinInputState extracts admin_pin from first field value. execute_touch_policy_set calls set_touch_policy then YubiKeyState::detect_all() for refresh. No mock_mode bypass of set_touch_policy (only detect_all is skipped in mock mode). LeaveCard fix is in code at line 217. Same disconnect issue: if disconnect(LeaveCard) fails it returns Err((Card, Error)) and let _ drops the card with RESET_CARD.
  implication: The code paths all look correct. The actual failure mode is unknown without observing the wire-level APDU exchange. Added tracing::debug! at every step: KDF check result, VERIFY SW, PUT DATA SW, same-session readback byte, disconnect result.

- timestamp: 2026-03-26T05
  checked: Whether same-session readback was ever tried after LeaveCard fix
  found: The readback was removed in commit 996db59 and NOT restored when the LeaveCard fix was added. There is no evidence of whether PUT DATA is actually persisting within the same session (card-internal state) before the disconnect happens.
  implication: Added same-session GET DATA readback in debug build. This will show whether the YubiKey accepted the write at all (in-session) vs returning SW 9000 but silently not applying the change.

- timestamp: 2026-03-26T06
  checked: pcsc crate version and Disposition::LeaveCard variant validity
  found: Cargo.lock resolves pcsc="2.8" spec to pcsc-2.9.0. In pcsc-2.9.0/src/lib.rs: Disposition enum has LeaveCard = ffi::SCARD_LEAVE_CARD. Card::disconnect(Disposition) exists. Card::drop uses Disposition::ResetCard. pcsc::Disposition::LeaveCard is a valid variant — current code compiles and is correct.
  implication: The LeaveCard call is syntactically and semantically correct. The question is whether it actually succeeds at runtime.

- timestamp: 2026-03-26T07
  checked: Diagnostic codes embedded in Ok() return string
  found: Modified set_touch_policy to rename sw -> verify_sw and put_sw, extract readback_byte as a named local (0xFF if GET DATA failed), and return Ok(format!("... [VERIFY={:#06X} PUT={:#06X} readback={:#04X}]", ...)). 87/87 tests pass.
  implication: User will see exact SW codes and readback byte in the TUI result screen without --debug flag or log file. Will reveal exact failure point.

- timestamp: 2026-03-26T04
  checked: User stderr output from debug eprintln! statements in app.rs
  found: "DEBUG: set_touch_policy Err: This YubiKey uses KDF PIN hashing. Touch policy changes require ykman on this device."
  implication: ROOT CAUSE CONFIRMED. KDF is active (DO 0xF9 non-empty and byte[0] != 0x00). The existing code detects KDF and bails instead of hashing. Fix: parse DO 0xF9 BER-TLV, extract salt_pw3+count+algo, compute Iterated S2K SHA-256, use hashed bytes for VERIFY.

## Resolution

root_cause: |
  DO 0xF9 on this YubiKey returns [81 01 00] — tag 0x81 (KDF algorithm byte), length 1,
  value 0x00 (no KDF / algorithm = none). The code read kdf_data[0] as a raw byte, saw 0x81
  (the TLV tag byte for "algorithm"), and incorrectly concluded that KDF was active.
  It then tried to parse tags 0x82/0x83/0x86 from a single-byte "no KDF" response — those tags
  don't exist in this response, so parse_kdf_do() returned an Err, and set_touch_policy() bailed
  with "Touch policy changes require ykman on this device" without ever sending VERIFY or PUT DATA.

  The fix required reading tag 0x81's VALUE (the byte after the length) — not the raw first byte
  of the DO 0xF9 response — to determine whether KDF is active. Algorithm value 0x00 means no KDF,
  so parse_kdf_do() must return Ok(None) immediately when the 0x81 tag's value is 0x00.

fix: |
  1. Added `sha2 = "0.10"` to Cargo.toml.
  2. Added `parse_kdf_do()` helper that parses DO 0xF9 BER-TLV properly: reads tag 0x81's VALUE
     (not the raw first byte) to determine algorithm; returns Ok(None) when algorithm == 0x00
     (no KDF); parses tags 0x82/0x83/0x86 only when KDF is active.
  3. Added `kdf_hash_pin()` public helper that takes raw DO 0xF9 bytes and a PIN, calls
     parse_kdf_do(), builds the S2K input, and returns SHA-256(input). Rejects non-SHA-256 algos.
  4. In set_touch_policy(), replaced the KDF bail with: if KDF active → kdf_hash_pin() → use
     hash as pin_to_verify; if inactive → raw PIN bytes. VERIFY APDU is built from pin_to_verify.
  5. Removed 3 debug eprintln! statements from src/app.rs::execute_touch_policy_set.
  6. Added 7 unit tests for parse_kdf_do and kdf_hash_pin (known vectors, edge cases, error paths).

verification: |
  94/94 cargo tests pass (7 new KDF tests added).
  User confirmed: gpg --card-status shows "UIF setting ......: Sign=on Decrypt=off Auth=off"
  after setting Signature touch policy to On. Original wizard flow now works end-to-end.
files_changed:
  - Cargo.toml
  - src/model/touch_policy.rs
  - src/app.rs
