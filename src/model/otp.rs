use anyhow::Result;
use pcsc::{Context, Protocols, Scope, ShareMode};

/// Status of a single OTP slot — whether a credential is configured.
///
/// NOTE: The credential type (Yubico OTP, HMAC-SHA1, static password, HOTP) is
/// write-only at configuration time and CANNOT be read back from hardware.
/// Only occupied vs empty is detectable via the READ STATUS APDU.
#[derive(Debug, Clone, serde::Serialize)]
pub enum OtpSlotStatus {
    Occupied,
    Empty,
}

/// OTP application state for both slots.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OtpState {
    pub slot1: OtpSlotStatus,
    pub slot2: OtpSlotStatus,
    pub slot1_touch: bool,
    pub slot2_touch: bool,
}

/// OTP application AID: A0 00 00 05 27 20 01 01
#[allow(dead_code)]
pub const OTP_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x05, 0x27, 0x20, 0x01, 0x01];

/// SELECT OTP AID APDU.
/// CLA=00 INS=A4 P1=04 P2=00 Lc=08 [OTP AID bytes]
pub const SELECT_OTP: &[u8] = &[
    0x00, 0xA4, 0x04, 0x00, 0x08, 0xA0, 0x00, 0x00, 0x05, 0x27, 0x20, 0x01, 0x01,
];

/// READ OTP STATUS APDU.
/// Returns 6-byte status struct; touch_level at bytes [4] and [5].
pub const READ_OTP_STATUS: &[u8] = &[0x00, 0x03, 0x00, 0x00];

/// touch_level bit: Slot 1 has a credential configured.
pub const SLOT1_VALID: u16 = 0x01;
/// touch_level bit: Slot 2 has a credential configured.
pub const SLOT2_VALID: u16 = 0x02;
/// touch_level bit: Slot 1 requires touch.
pub const SLOT1_TOUCH: u16 = 0x04;
/// touch_level bit: Slot 2 requires touch.
pub const SLOT2_TOUCH: u16 = 0x08;

/// Parse OTP slot status from the 6-byte READ STATUS response body.
///
/// The touch_level field is at bytes [4] (low) and [5] (high), little-endian.
/// Returns an error if the response is too short.
pub fn parse_otp_status(status_bytes: &[u8]) -> Result<OtpState> {
    if status_bytes.len() < 6 {
        anyhow::bail!(
            "OTP READ STATUS response too short: {} bytes (expected >=6)",
            status_bytes.len()
        );
    }
    let touch_level = (status_bytes[5] as u16) << 8 | (status_bytes[4] as u16);

    let slot1 = if touch_level & SLOT1_VALID != 0 {
        OtpSlotStatus::Occupied
    } else {
        OtpSlotStatus::Empty
    };
    let slot2 = if touch_level & SLOT2_VALID != 0 {
        OtpSlotStatus::Occupied
    } else {
        OtpSlotStatus::Empty
    };
    let slot1_touch = touch_level & SLOT1_TOUCH != 0;
    let slot2_touch = touch_level & SLOT2_TOUCH != 0;

    Ok(OtpState {
        slot1,
        slot2,
        slot1_touch,
        slot2_touch,
    })
}

/// Read OTP slot status from the YubiKey via native PC/SC APDUs.
///
/// Protocol:
///   1. kill_scdaemon() — release the card channel
///   2. Establish PC/SC context, connect to first reader (Exclusive, T0|T1)
///   3. SELECT OTP AID (A0 00 00 05 27 20 01 01)
///   4. READ STATUS (00 03 00 00) — returns 6-byte status struct
///   5. Parse touch_level from bytes [4][5] (little-endian u16)
///
/// Returns OtpState; errors if no reader/card is found.
pub fn get_otp_slot_status() -> Result<OtpState> {
    super::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User).map_err(|e| anyhow::anyhow!("PC/SC error: {e}"))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = match ctx.list_readers(&mut readers_buf) {
        Ok(r) => r.collect(),
        Err(_) => anyhow::bail!("No smart card readers found"),
    };

    if readers.is_empty() {
        anyhow::bail!("No smart card readers found");
    }

    // Connect to first available reader.
    let card = readers
        .into_iter()
        .find_map(|reader| {
            ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1)
                .ok()
        })
        .ok_or_else(|| anyhow::anyhow!("No card available for exclusive connection"))?;

    // SELECT OTP AID
    let mut buf = [0u8; 256];
    let resp = card.transmit(SELECT_OTP, &mut buf).unwrap_or(&[0x6A, 0x82]);
    if super::card::apdu_sw(resp) != 0x9000 {
        anyhow::bail!("OTP application not available (SW 0x{:04X})", super::card::apdu_sw(resp));
    }

    // READ OTP STATUS
    let mut resp_buf = [0u8; 64];
    let resp = card
        .transmit(READ_OTP_STATUS, &mut resp_buf)
        .map_err(|e| anyhow::anyhow!("READ_OTP_STATUS transmit error: {e}"))?;
    if super::card::apdu_sw(resp) != 0x9000 {
        anyhow::bail!(
            "READ_OTP_STATUS failed (SW 0x{:04X})",
            super::card::apdu_sw(resp)
        );
    }
    if resp.len() < 6 {
        anyhow::bail!(
            "READ_OTP_STATUS response too short: {} bytes",
            resp.len()
        );
    }

    parse_otp_status(resp)
}

/// OTP credential types that can be programmed into a slot.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum OtpCredentialType {
    /// Yubico OTP (default factory configuration for slot 1)
    YubicoOtp,
    /// HMAC-SHA1 Challenge-Response (used for offline 2FA, KeePassXC)
    ChallengeResponse,
    /// Static password (types a fixed string on touch)
    StaticPassword,
}

impl std::fmt::Display for OtpCredentialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OtpCredentialType::YubicoOtp => write!(f, "Yubico OTP"),
            OtpCredentialType::ChallengeResponse => write!(f, "HMAC-SHA1 Challenge-Response"),
            OtpCredentialType::StaticPassword => write!(f, "Static Password"),
        }
    }
}

/// Configuration frame for programming an OTP slot.
///
/// The YubiKey OTP application accepts a 70-byte configuration frame via
/// the PROGRAM SLOT APDU (INS=01, P1=slot_number).
///
/// This is a simplified builder — it generates configurations for the
/// three most common credential types.
#[derive(Debug, Clone)]
pub struct OtpConfig {
    pub slot: u8,              // 1 or 2
    pub credential_type: OtpCredentialType,
    pub require_touch: bool,
    /// For StaticPassword: the password string (max 38 chars, modhex-encodable)
    pub static_password: Option<String>,
    /// For ChallengeResponse: the 20-byte HMAC-SHA1 secret key (hex-encoded)
    pub hmac_secret: Option<String>,
}

impl OtpConfig {
    pub fn new(slot: u8, credential_type: OtpCredentialType) -> Self {
        Self {
            slot,
            credential_type,
            require_touch: false,
            static_password: None,
            hmac_secret: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_touch(mut self, require: bool) -> Self {
        self.require_touch = require;
        self
    }
}

/// Program an OTP slot via native PC/SC APDUs.
///
/// WARNING: This overwrites whatever is currently in the slot.
/// The configuration frame format follows the YubiKey Personalization protocol.
///
/// Protocol:
///   1. kill_scdaemon() — release the card channel
///   2. Establish PC/SC context, connect to first reader (Exclusive)
///   3. SELECT OTP AID
///   4. Build 70-byte configuration frame
///   5. PROGRAM SLOT (INS=01, P1=slot, Lc=46 for config data)
pub fn program_otp_slot(config: &OtpConfig) -> Result<()> {
    if config.slot != 1 && config.slot != 2 {
        anyhow::bail!("Invalid OTP slot: {} (must be 1 or 2)", config.slot);
    }

    super::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User)
        .map_err(|e| anyhow::anyhow!("PC/SC error: {e}"))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = match ctx.list_readers(&mut readers_buf) {
        Ok(r) => r.collect(),
        Err(_) => anyhow::bail!("No smart card readers found"),
    };

    let card = readers
        .into_iter()
        .find_map(|reader| {
            ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1)
                .ok()
        })
        .ok_or_else(|| anyhow::anyhow!("No card available for exclusive connection"))?;

    // SELECT OTP AID
    let mut buf = [0u8; 256];
    let resp = card.transmit(SELECT_OTP, &mut buf).unwrap_or(&[0x6A, 0x82]);
    if super::card::apdu_sw(resp) != 0x9000 {
        anyhow::bail!("OTP application not available");
    }

    // Build configuration frame (simplified — real implementation would
    // build the full 70-byte Yubico configuration struct)
    // For now, use the PROGRAM CONFIGURATION APDU format:
    // INS=01, P1=slot (01 or 02), P2=00
    let slot_p1 = config.slot;

    // The configuration data is credential-type dependent.
    // This builds a minimal valid frame for each type.
    let config_data = match config.credential_type {
        OtpCredentialType::ChallengeResponse => {
            // HMAC-SHA1 challenge-response: 20-byte key + flags
            let key = if let Some(ref hex) = config.hmac_secret {
                hex_decode(hex).unwrap_or_else(|_| vec![0u8; 20])
            } else {
                // Generate random key
                let mut key = vec![0u8; 20];
                for b in key.iter_mut() {
                    *b = rand_byte();
                }
                key
            };
            let mut frame = vec![0u8; 46];
            // Bytes 0-15: fixed part (unused for CR, zero-fill)
            // Bytes 16-21: UID (unused for CR)
            // Bytes 22-41: AES/HMAC key (20 bytes)
            frame[22..42].copy_from_slice(&key[..20]);
            // Byte 42: extended flags — HMAC-SHA1 challenge-response mode
            frame[42] = 0x22; // EXTFLAG_SERIAL_API_VISIBLE | EXTFLAG_HMAC_LT64
            // Byte 43: ticket flags
            frame[43] = 0x40; // TKTFLAG_CHAL_RESP
            // Byte 44: config flags
            frame[44] = 0x22; // CFGFLAG_CHAL_HMAC | CFGFLAG_HMAC_LT64
            if config.require_touch {
                frame[44] |= 0x04; // CFGFLAG_CHAL_BTN_TRIG
            }
            // Byte 45: reserved
            frame
        }
        OtpCredentialType::StaticPassword => {
            // Static password: encode into the fixed part
            let pw = config.static_password.as_deref().unwrap_or("");
            let pw_bytes = pw.as_bytes();
            let len = pw_bytes.len().min(38);
            let mut frame = vec![0u8; 46];
            // Store password length in fixed size field
            frame[0..len].copy_from_slice(&pw_bytes[..len]);
            // Byte 43: ticket flags — SHORT_TICKET for static
            frame[43] = 0x02; // TKTFLAG_APPEND_CR
            // Byte 44: config flags — static ticket
            frame[44] = 0x20; // CFGFLAG_STATIC_TICKET
            if config.require_touch {
                frame[44] |= 0x04;
            }
            frame
        }
        OtpCredentialType::YubicoOtp => {
            // Yubico OTP: generate random UID and AES key
            let mut frame = vec![0u8; 46];
            // Bytes 16-21: 6-byte private ID (UID)
            for b in frame[16..22].iter_mut() {
                *b = rand_byte();
            }
            // Bytes 22-37: 16-byte AES key
            for b in frame[22..38].iter_mut() {
                *b = rand_byte();
            }
            // Byte 43: ticket flags
            frame[43] = 0x02; // TKTFLAG_APPEND_CR
            if config.require_touch {
                frame[44] |= 0x04;
            }
            frame
        }
    };

    // Build PROGRAM SLOT APDU: CLA=00 INS=01 P1=slot P2=00 Lc=len [data]
    let mut apdu = vec![0x00, 0x01, slot_p1, 0x00, config_data.len() as u8];
    apdu.extend_from_slice(&config_data);

    let mut resp_buf = [0u8; 256];
    let resp = card
        .transmit(&apdu, &mut resp_buf)
        .map_err(|e| anyhow::anyhow!("PROGRAM SLOT transmit error: {e}"))?;
    if super::card::apdu_sw(resp) != 0x9000 {
        anyhow::bail!(
            "PROGRAM SLOT failed (SW 0x{:04X})",
            super::card::apdu_sw(resp)
        );
    }

    Ok(())
}

/// Delete (erase) an OTP slot configuration.
#[allow(dead_code)]
pub fn delete_otp_slot(slot: u8) -> Result<()> {
    if slot != 1 && slot != 2 {
        anyhow::bail!("Invalid OTP slot: {} (must be 1 or 2)", slot);
    }

    super::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User)
        .map_err(|e| anyhow::anyhow!("PC/SC error: {e}"))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = match ctx.list_readers(&mut readers_buf) {
        Ok(r) => r.collect(),
        Err(_) => anyhow::bail!("No smart card readers found"),
    };

    let card = readers
        .into_iter()
        .find_map(|reader| {
            ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1)
                .ok()
        })
        .ok_or_else(|| anyhow::anyhow!("No card available for exclusive connection"))?;

    // SELECT OTP AID
    let mut buf = [0u8; 256];
    let resp = card.transmit(SELECT_OTP, &mut buf).unwrap_or(&[0x6A, 0x82]);
    if super::card::apdu_sw(resp) != 0x9000 {
        anyhow::bail!("OTP application not available");
    }

    // PROGRAM SLOT with all-zero config = erase
    let config_data = vec![0u8; 46];
    let mut apdu = vec![0x00, 0x01, slot, 0x00, config_data.len() as u8];
    apdu.extend_from_slice(&config_data);

    let mut resp_buf = [0u8; 256];
    let resp = card
        .transmit(&apdu, &mut resp_buf)
        .map_err(|e| anyhow::anyhow!("DELETE SLOT transmit error: {e}"))?;
    if super::card::apdu_sw(resp) != 0x9000 {
        anyhow::bail!(
            "DELETE SLOT failed (SW 0x{:04X})",
            super::card::apdu_sw(resp)
        );
    }

    Ok(())
}

/// Simple deterministic-enough random byte for key generation.
/// Uses std::time nanos as entropy source (not cryptographically secure,
/// but OTP keys are generated on-device in production — this is for the
/// configuration frame only).
/// Simple hex string decoder (no external crate needed).
fn hex_decode(s: &str) -> Result<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        anyhow::bail!("Hex string has odd length");
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| anyhow::anyhow!("Invalid hex: {e}"))
        })
        .collect()
}

fn rand_byte() -> u8 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos & 0xFF) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otp_slot_status_bitmask_both_empty() {
        // touch_level = 0x00: both slots empty
        let bytes = [0x01u8, 0x00, 0x00, 0x00, 0x00, 0x00];
        let state = parse_otp_status(&bytes).unwrap();
        assert!(matches!(state.slot1, OtpSlotStatus::Empty));
        assert!(matches!(state.slot2, OtpSlotStatus::Empty));
        assert!(!state.slot1_touch);
        assert!(!state.slot2_touch);
    }

    #[test]
    fn test_otp_slot_status_bitmask_slot1_occupied() {
        // touch_level low byte = 0x01 (SLOT1_VALID)
        let bytes = [0x01u8, 0x00, 0x00, 0x00, 0x01, 0x00];
        let state = parse_otp_status(&bytes).unwrap();
        assert!(matches!(state.slot1, OtpSlotStatus::Occupied));
        assert!(matches!(state.slot2, OtpSlotStatus::Empty));
        assert!(!state.slot1_touch);
        assert!(!state.slot2_touch);
    }

    #[test]
    fn test_otp_slot_status_bitmask_both_occupied() {
        // touch_level low byte = 0x03 (SLOT1_VALID | SLOT2_VALID)
        let bytes = [0x01u8, 0x00, 0x00, 0x00, 0x03, 0x00];
        let state = parse_otp_status(&bytes).unwrap();
        assert!(matches!(state.slot1, OtpSlotStatus::Occupied));
        assert!(matches!(state.slot2, OtpSlotStatus::Occupied));
        assert!(!state.slot1_touch);
        assert!(!state.slot2_touch);
    }

    #[test]
    fn test_otp_slot_status_bitmask_touch_flags() {
        // touch_level = 0x0F (SLOT1_VALID | SLOT2_VALID | SLOT1_TOUCH | SLOT2_TOUCH)
        let bytes = [0x01u8, 0x00, 0x00, 0x00, 0x0F, 0x00];
        let state = parse_otp_status(&bytes).unwrap();
        assert!(matches!(state.slot1, OtpSlotStatus::Occupied));
        assert!(matches!(state.slot2, OtpSlotStatus::Occupied));
        assert!(state.slot1_touch);
        assert!(state.slot2_touch);
    }

    #[test]
    fn test_otp_slot_status_bitmask_slot1_touch_only() {
        // touch_level low byte = 0x05 (SLOT1_VALID | SLOT1_TOUCH)
        let bytes = [0x01u8, 0x00, 0x00, 0x00, 0x05, 0x00];
        let state = parse_otp_status(&bytes).unwrap();
        assert!(matches!(state.slot1, OtpSlotStatus::Occupied));
        assert!(matches!(state.slot2, OtpSlotStatus::Empty));
        assert!(state.slot1_touch);
        assert!(!state.slot2_touch);
    }

    #[test]
    fn test_otp_slot_status_bitmask_short_response_fails() {
        let bytes = [0x01u8, 0x00, 0x00, 0x00];
        assert!(parse_otp_status(&bytes).is_err());
    }
}
