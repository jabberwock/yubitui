use anyhow::Result;
use pcsc::{Context, Protocols, Scope, ShareMode};

#[derive(Debug, Clone, serde::Serialize)]
pub struct PivState {
    pub slots: Vec<SlotInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SlotInfo {
    pub slot: String,
    /// Short algorithm name, e.g. "RSA-2048" or "ECDSA-P256".
    pub algorithm: Option<String>,
    /// Subject common name (CN) from the X.509 certificate.
    pub subject: Option<String>,
    /// Issuer common name (CN) from the X.509 certificate.
    pub issuer: Option<String>,
    /// Certificate validity as "YYYY-MM-DD – YYYY-MM-DD".
    pub validity: Option<String>,
}

impl SlotInfo {
    pub fn occupied(slot: impl Into<String>) -> Self {
        Self {
            slot: slot.into(),
            algorithm: None,
            subject: None,
            issuer: None,
            validity: None,
        }
    }
}

/// PIV application AID.
#[allow(dead_code)]
pub const PIV_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x03, 0x08, 0x00, 0x00, 0x10, 0x00];

/// SELECT PIV AID APDU.
/// CLA=00 INS=A4 P1=04 P2=00 Lc=09 [PIV AID bytes]
#[allow(dead_code)]
pub const SELECT_PIV: &[u8] = &[
    0x00, 0xA4, 0x04, 0x00, 0x09, 0xA0, 0x00, 0x00, 0x03, 0x08, 0x00, 0x00, 0x10, 0x00, 0x01,
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
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User).map_err(|e| anyhow::anyhow!("PC/SC error: {e}"))?;

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
        ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1)
            .ok()
    }) {
        Some(c) => c,
        None => return Ok(PivState { slots: vec![] }),
    };

    // SELECT PIV AID
    let mut buf = [0u8; 256];
    let resp = card.transmit(SELECT_PIV, &mut buf).unwrap_or(&[0x6A, 0x82]);
    if super::card::apdu_sw(resp) != 0x9000 {
        // PIV application not available (best-effort per D-14)
        return Ok(PivState { slots: vec![] });
    }

    // PIV GET DATA APDUs per slot
    // Format: CLA=00 INS=CB P1=3F P2=FF Lc=05 5C 03 5F C1 XX
    // where XX is the slot object ID byte
    let piv_slots: &[(&str, [u8; 10])] = &[
        (
            "9a",
            [0x00, 0xCB, 0x3F, 0xFF, 0x05, 0x5C, 0x03, 0x5F, 0xC1, 0x05],
        ),
        (
            "9c",
            [0x00, 0xCB, 0x3F, 0xFF, 0x05, 0x5C, 0x03, 0x5F, 0xC1, 0x0A],
        ),
        (
            "9d",
            [0x00, 0xCB, 0x3F, 0xFF, 0x05, 0x5C, 0x03, 0x5F, 0xC1, 0x0B],
        ),
        (
            "9e",
            [0x00, 0xCB, 0x3F, 0xFF, 0x05, 0x5C, 0x03, 0x5F, 0xC1, 0x01],
        ),
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
            // Strip trailing 2-byte SW word before parsing TLV.
            let data = if resp.len() >= 2 { &resp[..resp.len() - 2] } else { resp };
            let mut info = SlotInfo::occupied(*slot_name);
            if let Some(der) = extract_tlv_value(data, 0x53).and_then(|d| extract_tlv_value(d, 0x70)) {
                if let Some(cert_info) = parse_cert_info(der) {
                    info.algorithm = cert_info.algorithm;
                    info.subject   = cert_info.subject;
                    info.issuer    = cert_info.issuer;
                    info.validity  = cert_info.validity;
                }
            }
            slots.push(info);
        }
        // SW 0x6A82 = empty slot (skip); other SWs = skip
    }

    Ok(PivState { slots })
}

// ============================================================================
// BER-TLV and X.509 helpers
// ============================================================================

struct CertInfo {
    algorithm: Option<String>,
    subject: Option<String>,
    issuer: Option<String>,
    validity: Option<String>,
}

/// Walk a flat BER-TLV byte stream and return the value for the first matching `tag`.
///
/// Handles both 1-byte tags and multi-byte DER lengths (up to 4-byte length fields).
/// Does NOT recurse into constructed TLVs — only searches at the top level.
fn extract_tlv_value(data: &[u8], tag: u8) -> Option<&[u8]> {
    let mut i = 0;
    while i < data.len() {
        let t = data[i];
        i += 1;
        if i >= data.len() { break; }

        // Decode BER length
        let len: usize = if data[i] & 0x80 == 0 {
            let l = data[i] as usize;
            i += 1;
            l
        } else {
            let n = (data[i] & 0x7F) as usize;
            i += 1;
            if n == 0 || n > 4 || i + n > data.len() { break; }
            let mut l: usize = 0;
            for _ in 0..n {
                l = (l << 8) | data[i] as usize;
                i += 1;
            }
            l
        };

        if i + len > data.len() { break; }
        if t == tag {
            return Some(&data[i..i + len]);
        }
        i += len;
    }
    None
}

/// Parse an X.509 DER certificate and extract display strings for the TUI.
fn parse_cert_info(der: &[u8]) -> Option<CertInfo> {
    use x509_parser::prelude::*;

    let (_, cert) = X509Certificate::from_der(der).ok()?;

    let subject = dn_common_name(cert.subject());
    let issuer  = dn_common_name(cert.issuer());

    // Algorithm OID → human readable
    let algorithm = oid_to_algorithm_name(cert.signature_algorithm.algorithm.to_id_string().as_str())
        .or_else(|| {
            // Fall back to algorithm in SubjectPublicKeyInfo
            oid_to_algorithm_name(
                cert.tbs_certificate
                    .subject_pki
                    .algorithm
                    .algorithm
                    .to_id_string()
                    .as_str(),
            )
        })
        .map(|s| s.to_string());

    // Validity window
    let not_before = cert.validity().not_before.to_datetime();
    let not_after  = cert.validity().not_after.to_datetime();
    let validity = Some(format!(
        "{} – {}",
        not_before.date().to_string(),
        not_after.date().to_string(),
    ));

    Some(CertInfo { algorithm, subject, issuer, validity })
}

/// Extract the first CN value from an X.509 distinguished name, falling back to
/// the full RFC4514 string if no CN attribute is present.
fn dn_common_name(name: &x509_parser::x509::X509Name<'_>) -> Option<String> {
    use x509_parser::prelude::*;

    // Try CN attribute first
    for rdn in name.iter() {
        for attr in rdn.iter() {
            if attr.attr_type() == &oid_registry::OID_X509_COMMON_NAME {
                if let Ok(s) = attr.attr_value().as_str() {
                    return Some(s.to_string());
                }
            }
        }
    }
    // Fall back to full DN string if non-empty
    let s = name.to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Map well-known signature / key algorithm OID strings to short names.
fn oid_to_algorithm_name(oid: &str) -> Option<&'static str> {
    match oid {
        // RSA
        "1.2.840.113549.1.1.1"  => Some("RSA"),
        "1.2.840.113549.1.1.11" => Some("RSA-SHA256"),
        "1.2.840.113549.1.1.12" => Some("RSA-SHA384"),
        "1.2.840.113549.1.1.13" => Some("RSA-SHA512"),
        // EC / ECDSA
        "1.2.840.10045.4.3.2"   => Some("ECDSA-P256"),
        "1.2.840.10045.4.3.3"   => Some("ECDSA-P384"),
        "1.2.840.10045.2.1"     => Some("EC"),
        // Ed25519 / Ed448
        "1.3.101.112"           => Some("Ed25519"),
        "1.3.101.113"           => Some("Ed448"),
        _ => None,
    }
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

                slots.push(SlotInfo::occupied(slot_name));
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
