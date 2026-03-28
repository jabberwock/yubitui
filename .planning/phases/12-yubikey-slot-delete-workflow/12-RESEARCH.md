# Phase 12: YubiKey Slot Delete Workflow - Research

**Researched:** 2026-03-28
**Domain:** OpenPGP individual key slot deletion via PUT DATA; PIV certificate/key deletion via PUT DATA and INS_MOVE_KEY; PIV management key authentication (3DES/AES challenge-response); textual-rs confirmation flow patterns
**Confidence:** HIGH for PIV cert/key deletion APDUs; HIGH for OpenPGP key slot deletion (attribute-change trick); MEDIUM for PIV management key auth (3DES understood, AES-192 firmware-version-dependent); HIGH for TUI confirmation patterns (already in codebase)

---

## Summary

Phase 12 is about letting users delete individual OpenPGP key slots (SIG, ENC, AUT) and PIV certificate/key slots without performing a full factory reset of the device. The core technical insight is:

**OpenPGP individual key deletion:** There is no "DELETE KEY" APDU in the OpenPGP card spec. The yubikey-manager SDK uses a workaround: send PUT DATA twice to change algorithm attributes (first to RSA4096, then back to RSA2048). The attribute change destroys the stored key material. This requires Admin PIN authentication (not management key). The APDU is `PUT DATA (0xDA)` with DO tags `0xC1` (SIG), `0xC2` (ENC), `0xC3` (AUT) and RSA attribute bytes. This is the ONLY protocol-compliant way to clear individual OpenPGP slots without full factory reset.

**PIV certificate deletion:** Send `PUT DATA (INS=0xDB, P1=0x3F, P2=0xFF)` with TLV `5C:[object-id]` + `53:` (empty data). Requires management key authentication first. Clears the certificate but NOT the key. Available on all firmware versions.

**PIV key deletion:** Send `MOVE KEY (INS=0xF6, P1=0xFF, P2=slot)` with no data. Requires management key authentication. Deletes the private key from the slot. **Requires YubiKey firmware 5.7.0 or later.** For older firmware, only certificate deletion is possible via PUT DATA — the key material itself cannot be removed without factory reset.

**PIV management key authentication:** Two-APDU challenge-response. First APDU sends `00 87 [algo] 9B 04 7C 02 81 00` (request challenge). Card returns 8-byte challenge. Second APDU sends encrypted response. Algorithm: 0x03 = 3DES (firmware <5.7), 0x0C = AES-256 (firmware 5.7+). The response is the 8-byte challenge encrypted under the management key using 3DES-ECB (or AES-256-ECB). This requires the `des` crate (v0.9.0-rc.3) or `aes` crate — neither is currently in Cargo.toml.

**Primary recommendation:** Implement OpenPGP slot delete (attribute-change trick, Admin PIN required) and PIV certificate delete (PUT DATA empty, management key required). Flag PIV key delete as YK 5.7+ only with a visible version check. For firmware-version-gated PIV key delete, use the management key the user already has stored in model state from the existing PIV default-key check; require it to be entered fresh for destructive operations.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| pcsc | 2.8 (already in Cargo.toml) | All APDU transmission for OpenPGP and PIV deletions | Project's established PC/SC interface |
| textual-rs | 0.3.9 (already in Cargo.toml) | ConfirmScreen + modal overlay for delete confirmation flow | All screens use it; ConfirmScreen already in `src/tui/widgets/popup.rs` |
| des | 0.9.0-rc.3 (NEW — not in Cargo.toml) | 3DES-ECB encrypt the PIV management key challenge-response | RustCrypto crates; required for PIV mgmt-key auth on firmware <5.7 |
| aes | 0.8 (NEW — if needed) | AES-256-ECB for PIV mgmt-key auth on firmware 5.7+ | RustCrypto crates; same ecosystem as `des` |
| cipher | 0.4 (pulled in by des/aes) | Block cipher traits — `BlockEncrypt` | Transitive dep of des/aes |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| anyhow | 1.0 (already in Cargo.toml) | Error propagation in model operations | All new model functions |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `des` crate for 3DES | OpenSSL via `openssl` crate | `des` is pure Rust, no system dep. `openssl` is larger, requires system OpenSSL. Use `des`. |
| Two-APDU challenge-response | Subprocess `yubico-piv-tool` | Project constraint: NO ykman/subprocess. Native APDUs only. |
| Attribute-change trick for OpenPGP delete | Full factory reset | Factory reset wipes all three slots. Attribute change deletes only the targeted slot. |

**Installation (new deps):**
```bash
cargo add des cipher
# Or if AES-256 mgmt key support needed for YK 5.7+:
cargo add des aes cipher
```

**Version verification:**
- des: `cargo search des --limit 1` → 0.9.0-rc.3 (2025-03-28)
- Note: 0.9.0-rc.3 is pre-release; use `des = { version = "0.9.0-rc.3" }` explicitly

---

## Architecture Patterns

### New Files Required

```
src/model/piv_delete.rs        # PIV cert delete APDU + management key auth challenge-response
src/model/openpgp_delete.rs    # OpenPGP slot delete via algorithm attribute change trick
src/tui/delete_slot.rs         # DeleteSlotScreen — unified "select slot + confirm + run" widget
```

### Modifications Required

```
src/model/mod.rs               # Add pub mod piv_delete; pub mod openpgp_delete
src/tui/mod.rs                 # Add pub mod delete_slot
src/tui/piv.rs                 # Add 'D' keybinding → push DeleteSlotScreen(PIV)
src/tui/keys.rs                # Wire existing "delete_key" action → actual DeleteSlotScreen(OpenPGP)
                               # Currently: delete_key only shows ConfirmScreen (no APDU triggered)
src/model/mock.rs              # No change needed — mock data already has openpgp/piv slots occupied
Cargo.toml                     # Add des crate dependency
```

### Pattern 1: OpenPGP Key Slot Delete (Attribute-Change Trick)

**Source:** yubikey-manager `yubikit/openpgp.py` `delete_key()` method (verified 2026-03-28)

The OpenPGP spec has no DELETE KEY command. Changing algorithm attributes twice destroys the key material. Requires Admin PIN (0x83) verification, not management key.

```rust
// Source: yubikit/openpgp.py delete_key() + PUT DATA semantics from OpenPGP spec 3.4.1
// DO tags for algorithm attributes: 0xC1=SIG, 0xC2=ENC, 0xC3=AUT

pub enum OpenPgpKeySlot { Sig, Enc, Aut }

impl OpenPgpKeySlot {
    pub fn algorithm_attr_tag(&self) -> u8 {
        match self {
            Self::Sig => 0xC1,
            Self::Enc => 0xC2,
            Self::Aut => 0xC3,
        }
    }
}

// RSA attribute bytes: [algo_id=0x01, n_len_hi, n_len_lo, e_len_hi, e_len_lo, format=0x00]
// RSA4096: n_len=4096=0x1000, e_len=17=0x0011
const RSA4096_ATTRS: &[u8] = &[0x01, 0x10, 0x00, 0x00, 0x11, 0x00];
// RSA2048: n_len=2048=0x0800, e_len=17=0x0011
const RSA2048_ATTRS: &[u8] = &[0x01, 0x08, 0x00, 0x00, 0x11, 0x00];

/// Delete a single OpenPGP key slot by changing algorithm attributes twice.
/// Requires Admin PIN to have been verified before calling (VERIFY INS=0x20 P2=0x83).
pub fn delete_openpgp_key(card: &pcsc::Card, slot: OpenPgpKeySlot, admin_pin: &str) -> Result<()> {
    // 1. Verify Admin PIN (P2=0x83)
    verify_openpgp_admin_pin(card, admin_pin)?;

    let tag = slot.algorithm_attr_tag();

    // 2. PUT DATA (INS=0xDA, P1=tag_hi=0x00, P2=tag_lo=tag) — change to RSA4096
    put_data_openpgp(card, 0x00, tag, RSA4096_ATTRS)?;

    // 3. PUT DATA again — change to RSA2048 (destroys key material)
    put_data_openpgp(card, 0x00, tag, RSA2048_ATTRS)?;

    Ok(())
}

fn verify_openpgp_admin_pin(card: &pcsc::Card, pin: &str) -> Result<()> {
    let pin_bytes = pin.as_bytes();
    // VERIFY: CLA=00 INS=20 P1=00 P2=83 Lc=[len] [pin]
    let mut apdu = vec![0x00u8, 0x20, 0x00, 0x83, pin_bytes.len() as u8];
    apdu.extend_from_slice(pin_bytes);
    let mut buf = [0u8; 16];
    let resp = card.transmit(&apdu, &mut buf)?;
    let sw = super::card::apdu_sw(resp);
    if sw != 0x9000 {
        anyhow::bail!("{}", super::card::apdu_error_message(sw, "verifying Admin PIN"));
    }
    Ok(())
}

fn put_data_openpgp(card: &pcsc::Card, p1: u8, p2: u8, data: &[u8]) -> Result<()> {
    // PUT DATA: CLA=00 INS=DA P1=p1 P2=p2 Lc=[len] [data]
    let mut apdu = vec![0x00u8, 0xDA, p1, p2, data.len() as u8];
    apdu.extend_from_slice(data);
    let mut buf = [0u8; 16];
    let resp = card.transmit(&apdu, &mut buf)?;
    let sw = super::card::apdu_sw(resp);
    if sw != 0x9000 {
        anyhow::bail!("{}", super::card::apdu_error_message(sw, "PUT DATA algorithm attributes"));
    }
    Ok(())
}
```

### Pattern 2: PIV Certificate Delete

**Source:** yubikey-manager `yubikit/piv.py` `delete_certificate()` method (verified 2026-03-28)

PUT DATA with empty 0x53 value erases the certificate data object. The private key is NOT deleted.

```rust
// PIV slot object IDs (from NIST 800-73 and Yubico extensions)
pub enum PivSlot {
    Authentication, // 9a — object tag bytes: 5F C1 05
    Signature,      // 9c — object tag bytes: 5F C1 0A
    KeyManagement,  // 9d — object tag bytes: 5F C1 0B
    CardAuth,       // 9e — object tag bytes: 5F C1 01
}

impl PivSlot {
    pub fn object_id_bytes(&self) -> &'static [u8] {
        match self {
            Self::Authentication => &[0x5F, 0xC1, 0x05],
            Self::Signature      => &[0x5F, 0xC1, 0x0A],
            Self::KeyManagement  => &[0x5F, 0xC1, 0x0B],
            Self::CardAuth       => &[0x5F, 0xC1, 0x01],
        }
    }
    pub fn slot_id(&self) -> u8 {
        match self {
            Self::Authentication => 0x9A,
            Self::Signature      => 0x9C,
            Self::KeyManagement  => 0x9D,
            Self::CardAuth       => 0x9E,
        }
    }
}

/// Delete a PIV certificate (not the key) from a slot.
/// Management key must be authenticated before calling.
/// Available on all YubiKey firmware versions.
pub fn delete_piv_certificate(card: &pcsc::Card, slot: PivSlot) -> Result<()> {
    let obj_id = slot.object_id_bytes();
    // PUT DATA: CLA=00 INS=DB P1=3F P2=FF
    // Data: 5C [obj_id_len] [obj_id] 53 00
    // (TLV: TAG_OBJ_ID=0x5C, TAG_OBJ_DATA=0x53 with empty value)
    let mut data: Vec<u8> = Vec::new();
    data.push(0x5C);
    data.push(obj_id.len() as u8);
    data.extend_from_slice(obj_id);
    data.push(0x53);
    data.push(0x00); // empty value = delete

    let mut apdu = vec![0x00u8, 0xDB, 0x3F, 0xFF, data.len() as u8];
    apdu.extend_from_slice(&data);

    let mut buf = [0u8; 16];
    let resp = card.transmit(&apdu, &mut buf)?;
    let sw = super::card::apdu_sw(resp);
    if sw != 0x9000 {
        anyhow::bail!("{}", super::card::apdu_error_message(sw, "deleting PIV certificate"));
    }
    Ok(())
}
```

### Pattern 3: PIV Key Delete (Firmware 5.7+ Only)

**Source:** yubikey-manager `yubikit/piv.py` `delete_key()` + Yubico extensions doc (verified 2026-03-28)

```rust
/// Delete a PIV private key from a slot.
/// Requires YubiKey firmware >= 5.7.0. Check version before calling.
/// Management key must be authenticated before calling.
pub fn delete_piv_key(card: &pcsc::Card, slot: PivSlot, firmware: &crate::model::Version) -> Result<()> {
    if firmware.major < 5 || (firmware.major == 5 && firmware.minor < 7) {
        anyhow::bail!("PIV key deletion requires firmware 5.7 or later (device is {}.{}.{})",
            firmware.major, firmware.minor, firmware.patch);
    }
    // MOVE KEY: CLA=00 INS=F6 P1=FF P2=[slot_id]
    let apdu = [0x00u8, 0xF6, 0xFF, slot.slot_id()];
    let mut buf = [0u8; 16];
    let resp = card.transmit(&apdu, &mut buf)?;
    let sw = super::card::apdu_sw(resp);
    if sw != 0x9000 {
        anyhow::bail!("{}", super::card::apdu_error_message(sw, "deleting PIV key"));
    }
    Ok(())
}
```

### Pattern 4: PIV Management Key Authentication

**Source:** Yubico APDU docs + yubikey-manager yubikit/piv.py `authenticate()` method (verified 2026-03-28)

This is a two-APDU challenge-response. The card sends a random 8-byte challenge; the client encrypts it under the management key and sends it back. Algorithm for firmware <5.7 is 3DES-ECB on an 8-byte block. For firmware 5.7+ it is AES-256-ECB on a 16-byte block.

```rust
// Requires `des = "0.9.0-rc.3"` in Cargo.toml
// use des::Des; use des::cipher::{BlockEncrypt, KeyInit};
// NOTE: 3DES (Triple-DES-EDE) key = 24 bytes; encrypt with first 8 bytes only in single-DES ECB
// The PIV spec says "single DES" for management key auth challenge when algo=0x03.
// HOWEVER: ykman source uses Triple DES (DES3 with 24-byte key) for the whole thing.

/// Default PIV management key (3DES) — 24 bytes.
pub const PIV_DEFAULT_MGMT_KEY_3DES: &[u8; 24] = &[
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
];

/// Authenticate PIV management key (3DES, algorithm=0x03).
/// Returns Err on wrong key or transmission error.
pub fn authenticate_piv_mgmt_key_3des(card: &pcsc::Card, key: &[u8; 24]) -> Result<()> {
    // Step 1: Request challenge
    // GENERAL AUTHENTICATE: CLA=00 INS=87 P1=03 P2=9B Lc=04 7C 02 81 00
    let step1 = [0x00u8, 0x87, 0x03, 0x9B, 0x04, 0x7C, 0x02, 0x81, 0x00];
    let mut buf1 = [0u8; 64];
    let resp1 = card.transmit(&step1, &mut buf1)?;
    let sw1 = super::card::apdu_sw(resp1);
    if sw1 != 0x9000 {
        anyhow::bail!("Management key challenge failed: SW {:04X}", sw1);
    }
    // Response layout: 7C 0A 81 08 [8 challenge bytes]
    if resp1.len() < 12 {
        anyhow::bail!("Challenge response too short");
    }
    let challenge = &resp1[4..12]; // bytes after 7C 0A 81 08

    // Step 2: Encrypt challenge with 3DES (EDE3) and send back
    // des crate: use des::TdesEde3; requires des = "0.9.0-rc.3"
    // let cipher = des::TdesEde3::new_from_slice(key)?;
    // let mut block = GenericArray::clone_from_slice(challenge);
    // cipher.encrypt_block(&mut block);
    // let response = block.as_slice();

    // GENERAL AUTHENTICATE step 2: 7C 0A 82 08 [8 response bytes]
    let mut step2_data = vec![0x7Cu8, 0x0A, 0x82, 0x08];
    // step2_data.extend_from_slice(response); // after encryption
    let mut step2 = vec![0x00u8, 0x87, 0x03, 0x9B, step2_data.len() as u8 + 8];
    step2.extend_from_slice(&step2_data);
    // step2.extend_from_slice(response);

    let mut buf2 = [0u8; 16];
    let resp2 = card.transmit(&step2, &mut buf2)?;
    let sw2 = super::card::apdu_sw(resp2);
    if sw2 == 0x9000 {
        Ok(())
    } else {
        anyhow::bail!("{}", super::card::apdu_error_message(sw2, "authenticating management key"));
    }
}
```

**IMPORTANT:** The plan must wire up actual 3DES encryption. The placeholder comment above shows the structure; the plan task must use `des::TdesEde3` from the `des` crate. The management key challenge bytes are in `resp1[4..12]` assuming the response layout `7C 0A 81 08 [challenge]`.

### Pattern 5: TUI Delete Flow (Existing Pattern)

The delete confirmation flow already exists in `src/tui/widgets/popup.rs` as `ConfirmScreen`. The `keys.rs` `delete_key` action already pushes a `ConfirmScreen` but does NOT yet trigger an APDU. The plan needs to:

1. Add a `run_worker_with_progress` call after confirmation in keys.rs to actually invoke the model operation
2. Extend `PivScreen` to add a slot selection + delete flow

The worker pattern (from fido2.rs delete flow) is:

```rust
// Source: src/tui/fido2.rs delete_credential handler
"delete_credential" => {
    // Push ConfirmScreen; after confirm, run worker
    let state = self.state.get_untracked();
    let Some(creds) = self.fido2_state.as_ref().and_then(|s| s.credentials.as_ref()) else { return; };
    let Some(cred) = creds.get(state.selected_index) else { return; };
    let cred_id = cred.credential_id.clone();
    let pin = state.cached_pin.clone();
    ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
        DeleteConfirmScreen::new(cred_id, pin)
    ))));
}
```

The model operation is invoked inside a `DeleteConfirmScreen` that wraps `ConfirmScreen` and, on confirm, calls `run_worker_with_progress`.

### Recommended Project Structure Additions

```
src/
├── model/
│   ├── openpgp_delete.rs    # delete_openpgp_key(card, slot, admin_pin)
│   └── piv_delete.rs        # delete_piv_certificate(card, slot), delete_piv_key(card, slot, fw)
│                            # authenticate_piv_mgmt_key_3des(card, key)
├── tui/
│   └── delete_slot.rs       # DeleteSlotScreen (for PIV) or inline in piv.rs
```

### Anti-Patterns to Avoid

- **Calling PUT DATA with no prior management key auth (PIV):** The card will return SW 0x6982. Management key auth MUST precede PUT DATA for PIV. OpenPGP uses Admin PIN (separate concept).
- **Using ykman or any subprocess:** Project constraint. All operations must be native PC/SC APDUs.
- **Attempting PIV key delete on firmware < 5.7:** The MOVE KEY APDU (INS=0xF6 P1=0xFF) will return SW 0x6D00 (unknown instruction). Must version-check before offering the option.
- **Factory reset as a workaround:** Phase motivation is specifically to avoid this. Do not offer factory reset as the delete flow.
- **Forgetting to kill scdaemon before card access:** Follow `piv::get_piv_state()` pattern — `card::kill_scdaemon()` + 50ms sleep at every APDU entry point.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Delete confirmation dialog | Custom ratatui overlay | `ModalScreen::new(ConfirmScreen::new(..., destructive=true))` | Already in `src/tui/widgets/popup.rs`; handles Esc/Enter/Y keys |
| Background APDU execution | Blocking the TUI event loop | `ctx.run_worker_with_progress()` | textual-rs worker pattern; fido2.rs uses it for delete_credential |
| 3DES encryption | Hand-rolled bytes | `des` crate `TdesEde3` from RustCrypto | Block cipher correctness is subtle; use audited crate |
| Admin PIN input | Custom text input widget | `PinInputWidget` from `src/tui/widgets/pin_input.rs` | Already exists; used by PinManagementScreen |

**Key insight:** The deletion confirmation + worker pattern is already proven in FIDO2 (credential delete) and OATH (account delete). This phase applies the same pattern to OpenPGP and PIV.

---

## Common Pitfalls

### Pitfall 1: Algorithm Attribute Format Differences (OpenPGP)

**What goes wrong:** Planner sends RSA4096 attribute bytes in the wrong format for different YubiKey firmware versions, causing PUT DATA to return SW 0x6A80 (incorrect data).
**Why it happens:** The attribute byte format changed between OpenPGP spec versions. Some firmware uses a 5-byte format, some 6-byte.
**How to avoid:** Use the format from yubikit exactly: `[0x01, n_len_hi, n_len_lo, e_len_hi, e_len_lo, 0x00]` — 6 bytes total. For RSA4096: `[0x01, 0x10, 0x00, 0x00, 0x11, 0x00]`. Verify SW on both PUT DATA calls; if first fails, abort — don't send the second.
**Warning signs:** SW 0x6A80 (incorrect data) on first PUT DATA; key not removed after two successful PUT DATAs.

### Pitfall 2: PIV Key Delete Firmware Gate

**What goes wrong:** Delete key button appears in TUI for all users, but the APDU fails with SW 0x6D00 on older firmware.
**Why it happens:** `MOVE KEY (INS=0xF6 P1=0xFF)` was added in firmware 5.7.0.
**How to avoid:** Check `yk.info.version` before showing the "Delete Key" option. If `major < 5 || (major == 5 && minor < 7)`: show "Delete Key (requires firmware 5.7+)" as a disabled/grayed label, not an actionable button. Only show the button for 5.7+. Use the version already in `YubiKeyState.info.version`.
**Warning signs:** SW 0x6D00 response, TUI crashes because worker returns error immediately.

### Pitfall 3: Management Key Auth Crypto Complexity

**What goes wrong:** Implementing the 3DES challenge-response incorrectly — using single-DES instead of 3DES-EDE, wrong key byte order, or wrong ECB mode.
**Why it happens:** The spec says "algorithm 0x03 = 3DES" but the actual operation is triple-DES-EDE (24-byte key, 8-byte block, ECB, no IV). Easy to confuse with single-DES or CBC mode.
**How to avoid:** Use `des::TdesEde3::new_from_slice(key).unwrap()` from the `des` crate. The 24-byte default key is `01 02 03 ... 08` repeated 3×. Call `cipher.encrypt_block(&mut block)` where `block` is the 8-byte challenge in a `GenericArray`. The `des` crate's `TdesEde3` handles the EDE keying schedule automatically.
**Warning signs:** SW 0x6982 (security condition not satisfied) on the second GENERAL AUTHENTICATE — means the encrypted response was wrong.

### Pitfall 4: PIV Certificate vs Key Delete Are Two Separate Operations

**What goes wrong:** User deletes the certificate (PUT DATA empty) but the private key remains on the card. The slot still "looks occupied" from the private key's perspective.
**Why it happens:** PIV stores certificate and private key as separate data objects. PUT DATA (INS=0xDB) only touches the certificate DO. The key is stored separately and requires MOVE KEY (INS=0xF6) to delete.
**How to avoid:** For a complete slot clear on firmware 5.7+: (1) delete certificate, then (2) delete key. Both operations require management key auth. For firmware <5.7: only certificate deletion is possible; communicate this in the UI ("Certificate cleared — key material cannot be removed without factory reset on this firmware version").
**Warning signs:** `piv::get_piv_state()` still returns the slot as occupied after certificate deletion (because the GET DATA APDU still finds the key's data).

**NOTE on `get_piv_state` correctness:** The current `src/model/piv.rs` uses `GET DATA (00 CB 3F FF 05 5C 03 5F C1 XX)` to check slot occupancy. This reads the **certificate** data object. SW 0x9000 means a certificate is present. After certificate-only deletion, this will return SW 0x6A82 (not found) for that slot — which is correct. After key-only deletion (no cert delete), the GET DATA for the cert DO would already return 0x6A82 (cert was never there in that scenario). The combination works as expected.

### Pitfall 5: OpenPGP Admin PIN Retry Counter

**What goes wrong:** Failed Admin PIN verification during the delete flow consumes a PIN retry. If the user enters the wrong PIN three times, the admin PIN is blocked.
**Why it happens:** VERIFY (INS=0x20 P2=0x83) decrements the Admin PIN retry counter on failure. The counter is only reset on success.
**How to avoid:** Use the existing `PinInputWidget` for Admin PIN entry. Display the current retry count in the confirmation screen (from `yk.openpgp.admin_pin_retries` if available, or fetch it with GET DATA 0xC4). Add a warning: "Admin PIN has N retries remaining — wrong PIN will consume one."
**Warning signs:** SW `0x63C2` (2 retries), `0x63C1` (1 retry), `0x6983` (blocked). Handle these in `apdu_error_message` (it already handles `0x63Cx` patterns).

### Pitfall 6: OpenPGP SELECT Required Before PUT DATA

**What goes wrong:** PUT DATA (0xDA) fails with SW 0x6A82 or 0x6D00 because the OpenPGP application was not selected first.
**Why it happens:** After `kill_scdaemon()` and reconnect, the CCID connection starts with no application selected.
**How to avoid:** Always SELECT OpenPGP AID before any OpenPGP APDU. Use `card::connect_to_openpgp_card()` which already does this. Similarly, SELECT PIV AID before PIV APDUs.
**Warning signs:** SW 0x6D00 (unknown instruction) on first PUT DATA after reconnect.

---

## OpenPGP vs PIV Deletion Decision Matrix

This decision matrix is critical for the planner — the two application domains have completely different protocols:

| Dimension | OpenPGP | PIV |
|-----------|---------|-----|
| Auth mechanism | Admin PIN (VERIFY INS=0x20 P2=0x83) | Management key (GENERAL AUTHENTICATE challenge-response) |
| Delete certificate | N/A (OpenPGP has no separate cert store in basic use) | PUT DATA (INS=0xDB) with empty 0x53 value |
| Delete key | PUT DATA attribute change trick × 2 | MOVE KEY (INS=0xF6 P1=0xFF P2=slot) |
| Firmware req | All firmware (attribute change is standard PUT DATA) | Key delete: 5.7+; Cert delete: all firmware |
| APDU to select app | SELECT OpenPGP AID (0xD2760001 2401) | SELECT PIV AID (0xA0000003 0800001000) |
| Effect | Key material destroyed; slot empty (all-zero fingerprint) | Cert and/or key deleted; GET DATA returns SW 0x6A82 |
| Reversible? | No — key material is gone permanently | No — private key is gone; cert can be re-imported |

---

## Code Examples

### OpenPGP Admin PIN Retry Count (for UI warning)

```rust
// Source: OpenPGP spec 3.4 — DO 0xC4 (PW Status Bytes)
// Byte layout: [max_user_pin, max_reset_code, max_admin_pin,
//               cur_user_pin, cur_reset_code, cur_admin_pin]
pub fn get_openpgp_pin_retries(card: &pcsc::Card) -> Result<(u8, u8, u8)> {
    let data = super::card::get_data(card, 0x00, 0xC4)?;
    if data.len() < 6 {
        anyhow::bail!("PW Status Bytes response too short");
    }
    // Returns (user_retries, reset_code_retries, admin_retries)
    Ok((data[3], data[4], data[5]))
}
```

### PIV Slot Object IDs (Standard + Retired)

```rust
// Source: NIST 800-73-4 + Yubico PIV introduction doc
// Standard slots used in this phase:
// 9A: 5F C1 05  (Authentication)
// 9C: 5F C1 0A  (Signature)
// 9D: 5F C1 0B  (Key Management)
// 9E: 5F C1 01  (Card Authentication — no PIN needed)
```

### TUI Worker Pattern for Delete (from fido2.rs)

```rust
// Source: src/tui/fido2.rs DeleteConfirmScreen pattern
// After ConfirmScreen returns "confirm", trigger a worker:
ctx.run_worker_with_progress(self.own_id.get().unwrap(), move || {
    // This closure runs on a background thread
    let result = crate::model::openpgp_delete::delete_openpgp_key(
        &card, slot, &admin_pin
    );
    WorkerResult::from(result)
});
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Factory reset to clear OpenPGP | PUT DATA attribute change × 2 | yubikey-manager API (discovered) | Only targeted slot is cleared |
| ykman subprocess for cert/key ops | Native PC/SC APDUs | Project v1.0 decision | No subprocess dependency |
| PIV key delete unavailable | MOVE KEY INS=0xF6 P1=0xFF (YK 5.7+) | YubiKey firmware 5.7 | Possible only on newer devices |

**Deprecated/outdated:**
- PIN-block-then-TERMINATE approach for OpenPGP partial reset: Only works for full factory reset, not individual slots.

---

## Open Questions

1. **PIV Management Key: AES-192 vs 3DES on Firmware 5.7+**
   - What we know: Firmware 5.7+ uses AES-192 as default management key (per Yubico docs). The `des` crate handles 3DES. AES-192 requires the `aes` crate.
   - What's unclear: The plan should support both. The firmware version is already in `YubiKeyState.info.version`. Check: if `>= 5.7` use AES-192 (algorithm 0x0C), else use 3DES (algorithm 0x03).
   - Recommendation: Add `aes` crate alongside `des`. Use firmware version gate to select algorithm. The default key bytes for AES-192 on 5.7+ are documented as: `010203040506070801020304050607080102030405060708` (same bytes as 3DES default). Confirm this in plan-phase spike.

2. **Does keys.rs Delete Action Need PIN Input Before Confirm?**
   - What we know: The existing `delete_key` action in `keys.rs` shows `ConfirmScreen` but never prompts for Admin PIN. The APDU requires it.
   - What's unclear: Should the flow be "Confirm first, then enter PIN" or "Enter PIN, then confirm"?
   - Recommendation: Enter PIN first (so user can cancel before destructive operation), then show confirm screen, then run APDU. Pattern: `PinInputScreen` → `ConfirmScreen(destructive=true)` → worker.

3. **PIV GET DATA vs GET METADATA for Slot Occupancy After Delete**
   - What we know: `piv::get_piv_state()` uses GET DATA to check occupancy. After certificate deletion, GET DATA returns 0x6A82. After key deletion only (no cert), GET DATA for the cert DO already returns 0x6A82.
   - What's unclear: For a "was this slot actually cleared?" check after delete, is re-running `get_piv_state()` sufficient, or does `GET METADATA (INS=0xF7, YK 5.3+)` provide more detail?
   - Recommendation: Re-run `get_piv_state()` after delete to refresh TUI state. This is sufficient for v1.1.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| pcsc | All APDU operations | Already in Cargo.toml | 2.8 | — |
| textual-rs | TUI screens | Already in Cargo.toml | 0.3.9 | — |
| des crate | PIV mgmt key 3DES auth | NOT in Cargo.toml | 0.9.0-rc.3 | Manual 3DES (fragile, not recommended) |
| aes crate | PIV mgmt key AES-192 auth (YK 5.7+) | NOT in Cargo.toml | 0.8.x | Skip AES-192 key support in v1.1 |

**Missing dependencies with no fallback:**
- `des` crate — required for PIV management key authentication on firmware <5.7. Without it, PIV certificate/key deletion cannot authenticate. Must add to Cargo.toml.

**Missing dependencies with fallback:**
- `aes` crate — if omitted, PIV operations only work with the default 3DES key on firmware <5.7. Users with YK 5.7+ custom management keys cannot authenticate. Acceptable for v1.1 if non-default AES-192 key support is deferred.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` + textual-rs Pilot |
| Config file | None (Cargo.toml `[dev-dependencies]`) |
| Quick run command | `cargo test --lib -- model::openpgp_delete model::piv_delete 2>&1` |
| Full suite command | `cargo test 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DEL-01 | OpenPGP key slot delete (SIG/ENC/AUT) executes correct PUT DATA APDUs | unit | `cargo test --lib -- model::openpgp_delete 2>&1` | No — Wave 0 |
| DEL-02 | PIV certificate delete sends correct PUT DATA (empty 0x53) | unit | `cargo test --lib -- model::piv_delete 2>&1` | No — Wave 0 |
| DEL-03 | PIV key delete requires firmware 5.7+ and returns clear error otherwise | unit | `cargo test --lib -- model::piv_delete::test_delete_key_version_gate 2>&1` | No — Wave 0 |
| DEL-04 | Management key 3DES auth computes correct challenge-response | unit | `cargo test --lib -- model::piv_delete::test_mgmt_key_auth 2>&1` | No — Wave 0 |
| DEL-05 | TUI ConfirmScreen (destructive) appears before any APDU executes | Pilot snapshot | `cargo test --test pilot -- delete_slot 2>&1` | No — Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test --lib -- model::openpgp_delete model::piv_delete 2>&1`
- **Per wave merge:** `cargo test 2>&1`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src/model/openpgp_delete.rs` — covers DEL-01; unit tests with mock APDU responses
- [ ] `src/model/piv_delete.rs` — covers DEL-02, DEL-03, DEL-04
- [ ] `tests/pilot/delete_slot.rs` — covers DEL-05 (Pilot snapshot test)

---

## Sources

### Primary (HIGH confidence)

- `yubikey-manager` GitHub `yubikit/piv.py` — `delete_certificate()`, `delete_key()`, `authenticate()` methods: verified PIV PUT DATA encoding, MOVE KEY APDU, and management key auth sequence (2026-03-28)
- `yubikey-manager` GitHub `yubikit/openpgp.py` — `delete_key()` method: verified attribute-change-trick byte sequence and DO tag values (2026-03-28)
- [Yubico PIV Extensions doc](https://developers.yubico.com/PIV/Introduction/Yubico_extensions.html) — MOVE KEY INS=0xF6 P1=0xFF requires YK 5.7+; Delete Key semantics confirmed
- [Yubico PIV Management Key Authentication APDU](https://docs.yubico.com/yesdk/users-manual/application-piv/apdu/auth-mgmt.html) — two-APDU challenge-response confirmed; 3DES algorithm 0x03
- [Yubico PIV Key Delete action doc](https://developers.yubico.com/yubico-piv-tool/Actions/key_delete.html) — firmware 5.7 requirement confirmed
- [Yubico ResetApplet doc](https://developers.yubico.com/ykneo-openpgp/ResetApplet.html) — TERMINATE/ACTIVATE are full factory reset only; no individual slot reset in OpenPGP spec
- `src/model/card.rs`, `src/model/piv.rs`, `src/tui/widgets/popup.rs`, `src/tui/fido2.rs` — project source patterns (kill_scdaemon, ConfirmScreen, worker pattern)

### Secondary (MEDIUM confidence)

- [Yubico APDU docs — PIV DELETE cert action](https://developers.yubico.com/yubico-piv-tool/Actions/delete_certificate.html) — confirmed management key auth required; specific APDU not in doc but implied from PUT DATA structure
- [Yubico PIV GET/PUT DATA](https://docs.yubico.com/yesdk/users-manual/application-piv/get-and-put-data.html) — INS=0xDB P1=3F P2=FF structure confirmed for PUT DATA
- WebSearch cross-verification: PIV default 3DES key = 24-byte `010203...` (repeated 3×); confirmed in multiple sources

### Tertiary (LOW confidence)

- `des` crate `TdesEde3` API — from crates.io description and Rust crypto ecosystem knowledge; not directly verified via Context7 or official docs in this research session. Mark for plan-phase spike: write a test that encrypts a known block with the default 3DES key and verify output against a known-good reference.

---

## Metadata

**Confidence breakdown:**
- OpenPGP attribute-change delete: HIGH — directly verified from yubikit Python source
- PIV certificate delete (PUT DATA empty): HIGH — verified from yubikit + Yubico docs
- PIV key delete (MOVE KEY): HIGH for API shape; MEDIUM for exact byte verification (firmware 5.7 req confirmed)
- PIV management key 3DES auth: HIGH for APDU structure; MEDIUM for `des` crate exact API (plan-phase spike needed)
- TUI confirmation pattern: HIGH — already proven in fido2.rs

**Research date:** 2026-03-28
**Valid until:** 2026-06-28 (YubiKey APDU protocols are stable; 90-day validity)
