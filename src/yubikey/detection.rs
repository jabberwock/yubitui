use anyhow::Result;
use std::process::Command;

use super::{FormFactor, Model, Version, YubiKeyInfo, YubiKeyState};

/// Parse a list of serial numbers from `ykman list --serials` output.
/// Each line is expected to contain a single decimal serial number.
pub fn parse_serial_list(output: &str) -> Vec<u32> {
    output
        .lines()
        .filter_map(|l| l.trim().parse::<u32>().ok())
        .collect()
}

/// Returns a list of serial numbers for all connected YubiKeys.
/// Uses `ykman list --serials`. Returns an empty vec if ykman is unavailable or
/// no keys are connected.
pub fn list_connected_serials() -> Result<Vec<u32>> {
    let ykman = match crate::yubikey::pin_operations::find_ykman() {
        Ok(path) => path,
        Err(_) => return Ok(vec![]),
    };
    let output = Command::new(ykman).args(["list", "--serials"]).output()?;
    if !output.status.success() {
        return Ok(vec![]);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_serial_list(&stdout))
}

/// Detect all connected YubiKey states.
/// Falls back to single-key detection when ykman is unavailable.
pub fn detect_all_yubikey_states() -> Result<Vec<YubiKeyState>> {
    let serials = list_connected_serials().unwrap_or_default();
    tracing::info!("ykman list --serials detected {} serial(s)", serials.len());

    // gpg only sees one card at a time; fall back to single detect
    match detect_yubikey_state()? {
        Some(state) => Ok(vec![state]),
        None => Ok(vec![]),
    }
}

pub fn detect_yubikeys() -> Result<Vec<YubiKeyInfo>> {
    let mut keys = Vec::new();

    // Use gpg --card-status to detect YubiKey without holding the card lock
    let output = Command::new("gpg")
        .arg("--card-status")
        .arg("--with-colons")
        .output()?;

    if !output.status.success() {
        return Ok(keys);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse serial number and version from the output
    let mut serial = 0;
    let mut version = Version {
        major: 0,
        minor: 0,
        patch: 0,
    };

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "serial" => {
                if parts.len() > 1 && !parts[1].is_empty() {
                    serial = parts[1].parse().unwrap_or(0);
                }
            }
            "version" => {
                if parts.len() > 1 && !parts[1].is_empty() {
                    // Format is "0304" for version 3.4
                    let ver_str = parts[1];
                    if ver_str.len() == 4 {
                        let major_str = &ver_str[0..2];
                        let minor_str = &ver_str[2..4];
                        version.major = major_str.parse().unwrap_or(0);
                        version.minor = minor_str.parse().unwrap_or(0);
                        version.patch = 0;
                    }
                }
            }
            _ => {}
        }
    }

    if serial != 0 {
        // Serial number intentionally omitted from logs — see CLAUDE.md security rules
        tracing::info!(
            "Found YubiKey via gpg --card-status (FW: {}.{}.{})",
            version.major,
            version.minor,
            version.patch
        );

        let model = detect_model_from_version(&version);
        let form_factor = FormFactor::UsbA;

        keys.push(YubiKeyInfo {
            serial,
            version,
            model,
            form_factor,
        });
    }

    Ok(keys)
}

pub fn detect_yubikey_state() -> Result<Option<YubiKeyState>> {
    let keys = detect_yubikeys()?;

    if keys.is_empty() {
        return Ok(None);
    }

    // For now, just use the first detected key
    let info = keys.into_iter().next().unwrap();

    // Try to get OpenPGP state (reuses the same gpg call, no card lock issues)
    let openpgp = super::openpgp::get_openpgp_state().ok();

    // Try to get PIV state
    let piv = super::piv::get_piv_state().ok();

    // Get PIN status (reuses the same gpg call)
    let pin_status = super::pin::get_pin_status()?;

    // Get touch policies via ykman openpgp info
    let touch_policies = match crate::yubikey::pin_operations::find_ykman() {
        Ok(ykman) => {
            match Command::new(ykman).args(["openpgp", "info"]).output() {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Some(super::touch_policy::parse_touch_policies(&stdout))
                }
                _ => None,
            }
        }
        Err(_) => None,
    };

    Ok(Some(YubiKeyState {
        info,
        openpgp,
        piv,
        pin_status,
        touch_policies,
    }))
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
    fn test_parse_serial_list_single() {
        let result = parse_serial_list("13390292\n");
        assert_eq!(result, vec![13390292u32]);
    }

    #[test]
    fn test_parse_serial_list_multiple() {
        let result = parse_serial_list("13390292\n99887766\n");
        assert_eq!(result, vec![13390292u32, 99887766u32]);
    }

    #[test]
    fn test_parse_serial_list_empty() {
        let result = parse_serial_list("");
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_parse_serial_list_invalid() {
        let result = parse_serial_list("not_a_number\n13390292\n");
        assert_eq!(result, vec![13390292u32]);
    }
}
