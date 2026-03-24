use anyhow::Result;
use std::process::Command;

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

/// Fetch attestation certificate for a given OpenPGP slot.
/// Returns PEM-encoded certificate string on success.
/// Fails if: slot is invalid, key was imported (not generated on-device), or PIN is blocked.
#[allow(dead_code)]
pub fn get_attestation_cert(slot: &str, serial: Option<u32>) -> Result<String> {
    if !VALID_ATTEST_SLOTS.contains(&slot) {
        anyhow::bail!("Invalid attestation slot '{}'. Valid: sig, enc, aut", slot);
    }

    let ykman = crate::yubikey::pin_operations::find_ykman()?;
    let mut cmd = Command::new(&ykman);
    if let Some(s) = serial {
        cmd.args(["--device", &s.to_string()]);
    }
    cmd.args(["openpgp", "keys", "attest", slot, "-"]);
    let output = cmd.output()?;

    parse_attestation_result(slot, &output.status, &output.stdout, &output.stderr)
}

/// Parse the result of ykman openpgp keys attest. Separated for testability.
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
