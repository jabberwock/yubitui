# Phase 14: OATH Import & Password Management - Research

**Researched:** 2026-03-29
**Domain:** YKOATH APDU protocol — URI import pre-fill + SET CODE / VALIDATE password lifecycle
**Confidence:** HIGH

## Summary

Phase 14 extends the existing OATH screen (Phase 13 shipped) with two feature clusters: (1) URI import with a preview step before commit, and (2) full OATH application password lifecycle (set / change / remove). Both clusters build on scaffolding that is already partially present in the codebase.

For URI import: `ImportUriScreen` exists in `src/tui/oath.rs` and already parses the URI and calls `put_credential` directly. The gap is that OATH-07 requires a **preview step** — showing issuer, account, secret, and algorithm to the user before adding. The existing screen skips straight to PUT on Enter. The fix is small: parse the URI into a struct, render a confirmation view, then PUT on second Enter.

For password management: `OathState.password_required` and the SW 0x6982 detection already exist. No password APDUs are implemented yet. The YKOATH protocol defines three APDUs: `SELECT` (returns challenge when auth is set), `VALIDATE` (proves knowledge of password, must be called each session after SELECT), and `SET CODE` (sets, changes, or removes the password). PBKDF2-HMAC-SHA1 with 1000 iterations and the device's `name` bytes as salt derives the actual key from the user's UTF-8 password. The `hmac` + `pbkdf2` + `sha1` crates are already in Rust's ecosystem and do not require new heavy dependencies.

The TUI pattern to follow is `PinAuthScreen` from `src/tui/fido2.rs` — a password-entry screen that, on success, pops itself and pushes a new instance of the parent screen with unlocked state. All four password operations (set when none exists, change with auth, remove with auth, and session authenticate when required) fit this modal screen pattern.

**Primary recommendation:** Split into three plans — (1) URI import preview step, (2) OATH password model layer (APDUs + PBKDF2 key derivation), (3) TUI screens for set/change/remove password.

## Project Constraints (from CLAUDE.md)

- Use Pilot and/or tmux for UI testing.
- Use `collab list` at session start; treat pending messages as blocking.
- Run `collab watch --role` with real context when focus changes.
- Message other workers only for: public API changes, new widgets they might use, behavioral regressions.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `hmac` | 0.12 | HMAC-SHA1 computation for VALIDATE and SET CODE | Part of RustCrypto suite already present |
| `sha1` | 0.10 | SHA-1 digest for HMAC | RustCrypto, pairs with `hmac` |
| `pbkdf2` | 0.12 | Key derivation from password (1000 rounds, HMAC-SHA1) | YKOATH spec requirement |

These crates are RustCrypto crates and are near-certainly already transitively present. Verify with `cargo tree | grep -E "hmac|sha1|pbkdf2"` before adding new deps.

### Already Present (no new deps)
| Library | Purpose |
|---------|---------|
| `pcsc` | Card APDU transport — already used throughout `model/oath.rs` |
| `anyhow` | Error propagation — used everywhere |
| `textual_rs` | Widget model — existing pattern |

**Installation (only if not transitively present):**
```bash
cargo add hmac sha1 pbkdf2
```

## Architecture Patterns

### Recommended Project Structure
No new files required. Changes go into:
```
src/
├── model/oath.rs        # New: derive_oath_key(), set_oath_password(), validate_oath_session(), remove_oath_password()
└── tui/oath.rs          # New: OathPasswordScreen, ImportUriPreviewScreen (or extend ImportUriScreen)
```

### Pattern 1: URI Import with Preview (OATH-07)

**What:** Parse the URI immediately on paste (or on Enter), then render a read-only confirmation view. A second Enter commits the PUT. This is a two-stage screen, not two separate push_screen calls — the existing `ImportUriScreen` just gains a `parsed: Option<ParsedUri>` field.

**When to use:** Any time the user pastes a URI.

```rust
// Extend existing ImportUriScreen state:
enum ImportUriStep {
    Paste,   // waiting for URI input
    Confirm, // URI parsed OK, show preview, wait for Enter
}

struct ImportUriScreen {
    input: RefCell<String>,
    step: Cell<ImportUriStep>,
    parsed: RefCell<Option<ParsedOtpAuth>>,  // issuer, account, secret, algorithm, type
    error: RefCell<Option<String>>,
    own_id: Cell<Option<WidgetId>>,
}

// ParsedOtpAuth carries all fields extracted from URI
struct ParsedOtpAuth {
    issuer: Option<String>,
    account: String,
    secret_b32: String,
    algorithm: OathAlgorithm,
    oath_type: OathType,
    digits: u8,
    period: u32,
}
```

**Compose for Confirm step:**
```
Header("Import OATH URI")
Label("Review before adding:")
Label("")
Label(format!("  Issuer:    {}", issuer.as_deref().unwrap_or("(none)")))
Label(format!("  Account:   {}", account))
Label(format!("  Secret:    {}", masked))
Label(format!("  Algorithm: {}", algorithm))
Label(format!("  Type:      {}", oath_type))
Label("")
Label("Press Enter to add, Esc to go back and edit.")
Footer
```

The `parse_otpauth_uri()` function already extracts `name` and `secret`; it must be extended to also return `issuer`, `algorithm`, `digits`, and `period`. The algorithm is in the `algorithm=` query parameter (`SHA1`, `SHA256`, `SHA512`) or defaults to SHA1.

### Pattern 2: OATH Password Model Layer (OATH-08/09/10)

**YKOATH password flow:**

```
SELECT → response contains TAG_CHALLENGE (0x74) and TAG_ALGORITHM (0x7b) when password is set
VALIDATE (INS=0xA3): TAG_RESPONSE (0x75) = HMAC(key, device_challenge)
                     TAG_CHALLENGE (0x74) = 8 random bytes (host challenge)
              ← response contains TAG_RESPONSE = HMAC(key, host_challenge) for mutual auth
SET CODE (INS=0x03): TAG_KEY (0x73) = [algorithm_byte | derived_key]
                     TAG_CHALLENGE (0x74) = 8 random bytes
                     TAG_RESPONSE (0x75) = HMAC(new_key, challenge)
              To remove: send TAG_KEY with empty value (Lc=0 or key tag with len=0)
```

**Key derivation (PBKDF2):**
```rust
// salt = device name bytes from SELECT response TAG_NAME (0x71)
// password = user UTF-8 string
// iterations = 1000
// output length = 16 bytes for HMAC-SHA1
fn derive_oath_key(password: &str, salt: &[u8]) -> [u8; 16] {
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(password.as_bytes(), salt, 1000, &mut key);
    key
}
```

**VALIDATE APDU:**
```rust
// INS = 0xA3
const VALIDATE_PREFIX: &[u8] = &[0x00, 0xA3, 0x00, 0x00];
// host_challenge = 8 random bytes (use rand::random or getrandom)
// response = HMAC-SHA1(key, device_challenge)[0..16]
```

**SET CODE APDU (set or change):**
```rust
// INS = 0x03
// algorithm byte for HMAC-SHA1 = 0x01 (matches existing OathAlgorithm encoding)
// key TLV = TAG_KEY (0x73) | len | 0x01 | derived_key[16 bytes]
// challenge TLV = TAG_CHALLENGE (0x74) | 0x08 | random[8]
// response TLV = TAG_RESPONSE (0x75) | 0x14 | HMAC-SHA1(new_key, challenge)[20 bytes]
```

**SET CODE APDU (remove/clear):**
```rust
// Send TAG_KEY (0x73) with length 0 — password removed
// No challenge/response TLVs needed when removing
```

**Status words:**
- `0x9000` — success
- `0x6982` — auth required (VALIDATE not called this session)
- `0x6984` — response mismatch (wrong password)
- `0x6a80` — wrong syntax

**Model functions to add to `src/model/oath.rs`:**
```rust
pub fn validate_oath_session(card: &Card, key: &[u8; 16], device_challenge: &[u8]) -> Result<()>
pub fn set_oath_password(password: &str, current_password: Option<&str>) -> Result<()>
pub fn remove_oath_password(current_password: &str) -> Result<()>
```

Note: `get_oath_state()` must be extended to return the `device_name` bytes and `device_challenge` from the SELECT response, so the caller can derive the key and call VALIDATE.

### Pattern 3: TUI Screens for Password Management (OATH-08/09/10)

Follow `PinManagementScreen` in `src/tui/pin.rs` — push modal screens for each operation. The OATH screen gets a new key binding `P` for "Password" which pushes `OathPasswordMenuScreen`.

```
OathPasswordMenuScreen
├── [S] Set password (when password_required == false)
├── [C] Change password (when password_required == true)
└── [R] Remove password (when password_required == true)
```

Each option pushes a dedicated screen:
- `SetOathPasswordScreen` — two inputs: new password + confirm. No auth needed (applet unprotected).
- `ChangeOathPasswordScreen` — three inputs: current password, new password, confirm.
- `RemoveOathPasswordScreen` — one input: current password, then confirm action.

**Session unlock (OATH-08 ongoing):** When any OATH operation returns SW 0x6982, push `OathUnlockScreen` (password prompt) which calls `validate_oath_session()` then retries the original operation. This mirrors `PinAuthScreen` in fido2.rs.

### Anti-Patterns to Avoid
- **Storing the derived key in OathState:** The derived key must not persist in application state between sessions — derive it fresh per session from the user's password entry.
- **Calling SET CODE without VALIDATE first when changing:** When a password already exists, VALIDATE must be called in the same card session before SET CODE or the card returns 0x6982.
- **Using a new PC/SC connection for VALIDATE + SET CODE:** Both APDUs must run within the same `Card` object/session. The current pattern of connect-per-function will need refactoring for the multi-APDU password change flow: open one connection, SELECT, VALIDATE, SET CODE, disconnect.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| PBKDF2 key derivation | Custom PBKDF2 loop | `pbkdf2::pbkdf2_hmac::<sha1::Sha1>` | RFC 2898 edge cases, iteration count correctness |
| HMAC-SHA1 | Manual HMAC | `hmac::Hmac::<sha1::Sha1>` | Length-extension attacks, padding |
| Base32 decode | Custom decoder | `base32_decode()` already in `model/oath.rs` | Already correct; no new dep needed |
| Percent-decode | Custom URL parser | `percent_decode()` already in `tui/oath.rs` | Already present |

## Common Pitfalls

### Pitfall 1: Same Card Session Required for VALIDATE + SET CODE
**What goes wrong:** Opening a new PC/SC connection for each APDU, then calling VALIDATE in one connection and SET CODE in another — the card treats them as separate sessions, SET CODE returns 0x6982.
**Why it happens:** Existing `model/oath.rs` functions each open their own connection (connect-per-function pattern). This is fine for single-APDU operations but breaks multi-step auth flows.
**How to avoid:** Refactor password-related functions to accept a `&Card` parameter, or extract a `with_oath_card(|card| { ... })` helper that opens one connection and runs a closure.
**Warning signs:** Consistent 0x6982 on SET CODE despite successful VALIDATE.

### Pitfall 2: Device Name as Salt — Must Be Read from SELECT Response
**What goes wrong:** Using a hardcoded salt or an empty salt for PBKDF2.
**Why it happens:** The YKOATH spec requires the `name` field (TAG_NAME 0x71) from the SELECT response as the PBKDF2 salt. The current `get_oath_state()` ignores the SELECT response body after checking SW.
**How to avoid:** Parse the SELECT response TLV to extract TAG_NAME; store it in `OathState` or pass it through to the key derivation call.
**Warning signs:** HMAC response mismatch (SW 0x6984) on VALIDATE even with correct password.

### Pitfall 3: ImportUriScreen Commits Immediately Without Preview
**What goes wrong:** Current `ImportUriScreen.import()` calls `put_credential` directly after parse — there is no preview step. OATH-07 requires showing fields before adding.
**Why it happens:** The screen was built before the preview requirement was specified; the comment in the source says "on success the credential is added immediately."
**How to avoid:** Add a `ParsedOtpAuth` intermediate state and a `Confirm` step (two-state machine inside `ImportUriScreen`). Parse on first Enter, show preview, PUT on second Enter.
**Warning signs:** OATH-07 acceptance test fails — user cannot see issuer/account/algorithm before confirming.

### Pitfall 4: Algorithm Field Ignored in URI Parser
**What goes wrong:** `parse_otpauth_uri()` currently extracts only `name`, `secret`, and `oath_type`. The `algorithm=` query parameter is documented as `// accepted and ignored`. PUT always uses `OathAlgorithm::Sha1`.
**Why it happens:** Algorithm was deferred when the parser was first written.
**How to avoid:** Extend `parse_otpauth_uri()` to return `ParsedOtpAuth` with all fields; map `SHA1`/`SHA256`/`SHA512` to `OathAlgorithm` variants.
**Warning signs:** SHA-256 TOTP credentials imported from URIs produce wrong codes.

### Pitfall 5: Mutual Auth Verification Skipped
**What goes wrong:** After VALIDATE, the card returns a TAG_RESPONSE that should be verified against `HMAC(key, host_challenge)` — skipping this means a rogue card could not be detected.
**Why it happens:** It is optional to verify in practice (most tools skip it for simplicity).
**How to avoid:** For correctness, verify the card's response; log a warning if it fails but do not block the user. This is a security nicety, not a hard requirement for v1.2.

## Code Examples

### VALIDATE APDU Construction
```rust
// Source: https://developers.yubico.com/OATH/YKOATH_Protocol.html
// INS=0xA3, P1=0x00, P2=0x00
// Data: TAG_RESPONSE (0x75) | 14 | hmac_bytes[0..14]
//       TAG_CHALLENGE (0x74) | 8  | host_challenge[8]
fn build_validate_apdu(key: &[u8; 16], device_challenge: &[u8], host_challenge: &[u8; 8]) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    type HmacSha1 = Hmac<sha1::Sha1>;
    let mut mac = HmacSha1::new_from_slice(key).unwrap();
    mac.update(device_challenge);
    let response = mac.finalize().into_bytes();
    // YKOATH truncates to 14 bytes for VALIDATE response
    let response14 = &response[..14];

    let mut apdu = vec![0x00, 0xA3, 0x00, 0x00];
    let data_len = 2 + 14 + 2 + 8; // TAG+LEN+response + TAG+LEN+challenge
    apdu.push(data_len as u8);
    apdu.push(0x75); apdu.push(14); apdu.extend_from_slice(response14);
    apdu.push(0x74); apdu.push(8);  apdu.extend_from_slice(host_challenge);
    apdu
}
```

### SET CODE APDU Construction (set new password)
```rust
// Source: https://developers.yubico.com/OATH/YKOATH_Protocol.html
// INS=0x03, P1=0x00, P2=0x00
// algorithm_byte = 0x21 (HMAC-SHA1)
// key TLV = 0x73 | (1+16) | 0x21 | derived_key[16]
// challenge TLV = 0x74 | 8 | random[8]
// response TLV = 0x75 | 20 | HMAC-SHA1(new_key, challenge)[20]  ← full 20 bytes here
fn build_set_code_apdu(new_key: &[u8; 16], challenge: &[u8; 8]) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    type HmacSha1 = Hmac<sha1::Sha1>;
    let mut mac = HmacSha1::new_from_slice(new_key).unwrap();
    mac.update(challenge);
    let response = mac.finalize().into_bytes(); // 20 bytes

    let mut apdu = vec![0x00, 0x03, 0x00, 0x00];
    let key_tlv_body = 1 + 16; // algo_byte + key
    let data_len = 2 + key_tlv_body + 2 + 8 + 2 + 20;
    apdu.push(data_len as u8);
    apdu.push(0x73); apdu.push(key_tlv_body as u8); apdu.push(0x21); apdu.extend_from_slice(new_key);
    apdu.push(0x74); apdu.push(8); apdu.extend_from_slice(challenge);
    apdu.push(0x75); apdu.push(20); apdu.extend_from_slice(&response);
    apdu
}
```

### SET CODE APDU (remove password)
```rust
// Send TAG_KEY with empty value — spec: "If length 0 is sent, authentication is removed"
fn build_remove_code_apdu() -> Vec<u8> {
    // 0x73 | 0x00 — key tag with zero length
    vec![0x00, 0x03, 0x00, 0x00, 0x02, 0x73, 0x00]
}
```

### PBKDF2 Key Derivation
```rust
// Source: https://developers.yubico.com/OATH/YKOATH_Protocol.html
fn derive_oath_key(password: &str, device_name_salt: &[u8]) -> [u8; 16] {
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(
        password.as_bytes(),
        device_name_salt,
        1000,
        &mut key,
    );
    key
}
```

### Extend OathState to Carry Auth Info
```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct OathState {
    pub credentials: Vec<OathCredential>,
    pub password_required: bool,
    // New fields for password management:
    pub device_name: Vec<u8>,      // TAG_NAME from SELECT, used as PBKDF2 salt
    pub device_challenge: Vec<u8>, // TAG_CHALLENGE from SELECT (empty if no password)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| ykman CLI for OATH password | Direct APDU in yubitui | Phase 5 (v1.0) | No ykman dep required |
| URI import goes directly to PUT | URI import shows preview | Phase 14 (this phase) | OATH-07 compliance |
| Password-protected OATH deferred | Full set/change/remove lifecycle | Phase 14 (this phase) | OATH-08/09/10 |

**Deprecated/outdated:**
- The `OathScreen` compose branch for `password_required` currently shows "Use the yubikey manager CLI to remove the password" — this message must be replaced by Phase 14's password management screens.

## Environment Availability

Step 2.6: SKIPPED (no external dependencies beyond pcsc which is already verified working in v1.0/v1.1).

## Open Questions

1. **getrandom / random for host challenge**
   - What we know: 8 random bytes needed for host challenge in VALIDATE and SET CODE.
   - What's unclear: Whether `rand` or `getrandom` is already in Cargo.toml; using `[0u8; 8]` is insecure but functional for development.
   - Recommendation: Check `cargo tree | grep rand`; if present use `rand::random::<[u8; 8]>()`; otherwise use a simple counter-based nonce or add `getrandom` as a minimal dep.

2. **Connection refactor scope**
   - What we know: VALIDATE + SET CODE must run in one card session; current model functions each open their own connection.
   - What's unclear: Whether a full refactor of `model/oath.rs` to pass `&Card` is needed, or just the password functions.
   - Recommendation: Add password functions with an internal helper that opens one connection and runs SELECT + VALIDATE + SET CODE in sequence. Do not refactor the existing single-APDU functions — scope creep risk.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| OATH-07 | User can import an OATH account by pasting an otpauth:// URI — issuer, account, secret, and algorithm are pre-filled from the URI; user confirms before adding | `ImportUriScreen` exists; needs 2-step state machine (Paste → Confirm) + `ParsedOtpAuth` struct returning all fields |
| OATH-08 | User can set an OATH application password when none is configured; subsequent OATH operations prompt for password only when SW 0x6982 is returned | `SET CODE` APDU (INS=0x03) with PBKDF2 key derivation; `OathPasswordMenuScreen` → `SetOathPasswordScreen`; SW 0x6982 detection already in `get_oath_state()` |
| OATH-09 | User can change an existing OATH application password after authenticating with the current password | `VALIDATE` (INS=0xA3) then `SET CODE` in same card session; `ChangeOathPasswordScreen` with 3 inputs |
| OATH-10 | User can remove the OATH application password after authenticating with the current password | `VALIDATE` then `SET CODE` with empty key TLV; `RemoveOathPasswordScreen` with 1 input + confirm |
</phase_requirements>

## Sources

### Primary (HIGH confidence)
- https://developers.yubico.com/OATH/YKOATH_Protocol.html — APDU specs for SELECT, VALIDATE, SET CODE; TLV formats; status words; PBKDF2 requirement
- `/Users/michael/code/yubitui/src/model/oath.rs` — existing APDU constants, base32_decode, parse_tlv, put/delete/get functions
- `/Users/michael/code/yubitui/src/tui/oath.rs` — existing ImportUriScreen, AddAccountScreen, DeleteConfirmScreen, parse_otpauth_uri()

### Secondary (MEDIUM confidence)
- WebSearch (YKOATH protocol) — confirmed PBKDF2-HMAC-SHA1, 1000 iterations, device name as salt

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — existing pcsc/hmac/pbkdf2 RustCrypto crates; YKOATH spec is authoritative
- Architecture: HIGH — existing patterns in codebase (PinManagementScreen, AddAccountScreen) are directly applicable
- Pitfalls: HIGH — APDU session lifetime and PBKDF2 salt are spec-defined; URI preview gap is visible in source code

**Research date:** 2026-03-29
**Valid until:** 2026-06-29 (YKOATH protocol is stable; APDU bytes do not change)
