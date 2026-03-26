use anyhow::Result;
use pcsc::{Context, Protocols, Scope, ShareMode};

#[derive(Debug, Clone)]
pub struct PivState {
    #[allow(dead_code)]
    pub slots: Vec<SlotInfo>,
}

#[derive(Debug, Clone)]
pub struct SlotInfo {
    #[allow(dead_code)]
    pub slot: String,
    #[allow(dead_code)]
    pub algorithm: Option<String>,
    #[allow(dead_code)]
    pub subject: Option<String>,
}

/// PIV application AID.
#[allow(dead_code)]
pub const PIV_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x03, 0x08, 0x00, 0x00, 0x10, 0x00];

/// SELECT PIV AID APDU.
/// CLA=00 INS=A4 P1=04 P2=00 Lc=09 [PIV AID bytes]
#[allow(dead_code)]
pub const SELECT_PIV: &[u8] = &[
    0x00, 0xA4, 0x04, 0x00, 0x09,
    0xA0, 0x00, 0x00, 0x03, 0x08, 0x00, 0x00, 0x10, 0x00, 0x01,
];

/// Detect which PIV slots have a key present using native PC/SC APDUs.
///
/// Protocol:
///   1. kill_scdaemon() — release the card channel
///   2. Establish PC/SC context, connect to first reader (Exclusive)
///   3. SELECT PIV AID
///   4. For each PIV slot, send GET DATA and check SW
///   5. SW 0x9000 → occupied; SW 0x6A82 → empty; other → skip
///
/// Returns PivState with empty slots if no PIV-capable card is found.
pub fn get_piv_state() -> Result<PivState> {
    super::card::kill_scdaemon();

    let ctx = Context::establish(Scope::User)
        .map_err(|e| anyhow::anyhow!("PC/SC error: {e}"))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = match ctx.list_readers(&mut readers_buf) {
        Ok(r) => r.collect(),
        Err(_) => return Ok(PivState { slots: vec![] }),
    };

    if readers.is_empty() {
        return Ok(PivState { slots: vec![] });
    }

    // Connect to first available reader
    let card = match readers.into_iter().find_map(|reader| {
        ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1).ok()
    }) {
        Some(c) => c,
        None => return Ok(PivState { slots: vec![] }),
    };

    // SELECT PIV AID
    let mut buf = [0u8; 256];
    let resp = card
        .transmit(SELECT_PIV, &mut buf)
        .unwrap_or(&[0x6A, 0x82]);
    if super::card::apdu_sw(resp) != 0x9000 {
        // PIV application not available (best-effort per D-14)
        return Ok(PivState { slots: vec![] });
    }

    // PIV GET DATA APDUs per slot
    // Format: CLA=00 INS=CB P1=3F P2=FF Lc=05 5C 03 5F C1 XX
    // where XX is the slot object ID byte
    let piv_slots: &[(&str, [u8; 10])] = &[
        ("9a", [0x00, 0xCB, 0x3F, 0xFF, 0x05, 0x5C, 0x03, 0x5F, 0xC1, 0x05]),
        ("9c", [0x00, 0xCB, 0x3F, 0xFF, 0x05, 0x5C, 0x03, 0x5F, 0xC1, 0x0A]),
        ("9d", [0x00, 0xCB, 0x3F, 0xFF, 0x05, 0x5C, 0x03, 0x5F, 0xC1, 0x0B]),
        ("9e", [0x00, 0xCB, 0x3F, 0xFF, 0x05, 0x5C, 0x03, 0x5F, 0xC1, 0x01]),
    ];

    let mut slots = Vec::new();
    let mut resp_buf = [0u8; 4096];

    for (slot_name, apdu) in piv_slots {
        let resp = match card.transmit(apdu.as_slice(), &mut resp_buf) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let sw = super::card::apdu_sw(resp);
        if sw == 0x9000 {
            slots.push(SlotInfo {
                slot: slot_name.to_string(),
                algorithm: None,
                subject: None,
            });
        }
        // SW 0x6A82 = empty slot (skip); other SWs = skip
    }

    Ok(PivState { slots })
}

/// Parse PIV state from `ykman piv info` output.
///
/// Kept with `#[allow(dead_code)]` so existing unit tests remain valid.
#[allow(dead_code)]
pub fn parse_piv_info(output: &str) -> PivState {
    let mut slots = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        // Look for slot information like "Slot 9a:"
        if line.starts_with("Slot ") {
            if let Some(slot_id) = line.split(':').next() {
                let slot_name = slot_id.trim_start_matches("Slot ").to_string();

                slots.push(SlotInfo {
                    slot: slot_name,
                    algorithm: None,
                    subject: None,
                });
            }
        }
    }

    PivState { slots }
}

/// Check whether a PIV GET DATA response indicates a slot is occupied.
///
/// SW 0x9000 = data present (occupied), SW 0x6A82 = not found (empty).
#[allow(dead_code)]
pub fn parse_piv_slot_presence(resp: &[u8]) -> bool {
    super::card::apdu_sw(resp) == 0x9000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_piv_info_with_slots() {
        let input = "Slot 9a:\n  Algorithm: RSA2048\nSlot 9c:\n  Algorithm: ECCP256\n";
        let state = parse_piv_info(input);
        assert_eq!(state.slots.len(), 2);
        assert_eq!(state.slots[0].slot, "9a");
        assert_eq!(state.slots[1].slot, "9c");
    }

    #[test]
    fn test_parse_piv_info_empty() {
        let state = parse_piv_info("");
        assert!(state.slots.is_empty());
    }

    #[test]
    fn test_parse_piv_slot_presence_occupied() {
        // SW 0x9000 with some data — slot occupied
        let resp = [0xABu8, 0xCD, 0x90, 0x00];
        assert!(parse_piv_slot_presence(&resp));
    }

    #[test]
    fn test_parse_piv_slot_presence_not_found() {
        // SW 0x6A82 — slot empty (data object not found)
        let resp = [0x6Au8, 0x82];
        assert!(!parse_piv_slot_presence(&resp));
    }
}
