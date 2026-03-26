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

        // GET DATA 0x004F — full AID (more reliable than parsing SELECT response,
        // which many YubiKey firmwares return as SW-only with no data body).
        let (serial, version) = match card::get_data(&card, 0x00, 0x4F) {
            Ok(aid) => {
                let s = card::serial_from_aid(&aid).unwrap_or(0);
                let v = if aid.len() >= 8 {
                    Version { major: aid[6], minor: aid[7], patch: 0 }
                } else {
                    Version { major: 0, minor: 0, patch: 0 }
                };
                (s, v)
            }
            Err(_) => (0, Version { major: 0, minor: 0, patch: 0 }),
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

        // Read OpenPGP state via direct GET DATA DOs
        let openpgp = read_openpgp_state_from_card(&card).ok();

        // Get PIV state (best-effort, no error on failure)
        let piv = super::piv::get_piv_state().ok();

        // Touch policies — read via native GET DATA 0xD6-0xD9
        let touch_policies = super::touch_policy::get_touch_policies_native(&card).ok();

        // GET DEVICE INFO from YubiKey Management AID — actual firmware version,
        // form factor, and NFC capability. Falls back gracefully on older firmware.
        let dev_info = card::get_device_info(&card);
        let info = if let Some(ref di) = dev_info {
            let fw = di.firmware.clone().unwrap_or_else(|| info.version.clone());
            let ff_byte = di.form_factor_byte.unwrap_or(0);
            let (model, form_factor) = model_from_form_factor(ff_byte, fw.major);
            let sn = di.serial.unwrap_or(info.serial);
            YubiKeyInfo { serial: sn, version: fw, model, form_factor }
        } else {
            info
        };

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

/// Read OpenPGP state from the card using direct GET DATA calls.
///
/// Uses flat GET DATA DOs (0x4F, 0xC5, 0xC1-C3) instead of parsing the nested
/// 0x6E/0x73 TLV structure. The nested approach fails when YubiKey includes the
/// outer 0x6E tag in the GET DATA response, making tlv_find scan past the content.
fn read_openpgp_state_from_card(
    card: &pcsc::Card,
) -> Result<super::openpgp::OpenPgpState> {
    // Version from GET DATA 0x004F (AID) — bytes 6-7 are the OpenPGP app version.
    let version = card::get_data(card, 0x00, 0x4F)
        .ok()
        .filter(|aid| aid.len() >= 8)
        .map(|aid| format!("{}.{}", aid[6], aid[7]))
        .unwrap_or_default();

    // GET DATA 0x00C5 — Fingerprints: 60 bytes = SIG(20) | ENC(20) | AUT(20).
    // Direct DO access avoids the 0x6E/0x73 TLV nesting issue.
    let c5 = card::get_data(card, 0x00, 0xC5).ok();
    let sig_fp = c5.as_deref().and_then(|b| if b.len() >= 20 { Some(b[..20].to_vec()) } else { None });
    let enc_fp = c5.as_deref().and_then(|b| if b.len() >= 40 { Some(b[20..40].to_vec()) } else { None });
    let aut_fp = c5.as_deref().and_then(|b| if b.len() >= 60 { Some(b[40..60].to_vec()) } else { None });

    // GET DATA 0x00C1/C2/C3 — Algorithm attributes per slot.
    let sig_algo_raw = card::get_data(card, 0x00, 0xC1).ok();
    let enc_algo_raw = card::get_data(card, 0x00, 0xC2).ok();
    let aut_algo_raw = card::get_data(card, 0x00, 0xC3).ok();

    let signature_key = build_key_info(sig_fp.as_deref(), sig_algo_raw.as_deref());
    let encryption_key = build_key_info(enc_fp.as_deref(), enc_algo_raw.as_deref());
    let authentication_key = build_key_info(aut_fp.as_deref(), aut_algo_raw.as_deref());

    // GET DATA 0x65 — Cardholder Related Data (TLV; name is tag 0x5B inside).
    let cardholder_name = card::get_data(card, 0x00, 0x65).ok().and_then(|ch_data| {
        // Response may include or omit the outer 0x65 tag; try both.
        let inner = if ch_data.first() == Some(&0x65) {
            card::tlv_find(&ch_data, 0x65).map(|v| v.to_vec()).unwrap_or(ch_data)
        } else {
            ch_data
        };
        card::tlv_find(&inner, 0x5B).and_then(|name_bytes| {
            let name = String::from_utf8_lossy(name_bytes).trim().to_string();
            if name.is_empty() { None } else { Some(name) }
        })
    });

    // GET DATA 0x5F50 — URL of public key.
    let public_key_url = card::get_data_2byte_tag(card, 0x5F, 0x50).ok().and_then(|url_bytes| {
        let url = String::from_utf8_lossy(&url_bytes).trim().to_string();
        if url.is_empty() { None } else { Some(url) }
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

/// Derive Model and FormFactor from the GET DEVICE INFO form-factor byte and
/// firmware major version.
///
/// Form factor byte encoding (from YubiKey SDK):
///   Low 7 bits: 0x01=USB-A keychain, 0x02=USB-A nano, 0x03=USB-C keychain,
///               0x04=USB-C nano, 0x05=USB-A+Lightning (Ci)
///   Bit 0x80: NFC capable
pub fn model_from_form_factor(ff_byte: u8, fw_major: u8) -> (Model, FormFactor) {
    let nfc = ff_byte & 0x80 != 0;
    let connector = ff_byte & 0x7F;
    match (fw_major, connector, nfc) {
        (5.., 0x01, true)  => (Model::YubiKey5NFC,  FormFactor::UsbA),
        (5.., 0x01, false) => (Model::YubiKey5,     FormFactor::UsbA),
        (5.., 0x02, _)     => (Model::YubiKey5Nano, FormFactor::Nano),
        (5.., 0x03, true)  => (Model::YubiKey5CNFC, FormFactor::UsbC),
        (5.., 0x03, false) => (Model::YubiKey5C,    FormFactor::UsbC),
        (5.., 0x04, _)     => (Model::YubiKey5CNano,FormFactor::Nano),
        (5.., 0x05, _)     => (Model::YubiKey5Ci,   FormFactor::UsbC),
        (4, 0x01, _)       => (Model::YubiKey4,     FormFactor::UsbA),
        (4, 0x03, _)       => (Model::YubiKey4C,    FormFactor::UsbC),
        (4, 0x02, _)       => (Model::YubiKey4Nano, FormFactor::Nano),
        (3, _, _)          => (Model::YubiKeyNeo,   FormFactor::UsbA),
        _                  => (Model::Unknown,       FormFactor::Unknown),
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
