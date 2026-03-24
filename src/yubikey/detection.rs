use anyhow::Result;
use std::process::Command;

use super::{FormFactor, Model, Version, YubiKeyInfo, YubiKeyState};

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
    let mut version = Version { major: 0, minor: 0, patch: 0 };
    
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
        tracing::info!("Found YubiKey via gpg --card-status (FW: {}.{}.{})",
                      version.major, version.minor, version.patch);
        
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

    Ok(Some(YubiKeyState {
        info,
        openpgp,
        piv,
        pin_status,
    }))
}

fn detect_model_from_version(version: &Version) -> Model {
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
