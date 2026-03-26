use anyhow::Result;
use pcsc::{Context, Protocols, Scope, ShareMode};

use super::{FormFactor, Model, Version, YubiKeyInfo, YubiKeyState};
use super::card;

/// Detect all connected YubiKeys by enumerating PC/SC readers.
///
/// For each reader that has the OpenPGP application selected successfully:
///   - Extracts serial and version from the AID select response
///   - Reads PIN status via GET DATA 0xC4
///   - Reads OpenPGP state via GET DATA 0x6E + 0x65
///   - Reads key attributes via GET DATA 0x6E algorithm attributes
///
/// Returns a Vec with one YubiKeyState per reader with a valid OpenPGP app.
/// Returns an empty vec if no YubiKey is found (no error).
pub fn detect_all_yubikey_states() -> Result<Vec<YubiKeyState>> {
    card::kill_scdaemon();

    let ctx = Context::establish(Scope::User)
        .map_err(|e| anyhow::anyhow!("PC/SC error: {e}"))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = match ctx.list_readers(&mut readers_buf) {
        Ok(r) => r.collect(),
        Err(_) => return Ok(vec![]),
    };

    if readers.is_empty() {
        tracing::debug!("No smart card readers found");
        return Ok(vec![]);
    }

    let mut states = Vec::new();

    for reader in readers {
        let card = match ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // SELECT OpenPGP AID
        let mut buf = [0u8; 256];
        let resp = match card.transmit(card::SELECT_OPENPGP, &mut buf) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if card::apdu_sw(resp) != 0x9000 {
            continue;
        }

        // AID select response (strip SW bytes)
        let aid_data = &resp[..resp.len().saturating_sub(2)];

        // Extract serial from AID bytes 10-13 (BCD-encoded)
        let serial = card::serial_from_aid(aid_data).unwrap_or(0);

        // Extract version from AID bytes 6-7
        let version = if aid_data.len() >= 8 {
            Version {
                major: aid_data[6],
                minor: aid_data[7],
                patch: 0,
            }
        } else {
            Version { major: 0, minor: 0, patch: 0 }
        };

        let model = detect_model_from_version(&version);

        let info = YubiKeyInfo {
            serial,
            version,
            model,
            form_factor: FormFactor::UsbA,
        };

        // Read PIN status (DO 0xC4 PW Status Bytes)
        let pin_status = read_pin_status_from_card(&card)
            .unwrap_or(super::pin::PinStatus {
                user_pin_retries: 3,
                admin_pin_retries: 3,
                reset_code_retries: 0,
                user_pin_blocked: false,
                admin_pin_blocked: false,
            });

        // Read OpenPGP state (DO 0x6E + 0x65 + 0x5F50)
        let openpgp = read_openpgp_state_from_card(&card, aid_data).ok();

        // Get PIV state (best-effort, no error on failure)
        let piv = super::piv::get_piv_state().ok();

        // Touch policies — read via native GET DATA 0xD6-0xD9
        let touch_policies = super::touch_policy::get_touch_policies_native(&card).ok();

        states.push(YubiKeyState {
            info,
            openpgp,
            piv,
            pin_status,
            touch_policies,
        });
    }

    tracing::info!(
        "PC/SC reader enumeration found {} YubiKey(s)",
        states.len()
    );

    Ok(states)
}

/// Kept for backward compat — delegates to detect_all_yubikey_states.
#[allow(dead_code)]
pub fn detect_yubikey_state() -> Result<Option<YubiKeyState>> {
    let mut all = detect_all_yubikey_states()?;
    if all.is_empty() {
        Ok(None)
    } else {
        Ok(Some(all.remove(0)))
    }
}

/// Read PIN retry counters from DO 0xC4 PW Status Bytes.
fn read_pin_status_from_card(card: &pcsc::Card) -> Result<super::pin::PinStatus> {
    let data = card::get_data(card, 0x00, 0xC4)?;
    if data.len() < 7 {
        anyhow::bail!(
            "Unexpected PW Status Bytes length: {}",
            data.len()
        );
    }
    Ok(super::pin::PinStatus {
        user_pin_retries: data[4],
        admin_pin_retries: data[6],
        reset_code_retries: data[5],
        user_pin_blocked: data[4] == 0,
        admin_pin_blocked: data[6] == 0,
    })
}

/// Read OpenPGP state from the card using GET DATA 0x6E (application related
/// data), 0x65 (cardholder), and 0x5F50 (URL).
fn read_openpgp_state_from_card(
    card: &pcsc::Card,
    aid_data: &[u8],
) -> Result<super::openpgp::OpenPgpState> {
    // Version from AID bytes 6-7
    let version = if aid_data.len() >= 8 {
        format!("{}.{}", aid_data[6], aid_data[7])
    } else {
        String::new()
    };

    // GET DATA 0x6E — Application Related Data (TLV-constructed)
    let app_data = match card::get_data(card, 0x00, 0x6E) {
        Ok(d) => d,
        Err(_) => {
            return Ok(super::openpgp::OpenPgpState {
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
    let disc = card::tlv_find(&app_data, 0x73);

    let (sig_fp, enc_fp, aut_fp, sig_algo_raw, enc_algo_raw, aut_algo_raw) =
        if let Some(d) = disc {
            (
                card::tlv_find(d, 0xC7).map(|b| b.to_vec()),
                card::tlv_find(d, 0xC8).map(|b| b.to_vec()),
                card::tlv_find(d, 0xC9).map(|b| b.to_vec()),
                card::tlv_find(d, 0xC1).map(|b| b.to_vec()),
                card::tlv_find(d, 0xC2).map(|b| b.to_vec()),
                card::tlv_find(d, 0xC3).map(|b| b.to_vec()),
            )
        } else {
            (None, None, None, None, None, None)
        };

    let signature_key = build_key_info(sig_fp.as_deref(), sig_algo_raw.as_deref());
    let encryption_key = build_key_info(enc_fp.as_deref(), enc_algo_raw.as_deref());
    let authentication_key = build_key_info(aut_fp.as_deref(), aut_algo_raw.as_deref());

    // GET DATA 0x65 — Cardholder Related Data
    let cardholder_name = card::get_data(card, 0x00, 0x65).ok().and_then(|ch_data| {
        card::tlv_find(&ch_data, 0x5B).and_then(|name_bytes| {
            let name = String::from_utf8_lossy(name_bytes).trim().to_string();
            if name.is_empty() { None } else { Some(name) }
        })
    });

    // GET DATA 0x5F50 — URL of public key
    let public_key_url = card::get_data_2byte_tag(card, 0x5F, 0x50).ok().and_then(|url_bytes| {
        if url_bytes.is_empty() {
            None
        } else {
            let url = String::from_utf8_lossy(&url_bytes).trim().to_string();
            if url.is_empty() { None } else { Some(url) }
        }
    });

    Ok(super::openpgp::OpenPgpState {
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
/// Returns None if the fingerprint is all-zeros (no key in slot).
fn build_key_info(
    fp_bytes: Option<&[u8]>,
    algo_bytes: Option<&[u8]>,
) -> Option<super::openpgp::KeyInfo> {
    let fp_bytes = fp_bytes?;
    if fp_bytes.iter().all(|&b| b == 0) {
        return None;
    }
    let fingerprint = format_fingerprint(fp_bytes);
    if fingerprint.is_empty() {
        return None;
    }
    let key_attributes = algo_bytes
        .map(parse_algorithm_attributes)
        .unwrap_or_else(|| "Unknown".to_string());
    Some(super::openpgp::KeyInfo {
        fingerprint,
        created: None,
        key_attributes,
    })
}

/// Format a fingerprint byte slice as an uppercase hex string.
pub fn format_fingerprint(fp: &[u8]) -> String {
    if fp.iter().all(|&b| b == 0) {
        return String::new();
    }
    fp.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join("")
}

/// Parse algorithm attribute bytes to a human-readable string.
///
/// First byte encodes the algorithm type:
///   0x01 = RSA (bytes 1-2 = bit length big-endian)
///   0x12 = ECDH (Cv25519 or other curve)
///   0x13 = ECDSA (P-256 or other curve)
///   0x16 = EdDSA (Ed25519)
pub fn parse_algorithm_attributes(data: &[u8]) -> String {
    if data.is_empty() {
        return "Unknown".to_string();
    }
    match data[0] {
        0x01 => {
            if data.len() >= 3 {
                let bits = u16::from_be_bytes([data[1], data[2]]);
                format!("RSA {}", bits)
            } else {
                "RSA".to_string()
            }
        }
        0x12 => "ECDH (Cv25519)".to_string(),
        0x13 => "ECDSA (P-256)".to_string(),
        0x16 => "EdDSA (Ed25519)".to_string(),
        _ => format!("Unknown (0x{:02X})", data[0]),
    }
}

pub fn detect_model_from_version(version: &Version) -> Model {
    // YubiKey 5 series
    if version.major >= 5 {
        return Model::YubiKey5;
    }

    // YubiKey 4 series
    if version.major == 4 {
        return Model::YubiKey4;
    }

    // YubiKey NEO
    if version.major == 3 {
        return Model::YubiKeyNeo;
    }

    Model::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::Version;

    #[test]
    fn test_detect_model_yubikey5() {
        let v = Version { major: 5, minor: 2, patch: 7 };
        assert_eq!(detect_model_from_version(&v), Model::YubiKey5);
    }

    #[test]
    fn test_detect_model_yubikey4() {
        let v = Version { major: 4, minor: 3, patch: 0 };
        assert_eq!(detect_model_from_version(&v), Model::YubiKey4);
    }

    #[test]
    fn test_detect_model_neo() {
        let v = Version { major: 3, minor: 1, patch: 0 };
        assert_eq!(detect_model_from_version(&v), Model::YubiKeyNeo);
    }

    #[test]
    fn test_detect_model_unknown() {
        let v = Version { major: 1, minor: 0, patch: 0 };
        assert_eq!(detect_model_from_version(&v), Model::Unknown);
    }

    #[test]
    fn test_format_fingerprint_all_zeros() {
        assert_eq!(format_fingerprint(&[0u8; 20]), "");
    }

    #[test]
    fn test_format_fingerprint_valid() {
        let fp = [0xABu8, 0xCD, 0xEF];
        assert_eq!(format_fingerprint(&fp), "ABCDEF");
    }

    #[test]
    fn test_parse_algorithm_rsa2048() {
        let data = [0x01u8, 0x08, 0x00]; // RSA 2048
        assert_eq!(parse_algorithm_attributes(&data), "RSA 2048");
    }

    #[test]
    fn test_parse_algorithm_rsa4096() {
        let data = [0x01u8, 0x10, 0x00]; // RSA 4096
        assert_eq!(parse_algorithm_attributes(&data), "RSA 4096");
    }

    #[test]
    fn test_parse_algorithm_eddsa() {
        let data = [0x16u8]; // EdDSA
        assert_eq!(parse_algorithm_attributes(&data), "EdDSA (Ed25519)");
    }

    #[test]
    fn test_parse_algorithm_ecdh() {
        let data = [0x12u8]; // ECDH
        assert_eq!(parse_algorithm_attributes(&data), "ECDH (Cv25519)");
    }

    #[test]
    fn test_parse_algorithm_empty() {
        assert_eq!(parse_algorithm_attributes(&[]), "Unknown");
    }
}
