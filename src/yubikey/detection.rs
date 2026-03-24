use anyhow::{Context, Result};

use super::{FormFactor, Model, Version, YubiKeyInfo, YubiKeyState};

pub fn detect_yubikeys() -> Result<Vec<YubiKeyInfo>> {
    let mut keys = Vec::new();

    // Use the yubikey crate's built-in detection
    match yubikey::YubiKey::open() {
        Ok(mut yk) => {
            tracing::info!("Found YubiKey via yubikey crate");
            
            let serial = yk.serial().into();
            let version = yk.version();
            
            let yubikey_version = Version {
                major: version.major,
                minor: version.minor,
                patch: version.patch,
            };
            
            // Detect model based on version
            let model = detect_model_from_version(&yubikey_version);
            let form_factor = FormFactor::UsbA; // Default, hard to detect
            
            keys.push(YubiKeyInfo {
                serial,
                version: yubikey_version,
                model,
                form_factor,
            });
        }
        Err(e) => {
            tracing::debug!("Could not open YubiKey: {:?}", e);
        }
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

    // Try to get OpenPGP state
    let openpgp = super::openpgp::get_openpgp_state().ok();

    // Try to get PIV state
    let piv = super::piv::get_piv_state().ok();

    // Get PIN status
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
