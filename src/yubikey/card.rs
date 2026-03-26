use anyhow::Result;

/// OpenPGP application AID prefix (6 bytes).
#[allow(dead_code)]
pub const OPENPGP_AID: &[u8] = &[0xD2, 0x76, 0x00, 0x01, 0x24, 0x01];

/// Full SELECT OpenPGP APDU: CLA=00 INS=A4 P1=04 P2=00 Lc=06 [AID].
/// No Le byte — matches gpg/scdaemon SELECT behavior and avoids leaving a
/// pending response buffer that corrupts subsequent GET DATA operations on
/// some YubiKey firmware versions.
#[allow(dead_code)]
pub const SELECT_OPENPGP: &[u8] = &[
    0x00, 0xA4, 0x04, 0x00, 0x06, 0xD2, 0x76, 0x00, 0x01, 0x24, 0x01,
];

/// YubiKey Management Application AID (8 bytes).
#[allow(dead_code)]
pub const YUBIKEY_MGMT_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x05, 0x27, 0x47, 0x11, 0x17];

/// SELECT YubiKey Management AID APDU (with Le=00 to request response data).
#[allow(dead_code)]
pub const SELECT_MGMT: &[u8] = &[
    0x00, 0xA4, 0x04, 0x00, 0x08, 0xA0, 0x00, 0x00, 0x05, 0x27, 0x47, 0x11, 0x17, 0x00,
];

/// GET DEVICE INFO APDU (INS=0x1D, returns firmware version, form factor, serial).
#[allow(dead_code)]
pub const GET_DEVICE_INFO: &[u8] = &[0x00, 0x1D, 0x00, 0x00, 0x00];

/// PIV application AID (9 bytes).
#[allow(dead_code)]
pub const PIV_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x03, 0x08, 0x00, 0x00, 0x10, 0x00];

/// Full SELECT PIV APDU: CLA=00 INS=A4 P1=04 P2=00 Lc=09 [AID].
#[allow(dead_code)]
pub const SELECT_PIV: &[u8] = &[
    0x00, 0xA4, 0x04, 0x00, 0x09, 0xA0, 0x00, 0x00, 0x03, 0x08, 0x00, 0x00, 0x10, 0x00, 0x01,
];

/// Kill scdaemon so it releases the card channel before we connect exclusively.
/// Errors are silently ignored — scdaemon may already be stopped.
#[allow(dead_code)]
pub fn kill_scdaemon() {
    let _ = std::process::Command::new("gpgconf")
        .args(["--kill", "scdaemon"])
        .output();
}

/// Connect to the first reader that has an OpenPGP application.
///
/// Protocol:
///   1. kill_scdaemon() — release the card channel
///   2. Establish PC/SC context (Scope::User)
///   3. List readers (2048-byte name buffer)
///   4. For each reader: connect(Exclusive, T0|T1), SELECT OpenPGP AID
///   5. Return (Card, aid_data) on first success
///
/// Returns an error if no YubiKey with OpenPGP is found.
#[allow(dead_code)]
pub fn connect_to_openpgp_card() -> Result<(pcsc::Card, Vec<u8>)> {
    use pcsc::{Context, Protocols, Scope, ShareMode};

    kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User).map_err(|e| anyhow::anyhow!("PC/SC error: {e}"))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = ctx
        .list_readers(&mut readers_buf)
        .map_err(|e| anyhow::anyhow!("No smart card readers found: {e}"))?
        .collect();

    if readers.is_empty() {
        anyhow::bail!("No smart card readers found");
    }

    for reader in readers {
        let card = match ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut buf = [0u8; 256];
        let resp = match card.transmit(SELECT_OPENPGP, &mut buf) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if apdu_sw(resp) != 0x9000 {
            continue;
        }

        // Strip the 2-byte SW from the AID response
        let aid_data = resp[..resp.len().saturating_sub(2)].to_vec();
        return Ok((card, aid_data));
    }

    anyhow::bail!("No YubiKey with OpenPGP application found")
}

/// GET DATA for a 1-byte P1:P2 tag.
///
/// Sends `[0x00, 0xCA, p1, p2, 0x00]` and returns the data bytes (SW stripped).
/// Handles T=0 SW 0x61xx ("xx bytes still available") by issuing GET RESPONSE
/// (INS=0xC0) in a loop until all data is assembled. This is required on YubiKey
/// firmware 5.4.x when Le=0x00 causes multi-part T=0 responses (e.g., DO 0x6E).
/// Without this, ignoring 0x61xx leaves the card expecting GET RESPONSE, which
/// causes the NEXT GET DATA (e.g., 0xC5) to return SW 0x6B00.
#[allow(dead_code)]
pub fn get_data(card: &pcsc::Card, p1: u8, p2: u8) -> Result<Vec<u8>> {
    let apdu = [0x00u8, 0xCA, p1, p2, 0x00];
    let mut buf = [0u8; 1024];
    let resp = card
        .transmit(&apdu, &mut buf)
        .map_err(|e| anyhow::anyhow!("GET DATA transmit error: {e}"))?;

    let sw = apdu_sw(resp);

    if sw == 0x9000 {
        return Ok(resp[..resp.len().saturating_sub(2)].to_vec());
    }

    // T=0: SW 0x61xx — normal processing, xx more bytes available via GET RESPONSE.
    // Issue GET RESPONSE (00 C0 00 00 Le) to collect pending data in a loop.
    if sw >> 8 == 0x61 {
        let mut full = resp[..resp.len().saturating_sub(2)].to_vec();
        let mut pending = (sw & 0xFF) as u8;
        loop {
            let get_resp = [0x00u8, 0xC0, 0x00, 0x00, pending];
            let mut rbuf = [0u8; 1024];
            let r = card
                .transmit(&get_resp, &mut rbuf)
                .map_err(|e| anyhow::anyhow!("GET RESPONSE transmit error: {e}"))?;
            let rsw = apdu_sw(r);
            full.extend_from_slice(&r[..r.len().saturating_sub(2)]);
            if rsw == 0x9000 {
                break;
            } else if rsw >> 8 == 0x61 {
                pending = (rsw & 0xFF) as u8;
            } else {
                tracing::debug!(
                    "GET RESPONSE SW {:04X} after GET DATA {:02X}{:02X}",
                    rsw,
                    p1,
                    p2
                );
                break; // partial data — return what we have
            }
        }
        return Ok(full);
    }

    tracing::debug!("GET DATA {:02X}{:02X} SW {:04X}", p1, p2, sw);
    anyhow::bail!(
        "{}",
        apdu_error_message(sw, &format!("reading DO {:02X}{:02X}", p1, p2))
    );
}

/// GET DATA for a 2-byte extended tag (e.g., 0x5F50 for URL).
///
/// Sends `[0x00, 0xCA, p1, p2, 0x00]` where P1:P2 form the 2-byte tag.
/// Returns the data bytes (SW stripped).
#[allow(dead_code)]
pub fn get_data_2byte_tag(card: &pcsc::Card, p1: u8, p2: u8) -> Result<Vec<u8>> {
    // Same wire format — P1 and P2 form the 2-byte extended tag
    get_data(card, p1, p2)
}

/// Extract the two-byte status word from an APDU response slice.
///
/// Returns 0 if the response is fewer than 2 bytes.
pub fn apdu_sw(resp: &[u8]) -> u16 {
    if resp.len() < 2 {
        return 0;
    }
    let n = resp.len();
    u16::from_be_bytes([resp[n - 2], resp[n - 1]])
}

/// Map an APDU status word to a plain-English user message.
///
/// SW codes go to `tracing::debug!` at the call site; this function produces
/// the human-readable message only. All messages include the `context` string
/// so the caller can say "while verifying PIN" or "while setting touch policy".
#[allow(dead_code)]
pub fn apdu_error_message(sw: u16, context: &str) -> String {
    let msg = match sw {
        0x9000 => return format!("Success ({})", context),
        sw if sw & 0xFFF0 == 0x63C0 => {
            let retries = sw & 0x000F;
            format!(
                "Wrong PIN — {} {} remaining",
                retries,
                if retries == 1 { "try" } else { "tries" }
            )
        }
        0x6300 => "Wrong PIN".to_string(),
        0x6982 => "Security condition not met — Admin PIN required".to_string(),
        0x6983 => "Authentication method blocked".to_string(),
        0x6A82 => "Data object not found".to_string(),
        0x6A80 => "Incorrect data in command".to_string(),
        0x6700 => "Wrong length".to_string(),
        _ => "Card operation failed — try removing and reinserting your YubiKey".to_string(),
    };
    format!("{} ({})", msg, context)
}

/// Extract the serial number from the OpenPGP AID select response.
///
/// AID layout (16 bytes):
///   [0..6]   D2 76 00 01 24 01  — RID prefix
///   [6]      version major
///   [7]      version minor
///   [8..10]  manufacturer ID (big-endian; Yubico = 0x0006)
///   [10..14] serial number (4 bytes, BCD-encoded on YubiKey)
///   [14..16] padding (00 00)
///
/// The 4 serial bytes are BCD-encoded: interpret as a hex string and parse as
/// decimal. Falls back to big-endian u32 if any nibble is A–F.
#[allow(dead_code)]
pub fn serial_from_aid(aid: &[u8]) -> Option<u32> {
    if aid.len() < 14 {
        return None;
    }
    if &aid[..6] != OPENPGP_AID {
        return None;
    }
    let hex_str = format!(
        "{:02X}{:02X}{:02X}{:02X}",
        aid[10], aid[11], aid[12], aid[13]
    );
    hex_str.parse::<u32>().ok().or_else(|| {
        // Invalid BCD (nibble A-F present) — fall back to big-endian u32
        Some(u32::from_be_bytes([aid[10], aid[11], aid[12], aid[13]]))
    })
}

/// Walk BER-TLV encoded data and return the value bytes for the first matching tag.
///
/// Handles:
/// - 1-byte tags (low 5 bits != 0x1F) and 2-byte tags (0xXX 0x1F prefix)
/// - Definite-form lengths: literal (< 0x80), 0x81-prefixed 1-byte, 0x82-prefixed 2-byte
///
/// Returns a slice pointing into `data` for zero-copy access.
#[allow(dead_code)]
pub fn tlv_find(data: &[u8], target_tag: u16) -> Option<&[u8]> {
    let mut i = 0;
    while i < data.len() {
        // Parse tag: if low 5 bits are all 1 it is a 2-byte tag
        let (tag, tag_len) = if data[i] & 0x1F == 0x1F {
            if i + 1 >= data.len() {
                break;
            }
            let t = ((data[i] as u16) << 8) | data[i + 1] as u16;
            (t, 2usize)
        } else {
            (data[i] as u16, 1usize)
        };
        i += tag_len;
        if i >= data.len() {
            break;
        }

        // Parse length (BER-TLV definite form)
        let (len, len_sz) = if data[i] == 0x82 {
            if i + 2 >= data.len() {
                break;
            }
            let l = ((data[i + 1] as usize) << 8) | data[i + 2] as usize;
            (l, 3usize)
        } else if data[i] == 0x81 {
            if i + 1 >= data.len() {
                break;
            }
            (data[i + 1] as usize, 2usize)
        } else {
            (data[i] as usize, 1usize)
        };
        i += len_sz;
        if i + len > data.len() {
            break;
        }

        if tag == target_tag {
            return Some(&data[i..i + len]);
        }
        i += len;
    }
    None
}

/// Information returned by the YubiKey Management Application's GET DEVICE INFO command.
///
/// Form factor byte encoding (low 7 bits = connector, high bit = NFC capable):
///   0x01 = USB-A keychain   0x02 = USB-A nano
///   0x03 = USB-C keychain   0x04 = USB-C nano
///   0x05 = USB-A + Lightning (5Ci)
///   0x80 bit set = NFC capable
pub struct DeviceInfo {
    /// Actual YubiKey firmware version (3 bytes from tag 0x05).
    pub firmware: Option<crate::yubikey::Version>,
    /// Raw form factor byte (tag 0x04). Use `form_factor_nfc()` helpers.
    pub form_factor_byte: Option<u8>,
    /// Serial number from management AID (tag 0x02). May be absent on older firmware.
    pub serial: Option<u32>,
}

/// Query the YubiKey Management Application on an already-connected card and
/// return device info (firmware version, form factor, serial).
///
/// Selects the management AID on the same connection. Fails silently — returns
/// `None` if the management AID is not supported (older firmware) or if the
/// GET DEVICE INFO APDU is unrecognised.
#[allow(dead_code)]
pub fn get_device_info(card: &pcsc::Card) -> Option<DeviceInfo> {
    let mut buf = [0u8; 512];

    // SELECT management AID
    let resp = card.transmit(SELECT_MGMT, &mut buf).ok()?;
    if apdu_sw(resp) != 0x9000 {
        return None;
    }

    // GET DEVICE INFO
    let resp = card.transmit(GET_DEVICE_INFO, &mut buf).ok()?;
    if apdu_sw(resp) != 0x9000 {
        return None;
    }
    // GET DEVICE INFO response layout (from ykman management.py):
    //   Newer firmware wraps inner TLV pairs in an outer 0x71 container tag.
    //   Older firmware starts with a bare length-prefix byte (not a TLV tag).
    // Detect which layout: if the first byte is 0x71, unwrap via tlv_find;
    // otherwise skip the leading length byte to reach the inner TLV pairs.
    let raw = &resp[..resp.len().saturating_sub(2)]; // strip SW bytes
    let data: &[u8] = if raw.first() == Some(&0x71) {
        tlv_find(raw, 0x71).unwrap_or(&[])
    } else if raw.is_empty() {
        raw
    } else {
        &raw[1..] // skip length-prefix byte
    };
    tracing::debug!(
        "get_device_info: raw_len={} first_byte={:02X?} data_len={}",
        raw.len(),
        raw.first(),
        data.len()
    );

    // Tag 0x05: firmware version (3 bytes: major.minor.patch)
    let firmware = tlv_find(data, 0x05).and_then(|v| {
        if v.len() >= 3 {
            Some(crate::yubikey::Version {
                major: v[0],
                minor: v[1],
                patch: v[2],
            })
        } else {
            None
        }
    });

    // Tag 0x04: form factor (1 byte)
    let form_factor_byte = tlv_find(data, 0x04).and_then(|v| v.first().copied());

    // Tag 0x02: serial number (4 bytes, big-endian) — only on newer firmware
    let serial = tlv_find(data, 0x02).and_then(|v| {
        if v.len() >= 4 {
            Some(u32::from_be_bytes([v[0], v[1], v[2], v[3]]))
        } else {
            None
        }
    });

    tracing::debug!(
        "get_device_info: firmware={:?} ff_byte={:?} serial={:?}",
        firmware,
        form_factor_byte,
        serial
    );
    Some(DeviceInfo {
        firmware,
        form_factor_byte,
        serial,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── apdu_sw ────────────────────────────────────────────────────────────────

    #[test]
    fn test_apdu_sw_success() {
        assert_eq!(apdu_sw(&[0x90, 0x00]), 0x9000);
    }

    #[test]
    fn test_apdu_sw_error() {
        assert_eq!(apdu_sw(&[0x69, 0x82]), 0x6982);
    }

    #[test]
    fn test_apdu_sw_empty() {
        assert_eq!(apdu_sw(&[]), 0);
    }

    #[test]
    fn test_apdu_sw_single_byte() {
        assert_eq!(apdu_sw(&[0x42]), 0);
    }

    #[test]
    fn test_apdu_sw_with_data_prefix() {
        // Data bytes followed by SW — should extract last 2
        assert_eq!(apdu_sw(&[0xAA, 0xBB, 0x90, 0x00]), 0x9000);
    }

    // ── apdu_error_message ────────────────────────────────────────────────────

    #[test]
    fn test_apdu_error_message_success_no_failed() {
        let msg = apdu_error_message(0x9000, "test");
        assert!(
            !msg.contains("failed"),
            "9000 message should not contain 'failed': {msg}"
        );
    }

    #[test]
    fn test_apdu_error_message_6982_security() {
        let msg = apdu_error_message(0x6982, "setting touch");
        assert!(
            msg.contains("Security condition not met"),
            "Expected 'Security condition not met' in: {msg}"
        );
        assert!(msg.contains("setting touch"), "Expected context in: {msg}");
    }

    #[test]
    fn test_apdu_error_message_63c2_two_tries() {
        let msg = apdu_error_message(0x63C2, "verifying PIN");
        assert!(msg.contains('2'), "Expected '2' in: {msg}");
        assert!(
            msg.contains("tries remaining"),
            "Expected 'tries remaining' in: {msg}"
        );
    }

    #[test]
    fn test_apdu_error_message_63c1_one_try() {
        let msg = apdu_error_message(0x63C1, "verifying PIN");
        assert!(msg.contains('1'), "Expected '1' in: {msg}");
        assert!(
            msg.contains("try remaining"),
            "Expected 'try remaining' in: {msg}"
        );
    }

    #[test]
    fn test_apdu_error_message_6a82_not_found() {
        let msg = apdu_error_message(0x6A82, "reading data");
        assert!(msg.contains("not found"), "Expected 'not found' in: {msg}");
    }

    #[test]
    fn test_apdu_error_message_ffff_unknown() {
        let msg = apdu_error_message(0xFFFF, "unknown op");
        assert!(
            msg.contains("try removing and reinserting"),
            "Expected reinserting hint in: {msg}"
        );
    }

    // ── serial_from_aid ───────────────────────────────────────────────────────

    #[test]
    fn test_serial_from_aid_valid() {
        // AID: D2 76 00 01 24 01 (prefix) 03 04 (version) 00 06 (mfr=Yubico) 09 07 45 82 (serial BCD) 00 00 (padding)
        let aid = [
            0xD2, 0x76, 0x00, 0x01, 0x24, 0x01, // prefix
            0x03, 0x04, // version 3.4
            0x00, 0x06, // manufacturer: Yubico
            0x09, 0x07, 0x45, 0x82, // serial BCD "09074582" = 9074582
            0x00, 0x00, // padding
        ];
        assert_eq!(serial_from_aid(&aid), Some(9074582));
    }

    #[test]
    fn test_serial_from_aid_too_short() {
        let aid = [0xD2, 0x76, 0x00, 0x01, 0x24];
        assert_eq!(serial_from_aid(&aid), None);
    }

    #[test]
    fn test_serial_from_aid_wrong_prefix() {
        let aid = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // wrong prefix
            0x03, 0x04, 0x00, 0x06, 0x09, 0x07, 0x45, 0x82, 0x00, 0x00,
        ];
        assert_eq!(serial_from_aid(&aid), None);
    }

    #[test]
    fn test_serial_from_aid_non_bcd_fallback() {
        // Serial bytes with A-F nibbles — should fall back to big-endian
        let aid = [
            0xD2, 0x76, 0x00, 0x01, 0x24, 0x01, // prefix
            0x05, 0x02, // version 5.2
            0x00, 0x06, // manufacturer
            0xAB, 0xCD, 0xEF, 0x01, // non-BCD serial
            0x00, 0x00,
        ];
        // "ABCDEF01" cannot parse as decimal, fallback = big-endian = 0xABCDEF01
        assert_eq!(serial_from_aid(&aid), Some(0xABCD_EF01));
    }

    // ── tlv_find ──────────────────────────────────────────────────────────────

    #[test]
    fn test_tlv_find_basic() {
        // TLV: tag=0xC4 len=0x07 val=[0x01,0x7F,0x7F,0x7F,0x03,0x00,0x03]
        let data = [0xC4u8, 0x07, 0x01, 0x7F, 0x7F, 0x7F, 0x03, 0x00, 0x03];
        let result = tlv_find(&data, 0xC4);
        assert_eq!(
            result,
            Some([0x01, 0x7F, 0x7F, 0x7F, 0x03, 0x00, 0x03].as_slice())
        );
    }

    #[test]
    fn test_tlv_find_not_found() {
        let data = [0xC4u8, 0x02, 0xAA, 0xBB];
        assert_eq!(tlv_find(&data, 0xC5), None);
    }

    #[test]
    fn test_tlv_find_two_byte_tag() {
        // 2-byte tag 0x5F50 (5F has low 5 bits = 1F, so next byte is part of tag)
        let data = [0x5Fu8, 0x50, 0x03, b'h', b't', b'p'];
        let result = tlv_find(&data, 0x5F50);
        assert_eq!(result, Some(b"htp".as_slice()));
    }

    #[test]
    fn test_tlv_find_81_length_encoding() {
        // tag=0xAB, length encoded as 0x81 0x04 (BER 1-byte extended length = 4)
        let data = [0xABu8, 0x81, 0x04, 0x01, 0x02, 0x03, 0x04];
        let result = tlv_find(&data, 0xAB);
        assert_eq!(result, Some([0x01u8, 0x02, 0x03, 0x04].as_slice()));
    }

    #[test]
    fn test_tlv_find_second_tag() {
        // Two TLVs: C4 02 AA BB, then C5 02 CC DD — find C5
        let data = [0xC4u8, 0x02, 0xAA, 0xBB, 0xC5, 0x02, 0xCC, 0xDD];
        let result = tlv_find(&data, 0xC5);
        assert_eq!(result, Some([0xCC, 0xDD].as_slice()));
    }
}
