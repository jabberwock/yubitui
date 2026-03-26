# Phase 5: Native Card Protocol - Research

**Researched:** 2026-03-25
**Domain:** PC/SC smart card communication, OpenPGP card protocol, PIV protocol, Rust pcsc crate
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** `pcsc` raw only — all card communication via hand-written APDUs. No `openpgp-card`, no `yubikey` crate. We reference ykman's open source Python implementation for the exact APDU byte sequences.
- **D-02:** Remove `openpgp-card`, `card-backend-pcsc`, and `yubikey` crates from Cargo.toml — they were added in anticipation of Phase 5 but won't be used given the pcsc-raw decision.
- **D-03:** `pcsc = "2.8"` stays — already used for factory reset, now extended to all card operations.
- **D-04:** Before every native PC/SC operation: `gpgconf --kill scdaemon` to release the card channel, then connect with `ShareMode::Exclusive`. scdaemon restarts automatically on the next gpg call. This is the pattern already established for factory reset.
- **D-05:** No explicit scdaemon restart after the operation — lazy restart on next gpg call is sufficient.
- **D-06:** Replace all `gpg --card-status` calls used for card state reads with direct PC/SC GET DATA APDUs.
- **D-07:** Key GET DATA DOs to implement: `00 CA 00 C4 00` (PW Status Bytes), `00 CA 00 6E 00` (Application Related Data), `00 CA 00 65 00` (Cardholder Related Data), `00 CA 00 5F 50 00` (URL), `00 CA 00 5E 00` (Login data)
- **D-08:** Serial number extracted from OpenPGP AID bytes 10-13 big-endian.
- **D-09/D-10:** Touch policy read via GET DATA DOs 0xD6-0xD9; set via PUT DATA. Policy byte: 00=off, 01=on, 02=fixed, 03=cached, 04=cached-fixed.
- **D-11:** Device detection via PC/SC reader enumeration + SELECT OpenPGP AID per reader.
- **D-12:** Key attributes from GET DATA 0x6E + touch policy DOs per slot.
- **D-13/D-14:** PIV info via native PC/SC; best-effort, empty list on failure.
- **D-15:** APDU SW errors shown as plain English with action. No raw SW codes in UI.
- **D-16:** SW codes go to `tracing::debug!` only.
- **D-17:** Shared `apdu_error_message(sw: u16, context: &str) -> String` helper.
- **Plan sequencing:** Plan 1 = device detection + card reads; Plan 2 = touch policy + PIV; Plan 3 = cleanup.
- **gpg operations that remain:** `gpg --batch --gen-key`, `gpg --edit-key`, `gpg --export-ssh-key`, `gpgconf --list-dirs`, `gpgconf --kill scdaemon`, `gpg-agent`.

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

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| NATIVE-PCSC-01 | All card I/O goes through the `pcsc` crate via raw APDUs | pcsc 2.9.0 API documented; factory_reset pattern is the template |
| NO-GPG-BIN-01 | No gpg binary calls for card state reads (gpg --card-status eliminated) | GET DATA APDUs for 0xC4, 0x6E, 0x65, 0x5F50, 0x5E documented with exact bytes |
| NO-YKMAN-BIN-01 | No ykman binary calls for any operation | All ykman operations mapped to APDUs; find_ykman() deleted in Plan 3 |

</phase_requirements>

---

## Summary

Phase 5 replaces all ykman CLI calls and all `gpg --card-status` card-read calls with direct PC/SC APDU operations using the `pcsc` crate. The codebase already has a working template in `factory_reset_openpgp()`: kill scdaemon, establish exclusive PC/SC context, SELECT the OpenPGP AID, send APDUs, interpret status words. Every new operation in this phase follows that exact pattern.

The technical foundation is solid. The `pcsc` crate (currently at 2.9.0 on crates.io; Cargo.toml pins "2.8" which resolves to 2.9.0) provides the correct API already in use. All APDU byte sequences are verified against ykman's open-source Python implementation. The serial number extraction, touch policy DOs, PW Status Bytes layout, and PIV GET DATA format are all confirmed from ykman source at developers.yubico.com.

The key implementation constraint is scdaemon coexistence: every native PC/SC operation must kill scdaemon first. This is already the established pattern in the codebase. Per 2024 blog research, the "kill scdaemon" approach remains valid for an end-user tool that cannot require scdaemon reconfiguration by the user.

**Primary recommendation:** Follow the `factory_reset_openpgp()` template exactly for all new PC/SC operations. Introduce `src/yubikey/card.rs` as the single home for all raw PC/SC primitives. Build `apdu_error_message()` and `apdu_sw()` there first; they are called by every operation site. Implement a minimal TLV walker for DO 0x6E nested data.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| pcsc | 2.9.0 (Cargo.toml "2.8" resolves here) | Raw PC/SC bindings for APDU transmit | Already in use; cross-platform (winscard/macOS PCSC.framework/pcsclite) |

### Crates to REMOVE per D-02

| Crate | Current Cargo.toml Version | Reason |
|-------|---------------------------|--------|
| openpgp-card | 0.5 | Raw APDU approach chosen; this crate is unused |
| card-backend-pcsc | 0.5 | Unused; backend for openpgp-card which is also being removed |
| yubikey | 0.8 | Unused; raw APDU approach chosen |

**Removal command:**
```bash
cargo remove openpgp-card card-backend-pcsc yubikey
cargo build  # verify no compile errors from removal
```

**Version verification:** `cargo search pcsc` confirmed `pcsc = "2.9.0"` as current registry version (2026-03-25).

---

## Architecture Patterns

### Recommended Project Structure

Introduce one new file; all existing files are modified in place:

```
src/yubikey/
├── card.rs          # NEW: PC/SC primitives
│                    #   - kill_scdaemon()
│                    #   - connect_to_openpgp_card() -> Result<(Context, Card, Vec<u8>)>
│                    #   - get_data(card, p1, p2) -> Result<Vec<u8>>
│                    #   - put_data(card, p1, p2, data) -> Result<()>
│                    #   - apdu_sw(resp: &[u8]) -> u16   (moved from pin_operations.rs)
│                    #   - apdu_error_message(sw, context) -> String
│                    #   - tlv_find(data, tag) -> Option<&[u8]>
├── detection.rs     # MODIFIED: PC/SC reader enumeration replaces ykman list --serials
├── pin.rs           # MODIFIED: GET DATA 0xC4 replaces gpg --card-status call
├── openpgp.rs       # MODIFIED: GET DATA 0x6E + 0x65 replaces gpg --card-status call
├── touch_policy.rs  # MODIFIED: GET DATA 0xD6-D9 + PUT DATA replaces ykman set-touch
├── piv.rs           # MODIFIED: native PIV PC/SC replaces ykman piv info
├── key_operations.rs # MODIFIED: get_key_attributes() uses GET DATA 0x6E
└── pin_operations.rs # MODIFIED: factory_reset stays; find_ykman() removed in Plan 3
```

### Pattern 1: Standard PC/SC Operation Template

**What:** Established pattern from `factory_reset_openpgp()`; used for ALL Phase 5 PC/SC ops.
**When to use:** Every native card operation.

```rust
// Source: src/yubikey/pin_operations.rs factory_reset_openpgp()
use pcsc::{Context, Protocols, Scope, ShareMode};

// 1. Release scdaemon's hold on the card channel
let _ = std::process::Command::new("gpgconf")
    .args(["--kill", "scdaemon"])
    .output();

// 2. Establish PC/SC context
let ctx = Context::establish(Scope::User)
    .map_err(|e| anyhow::anyhow!("PC/SC error: {e}"))?;

// 3. Enumerate readers (2048-byte buffer is sufficient for all reader names)
let mut readers_buf = [0u8; 2048];
let readers: Vec<_> = ctx
    .list_readers(&mut readers_buf)
    .map_err(|e| anyhow::anyhow!("No smart card readers found: {e}"))?
    .collect();

// 4. Connect exclusive; try each reader until one has the target app
for reader in readers {
    let card = match ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1) {
        Ok(c) => c,
        Err(_) => continue,
    };

    // 5. SELECT OpenPGP AID
    let select_openpgp = [0x00u8, 0xA4, 0x04, 0x00, 0x06,
                           0xD2, 0x76, 0x00, 0x01, 0x24, 0x01];
    let mut buf = [0u8; 256];
    let resp = match card.transmit(&select_openpgp, &mut buf) {
        Ok(r) => r,
        Err(_) => continue,
    };
    if apdu_sw(resp) != 0x9000 { continue; }

    // 6. AID select response data is the echoed AID — use for serial extraction
    let aid_data = &resp[..resp.len()-2];

    // 7. Proceed with GET DATA / PUT DATA operations
}
```

### Pattern 2: GET DATA APDU

**What:** Read a card data object by tag.
**When to use:** PW Status Bytes (0xC4), Application Related Data (0x6E), Cardholder Data (0x65), URL (0x5F50), Login (0x5E), touch policy DOs (0xD6-0xD9).

```rust
// GET DATA for a 1-byte P1:P2 tag (e.g., 0x00C4):
// CLA=00 INS=CA P1=HI P2=LO Le=00
let get_pw_status = [0x00u8, 0xCA, 0x00, 0xC4, 0x00];
let mut buf = [0u8; 256];
let resp = card.transmit(&get_pw_status, &mut buf)
    .map_err(|e| anyhow::anyhow!("GET DATA transmit error: {e}"))?;

if apdu_sw(resp) != 0x9000 {
    tracing::debug!("GET DATA 0xC4 SW {:04X}", apdu_sw(resp));
    anyhow::bail!("{}", apdu_error_message(apdu_sw(resp), "reading PIN status"));
}
let data = &resp[..resp.len()-2];  // strip 2-byte SW

// GET DATA for a 2-byte extended tag (e.g., 0x5F50 for URL):
// CLA=00 INS=CA P1=5F P2=50 Le=00
let get_url = [0x00u8, 0xCA, 0x5F, 0x50, 0x00];

// GET DATA for Application Related Data 0x6E — use larger buffer:
let get_app_data = [0x00u8, 0xCA, 0x00, 0x6E, 0x00];
let mut buf_large = [0u8; 1024];  // 0x6E can be 500+ bytes with fingerprints
```

### Pattern 3: PW Status Bytes (DO 0xC4) Layout

**What:** 7-byte response containing PIN retry counters. Replaces `parse_pin_status()` gpg text parsing.
**Source:** ykman openpgp.py DO.PW_STATUS_BYTES; OpenPGP card spec 3.4 Table 4.

```
Byte 0:   PIN format flags (0x00 = standard UTF-8 encoding)
Byte 1:   Max User PIN length
Byte 2:   Max Reset Code length
Byte 3:   Max Admin PIN length
Byte 4:   PW1 (User PIN) remaining tries
Byte 5:   RC (Reset Code) remaining tries
Byte 6:   PW3 (Admin PIN) remaining tries
```

```rust
if data.len() < 7 {
    anyhow::bail!("Unexpected PW Status Bytes length: {}", data.len());
}
let user_retries  = data[4];
let reset_retries = data[5];
let admin_retries = data[6];
// Same field order as the gpg --card-status "PIN retry counter : U R A" line
// (no more field-swap risk — values are at fixed offsets in the spec)
```

### Pattern 4: Serial Number from OpenPGP AID Select Response

**What:** Extract 4-byte BCD-encoded serial from the AID echoed in SELECT response.
**Source:** yubikit/openpgp.py OpenPgpAid.serial property.

```
AID layout (16 bytes):
  [0..6]   D2 76 00 01 24 01     — RID (6 bytes)
  [6]      version major
  [7]      version minor
  [8..10]  manufacturer ID (2 bytes big-endian; Yubico = 0x0006)
  [10..14] serial number (4 bytes, BCD-encoded on YubiKey)
  [14..16] padding (00 00)
```

```rust
const OPENPGP_AID_PREFIX: &[u8] = &[0xD2, 0x76, 0x00, 0x01, 0x24, 0x01];

fn serial_from_aid(aid: &[u8]) -> Option<u32> {
    if aid.len() < 14 { return None; }
    if &aid[..6] != OPENPGP_AID_PREFIX { return None; }
    // BCD: interpret the 4 hex bytes as decimal digits
    // e.g., bytes [0x09, 0x07, 0x45, 0x82] -> hex string "09074582" -> 9074582u32
    let hex_str = format!("{:02X}{:02X}{:02X}{:02X}",
                          aid[10], aid[11], aid[12], aid[13]);
    hex_str.parse::<u32>().ok().or_else(|| {
        // Invalid BCD (nibble A-F) — fall back to big-endian u32
        Some(u32::from_be_bytes([aid[10], aid[11], aid[12], aid[13]]))
    })
}
```

### Pattern 5: Touch Policy (UIF) GET DATA and PUT DATA

**What:** Read and write the User Interaction Flag per OpenPGP key slot.
**Source:** yubikit/openpgp.py DO tags UIF_SIG/DEC/AUT/ATT and UIF struct.

```
Touch policy DOs (YubiKey proprietary extension):
  0xD6 = UIF_SIG   (Signature key)
  0xD7 = UIF_DEC   (Decryption key)
  0xD8 = UIF_AUT   (Authentication key)
  0xD9 = UIF_ATT   (Attestation key)

Policy byte values (source: yubikit/openpgp.py UIF enum):
  0x00 = Off
  0x01 = On
  0x02 = Fixed  (IRREVERSIBLE without factory reset)
  0x03 = Cached
  0x04 = Cached-Fixed (IRREVERSIBLE)

GET DATA response: [policy_byte, feature_mgmt_byte]
  Byte 0: policy value (0x00-0x04)
  Byte 1: 0x20 (GENERAL_FEATURE_MANAGEMENT.BUTTON — always 0x20 on YubiKey)

PUT DATA: requires Admin PIN verified in current session
  CLA=00 INS=DA P1=00 P2=D6 Lc=02 [policy_byte] [0x20]
```

```rust
// Read touch policy for signature slot:
let get_uif_sig = [0x00u8, 0xCA, 0x00, 0xD6, 0x00];

// Set touch policy for signature slot to "On" (requires Admin PIN verified):
let put_uif_sig_on = [0x00u8, 0xDA, 0x00, 0xD6, 0x02, 0x01, 0x20];

// Admin PIN verification before PUT DATA:
// CLA=00 INS=20 P1=00 P2=83 Lc=[len] [admin_pin_bytes]
let mut verify_cmd = vec![0x00u8, 0x20, 0x00, 0x83, admin_pin.len() as u8];
verify_cmd.extend_from_slice(admin_pin.as_bytes());
```

### Pattern 6: PIV AID SELECT and GET DATA

**What:** Select the PIV application and read certificate slot presence.
**Source:** yubikit/core/smartcard/__init__.py AID.PIV; yubikit/piv.py OBJECT_ID.

```
PIV AID (9 bytes): A0 00 00 03 08 00 00 10 00
SELECT command:   00 A4 04 00 09 A0 00 00 03 08 00 00 10 00 01
  Note: trailing 0x01 is Le (expected response length)

PIV certificate slot object IDs (source: yubikit/piv.py):
  9A (Authentication): 5F C1 05
  9C (Signature):      5F C1 0A
  9D (Key Management): 5F C1 0B
  9E (Card Auth):      5F C1 01

PIV GET DATA for certificate slot:
  CLA=00 INS=CB P1=3F P2=FF Lc=05 5C 03 [obj_id_b0] [obj_id_b1] [obj_id_b2]

Response: SW 9000 = data present; SW 6A82 = object not found (slot empty)
```

```rust
// SELECT PIV AID
let select_piv: &[u8] = &[
    0x00, 0xA4, 0x04, 0x00, 0x09,
    0xA0, 0x00, 0x00, 0x03, 0x08, 0x00, 0x00, 0x10, 0x00, 0x01,
];

// GET DATA for PIV slot 9A certificate:
let get_9a: &[u8] = &[0x00, 0xCB, 0x3F, 0xFF, 0x05, 0x5C, 0x03, 0x5F, 0xC1, 0x05];
let get_9c: &[u8] = &[0x00, 0xCB, 0x3F, 0xFF, 0x05, 0x5C, 0x03, 0x5F, 0xC1, 0x0A];
let get_9d: &[u8] = &[0x00, 0xCB, 0x3F, 0xFF, 0x05, 0x5C, 0x03, 0x5F, 0xC1, 0x0B];
let get_9e: &[u8] = &[0x00, 0xCB, 0x3F, 0xFF, 0x05, 0x5C, 0x03, 0x5F, 0xC1, 0x01];
// Best-effort: SW 6A82 = slot empty, not an error
```

### Pattern 7: apdu_error_message() Helper (D-17)

**What:** Shared SW-to-English mapper. Every APDU error site calls this; SW goes only to debug log.

```rust
pub fn apdu_error_message(sw: u16, context: &str) -> String {
    let msg = match sw {
        0x6300 => "Wrong PIN".to_string(),
        sw if sw & 0xFFF0 == 0x63C0 => {
            let retries = sw & 0x000F;
            format!("Wrong PIN — {} {} remaining", retries,
                    if retries == 1 { "try" } else { "tries" })
        }
        0x6982 => "Security condition not met — Admin PIN required".to_string(),
        0x6983 => "Authentication method blocked".to_string(),
        0x6A82 => "Data object not found".to_string(),
        0x6A80 => "Incorrect data in command".to_string(),
        0x6700 => "Wrong length".to_string(),
        _ => "Card operation failed — try removing and reinserting your YubiKey".to_string(),
    };
    format!("{} ({})", msg, context)
}

// Usage at every error site:
if apdu_sw(resp) != 0x9000 {
    tracing::debug!("APDU SW {:04X} during {}", apdu_sw(resp), context_str);
    anyhow::bail!("{}", apdu_error_message(apdu_sw(resp), context_str));
}
```

### Pattern 8: Minimal TLV Walker for DO 0x6E

**What:** DO 0x6E (Application Related Data) is TLV-constructed and contains nested sub-DOs. A flat byte read will not work.
**When to use:** Parsing GET DATA 0x6E response to extract fingerprints and algorithm attributes.

```rust
// Source: OpenPGP card spec 3.4 DO structure; ykman openpgp.py DO tag constants
/// Walk BER-TLV encoded data and return the value bytes for the first matching tag.
/// Handles 1-byte and 2-byte tags; handles 1-byte and 2-byte BER-TLV lengths.
fn tlv_find<'a>(data: &'a [u8], target_tag: u16) -> Option<&'a [u8]> {
    let mut i = 0;
    while i < data.len() {
        // Parse tag: if low 5 bits are all 1 it is a 2-byte tag
        let (tag, tag_len) = if data[i] & 0x1F == 0x1F {
            if i + 1 >= data.len() { break; }
            let t = ((data[i] as u16) << 8) | data[i + 1] as u16;
            (t, 2usize)
        } else {
            (data[i] as u16, 1usize)
        };
        i += tag_len;
        if i >= data.len() { break; }

        // Parse length (BER-TLV: 0x81 = 1-byte follows; 0x82 = 2-bytes follow)
        let (len, len_sz) = if data[i] == 0x82 {
            if i + 2 >= data.len() { break; }
            let l = ((data[i + 1] as usize) << 8) | data[i + 2] as usize;
            (l, 3usize)
        } else if data[i] == 0x81 {
            if i + 1 >= data.len() { break; }
            (data[i + 1] as usize, 2usize)
        } else {
            (data[i] as usize, 1usize)
        };
        i += len_sz;
        if i + len > data.len() { break; }

        if tag == target_tag {
            return Some(&data[i..i + len]);
        }
        i += len;
    }
    None
}

// Usage for DO 0x6E fingerprint extraction:
// let disc_data = tlv_find(&app_data, 0x73)?;  // 0x73 = Discretionary Data Objects
// let sig_fp    = tlv_find(disc_data, 0xC7);   // 0xC7 = fingerprint SIG (20 bytes, or None if empty)
// let enc_fp    = tlv_find(disc_data, 0xC8);   // 0xC8 = fingerprint DEC
// let aut_fp    = tlv_find(disc_data, 0xC9);   // 0xC9 = fingerprint AUT
// let sig_algo  = tlv_find(disc_data, 0xC1);   // 0xC1 = algorithm attributes SIG
// let aid_do    = tlv_find(&app_data, 0x4F);   // 0x4F = AID (for serial extraction)
```

### Pattern 9: PC/SC Transaction for Multi-APDU Sequences

**What:** Wrap multi-step APDU sequences in a transaction for atomicity.
**When to use:** Any operation that issues more than one APDU (SELECT + GET DATA, or VERIFY + PUT DATA).
**Source:** pcsc docs.rs Transaction documentation.

```rust
// card.transaction() requires &mut card
let tx = card.transaction()
    .map_err(|e| anyhow::anyhow!("Failed to begin card transaction: {e}"))?;
let resp1 = tx.transmit(&select_aid, &mut buf1)?;
let resp2 = tx.transmit(&get_data,   &mut buf2)?;
// Transaction releases when tx is dropped (end of scope or explicit drop)
```

### Anti-Patterns to Avoid

- **Skipping scdaemon kill:** `connect(Exclusive)` will return `Error::SharingViolation` intermittently if scdaemon is running. The factory_reset pattern is: kill first, then connect.
- **Using `Protocols::ANY` for operational APDUs:** Factory reset uses `T0 | T1`. Use the same for consistency.
- **Flat byte reads on DO 0x6E:** The Application Related Data DO is a nested TLV. Use `tlv_find()` to navigate to sub-DOs. Attempting to read fingerprints at a fixed offset will produce garbage.
- **Showing raw SW codes in the UI:** All SW codes go to `tracing::debug!`. User-visible messages go through `apdu_error_message()`.
- **Sending plaintext PIN when KDF is active:** Check DO 0xF9 before VERIFY. If KDF DO is non-empty, return a clear error rather than a confusing "wrong PIN."
- **Buffer too small for DO 0x6E:** Use at least 1024 bytes for the 0x6E response buffer. The response can be 500+ bytes when multiple fingerprints and algorithm attributes are present. `[0u8; 256]` is sufficient for all other DOs.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cross-platform PC/SC access | Custom FFI or platform detection | `pcsc` crate (already in Cargo.toml) | Handles winscard/macOS PCSC.framework/pcsclite transparently |
| Full TLV parser | Generic TLV library | Inline `tlv_find()` function (~30 lines) | Only 5-6 specific tags needed; adding a dep for this is overkill |
| OpenPGP card abstraction | openpgp-card crate | Hand-written APDUs (D-01 locked) | User decision; ykman source provides all needed byte sequences |
| PIV abstraction | yubikey crate PIV | Hand-written APDUs (D-01 locked) | Same rationale; slot tags verified from ykman source |

**Key insight:** The "don't hand-roll" items are all crates that were already in Cargo.toml but are now being removed per D-02. The approach is purposely lower-level: write the APDUs, verify against ykman source, parse the bytes. This gives full control and eliminates abstraction layer bugs.

---

## Common Pitfalls

### Pitfall 1: scdaemon Reclaims the Card After gpg Calls

**What goes wrong:** `connect(Exclusive)` returns `Error::SharingViolation` intermittently.
**Why it happens:** scdaemon monitors card events and reclaims exclusive access within seconds of any gpg operation completing.
**How to avoid:** Every PC/SC function begins with `gpgconf --kill scdaemon`. This is the established project pattern from factory_reset.
**Warning signs:** Failure only happens when a gpg call preceded the PC/SC call in the same session.

### Pitfall 2: DO 0x6E Is Nested TLV — Flat Reads Silently Corrupt Data

**What goes wrong:** Fingerprint or algorithm values are garbage, or the code panics on slice out of bounds.
**Why it happens:** Developers familiar with the flat gpg --card-status text output assume card binary data has similar structure.
**How to avoid:** Use `tlv_find()`. The outer response for GET DATA 0x6E strips the SW, leaving a TLV stream. Navigate: outer 0x6E tag, then find 0x73 (discretionary data), then find 0xC7/C8/C9 for fingerprints and 0xC1/C2/C3 for algorithm attributes.
**Warning signs:** Fingerprints contain non-hex characters; algorithm "ed25519" shows as something else.

### Pitfall 3: Serial Number Is BCD-Encoded, Not Binary

**What goes wrong:** Detected serial does not match what `gpg --card-status` or the YubiKey Manager GUI shows.
**Why it happens:** The 4-byte serial field in the AID is BCD (binary-coded decimal): each byte represents two decimal digits in hex. Treating it as a binary integer gives the wrong number.
**How to avoid:** Format the 4 bytes as a hex string, then parse as decimal: `format!("{:02X}{:02X}{:02X}{:02X}", b0, b1, b2, b3).parse::<u32>()`. Fall back to big-endian u32 if any nibble is A-F (non-standard device).
**Warning signs:** Serial number reported by yubitui does not match ykman output.

### Pitfall 4: Touch Policy PUT DATA Requires Admin PIN in Current Session

**What goes wrong:** PUT DATA for UIF DOs returns SW 0x6982 (Security condition not met) even with a correct Admin PIN.
**Why it happens:** The Admin PIN must be VERIFIED in the current card session via a VERIFY APDU (INS=0x20, P2=0x83) before PUT DATA will be accepted. The verification is session-scoped — it is cleared when the exclusive connection is closed.
**How to avoid:** The set_touch_policy() function must accept the Admin PIN as a parameter, send VERIFY before PUT DATA, all within the same `Card` object and ideally within a transaction.
**Warning signs:** PUT DATA consistently returns 0x6982 regardless of Admin PIN correctness.

### Pitfall 5: KDF Invalidates Plaintext PIN VERIFY

**What goes wrong:** VERIFY APDU returns SW 0x6300 (wrong PIN) when the PIN is correct.
**Why it happens:** YubiKey firmware 5.2.3+ optionally uses a Key Derivation Function for PIN storage (DO 0xF9). When KDF is enabled, the PIN must be salted+hashed before transmission. Sending plaintext to a KDF-enabled card fails.
**How to avoid:** After SELECT AID and before any VERIFY, send GET DATA 0xF9. If the response data is non-empty and the KDF algorithm field is non-zero, return a user-readable error: "This YubiKey has KDF PIN hashing enabled. Use ykman to manage touch policy on this key." KDF implementation is out of scope for Phase 5.
**Warning signs:** VERIFY consistently returns 0x6300 despite the user entering the correct PIN.

### Pitfall 6: PIV AID Select Fails on Non-PIV or Older Devices

**What goes wrong:** The piv.rs code path errors out on older YubiKeys or YubiKey NEO.
**Why it happens:** Not all YubiKey models have a PIV application. SELECT PIV AID returns SW 0x6A82.
**How to avoid:** Per D-14: PIV is best-effort. Wrap the PIV detection in a function that returns `PivState { slots: vec![] }` on any non-9000 SELECT response. Log at debug. Never propagate a PIV detection failure to the user.
**Warning signs:** App crashes or shows an error on YubiKey NEO or YubiKey 4.

### Pitfall 7: Card Removal Between APDUs in a Sequence

**What goes wrong:** Second or third APDU returns a transport error after the card is physically removed.
**Why it happens:** Multi-step sequences (SELECT + GET DATA, VERIFY + PUT DATA) are not atomic without a transaction.
**How to avoid:** Use `card.transaction()` to wrap multi-APDU sequences. The Transaction type exposes the same `transmit()` method and holds a platform-level exclusive lock for the duration.
**Warning signs:** Intermittent partial read failures that correlate with card insertion/removal.

---

## Code Examples

### Full GET DATA 0xC4 Flow (Replacement for get_pin_status())

```rust
// Source: factory_reset_openpgp() pattern + DO 0xC4 spec from ykman openpgp.py
pub fn get_pin_status_native() -> Result<PinStatus> {
    use pcsc::{Context, Protocols, Scope, ShareMode};
    let _ = std::process::Command::new("gpgconf").args(["--kill","scdaemon"]).output();
    let ctx = Context::establish(Scope::User)?;
    let mut rbuf = [0u8; 2048];
    let readers: Vec<_> = ctx.list_readers(&mut rbuf)?.collect();
    for reader in readers {
        let card = match ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1) {
            Ok(c) => c, Err(_) => continue,
        };
        let select = [0x00,0xA4,0x04,0x00,0x06,0xD2,0x76,0x00,0x01,0x24,0x01];
        let mut buf = [0u8; 256];
        let r = card.transmit(&select, &mut buf).ok()?;
        if apdu_sw(r) != 0x9000 { continue; }

        let get_c4 = [0x00u8, 0xCA, 0x00, 0xC4, 0x00];
        let r2 = card.transmit(&get_c4, &mut buf)?;
        if apdu_sw(r2) != 0x9000 {
            anyhow::bail!("{}", apdu_error_message(apdu_sw(r2), "reading PIN status"));
        }
        let data = &r2[..r2.len()-2];
        if data.len() < 7 { anyhow::bail!("Unexpected PW Status length"); }
        return Ok(PinStatus {
            user_pin_retries:   data[4],
            reset_code_retries: data[5],
            admin_pin_retries:  data[6],
            user_pin_blocked:   data[4] == 0,
            admin_pin_blocked:  data[6] == 0,
        });
    }
    anyhow::bail!("No YubiKey with OpenPGP application found")
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `ykman list --serials` for device detection | PC/SC reader enumeration + SELECT AID | Phase 5 Plan 1 | No ykman binary required |
| `gpg --card-status` text parse for PIN counters | GET DATA 0xC4 binary parse | Phase 5 Plan 1 | Eliminates field-swap class of bugs |
| `gpg --card-status` text parse for key info | GET DATA 0x6E TLV parse | Phase 5 Plan 1 | Direct from card; no gpg call needed |
| `ykman openpgp info` for touch policy read | GET DATA 0xD6-0xD9 per slot | Phase 5 Plan 2 | Faster; no subprocess |
| `ykman openpgp keys set-touch --force` | VERIFY + PUT DATA APDUs | Phase 5 Plan 2 | No ykman binary required |
| `ykman piv info` for PIV slot detection | SELECT PIV + GET DATA per slot | Phase 5 Plan 2 | No ykman binary required |

**Deprecated/outdated in this codebase after Phase 5:**
- `find_ykman()` in pin_operations.rs — deleted in Plan 3
- `list_connected_serials()` in detection.rs — replaced by PC/SC enumeration (function deleted)
- `parse_touch_policies(output: &str)` — ykman text output parser; logic superseded but parser kept for existing unit tests (file kept, caller changed)
- `parse_card_status(output: &str)` in openpgp.rs — gpg text parser; superseded by 0x6E TLV parsing (function kept for existing unit tests)
- `parse_pin_status(output: &str)` in pin.rs — gpg text parser; superseded by 0xC4 parsing (function kept for existing unit tests)

---

## Open Questions

1. **DO 0x6E fingerprint sub-DOs when no key is loaded**
   - What we know: DO 0x6E exists even on a fresh card with no keys. The spec says fingerprints are zero-filled when absent.
   - What's unclear: Are 0xC7/C8/C9 sub-DOs present in the TLV with zero-filled 20-byte values, or are they absent entirely?
   - Recommendation: Treat `tlv_find()` returning `None` as "no key in slot." If present but all-zero bytes, also treat as no key.

2. **KDF prevalence**
   - What we know: KDF was introduced in YubiKey firmware 5.2.3 (2019). It is opt-in.
   - What's unclear: What fraction of users have KDF enabled?
   - Recommendation: Phase 5 detects KDF and shows a graceful error for the touch-policy set path. Implement the check; it is 1-2 lines.

3. **scdaemon kill timing on Windows**
   - What we know: `gpgconf --kill scdaemon` works on Linux and macOS. On Windows the scdaemon process may need a moment to release WinSCard handles.
   - What's unclear: Is a sleep ever needed on Windows between kill and connect?
   - Recommendation: Do not add a sleep initially. If Windows CI shows intermittent failures on the card connect step, add a 200ms sleep as a follow-up.

---

## Environment Availability

Step 2.6: Environment availability audit for Phase 5 external dependencies.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| pcsc crate | All PC/SC ops | Build-time dep (not a runtime binary) | 2.9.0 | No fallback needed |
| PC/SC.framework | macOS card access | macOS native | macOS built-in | No fallback |
| pcscd | Linux card access | Not applicable on macOS dev machine | — | Linux CI installs libpcsclite-dev (already in CI config) |
| winscard.dll | Windows card access | Windows native | Windows built-in | No fallback |
| gpgconf | scdaemon kill step | Present as part of GPG suite (remains in scope) | From GPG install | If gpgconf absent, log warning and proceed; scdaemon may not be running |
| cargo clippy | CI gate | Available | rustup managed | — |

**Missing dependencies with no fallback:** None that block Phase 5.

**Missing dependencies with fallback:** None.

---

## Sources

### Primary (HIGH confidence)
- https://developers.yubico.com/yubikey-manager/API_Documentation/_modules/yubikit/openpgp.html — OpenPgpAid serial extraction; INS constants; UIF/DO tags; PW Status Bytes layout; VERIFY/GET DATA/PUT DATA APDU structure
- https://github.com/Yubico/yubikey-manager/blob/main/yubikit/piv.py — PIV slot tags (9A/9C/9D/9E), GET DATA format (INS=0xCB), slot object IDs
- https://github.com/Yubico/yubikey-manager/blob/main/yubikit/core/smartcard/__init__.py — PIV AID bytes; OpenPGP AID bytes; SELECT APDU pattern
- https://docs.rs/pcsc/latest/pcsc/struct.Card.html — Card::transmit signature; Card::transaction; buffer sizing notes
- src/yubikey/pin_operations.rs — factory_reset_openpgp() as the established project template

### Secondary (MEDIUM confidence)
- https://blog.apdu.fr/posts/2024/12/gnupg-and-pcsc-conflicts-episode-3/ — scdaemon/PC/SC conflict current state (December 2024); kill approach confirmed valid
- https://crates.io/crates/pcsc/2.9.0 — confirmed pcsc 2.9.0 as current registry version

### Tertiary (LOW confidence — flagged)
- OpenPGP card spec 3.4 (gnupg.org/ftp/specs/) — DO structure inferred from ykman source; direct spec PDF not fetched. TLV encoding details should be verified against the PDF if edge-case parsing issues arise.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — pcsc crate in Cargo.toml; version confirmed from crates.io
- APDU byte sequences: HIGH — verified from ykman open-source Python source
- PW Status Bytes layout: HIGH — field offsets confirmed from ykman and project's existing pin.rs comments
- TLV structure for DO 0x6E: MEDIUM — tag constants confirmed from ykman; exact encoding of empty/absent sub-DOs not tested on hardware in this session
- scdaemon kill approach: HIGH — established in project (factory_reset works); confirmed valid in 2024 blog
- PIV slot object IDs: HIGH — from ykman piv.py directly

**Research date:** 2026-03-25
**Valid until:** 2026-09-25 (stable protocol specs; pcsc crate 2.x API is stable)
