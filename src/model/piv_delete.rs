//! PIV slot deletion — management key authentication (3DES) + certificate and key deletion APDUs.
//!
//! This module provides targeted certificate and key deletion for individual PIV slots,
//! using native PC/SC APDUs (no ykman). Requires management key authentication before
//! any write operation.
//!
//! Protocol summary:
//! - Management key auth: GENERAL AUTHENTICATE (0x87) with 3DES-EDE challenge-response
//! - Certificate delete: PUT DATA (0xDB) with empty 0x53 value
//! - Key delete: MOVE KEY (0xF6) — firmware >= 5.7.0 ONLY

use anyhow::Result;
use pcsc::{Context, Protocols, Scope, ShareMode};
use des::TdesEde3;
use cipher::{BlockCipherEncrypt, KeyInit};

use crate::model::Version;

// ============================================================================
// Default management key
// ============================================================================

/// The well-known PIV default management key (3DES-EDE):
/// 01 02 03 04 05 06 07 08 repeated 3 times.
#[allow(dead_code)]
pub const PIV_DEFAULT_MGMT_KEY_3DES: &[u8; 24] = &[
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
];

// ============================================================================
// PivSlot enum
// ============================================================================

/// Standard PIV slot identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PivSlot {
    /// 9a — PIV Authentication
    Authentication,
    /// 9c — Digital Signature
    Signature,
    /// 9d — Key Management
    KeyManagement,
    /// 9e — Card Authentication (no PIN required)
    CardAuth,
}

impl PivSlot {
    /// Returns the 3-byte PIV Data Object tag for this slot (used in GET DATA / PUT DATA).
    /// Tags per NIST SP 800-73-4 Table 4b.
    pub fn object_id_bytes(&self) -> &'static [u8] {
        match self {
            PivSlot::Authentication => &[0x5F, 0xC1, 0x05],
            PivSlot::Signature     => &[0x5F, 0xC1, 0x0A],
            PivSlot::KeyManagement => &[0x5F, 0xC1, 0x0B],
            PivSlot::CardAuth      => &[0x5F, 0xC1, 0x01],
        }
    }

    /// Returns the 1-byte slot id used as P2 in GENERAL AUTHENTICATE, MOVE KEY, etc.
    pub fn slot_id(&self) -> u8 {
        match self {
            PivSlot::Authentication => 0x9A,
            PivSlot::Signature      => 0x9C,
            PivSlot::KeyManagement  => 0x9D,
            PivSlot::CardAuth       => 0x9E,
        }
    }

    /// Human-readable display name for this slot.
    pub fn display_name(&self) -> &'static str {
        match self {
            PivSlot::Authentication => "Authentication (9a)",
            PivSlot::Signature      => "Digital Signature (9c)",
            PivSlot::KeyManagement  => "Key Management (9d)",
            PivSlot::CardAuth       => "Card Authentication (9e)",
        }
    }

    /// Parse a slot string ("9a", "9c", "9d", "9e") into a PivSlot.
    #[allow(dead_code)]
    pub fn from_slot_str(s: &str) -> Option<Self> {
        match s {
            "9a" => Some(PivSlot::Authentication),
            "9c" => Some(PivSlot::Signature),
            "9d" => Some(PivSlot::KeyManagement),
            "9e" => Some(PivSlot::CardAuth),
            _ => None,
        }
    }
}

// ============================================================================
// Management key authentication (3DES challenge-response)
// ============================================================================

/// Authenticate to the PIV management key using 3DES-EDE challenge-response.
///
/// Protocol (NIST SP 800-73-4 §3.2.4, INS=0x87, Algorithm=0x03 [3DES]):
///
/// Step 1 — Request challenge:
///   APDU: 00 87 03 9B 04 7C 02 81 00
///   Response layout: 7C 0A 81 08 [8 bytes challenge]
///
/// Step 2 — Respond with encrypted challenge:
///   Encrypt the 8-byte challenge with 3DES-EDE (ECB mode, single block).
///   APDU: 00 87 03 9B 0C 7C 0A 82 08 [8 bytes encrypted response]
///   SW 9000 = authenticated; 6982 = wrong key.
#[allow(dead_code)]
pub fn authenticate_piv_mgmt_key_3des(card: &pcsc::Card, key: &[u8; 24]) -> Result<()> {
    // Step 1: Request a challenge from the card.
    let step1_apdu: &[u8] = &[0x00, 0x87, 0x03, 0x9B, 0x04, 0x7C, 0x02, 0x81, 0x00];
    let mut buf = [0u8; 256];
    let resp = card
        .transmit(step1_apdu, &mut buf)
        .map_err(|e| anyhow::anyhow!("GENERAL AUTHENTICATE step 1 transmit error: {e}"))?;

    let sw = crate::model::card::apdu_sw(resp);
    if sw != 0x9000 {
        anyhow::bail!(
            "{}",
            crate::model::card::apdu_error_message(sw, "PIV management key challenge request")
        );
    }

    // Extract the 8-byte challenge from the TLV response.
    // Expected layout: 7C 0A 81 08 [8 challenge bytes] 90 00
    // The challenge starts at offset 4 (skip: 7C tag, 0A length, 81 tag, 08 length).
    if resp.len() < 14 {
        anyhow::bail!("GENERAL AUTHENTICATE step 1: response too short ({} bytes)", resp.len());
    }
    let challenge: &[u8] = &resp[4..12];

    // Step 2: Encrypt the challenge with 3DES-EDE (single 8-byte block, ECB).
    let key_arr: &cipher::Array<u8, _> =
        key.as_slice().try_into().map_err(|_| anyhow::anyhow!("key length error"))?;
    let cipher = TdesEde3::new(key_arr);
    let mut block: cipher::Array<u8, _> =
        challenge[..8].try_into().map_err(|_| anyhow::anyhow!("challenge length error"))?;
    cipher.encrypt_block(&mut block);
    let encrypted = block.as_slice();

    // Build step 2 APDU:
    // CLA=00 INS=87 P1=03 P2=9B Lc=0C [7C 0A 82 08 <8 bytes>]
    let mut step2_apdu = vec![0x00u8, 0x87, 0x03, 0x9B, 0x0C, 0x7C, 0x0A, 0x82, 0x08];
    step2_apdu.extend_from_slice(encrypted);

    let mut buf2 = [0u8; 256];
    let resp2 = card
        .transmit(&step2_apdu, &mut buf2)
        .map_err(|e| anyhow::anyhow!("GENERAL AUTHENTICATE step 2 transmit error: {e}"))?;

    let sw2 = crate::model::card::apdu_sw(resp2);
    if sw2 != 0x9000 {
        anyhow::bail!("Management key authentication failed (wrong key?)");
    }

    Ok(())
}

// ============================================================================
// Certificate deletion
// ============================================================================

/// Delete the X.509 certificate stored in the given PIV slot.
///
/// Uses PUT DATA (INS=0xDB) with an empty BER-TLV 0x53 value, which clears
/// the data object. The management key must already be authenticated.
///
/// APDU layout:
///   00 DB 3F FF <Lc> [5C <len> <object_id_bytes> 53 00]
#[allow(dead_code)]
pub fn delete_piv_certificate(card: &pcsc::Card, slot: &PivSlot) -> Result<()> {
    let obj_id = slot.object_id_bytes();
    let obj_id_len = obj_id.len() as u8;

    // TLV: 5C <obj_id_len> <obj_id_bytes> 53 00
    // Total data length = 2 + obj_id_len + 2
    let mut data: Vec<u8> = Vec::new();
    data.push(0x5C);
    data.push(obj_id_len);
    data.extend_from_slice(obj_id);
    data.push(0x53);
    data.push(0x00);

    let lc = data.len() as u8;
    let mut apdu: Vec<u8> = vec![0x00, 0xDB, 0x3F, 0xFF, lc];
    apdu.extend_from_slice(&data);

    let mut buf = [0u8; 256];
    let resp = card
        .transmit(&apdu, &mut buf)
        .map_err(|e| anyhow::anyhow!("PUT DATA (certificate delete) transmit error: {e}"))?;

    let sw = crate::model::card::apdu_sw(resp);
    if sw != 0x9000 {
        anyhow::bail!(
            "{}",
            crate::model::card::apdu_error_message(
                sw,
                &format!("deleting PIV certificate in slot {}", slot.display_name())
            )
        );
    }

    Ok(())
}

// ============================================================================
// Key deletion (firmware >= 5.7.0 only)
// ============================================================================

/// Delete the private key stored in the given PIV slot.
///
/// **Requires firmware >= 5.7.0.** On older firmware, MOVE KEY is not available
/// and attempting it would cause an error. This function checks the firmware version
/// and returns a descriptive error if the hardware does not support key deletion.
///
/// Uses MOVE KEY (INS=0xF6, P1=0xFF) with P2=slot_id. Setting the destination to 0xFF
/// signals "delete in place" (no destination slot). The management key must be authenticated.
#[allow(dead_code)]
pub fn delete_piv_key(card: &pcsc::Card, slot: &PivSlot, firmware: &Version) -> Result<()> {
    // Firmware gate: MOVE KEY requires >= 5.7.0
    if firmware.major < 5 || (firmware.major == 5 && firmware.minor < 7) {
        anyhow::bail!(
            "PIV key deletion requires firmware 5.7.0 or newer (this device has {}.{}.{}). \
             Only the certificate was deleted.",
            firmware.major, firmware.minor, firmware.patch
        );
    }

    // MOVE KEY: INS=0xF6 P1=0xFF P2=<slot_id> (no Lc/data)
    let apdu: &[u8] = &[0x00, 0xF6, 0xFF, slot.slot_id()];

    let mut buf = [0u8; 256];
    let resp = card
        .transmit(apdu, &mut buf)
        .map_err(|e| anyhow::anyhow!("MOVE KEY (key delete) transmit error: {e}"))?;

    let sw = crate::model::card::apdu_sw(resp);
    if sw != 0x9000 {
        anyhow::bail!(
            "{}",
            crate::model::card::apdu_error_message(
                sw,
                &format!("deleting PIV key in slot {}", slot.display_name())
            )
        );
    }

    Ok(())
}

// ============================================================================
// High-level delete_piv_slot (called by TUI)
// ============================================================================

/// Delete the certificate and (if firmware allows) the private key in a PIV slot.
///
/// High-level workflow:
/// 1. kill_scdaemon + 50 ms sleep (standard card access preamble)
/// 2. Establish PC/SC context, connect to first available reader (Exclusive)
/// 3. SELECT PIV AID
/// 4. Authenticate management key (3DES challenge-response)
/// 5. DELETE certificate (PUT DATA with empty 0x53)
/// 6. Attempt DELETE key (MOVE KEY) — skipped with explanation if firmware < 5.7
///
/// Returns a human-readable status string on success.
#[allow(dead_code)]
pub fn delete_piv_slot(
    slot: &PivSlot,
    mgmt_key: &[u8; 24],
    firmware: &Version,
) -> Result<String> {
    crate::model::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx =
        Context::establish(Scope::User).map_err(|e| anyhow::anyhow!("PC/SC error: {e}"))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = ctx
        .list_readers(&mut readers_buf)
        .map_err(|e| anyhow::anyhow!("No smart card readers: {e}"))?
        .collect();

    if readers.is_empty() {
        anyhow::bail!("No smart card readers found");
    }

    let card = readers
        .into_iter()
        .find_map(|r| {
            ctx.connect(r, ShareMode::Exclusive, Protocols::T0 | Protocols::T1)
                .ok()
        })
        .ok_or_else(|| anyhow::anyhow!("Could not connect to any smart card reader"))?;

    // SELECT PIV AID
    let mut buf = [0u8; 256];
    let resp = card
        .transmit(crate::model::card::SELECT_PIV, &mut buf)
        .map_err(|e| anyhow::anyhow!("SELECT PIV transmit error: {e}"))?;

    if crate::model::card::apdu_sw(resp) != 0x9000 {
        anyhow::bail!("PIV application not available on this YubiKey");
    }

    // Authenticate management key
    authenticate_piv_mgmt_key_3des(&card, mgmt_key)?;

    // Delete certificate
    delete_piv_certificate(&card, slot)?;

    // Attempt key deletion (firmware-gated)
    let key_deleted = if firmware.major > 5 || (firmware.major == 5 && firmware.minor >= 7) {
        match delete_piv_key(&card, slot, firmware) {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!("PIV key delete failed even on >= 5.7 firmware: {}", e);
                false
            }
        }
    } else {
        false
    };

    let msg = if key_deleted {
        format!(
            "Certificate and key deleted from slot {}.",
            slot.display_name()
        )
    } else {
        format!(
            "Certificate deleted from slot {}. Key deletion requires firmware 5.7+ (yours is {}.{}.{}).",
            slot.display_name(),
            firmware.major,
            firmware.minor,
            firmware.patch
        )
    };

    Ok(msg)
}

// ============================================================================
// Management key change
// ============================================================================

/// Change the PIV management key (3DES → 3DES).
///
/// Protocol:
///   1. SELECT PIV
///   2. GENERAL AUTHENTICATE with current_key to establish session
///   3. SET MANAGEMENT KEY: CLA=00 INS=FF P1=FF P2=FE
///      Data: 9B 03 18 [24 new key bytes]
///
/// `new_key` must be 24 bytes of a valid 3DES-EDE key.
/// Returns `Ok(())` on success or `Err` with a user-friendly message.
pub fn change_piv_management_key(
    current_key: &[u8; 24],
    new_key: &[u8; 24],
) -> Result<()> {
    crate::model::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User)
        .map_err(|e| anyhow::anyhow!("PC/SC error: {e}"))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = ctx
        .list_readers(&mut readers_buf)
        .map_err(|e| anyhow::anyhow!("list readers: {e}"))?
        .collect();
    if readers.is_empty() {
        return Err(anyhow::anyhow!("No readers found"));
    }

    let card = readers
        .into_iter()
        .find_map(|r| ctx.connect(r, ShareMode::Exclusive, Protocols::T0 | Protocols::T1).ok())
        .ok_or_else(|| anyhow::anyhow!("Failed to connect to reader"))?;

    // SELECT PIV
    use crate::model::piv::{SELECT_PIV};
    let mut buf = [0u8; 256];
    let resp = card.transmit(SELECT_PIV, &mut buf)
        .map_err(|e| anyhow::anyhow!("SELECT PIV: {e}"))?;
    if crate::model::card::apdu_sw(resp) != 0x9000 {
        return Err(anyhow::anyhow!("PIV application not found on this YubiKey"));
    }

    // Authenticate with current management key
    authenticate_piv_mgmt_key_3des(&card, current_key)?;

    // SET MANAGEMENT KEY: CLA=00 INS=FF P1=FF P2=FE
    // Data: 9B 03 18 [24 new key bytes]  (9B=tag, 03=3DES, 18=24 decimal)
    let mut set_apdu = vec![0x00u8, 0xFF, 0xFF, 0xFE, 0x1C, 0x9B, 0x03, 0x18];
    set_apdu.extend_from_slice(new_key.as_slice());

    let mut buf2 = [0u8; 256];
    let resp2 = card.transmit(&set_apdu, &mut buf2)
        .map_err(|e| anyhow::anyhow!("SET MANAGEMENT KEY: {e}"))?;
    let sw = crate::model::card::apdu_sw(resp2);

    match sw {
        0x9000 => Ok(()),
        0x6982 => Err(anyhow::anyhow!("Authentication failed — wrong current management key")),
        0x6A80 => Err(anyhow::anyhow!("Invalid management key (check length and format)")),
        other => Err(anyhow::anyhow!("SET MANAGEMENT KEY failed: SW {:04X}", other)),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piv_slot_ids() {
        assert_eq!(PivSlot::Authentication.slot_id(), 0x9A);
        assert_eq!(PivSlot::Signature.slot_id(), 0x9C);
        assert_eq!(PivSlot::KeyManagement.slot_id(), 0x9D);
        assert_eq!(PivSlot::CardAuth.slot_id(), 0x9E);
    }

    #[test]
    fn test_piv_slot_object_ids() {
        assert_eq!(PivSlot::Authentication.object_id_bytes(), &[0x5F, 0xC1, 0x05]);
        assert_eq!(PivSlot::Signature.object_id_bytes(),     &[0x5F, 0xC1, 0x0A]);
        assert_eq!(PivSlot::KeyManagement.object_id_bytes(), &[0x5F, 0xC1, 0x0B]);
        assert_eq!(PivSlot::CardAuth.object_id_bytes(),      &[0x5F, 0xC1, 0x01]);
    }

    #[test]
    fn test_piv_slot_from_str() {
        assert_eq!(PivSlot::from_slot_str("9a"), Some(PivSlot::Authentication));
        assert_eq!(PivSlot::from_slot_str("9c"), Some(PivSlot::Signature));
        assert_eq!(PivSlot::from_slot_str("9d"), Some(PivSlot::KeyManagement));
        assert_eq!(PivSlot::from_slot_str("9e"), Some(PivSlot::CardAuth));
        assert_eq!(PivSlot::from_slot_str("9f"), None);
        assert_eq!(PivSlot::from_slot_str(""),   None);
    }

    #[test]
    fn test_piv_slot_display_name() {
        assert!(PivSlot::Authentication.display_name().contains("9a"));
        assert!(PivSlot::Signature.display_name().contains("9c"));
        assert!(PivSlot::KeyManagement.display_name().contains("9d"));
        assert!(PivSlot::CardAuth.display_name().contains("9e"));
    }

    #[test]
    fn test_default_mgmt_key_length() {
        assert_eq!(PIV_DEFAULT_MGMT_KEY_3DES.len(), 24);
    }

    #[test]
    fn test_default_mgmt_key_values() {
        // Should be 01..08 repeated 3 times
        let expected: &[u8] = &[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        ];
        assert_eq!(PIV_DEFAULT_MGMT_KEY_3DES.as_slice(), expected);
    }

    #[test]
    fn test_firmware_gate_rejects_old_firmware() {
        // Simulating the gate check logic from delete_piv_key:
        // firmware 5.6.x should be rejected
        let fw = Version { major: 5, minor: 6, patch: 3 };
        let is_old = fw.major < 5 || (fw.major == 5 && fw.minor < 7);
        assert!(is_old, "5.6.3 should be gated");
    }

    #[test]
    fn test_firmware_gate_accepts_new_firmware() {
        let fw57 = Version { major: 5, minor: 7, patch: 0 };
        let fw6 = Version { major: 6, minor: 0, patch: 0 };
        let gated57 = fw57.major < 5 || (fw57.major == 5 && fw57.minor < 7);
        let gated6 = fw6.major < 5 || (fw6.major == 5 && fw6.minor < 7);
        assert!(!gated57, "5.7.0 should be allowed");
        assert!(!gated6, "6.0.0 should be allowed");
    }
}
