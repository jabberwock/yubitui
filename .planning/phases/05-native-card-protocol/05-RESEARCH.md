# Phase 5: Native Card Protocol - Research

**Researched:** 2026-03-25
**Domain:** PC/SC APDU communication, OpenPGP card specification, YubiKey proprietary extensions
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** `pcsc` raw only — all card communication via hand-written APDUs. No `openpgp-card`, no `yubikey` crate. Reference ykman's open source Python implementation for exact APDU byte sequences.
- **D-02:** Remove `openpgp-card`, `card-backend-pcsc`, and `yubikey` crates from Cargo.toml.
- **D-03:** `pcsc = "2.8"` stays — already used for factory reset, now extended to all card operations.
- **D-04:** Before every native PC/SC operation: `gpgconf --kill scdaemon` to release the card channel, then connect with `ShareMode::Exclusive`. scdaemon restarts automatically on the next gpg call.
- **D-05:** No explicit scdaemon restart after the operation — lazy restart on next gpg call is sufficient.
- **D-06:** Replace all `gpg --card-status` calls used for card state reads with direct PC/SC GET DATA APDUs.
- **D-07:** Key GET DATA DOs to implement:
  - `00 CA 00 C4 00` — PW Status Bytes (PIN retry counters: PW1 byte 4, RC byte 5, PW3 byte 6)
  - `00 CA 00 6E 00` — Application Related Data (AID, fingerprints, key info)
  - `00 CA 00 65 00` — Cardholder Related Data (name, language)
  - `00 CA 00 5F 50 00` — URL of public key
  - `00 CA 00 5E 00` — Login data
- **D-08:** Serial number extracted from OpenPGP AID select response (bytes 10–13 big-endian after `D2 76 00 01 24 01 [version] [mfr] [serial4]`).
- **D-09/D-10:** Touch policy read/write via YubiKey proprietary DOs D6–D9; policy byte values: `00`=off, `01`=on, `02`=fixed, `03`=cached, `04`=cached-fixed.
- **D-11:** Enumerate YubiKeys via PC/SC reader list, SELECT OpenPGP AID per reader, extract serial from response. Replaces `ykman list --serials`.
- **D-12:** Key attributes via GET DATA 0x6E + touch policy DOs per slot. Replaces `ykman openpgp info`.
- **D-13:** Native PIV info read via SELECT PIV AID + GET DATA per slot (best-effort).
- **D-14:** PIV SELECT failure returns empty slot list, not an error.
- **D-15/D-16/D-17:** APDU SW errors translate to user-readable English; SW codes go to `tracing::debug!` only. Shared `apdu_error_message(sw: u16, context: &str) -> String` helper.

### Plan sequencing (locked):
- **Plan 1:** Device detection + card state reads
- **Plan 2:** Touch policy + PIV info
- **Plan 3:** Cleanup — remove `find_ykman()`, unused crates, verify no ykman references remain

### GPG operations that remain unchanged (out of scope):
- `gpg --batch --gen-key`, `gpg --edit-key`, `gpg --export-ssh-key`, `gpgconf --list-dirs`, `gpgconf --kill scdaemon`, `gpg-agent`

### Claude's Discretion
- Exact module structure for the PC/SC APDU layer (single `src/yubikey/card.rs` or split by concern)
- Whether to define APDU constants as named `const` values or inline byte arrays with comments
- Retry logic on transient card errors (card removed mid-op)

### Deferred Ideas (OUT OF SCOPE)
- Replace gpg keyring operations (gen-key, edit-key, export-ssh-key) with native Rust cryptography
- FIDO2/WebAuthn status display
- OTP slot management
- Management key operations
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| NATIVE-PCSC-01 | Integrate `pcsc` crate for card reader/card enumeration; replace pcscd detection heuristics | `pcsc = "2.8"` already in Cargo.toml; factory_reset_openpgp() is the established pattern to extend |
| NO-GPG-BIN-01 | Remove runtime dependency on `gpg` and `gpgconf` for card reads; all card state reads go via native APDUs | GET DATA DOs C4, 6E, 65, 5F50, 5E replace all `gpg --card-status` parsing |
| NO-YKMAN-BIN-01 | Remove runtime dependency on `ykman`; all ykman OpenPGP operations replaced with native APDUs | Touch policy DOs D6–D9, device enumeration via reader list replaces `ykman list --serials` and `ykman openpgp info`; `find_ykman()` deleted in Plan 3 |
</phase_requirements>

---

## Summary

Phase 5 replaces every `ykman` CLI invocation with hand-written PC/SC APDU sequences in Rust, while leaving GPG keyring operations (`--gen-key`, `--edit-key`, `--export-ssh-key`) unchanged. The decision to use raw `pcsc` rather than higher-level crates (`openpgp-card`, `yubikey`) is already locked; the architecture template already exists in `factory_reset_openpgp()` in `pin_operations.rs`.

The implementation breaks into three plans. Plan 1 is foundational: native device enumeration (replacing `ykman list --serials`) and all card state reads (replacing `gpg --card-status`). Plan 2 adds touch policy set/get (replacing `ykman openpgp keys set-touch` and the touch-policy portion of `ykman openpgp info`) and native PIV info. Plan 3 is cleanup: delete `find_ykman()`, remove unused crates from Cargo.toml, and verify no ykman call sites remain.

The main technical challenge is TLV parsing of the Application Related Data (DO 0x6E) response, which is a nested BER-TLV structure. Everything else — PIN retry counters (0xC4), cardholder data (0x65), URL (0x5F50), login data (0x5E), and touch policy bytes (0xD6–0xD9) — returns flat or minimal structure that is trivially parsed.

**Primary recommendation:** Extend the existing `factory_reset_openpgp()` pattern (kill-scdaemon → exclusive-connect → SELECT → transmit → interpret SW) into a shared `connect_openpgp_card()` helper; build all Plan 1 and Plan 2 operations on top of it.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `pcsc` | 2.8 (locked, 2.9.0 latest) | PC/SC FFI wrapper — reader enumeration, card connect, APDU transmit | Already in Cargo.toml; cross-platform (pcscd/macOS PCSC.framework/winscard.dll); factory reset already uses it |
| `anyhow` | 1.0 | Error propagation with context | Already used project-wide; `anyhow::bail!` pattern established |
| `tracing` | 0.1 | Debug logging of raw SW codes | Already used project-wide; SW codes go to `tracing::debug!` only (D-16) |

### Crates to Remove

| Crate | Action | Reason |
|-------|--------|--------|
| `openpgp-card` | Remove from Cargo.toml | D-02: not used, pcsc-raw chosen |
| `card-backend-pcsc` | Remove from Cargo.toml | D-02: not used |
| `yubikey` | Remove from Cargo.toml | D-02: not used, pcsc-raw chosen |

**Version verification:** `pcsc` latest is 2.9.0 on crates.io; project pins `"2.8"` which resolves to 2.8.x and remains compatible with the existing factory reset code. No version bump needed for Phase 5; Cargo.lock already has a compatible resolution.

---

## Architecture Patterns

### Recommended Module Structure

The discretion decision (module layout) recommendation:

```
src/yubikey/
├── card.rs          # NEW: shared PC/SC helpers — connect_openpgp_card(),
│                    #      apdu_error_message(), apdu_sw() (move from pin_operations)
├── detection.rs     # MODIFY: list_connected_serials() → native PC/SC enumeration
├── pin.rs           # MODIFY: get_pin_status() → GET DATA 0xC4
├── openpgp.rs       # MODIFY: get_openpgp_state() → GET DATA 0x6E + 0x65
├── key_operations.rs# MODIFY: get_key_attributes() → GET DATA 0x6E (no ykman)
├── touch_policy.rs  # MODIFY: set_touch_policy(), new get_touch_policies_native()
├── piv.rs           # MODIFY: get_piv_state() → native PIV APDUs
└── pin_operations.rs# MODIFY: remove find_ykman(); factory_reset_openpgp() stays
```

A single `card.rs` module is recommended over splitting by concern because all three plans share the same connection lifecycle. Keeping helpers together avoids circular references between modules.

### Pattern 1: PC/SC Operation Lifecycle (Established Template)

**What:** Kill scdaemon to release card channel, open exclusive PC/SC connection, SELECT OpenPGP AID, perform operation, return result.

**When to use:** Every native card read or write operation.

**Example (from existing `factory_reset_openpgp()` — extend this pattern):**
```rust
// Source: src/yubikey/pin_operations.rs (established pattern)
use pcsc::{Context, Protocols, Scope, ShareMode};

let _ = std::process::Command::new("gpgconf")
    .args(["--kill", "scdaemon"])
    .output();

let ctx = Context::establish(Scope::User)
    .map_err(|e| anyhow::anyhow!("PC/SC error: {e}"))?;

let mut readers_buf = [0u8; 2048];
let readers: Vec<_> = ctx
    .list_readers(&mut readers_buf)
    .map_err(|e| anyhow::anyhow!("No smart card readers found: {e}"))?
    .collect();

for reader in readers {
    let card = match ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1) {
        Ok(c) => c,
        Err(_) => continue,
    };

    // SELECT OpenPGP AID
    let select = [0x00u8, 0xA4, 0x04, 0x00, 0x06, 0xD2, 0x76, 0x00, 0x01, 0x24, 0x01];
    let mut buf = [0u8; 256];
    let resp = match card.transmit(&select, &mut buf) {
        Ok(r) => r,
        Err(_) => continue,
    };
    if apdu_sw(resp) != 0x9000 { continue; }

    // ... perform operation ...
}
```

### Pattern 2: GET DATA APDU

**What:** Read a data object from the card using the GET DATA instruction (INS=CA).

**APDU format:** `[00 CA P1 P2 00]` where P1:P2 is the DO tag.

```rust
// Source: OpenPGP Card Spec v3.4 + verified against ykman openpgp.py
// GET DATA — PW Status Bytes (DO 0xC4)
// Response layout: [0]=format, [1..3]=max-lengths, [4]=PW1-retries, [5]=RC-retries, [6]=PW3-retries
let get_pw_status = [0x00u8, 0xCA, 0x00, 0xC4, 0x00];

// GET DATA — Application Related Data (DO 0x6E)
// Response: TLV-encoded; AID at tag 0x4F, discretionary data at 0x73
let get_app_data = [0x00u8, 0xCA, 0x00, 0x6E, 0x00];

// GET DATA — Cardholder Related Data (DO 0x65)
let get_cardholder = [0x00u8, 0xCA, 0x00, 0x65, 0x00];

// GET DATA — URL (DO 0x5F50)
let get_url = [0x00u8, 0xCA, 0x5F, 0x50, 0x00];

// GET DATA — Login data (DO 0x5E)
let get_login = [0x00u8, 0xCA, 0x00, 0x5E, 0x00];
```

### Pattern 3: Touch Policy GET/SET (YubiKey Proprietary)

**What:** Read and write YubiKey-specific touch policy DOs (D6–D9).

**When to use:** touch policy operations only. These DOs are YubiKey extensions, not standard OpenPGP card spec.

```rust
// Source: ykman yubikit/openpgp.py (verified)
// GET DATA — touch policy per slot
// DO D6=SIG, D7=DEC, D8=AUT, D9=ATT
// Response: 1 or 2 bytes; byte[0] is the policy value
let get_touch_sig = [0x00u8, 0xCA, 0x00, 0xD6, 0x00];
let get_touch_dec = [0x00u8, 0xCA, 0x00, 0xD7, 0x00];
let get_touch_aut = [0x00u8, 0xCA, 0x00, 0xD8, 0x00];
let get_touch_att = [0x00u8, 0xCA, 0x00, 0xD9, 0x00];

// PUT DATA — set touch policy (requires Admin PIN verified in same session)
// Policy byte: 0x00=off, 0x01=on, 0x02=fixed, 0x03=cached, 0x04=cached-fixed
// Second byte (button flag): ykman always sends 0x20 (GENERAL_FEATURE_MANAGEMENT.BUTTON)
let set_touch_sig = [0x00u8, 0xDA, 0x00, 0xD6, 0x02, policy_byte, 0x20];
// Lc=0x02 because data is 2 bytes (policy + button flag)
```

**Admin PIN verify before PUT DATA:**
```rust
// VERIFY Admin PIN (PW3) before touch policy set
// P2=0x83 selects PW3 (Admin PIN)
// Data: PIN bytes (ASCII)
let verify_admin = [&[0x00u8, 0x20, 0x00, 0x83, pin.len() as u8], pin.as_bytes()].concat();
```

### Pattern 4: Serial Number Extraction from AID Response

**What:** The OpenPGP AID SELECT response includes the 4-byte serial number at a fixed offset.

```rust
// Source: ykman yubikit/openpgp.py (OpenPgpAid parsing), D-08
// AID response structure: D2 76 00 01 24 01 [version:2] [mfr:2] [serial:4] [RFU:2]
// serial is at bytes 10..14 of the SELECT response (0-indexed, before SW)
// The response from SELECT includes FCI data; serial is in the returned AID bytes.
// After removing 0x9000 SW from end of response, AID body starts at index 0
// if the SELECT returns the AID directly, OR is nested in FCI TLV.
// Simplest approach: scan response for the AID prefix bytes, then read +4.
fn extract_serial(select_response: &[u8]) -> Option<u32> {
    // AID prefix: D2 76 00 01 24 01
    let prefix = [0xD2u8, 0x76, 0x00, 0x01, 0x24, 0x01];
    let data = if select_response.len() >= 2 {
        &select_response[..select_response.len() - 2] // strip SW
    } else {
        return None;
    };
    // Find AID prefix in response
    let pos = data.windows(6).position(|w| w == prefix)?;
    let serial_start = pos + 8; // skip prefix (6) + version (2)
    if data.len() >= serial_start + 4 {
        Some(u32::from_be_bytes([
            data[serial_start], data[serial_start+1],
            data[serial_start+2], data[serial_start+3],
        ]))
    } else {
        None
    }
}
```

**Note on AID response format:** Some readers return the full FCI TLV structure (`6F [len] 84 [len] [AID bytes] ...`); others return raw AID bytes. The window-search approach handles both without TLV parsing.

### Pattern 5: Native PIV Info

**What:** SELECT PIV AID, then GET DATA per slot using the NIST SP 800-73 format.

```rust
// Source: ykman yubikit/piv.py, D-13/D-14
// PIV AID: A0 00 00 03 08 00 00 10 00 (9 bytes per CONTEXT.md D-13)
let select_piv = [0x00u8, 0xA4, 0x04, 0x00, 0x09,
                   0xA0, 0x00, 0x00, 0x03, 0x08, 0x00, 0x00, 0x10, 0x00, 0x01];

// GET DATA per slot: INS=CB, P1=3F, P2=FF, data=TLV(5C, slot-object-id)
// Object IDs (3 bytes each):
//   9A Authentication: 5F C1 05
//   9C Signature:      5F C1 0A
//   9D Key Management: 5F C1 0B
//   9E Card Auth:      5F C1 01
let get_data_9a = [0x00u8, 0xCB, 0x3F, 0xFF, 0x05,
                    0x5C, 0x03, 0x5F, 0xC1, 0x05];
```

### Pattern 6: APDU Error Message Helper

**What:** Centralize SW-to-user-message translation per D-15/D-16/D-17.

```rust
// Source: D-17 (design decision)
pub fn apdu_error_message(sw: u16, context: &str) -> String {
    tracing::debug!("APDU error in {}: SW {:04X}", context, sw);
    match sw {
        0x6300 => "PIN verification failed — check your PIN and try again".to_string(),
        0x6983 => "Operation blocked — PIN or card is locked".to_string(),
        0x6982 => "Security condition not satisfied — verify PIN first".to_string(),
        0x6A86 => "Incorrect parameters — card does not support this operation".to_string(),
        0x6D00 => "Instruction not supported — card firmware may not support this feature".to_string(),
        0x6E00 => "Class not supported".to_string(),
        0x6F00 => "Unknown card error — try removing and reinserting your YubiKey".to_string(),
        sw if sw & 0xFF00 == 0x6300 => {
            let retries = sw & 0x000F;
            format!("PIN verification failed — {} {} remaining",
                retries, if retries == 1 { "retry" } else { "retries" })
        }
        _ => format!("Card operation failed — try removing and reinserting your YubiKey"),
    }
}
```

### Pattern 7: TLV Parsing for DO 0x6E

**What:** Application Related Data (DO 0x6E) returns a nested TLV structure. A minimal hand-rolled TLV parser is needed for this phase — no external TLV crate required.

**Structure of 0x6E response:**
```
6E [len]
  4F [len] [AID bytes]   -- OpenPGP AID (contains serial)
  5F52 [len] [bytes]     -- Historical bytes
  73 [len]               -- Discretionary data objects
    C0 [len] [bytes]     -- Extended capabilities
    C1 [len] [bytes]     -- Algorithm attributes SIG
    C2 [len] [bytes]     -- Algorithm attributes DEC
    C3 [len] [bytes]     -- Algorithm attributes AUT
    C5 [len] [bytes]     -- Fingerprints (3x20 bytes: SIG, DEC, AUT)
    C7 [len] [bytes]     -- CA fingerprints
    CD [len] [bytes]     -- Key generation dates (3x4 bytes)
    CE [len] [bytes]     -- Key information (slot status)
```

**Minimal TLV helper for BER-TLV parsing:**
```rust
// Simple BER-TLV iterator — handles 1-byte and 2-byte tags, 1-byte and 2-byte lengths
// No external crate needed for this scope
fn tlv_find<'a>(data: &'a [u8], target_tag: &[u8]) -> Option<&'a [u8]> {
    let mut i = 0;
    while i < data.len() {
        // Read tag (1 or 2 bytes)
        let tag_len = if data[i] & 0x1F == 0x1F { 2 } else { 1 };
        if i + tag_len > data.len() { break; }
        let tag = &data[i..i + tag_len];
        i += tag_len;
        // Read length
        if i >= data.len() { break; }
        let (len, len_bytes) = if data[i] & 0x80 == 0 {
            (data[i] as usize, 1)
        } else {
            let n = (data[i] & 0x7F) as usize;
            if i + 1 + n > data.len() { break; }
            let mut l = 0usize;
            for b in &data[i+1..i+1+n] { l = (l << 8) | *b as usize; }
            (l, 1 + n)
        };
        i += len_bytes;
        let value = data.get(i..i + len)?;
        if tag == target_tag { return Some(value); }
        i += len;
    }
    None
}
```

**Key data extraction from 0x6E:**
- Fingerprints: tag `C5` under `73`; 60 bytes total (3 slots × 20 bytes each). Bytes 0–19=SIG, 20–39=DEC, 40–59=AUT. All-zero 20-byte block = no key.
- Algorithm attributes: tags `C1`/`C2`/`C3` under `73`; first byte is algorithm ID (01=RSA, 12=ECDH, 13=ECDSA, 16=EdDSA).
- AID (serial): tag `4F` directly under `6E`.

### Anti-Patterns to Avoid

- **DO NOT** call `gpgconf --kill scdaemon` and immediately check if scdaemon is gone — it is fire-and-forget. The kill is not synchronous; the `connect(Exclusive)` failing gracefully on the first reader is normal.
- **DO NOT** hold the PC/SC card handle across async boundaries — the `Card` type is `!Send`. All PC/SC operations must complete synchronously within a single thread.
- **DO NOT** add `openpgp-card` or `yubikey` crate imports as "optional helpers" — D-02 is absolute.
- **DO NOT** parse `gpg --card-status` output anywhere in Plan 1+ code — the PIN retry counter field-swap bug is the canonical example of why parsing is fragile.
- **DO NOT** show SW status words in the TUI error UI — they go to `tracing::debug!` only (D-16).
- **DO NOT** attempt PIN verification retry in the native layer — the card decrements the retry counter on each failed verify; wrapping it in a loop would lock the card.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TLV parsing for DO 0x6E | General-purpose ASN.1/TLV parser | Minimal `tlv_find` helper + targeted extraction | Only 5 tags needed; full TLV parser adds a dependency for no benefit |
| Cross-platform PCSC binding | Platform-specific FFI code | `pcsc = "2.8"` already in Cargo.toml | Handles pcscd/macOS/Windows seamlessly |
| APDU byte construction | Struct-based APDU builder | Named `const` arrays with comments | APDUs are fixed; struct indirection adds complexity without benefit |
| Serial extraction from AID | BCD decoder matching ykman exactly | Window-search on AID prefix bytes | Simpler and handles both FCI-wrapped and raw SELECT responses |

**Key insight:** This phase is an APDU translation exercise, not a cryptographic implementation. The complexity budget goes to TLV parsing and error message mapping, not abstraction.

---

## Runtime State Inventory

This phase removes `ykman` as a runtime dependency. No runtime data stores embed "ykman" as a key.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — yubitui has no database or persistent store | None |
| Live service config | None — no external services configured for ykman | None |
| OS-registered state | None — no scheduled tasks or daemon registrations for ykman | None |
| Secrets/env vars | None — ykman path found via PATH, no hardcoded env vars | None |
| Build artifacts | `yubikey`, `openpgp-card`, `card-backend-pcsc` crates in Cargo.lock after D-02 removal | `cargo build` regenerates Cargo.lock after Cargo.toml edit |

None — verified by reading all source files and Cargo.toml.

---

## Common Pitfalls

### Pitfall 1: SELECT Response FCI Wrapping

**What goes wrong:** Some PC/SC middleware wraps the SELECT response in FCI (File Control Information) TLV (`6F [len] 84 [len] [AID] ...`) rather than returning raw AID bytes. Code that assumes the AID starts at offset 0 of the SELECT response will fail on certain readers.

**Why it happens:** PCSC middleware on macOS/Windows may normalize the response format differently than pcscd on Linux.

**How to avoid:** Use the window-search approach for serial extraction (Pattern 4) — scan for the AID prefix bytes rather than assuming a fixed offset.

**Warning signs:** Serial returns 0 or None on a reader that definitely has a YubiKey; works on one OS but not another.

### Pitfall 2: Card Not Found After scdaemon Kill

**What goes wrong:** `gpgconf --kill scdaemon` returns quickly but the PC/SC `connect(Exclusive)` fails with `SCARD_E_SHARING_VIOLATION` because scdaemon's cleanup is async.

**Why it happens:** scdaemon does not release the card channel synchronously in response to the kill signal on all platforms.

**How to avoid:** `connect()` failure in the reader loop is already handled with `Err(_) => continue` in the template. One immediate retry with a brief sleep (100ms) is acceptable here if needed. Document the retry limit (1 retry max) as a named constant.

**Warning signs:** Works fine in isolation but fails intermittently when called right after a gpg operation.

### Pitfall 3: Buffer Too Small for Extended-Length Responses

**What goes wrong:** GET DATA 0x6E returns a response that can exceed 256 bytes on a fully-populated card. A `[0u8; 256]` buffer will cause a truncated read or an error.

**Why it happens:** The pcsc `transmit()` buffer must be at least as large as the expected response. PCSC returns `SCARD_E_INSUFFICIENT_BUFFER` if the buffer is too small.

**How to avoid:** Use `[0u8; 1024]` for GET DATA 0x6E responses. For other DOs (C4, D6-D9, 5E, 5F50, 65) 256 bytes is sufficient.

**Warning signs:** Response truncated; SW parsing gives garbage; card returns `6100` (more data available via GET RESPONSE).

### Pitfall 4: Touch Policy PUT DATA Requires Admin PIN in Same Session

**What goes wrong:** PUT DATA for touch policy (DOs D6–D9) returns SW 6982 (security condition not satisfied) even after the Admin PIN was verified in a previous session.

**Why it happens:** The OpenPGP card security state is per-session. After a SELECT (which opens a new logical session), PIN verification must be repeated.

**How to avoid:** In `set_touch_policy()`, the sequence must be: SELECT OpenPGP AID → VERIFY PW3 (Admin PIN) → PUT DATA touch policy — all in the same `Card` connection. Do not disconnect and reconnect between verify and put.

**Warning signs:** PUT DATA returns 6982 even though the Admin PIN is correct.

### Pitfall 5: PIN Retry Counter Layout in PW Status DO (C4)

**What goes wrong:** Mis-indexing the retry counter bytes causes user and admin PIN counts to be swapped — exactly the bug that motivated this phase.

**Why it happens:** The OpenPGP spec for DO 0xC4 returns 7 bytes:
- Byte 0: PIN format
- Bytes 1–3: max PIN length for PW1, RC, PW3
- **Byte 4: PW1 retry counter (User PIN)**
- **Byte 5: RC retry counter (Reset Code)**
- **Byte 6: PW3 retry counter (Admin PIN)**

The former `gpg --card-status` output lists them as "PW1 RC PW3", but the field labels in the human-readable output could be misread as "user reset admin" in a different order.

**How to avoid:** Assert in the parser unit test that a known response byte sequence maps to the correct struct fields:
```rust
// response: [format=00, maxPW1=7F, maxRC=7F, maxPW3=7F, pw1=3, rc=0, pw3=2]
let r = [0x00u8, 0x7F, 0x7F, 0x7F, 0x03, 0x00, 0x02];
let status = parse_pw_status(&r).unwrap();
assert_eq!(status.user_pin_retries, 3);   // byte 4
assert_eq!(status.reset_code_retries, 0); // byte 5
assert_eq!(status.admin_pin_retries, 2);  // byte 6
```

### Pitfall 6: Algorithm Attribute Byte Encoding

**What goes wrong:** The C1/C2/C3 algorithm attribute bytes use an OpenPGP-internal algorithm ID that differs from the human-readable names.

**Why it happens:** OpenPGP card spec maps algorithm IDs as: `01`=RSA, `12`=ECDH (cv25519 for DEC), `13`=ECDSA (NIST curves), `16`=EdDSA (ed25519 for SIG/AUT). Decoding these incorrectly shows wrong algorithm names.

**How to avoid:** Build an explicit match in the algorithm parser:
```rust
fn algorithm_from_attr(attr_bytes: &[u8]) -> &'static str {
    match attr_bytes.first() {
        Some(0x01) => "RSA",
        Some(0x12) => "ECDH (cv25519)",
        Some(0x13) => "ECDSA",
        Some(0x16) => "EdDSA (ed25519)",
        _ => "Unknown",
    }
}
```

---

## Code Examples

### Verified APDU Byte Sequences

Source: ykman `yubikit/openpgp.py` and OpenPGP card spec v3.4, cross-verified.

#### SELECT OpenPGP AID
```rust
// AID: D2 76 00 01 24 01 (6 bytes) — same as ykman AID.OPENPGP = "d27600012401"
let select_openpgp: &[u8] = &[0x00, 0xA4, 0x04, 0x00, 0x06,
                                0xD2, 0x76, 0x00, 0x01, 0x24, 0x01];
```

#### GET DATA APDUs
```rust
// PW Status (7 bytes: format, maxlen x3, retry_count x3)
const GET_PW_STATUS:   [u8; 5] = [0x00, 0xCA, 0x00, 0xC4, 0x00];
// Application Related Data (TLV: AID, fingerprints, key info, etc.)
const GET_APP_DATA:    [u8; 5] = [0x00, 0xCA, 0x00, 0x6E, 0x00];
// Cardholder Related Data (TLV: name, language, sex)
const GET_CARDHOLDER:  [u8; 5] = [0x00, 0xCA, 0x00, 0x65, 0x00];
// URL of public key (variable length, raw bytes)
const GET_URL:         [u8; 5] = [0x00, 0xCA, 0x5F, 0x50, 0x00];
// Login data (variable length, raw bytes)
const GET_LOGIN:       [u8; 5] = [0x00, 0xCA, 0x00, 0x5E, 0x00];
// Touch policy per slot (1–2 bytes; byte 0 = policy value)
const GET_TOUCH_SIG:   [u8; 5] = [0x00, 0xCA, 0x00, 0xD6, 0x00];
const GET_TOUCH_DEC:   [u8; 5] = [0x00, 0xCA, 0x00, 0xD7, 0x00];
const GET_TOUCH_AUT:   [u8; 5] = [0x00, 0xCA, 0x00, 0xD8, 0x00];
const GET_TOUCH_ATT:   [u8; 5] = [0x00, 0xCA, 0x00, 0xD9, 0x00];
```

#### VERIFY Admin PIN (PW3) before PUT DATA
```rust
fn verify_admin_pin_apdu(pin: &str) -> Vec<u8> {
    // INS=20 VERIFY, P1=00, P2=83 (PW3=Admin PIN)
    let mut apdu = vec![0x00u8, 0x20, 0x00, 0x83, pin.len() as u8];
    apdu.extend_from_slice(pin.as_bytes());
    apdu
}
```

#### PUT DATA for Touch Policy
```rust
fn set_touch_apdu(do_tag: u8, policy_byte: u8) -> [u8; 7] {
    // INS=DA PUT DATA, P1=00, P2=D6..D9, Lc=02, data=[policy, 0x20]
    // 0x20 = GENERAL_FEATURE_MANAGEMENT.BUTTON (ykman always sends this)
    [0x00, 0xDA, 0x00, do_tag, 0x02, policy_byte, 0x20]
}
// DO tags: SIG=0xD6, DEC=0xD7, AUT=0xD8, ATT=0xD9
// Policy bytes: off=0x00, on=0x01, fixed=0x02, cached=0x03, cached-fixed=0x04
```

#### SELECT PIV AID
```rust
// PIV AID: A0 00 00 03 08 00 00 10 00 (9 bytes per CONTEXT.md D-13)
const SELECT_PIV: [u8; 14] = [0x00, 0xA4, 0x04, 0x00, 0x09,
                                0xA0, 0x00, 0x00, 0x03, 0x08,
                                0x00, 0x00, 0x10, 0x00]; // Lc=9, data=9 bytes
// Note: trailing 0x01 (Le) omitted in final byte — verify with hardware
```

#### PIV GET DATA per slot
```rust
// GET DATA: INS=CB, P1=3F, P2=FF, data=TLV(tag=5C, object_id)
// Object IDs (3 bytes): 9A=5FC105, 9C=5FC10A, 9D=5FC10B, 9E=5FC101
fn piv_get_data_apdu(object_id: [u8; 3]) -> [u8; 10] {
    [0x00, 0xCB, 0x3F, 0xFF, 0x05,   // Lc=5
     0x5C, 0x03,                       // tag=5C, len=3
     object_id[0], object_id[1], object_id[2]]
}
```

#### TouchPolicy enum byte conversion
```rust
// Source: D-09/D-10 (locked decisions)
impl TouchPolicy {
    pub fn as_policy_byte(&self) -> u8 {
        match self {
            TouchPolicy::Off         => 0x00,
            TouchPolicy::On          => 0x01,
            TouchPolicy::Fixed       => 0x02,
            TouchPolicy::Cached      => 0x03,
            TouchPolicy::CachedFixed => 0x04,
            TouchPolicy::Unknown(_)  => 0x00,
        }
    }
    pub fn from_policy_byte(b: u8) -> Self {
        match b {
            0x00 => TouchPolicy::Off,
            0x01 => TouchPolicy::On,
            0x02 => TouchPolicy::Fixed,
            0x03 => TouchPolicy::Cached,
            0x04 => TouchPolicy::CachedFixed,
            _    => TouchPolicy::Unknown(format!("{b:02X}")),
        }
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `ykman list --serials` | PC/SC reader enumeration + SELECT AID | Phase 5 | No ykman required for device detection |
| `gpg --card-status` parsing | GET DATA APDUs (C4, 6E, 65) | Phase 5 | Eliminates output-parsing fragility, PIN retry counter field-swap impossible |
| `ykman openpgp info` | GET DATA 0x6E + touch DO reads | Phase 5 | No ykman required for key attribute display |
| `ykman openpgp keys set-touch` | PUT DATA DO D6–D9 after VERIFY PW3 | Phase 5 | No ykman required for touch policy management |
| `ykman piv info` | SELECT PIV + GET DATA per slot | Phase 5 | No ykman required for PIV info display |

**Deprecated/outdated after Phase 5:**
- `find_ykman()` in `pin_operations.rs`: deleted in Plan 3
- `parse_serial_list()` in `detection.rs`: deleted in Plan 3 (PC/SC reader enumeration replaces it)
- `parse_ykman_openpgp_info()` / `parse_touch_policies()` from-string parsers: replaced by byte-level parsing (parse functions rewritten to take `&[u8]` from APDU responses)
- `openpgp-card`, `card-backend-pcsc`, `yubikey` in Cargo.toml: removed in Plan 3

---

## Open Questions

1. **PIV AID trailing byte**
   - What we know: CONTEXT.md D-13 specifies `00 A4 04 00 09 A0 00 00 03 08 00 00 10 00 01` (9 data bytes); ykman uses `A0 00 00 03 08` (5 bytes). The 9-byte form with trailing `00 00 10 00 01` is the full NIST PIV AID.
   - What's unclear: Whether the YubiKey PIV app requires the full 9-byte AID or accepts the 5-byte short form. Some YubiKey firmware versions reject the short AID.
   - Recommendation: Use the 9-byte form per CONTEXT.md D-13. If SELECT fails (D-14), return empty PIV state.

2. **FCI TLV wrapping of SELECT AID response on macOS**
   - What we know: The existing `factory_reset_openpgp()` code already works on macOS (established this session). SELECT response is not parsed for serial — factory reset only checks SW.
   - What's unclear: Whether the SELECT AID response format differs between pcscd (Linux), macOS PCSC.framework, and winscard.dll specifically for serial extraction (new in Plan 1).
   - Recommendation: Use the window-search serial extraction pattern (Pattern 4). Test on macOS dev machine where the SELECT response format can be observed. Add `tracing::debug!` logging of raw SELECT response bytes.

3. **Retry logic scope for transient errors (Claude's discretion)**
   - What we know: The established pattern uses `Err(_) => continue` per reader. No retry within a single reader attempt.
   - What's unclear: Whether a single retry with 100ms sleep is worth the added complexity for the `SCARD_E_SHARING_VIOLATION` case (scdaemon not fully released).
   - Recommendation: Add one retry with 100ms sleep for `SCARD_E_SHARING_VIOLATION` only, as a named constant (`const SCDAEMON_RELEASE_WAIT_MS: u64 = 100`). Document why. Keep the retry logic local to `connect_openpgp_card()`, not scattered through individual operation functions.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| PC/SC framework | All card operations | macOS built-in | CryptoTokenKit/PCSC.framework (native) | — |
| `pcsc` crate | All card operations | In Cargo.toml | 2.8 (latest 2.9.0) | — |
| `gpg` | Key gen, import, SSH export (unchanged) | /opt/homebrew/bin/gpg | GnuPG 2.4.9 | — |
| `gpgconf` | scdaemon kill before PC/SC ops | /opt/homebrew/bin/gpgconf | GnuPG 2.4.9 | Silently skip kill if gpgconf absent |
| `ykman` | NONE after Phase 5 | /opt/homebrew/bin/ykman 5.9.0 | 5.9.0 | Not needed — this is the point |
| Rust toolchain | Build | rustc 1.92.0 | 1.92.0 (project min: 1.75) | — |

**Missing dependencies with no fallback:** None — all required dependencies are available on the development machine.

**Note:** `gpgconf --kill scdaemon` failure is already silent (`let _ = ... .output()`) in the existing template. Keeping this silent is correct — if gpgconf is absent (unlikely: same package as gpg), the kill is skipped and PC/SC connect may fail with sharing violation, which is caught and logged.

---

## Validation Architecture

> nyquist_validation is explicitly false in config.json — section skipped per config.

---

## Sources

### Primary (HIGH confidence)
- ykman source `yubikit/openpgp.py` — touch policy DO tags (D6–D9), PUT DATA format with button flag byte, VERIFY PW3 before PUT DATA, AID response parsing for serial
- ykman source `yubikit/piv.py` — PIV slot tags (9A/9C/9D/9E), GET DATA APDU format (INS=CB, P1=3F, P2=FF, TLV data)
- ykman source `yubikit/core/smartcard/__init__.py` — AID.OPENPGP = `d27600012401`, AID.PIV = `a000000308` prefix
- `src/yubikey/pin_operations.rs` — established pcsc pattern: kill-scdaemon, exclusive connect, SELECT, transmit, apdu_sw helper

### Secondary (MEDIUM confidence)
- `docs.rs/pcsc` — Context, Card, transmit(), ShareMode::Exclusive, Protocols::T0|T1 usage confirmed
- OpenPGP Card Spec v3.4 (referenced in CONTEXT.md canonical refs) — DO structure for 0xC4, 0x6E, 0x65, algorithm attribute byte encoding

### Tertiary (LOW confidence)
- PIV AID trailing byte (full 9-byte vs 5-byte): CONTEXT.md asserts 9-byte form; unverified against live YubiKey firmware

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — pcsc already in use, pattern established in codebase
- Architecture: HIGH — plan sequencing and module boundaries locked in CONTEXT.md; only module layout is discretion
- APDU byte sequences: HIGH for GET DATA / touch policy DOs (ykman open source verified); MEDIUM for PIV object IDs (standard reference, not hardware-tested)
- Pitfalls: HIGH — factory reset session provided direct evidence for pitfalls 2, 4, 5; others from OpenPGP spec and cross-platform PCSC experience

**Research date:** 2026-03-25
**Valid until:** 2026-06-25 (stable protocol; pcsc crate version is pinned; ykman APDU sequences change only with new firmware)
