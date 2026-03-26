use anyhow::Result;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;

/// Valid slots for attestation (att slot cannot self-attest).
pub const VALID_ATTEST_SLOTS: &[&str] = &["sig", "enc", "aut"];

/// Slot display names for UI.
#[allow(dead_code)]
pub fn slot_display_name(slot: &str) -> &str {
    match slot {
        "sig" => "Signature",
        "enc" => "Encryption",
        "aut" => "Authentication",
        _ => "Unknown",
    }
}

/// Map an OpenPGP slot name to the CRT tag used by the YubiKey ATTEST command.
///
/// INS=0xFB (YubiKey proprietary ATTEST), P1 = CRT tag per slot.
fn slot_to_crt_tag(slot: &str) -> Option<u8> {
    match slot {
        "sig" => Some(0xB6),
        "enc" => Some(0xB8),
        "aut" => Some(0xA4),
        _ => None,
    }
}

/// Fetch the attestation certificate for a given OpenPGP slot via native PC/SC.
///
/// Uses the YubiKey-proprietary ATTEST command: INS=0xFB, P1=CRT_TAG, P2=0x00.
/// Returns a PEM-encoded certificate string on success.
///
/// Fails if:
///   - slot is invalid (not sig/enc/aut)
///   - key was imported rather than generated on-device (SW 0x6A88)
///   - card error occurs
///
/// `serial` is kept for API compatibility but unused (connect_to_openpgp_card
/// connects to the first OpenPGP card; Plan 3 may add per-serial selection).
#[allow(dead_code)]
pub fn get_attestation_cert(slot: &str, _serial: Option<u32>) -> Result<String> {
    if !VALID_ATTEST_SLOTS.contains(&slot) {
        anyhow::bail!("Invalid attestation slot '{}'. Valid: sig, enc, aut", slot);
    }

    let crt_tag = slot_to_crt_tag(slot)
        .ok_or_else(|| anyhow::anyhow!("Unknown slot: {}", slot))?;

    let (card, _aid) = super::card::connect_to_openpgp_card()?;

    // YubiKey proprietary ATTEST: CLA=00 INS=FB P1=CRT_TAG P2=00 Le=00
    let attest_apdu = [0x00u8, 0xFB, crt_tag, 0x00, 0x00];
    let mut buf = [0u8; 4096];
    let resp = card
        .transmit(&attest_apdu, &mut buf)
        .map_err(|e| anyhow::anyhow!("ATTEST transmit error: {e}"))?;

    let sw = super::card::apdu_sw(resp);
    if sw != 0x9000 {
        let msg = if sw == 0x6A88 {
            format!(
                "Key not generated on this YubiKey — attestation requires on-device key generation ({})",
                slot
            )
        } else {
            super::card::apdu_error_message(sw, &format!("attesting slot {}", slot))
        };
        anyhow::bail!("{}", msg);
    }

    // Response data is DER-encoded certificate (strip 2-byte SW trailer)
    let der = &resp[..resp.len().saturating_sub(2)];
    if der.is_empty() {
        anyhow::bail!("Attestation returned empty certificate for slot {}", slot);
    }

    // Encode DER to PEM
    let b64 = BASE64_STANDARD.encode(der);
    // Wrap at 64 chars per line (PEM convention)
    let wrapped: String = b64
        .as_bytes()
        .chunks(64)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
        wrapped
    ))
}

/// Parse the result of ykman openpgp keys attest. Separated for testability.
///
/// Kept with `#[allow(dead_code)]` so existing unit tests remain valid.
#[allow(dead_code)]
pub fn parse_attestation_result(
    slot: &str,
    status: &std::process::ExitStatus,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<String> {
    if !status.success() {
        let stderr_str = String::from_utf8_lossy(stderr);
        anyhow::bail!("Attestation failed for slot {}: {}", slot, stderr_str.trim());
    }
    let pem = String::from_utf8_lossy(stdout).trim().to_string();
    if pem.is_empty() {
        anyhow::bail!("Attestation returned empty certificate for slot {}", slot);
    }
    Ok(pem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn success_status() -> std::process::ExitStatus {
        #[cfg(windows)]
        {
            Command::new("cmd")
                .args(["/C", "exit", "0"])
                .status()
                .unwrap()
        }
        #[cfg(not(windows))]
        {
            Command::new("true").status().unwrap()
        }
    }

    fn failure_status() -> std::process::ExitStatus {
        #[cfg(windows)]
        {
            Command::new("cmd")
                .args(["/C", "exit", "1"])
                .status()
                .unwrap()
        }
        #[cfg(not(windows))]
        {
            Command::new("false").status().unwrap()
        }
    }

    #[test]
    fn test_valid_attest_slots() {
        assert_eq!(VALID_ATTEST_SLOTS.len(), 3);
        assert!(VALID_ATTEST_SLOTS.contains(&"sig"));
        assert!(VALID_ATTEST_SLOTS.contains(&"enc"));
        assert!(VALID_ATTEST_SLOTS.contains(&"aut"));
        assert!(!VALID_ATTEST_SLOTS.contains(&"att"));
    }

    #[test]
    fn test_slot_display_name() {
        assert_eq!(slot_display_name("sig"), "Signature");
        assert_eq!(slot_display_name("enc"), "Encryption");
        assert_eq!(slot_display_name("aut"), "Authentication");
        assert_eq!(slot_display_name("att"), "Unknown");
    }

    #[test]
    fn test_slot_to_crt_tag() {
        assert_eq!(slot_to_crt_tag("sig"), Some(0xB6));
        assert_eq!(slot_to_crt_tag("enc"), Some(0xB8));
        assert_eq!(slot_to_crt_tag("aut"), Some(0xA4));
        assert_eq!(slot_to_crt_tag("att"), None);
        assert_eq!(slot_to_crt_tag("bad"), None);
    }

    #[test]
    fn test_parse_attestation_result_success() {
        let status = success_status();
        let pem = b"-----BEGIN CERTIFICATE-----\nTEST\n-----END CERTIFICATE-----";
        let result = parse_attestation_result("sig", &status, pem, b"");
        assert!(result.is_ok());
        let cert = result.unwrap();
        assert!(cert.contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn test_parse_attestation_result_empty() {
        let status = success_status();
        let result = parse_attestation_result("sig", &status, b"", b"");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("empty certificate"), "Expected 'empty certificate' in: {err}");
    }

    #[test]
    fn test_parse_attestation_result_failure() {
        let status = failure_status();
        let result = parse_attestation_result("sig", &status, b"", b"some error");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Attestation failed"), "Expected 'Attestation failed' in: {err}");
    }
}
