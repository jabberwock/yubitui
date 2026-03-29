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

/// Read OTP slot status using an already-connected exclusive PC/SC card handle.
///
/// Use this from the detection loop where an exclusive card connection is already held.
/// Selects the OTP AID on the existing card handle and reads status without
/// creating a new context (which would conflict with the outer exclusive connection).
pub fn get_otp_slot_status_from_card(card: &pcsc::Card) -> Result<OtpState> {
    let mut buf = [0u8; 256];
    let resp = card.transmit(SELECT_OTP, &mut buf).unwrap_or(&[0x6A, 0x82]);
    if super::card::apdu_sw(resp) != 0x9000 {
        anyhow::bail!("OTP application not available (SW 0x{:04X})", super::card::apdu_sw(resp));
    }

    let mut resp_buf = [0u8; 64];
    let resp = card
        .transmit(READ_OTP_STATUS, &mut resp_buf)
        .map_err(|e| anyhow::anyhow!("READ_OTP_STATUS transmit error: {e}"))?;
    if super::card::apdu_sw(resp) != 0x9000 {
        anyhow::bail!("READ_OTP_STATUS failed (SW 0x{:04X})", super::card::apdu_sw(resp));
    }
    if resp.len() < 6 {
        anyhow::bail!("READ_OTP_STATUS response too short: {} bytes", resp.len());
    }

    parse_otp_status(resp)
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
