use anyhow::Result;

/// Which OpenPGP key slot to target for deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenPgpKeySlot {
    /// Signature key — DO tag 0xC1.
    Sig,
    /// Encryption key — DO tag 0xC2.
    Enc,
    /// Authentication key — DO tag 0xC3.
    Aut,
}

impl OpenPgpKeySlot {
    /// Returns the Algorithm Attributes DO tag byte (P2 for PUT DATA 0xDA).
    ///
    /// The GET/PUT DATA command uses P1=0x00 and P2=tag for tags < 0x0100.
    pub fn algorithm_attr_tag(self) -> u8 {
        match self {
            OpenPgpKeySlot::Sig => 0xC1,
            OpenPgpKeySlot::Enc => 0xC2,
            OpenPgpKeySlot::Aut => 0xC3,
        }
    }

    /// Human-readable slot name for UI display.
    pub fn display_name(self) -> &'static str {
        match self {
            OpenPgpKeySlot::Sig => "Signature",
            OpenPgpKeySlot::Enc => "Encryption",
            OpenPgpKeySlot::Aut => "Authentication",
        }
    }
}

/// RSA-4096 algorithm attributes (key type 01, modulus len 4096, public exponent 17 bits, import format 00).
pub const RSA4096_ATTRS: &[u8; 6] = &[0x01, 0x10, 0x00, 0x00, 0x11, 0x00];

/// RSA-2048 algorithm attributes (key type 01, modulus len 2048, public exponent 17 bits, import format 00).
pub const RSA2048_ATTRS: &[u8; 6] = &[0x01, 0x08, 0x00, 0x00, 0x11, 0x00];

/// Delete a single OpenPGP key slot on the card using the attribute-change trick.
///
/// The OpenPGP card spec has no "DELETE KEY" command. The established technique
/// is to PUT DATA the algorithm attributes for an RSA key type (which triggers the
/// card to destroy any existing key material for that slot) — first to RSA-4096
/// (forces allocation of a new, empty RSA key buffer), then to RSA-2048 (frees the
/// 4096-bit buffer, leaving the slot empty).
///
/// # Steps
/// 1. kill_scdaemon() + 50 ms sleep (releases the shared card channel)
/// 2. connect_to_openpgp_card() — SELECT OpenPGP AID
/// 3. VERIFY Admin PIN (0x20, P2=0x83)
/// 4. PUT DATA (0xDA) algorithm attributes → RSA4096_ATTRS
/// 5. PUT DATA (0xDA) algorithm attributes → RSA2048_ATTRS
///
/// # Errors
/// Returns a user-readable error string for wrong PIN (with retry count), blocked
/// PIN (0x6983), and any APDU-level failures during PUT DATA.
pub fn delete_openpgp_key(slot: OpenPgpKeySlot, admin_pin: &str) -> Result<()> {
    use super::card::{apdu_sw, connect_to_openpgp_card};

    let (card, _aid) = connect_to_openpgp_card()?;

    // VERIFY Admin PIN — CLA=00 INS=20 P1=00 P2=83 Lc=len data=pin_bytes
    let pin_bytes = admin_pin.as_bytes();
    let pin_len = pin_bytes.len() as u8;
    let mut verify_apdu = vec![0x00u8, 0x20, 0x00, 0x83, pin_len];
    verify_apdu.extend_from_slice(pin_bytes);

    let mut buf = [0u8; 256];
    let resp = card
        .transmit(&verify_apdu, &mut buf)
        .map_err(|e| anyhow::anyhow!("VERIFY Admin PIN transmit error: {e}"))?;

    let sw = apdu_sw(resp);
    match sw {
        0x9000 => {} // success — continue
        sw if sw & 0xFF00 == 0x6300 => {
            let retries = sw & 0x000F;
            anyhow::bail!(
                "Wrong Admin PIN ({} {} remaining)",
                retries,
                if retries == 1 { "retry" } else { "retries" }
            );
        }
        0x6983 => {
            anyhow::bail!("Admin PIN is blocked — use factory reset to recover");
        }
        _ => {
            anyhow::bail!(
                "{}",
                super::card::apdu_error_message(sw, "verifying Admin PIN")
            );
        }
    }

    // PUT DATA — set algorithm attributes to RSA-4096 (destroys key material)
    put_data_algorithm_attrs(&card, slot.algorithm_attr_tag(), RSA4096_ATTRS)?;

    // PUT DATA — set algorithm attributes to RSA-2048 (frees 4096-bit buffer)
    put_data_algorithm_attrs(&card, slot.algorithm_attr_tag(), RSA2048_ATTRS)?;

    Ok(())
}

/// Send a PUT DATA (INS=0xDA) APDU to write algorithm attribute bytes for a slot.
///
/// P1=0x00, P2=tag (0xC1/C2/C3), Lc=6, data=attrs.
fn put_data_algorithm_attrs(card: &pcsc::Card, tag: u8, attrs: &[u8; 6]) -> Result<()> {
    use super::card::apdu_sw;

    let mut apdu = vec![0x00u8, 0xDA, 0x00, tag, attrs.len() as u8];
    apdu.extend_from_slice(attrs);

    let mut buf = [0u8; 256];
    let resp = card
        .transmit(&apdu, &mut buf)
        .map_err(|e| anyhow::anyhow!("PUT DATA transmit error: {e}"))?;

    let sw = apdu_sw(resp);
    if sw != 0x9000 {
        anyhow::bail!(
            "{}",
            super::card::apdu_error_message(sw, &format!("setting algorithm attributes (tag {:02X})", tag))
        );
    }
    Ok(())
}
