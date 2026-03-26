use anyhow::Result;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OpenPgpState {
    pub card_present: bool,
    pub version: String,
    pub signature_key: Option<KeyInfo>,
    pub encryption_key: Option<KeyInfo>,
    pub authentication_key: Option<KeyInfo>,
    pub cardholder_name: Option<String>,
    pub public_key_url: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct KeyInfo {
    pub fingerprint: String,
    pub created: Option<String>,
    pub key_attributes: String,
}

/// Read OpenPGP card state via native PC/SC GET DATA APDUs.
///
/// Uses:
///   - AID select response for firmware version
///   - GET DATA 0x6E for fingerprints and algorithm attributes (TLV-parsed)
///   - GET DATA 0x65 for cardholder name (tag 0x5B)
///   - GET DATA 0x5F50 for URL of public key
///
/// On any connect failure, returns OpenPgpState with card_present=false.
#[allow(dead_code)]
pub fn get_openpgp_state() -> Result<OpenPgpState> {
    let (card, aid_data) = match super::card::connect_to_openpgp_card() {
        Ok(pair) => pair,
        Err(_) => {
            return Ok(OpenPgpState {
                card_present: false,
                version: String::new(),
                signature_key: None,
                encryption_key: None,
                authentication_key: None,
                cardholder_name: None,
                public_key_url: None,
            });
        }
    };

    // Version from AID bytes 6-7
    let version = if aid_data.len() >= 8 {
        format!("{}.{}", aid_data[6], aid_data[7])
    } else {
        String::new()
    };

    // GET DATA 0x6E — Application Related Data (TLV-constructed)
    let app_data = match super::card::get_data(&card, 0x00, 0x6E) {
        Ok(d) => d,
        Err(_) => {
            return Ok(OpenPgpState {
                card_present: true,
                version,
                signature_key: None,
                encryption_key: None,
                authentication_key: None,
                cardholder_name: None,
                public_key_url: None,
            });
        }
    };

    // Navigate into Discretionary Data Objects (tag 0x73)
    let disc = super::card::tlv_find(&app_data, 0x73);

    let (sig_fp, enc_fp, aut_fp, sig_algo, enc_algo, aut_algo) = if let Some(d) = disc {
        (
            super::card::tlv_find(d, 0xC7).map(|b| b.to_vec()),
            super::card::tlv_find(d, 0xC8).map(|b| b.to_vec()),
            super::card::tlv_find(d, 0xC9).map(|b| b.to_vec()),
            super::card::tlv_find(d, 0xC1).map(|b| b.to_vec()),
            super::card::tlv_find(d, 0xC2).map(|b| b.to_vec()),
            super::card::tlv_find(d, 0xC3).map(|b| b.to_vec()),
        )
    } else {
        (None, None, None, None, None, None)
    };

    let signature_key = build_key_info(sig_fp.as_deref(), sig_algo.as_deref());
    let encryption_key = build_key_info(enc_fp.as_deref(), enc_algo.as_deref());
    let authentication_key = build_key_info(aut_fp.as_deref(), aut_algo.as_deref());

    // GET DATA 0x65 — Cardholder Related Data
    let cardholder_name = super::card::get_data(&card, 0x00, 0x65)
        .ok()
        .and_then(|ch_data| {
            super::card::tlv_find(&ch_data, 0x5B).and_then(|name_bytes| {
                let name = String::from_utf8_lossy(name_bytes).trim().to_string();
                if name.is_empty() { None } else { Some(name) }
            })
        });

    // GET DATA 0x5F50 — URL of public key
    let public_key_url = super::card::get_data_2byte_tag(&card, 0x5F, 0x50)
        .ok()
        .and_then(|url_bytes| {
            if url_bytes.is_empty() {
                None
            } else {
                let url = String::from_utf8_lossy(&url_bytes).trim().to_string();
                if url.is_empty() { None } else { Some(url) }
            }
        });

    Ok(OpenPgpState {
        card_present: true,
        version,
        signature_key,
        encryption_key,
        authentication_key,
        cardholder_name,
        public_key_url,
    })
}

/// Build a KeyInfo from raw fingerprint bytes and algorithm attribute bytes.
/// Returns None if the fingerprint is all-zeros or absent (no key in slot).
#[allow(dead_code)]
fn build_key_info(fp_bytes: Option<&[u8]>, algo_bytes: Option<&[u8]>) -> Option<KeyInfo> {
    let fp_bytes = fp_bytes?;
    if fp_bytes.iter().all(|&b| b == 0) {
        return None;
    }
    let fingerprint = super::detection::format_fingerprint(fp_bytes);
    if fingerprint.is_empty() {
        return None;
    }
    let key_attributes = algo_bytes
        .map(super::detection::parse_algorithm_attributes)
        .unwrap_or_else(|| "Unknown".to_string());
    Some(KeyInfo {
        fingerprint,
        created: None,
        key_attributes,
    })
}

#[allow(dead_code)]
pub fn parse_card_status(output: &str) -> Result<OpenPgpState> {
    let mut signature_key = None;
    let mut encryption_key = None;
    let mut authentication_key = None;
    let mut cardholder_name = None;
    let mut public_key_url = None;
    let mut version = String::new();
    let mut key_attributes = "rsa2048".to_string(); // default

    for line in output.lines() {
        let line = line.trim();

        if line.starts_with("Version ..........:") {
            version = line
                .split(':')
                .nth(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
        } else if line.starts_with("Signature key .....:") {
            let key = line.split(':').nth(1).map(|s| s.trim());
            if let Some(key_str) = key {
                if key_str != "[none]" && !key_str.is_empty() {
                    signature_key = Some(KeyInfo {
                        fingerprint: key_str.to_string(),
                        created: None,
                        key_attributes: key_attributes.clone(),
                    });
                }
            }
        } else if line.starts_with("Encryption key.....:") {
            let key = line.split(':').nth(1).map(|s| s.trim());
            if let Some(key_str) = key {
                if key_str != "[none]" && !key_str.is_empty() {
                    encryption_key = Some(KeyInfo {
                        fingerprint: key_str.to_string(),
                        created: None,
                        key_attributes: key_attributes.clone(),
                    });
                }
            }
        } else if line.starts_with("Authentication key:") {
            let key = line.split(':').nth(1).map(|s| s.trim());
            if let Some(key_str) = key {
                if key_str != "[none]" && !key_str.is_empty() {
                    authentication_key = Some(KeyInfo {
                        fingerprint: key_str.to_string(),
                        created: None,
                        key_attributes: key_attributes.clone(),
                    });
                }
            }
        } else if line.starts_with("Name of cardholder:") {
            let name = line.split(':').nth(1).map(|s| s.trim());
            if let Some(name_str) = name {
                if name_str != "[not set]" && !name_str.is_empty() {
                    cardholder_name = Some(name_str.to_string());
                }
            }
        } else if line.starts_with("URL of public key :") {
            // Use split_once so that the URL itself (which contains ':') is kept intact
            let url_str = line.split_once(':').map(|(_label, rest)| rest.trim());
            if let Some(u) = url_str {
                if u != "[not set]" && !u.is_empty() {
                    public_key_url = Some(u.to_string());
                }
            }
        } else if line.starts_with("Key attributes ...:") {
            // Parse something like "rsa2048 rsa2048 rsa2048" or "ed25519 cv25519 ed25519"
            key_attributes = line
                .split(':')
                .nth(1)
                .map(|s| s.split_whitespace().next().unwrap_or("rsa2048").to_string())
                .unwrap_or_else(|| "rsa2048".to_string());
        }
    }

    Ok(OpenPgpState {
        card_present: true,
        version,
        signature_key,
        encryption_key,
        authentication_key,
        cardholder_name,
        public_key_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_card_status_full() {
        let input = "\
Version ..........: 3.4\n\
Name of cardholder: Test User\n\
URL of public key : https://example.com/key.asc\n\
Key attributes ...: ed25519 cv25519 ed25519\n\
Signature key .....: ABCD 1234 5678 9012 3456  7890 ABCD EF01 2345 6789\n\
Encryption key.....: 1111 2222 3333 4444 5555  6666 7777 8888 9999 0000\n\
Authentication key: AAAA BBBB CCCC DDDD EEEE  FFFF 0000 1111 2222 3333\n\
";
        let state = parse_card_status(input).unwrap();
        assert!(state.card_present);
        assert_eq!(state.version, "3.4");
        assert_eq!(state.cardholder_name.as_deref(), Some("Test User"));
        assert_eq!(
            state.public_key_url.as_deref(),
            Some("https://example.com/key.asc")
        );
        assert!(state.signature_key.is_some());
        assert!(state.encryption_key.is_some());
        assert!(state.authentication_key.is_some());
        let sig = state.signature_key.unwrap();
        assert!(sig.fingerprint.contains("ABCD"));
        assert_eq!(sig.key_attributes, "ed25519");
    }

    #[test]
    fn test_parse_card_status_empty() {
        let state = parse_card_status("").unwrap();
        assert!(state.card_present);
        assert!(state.signature_key.is_none());
        assert!(state.encryption_key.is_none());
        assert!(state.authentication_key.is_none());
        assert!(state.cardholder_name.is_none());
    }

    #[test]
    fn test_parse_card_status_none_keys() {
        let input = "\
Signature key .....: [none]\n\
Encryption key.....: [none]\n\
Authentication key: [none]\n\
";
        let state = parse_card_status(input).unwrap();
        assert!(state.signature_key.is_none());
        assert!(state.encryption_key.is_none());
        assert!(state.authentication_key.is_none());
    }

    #[test]
    fn test_parse_card_status_rsa() {
        let input = "Key attributes ...: rsa4096 rsa4096 rsa4096\n";
        let state = parse_card_status(input).unwrap();
        // key_attributes is updated but no key lines present so keys are None
        // The key_attributes default before this line is rsa2048, after it becomes rsa4096
        // We verify the state parses without error
        assert!(state.card_present);
        assert!(state.signature_key.is_none());
    }
}
