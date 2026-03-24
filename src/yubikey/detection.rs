use anyhow::{Context, Result};
use pcsc::{Card, Context as PcscContext, Protocols, Scope, ShareMode};
use std::ffi::CString;

use super::{FormFactor, Model, Version, YubiKeyInfo, YubiKeyState};

pub fn detect_yubikeys() -> Result<Vec<YubiKeyInfo>> {
    let ctx = PcscContext::establish(Scope::System)?;
    let mut readers_buf = [0; 2048];
    let mut readers = ctx.list_readers(&mut readers_buf)?;

    let mut keys = Vec::new();

    for reader in readers {
        if let Ok(reader_str) = reader.to_str() {
            if let Ok(info) = detect_yubikey_from_reader(&ctx, reader_str) {
                keys.push(info);
            }
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

fn detect_yubikey_from_reader(ctx: &PcscContext, reader: &str) -> Result<YubiKeyInfo> {
    // Only process Yubico readers
    if !reader.to_lowercase().contains("yubico") && !reader.to_lowercase().contains("yubikey") {
        anyhow::bail!("Not a YubiKey reader");
    }

    let reader_cstr = CString::new(reader)?;
    let card = ctx
        .connect(&reader_cstr, ShareMode::Shared, Protocols::T1)?;

    let serial = get_serial(&card)?;
    let version = get_version(&card)?;
    let model = detect_model(&version, reader);
    let form_factor = detect_form_factor(&model, reader);

    Ok(YubiKeyInfo {
        serial,
        version,
        model,
        form_factor,
    })
}

fn get_serial(card: &Card) -> Result<u32> {
    // PIV GET DATA for serial number (0x5FC102)
    let apdu = [
        0x00, 0xF8, 0x00, 0x00, // CLA INS P1 P2
    ];

    let mut response = [0u8; 256];
    let response_slice = card
        .transmit(&apdu, &mut response)
        .context("Failed to read serial number")?;

    if response_slice.len() < 4 {
        anyhow::bail!("Invalid serial number response");
    }

    // Serial is 4 bytes big-endian
    let serial = u32::from_be_bytes([
        response_slice[0],
        response_slice[1],
        response_slice[2],
        response_slice[3],
    ]);

    Ok(serial)
}

fn get_version(card: &Card) -> Result<Version> {
    // YubiKey-specific APDU to get firmware version
    let apdu = [
        0x00, 0xF1, 0x00, 0x00, // CLA INS P1 P2
    ];

    let mut response = [0u8; 256];
    let response_slice = card
        .transmit(&apdu, &mut response)
        .context("Failed to read version")?;

    if response_slice.len() < 3 {
        anyhow::bail!("Invalid version response");
    }

    Ok(Version {
        major: response_slice[0],
        minor: response_slice[1],
        patch: response_slice[2],
    })
}

fn detect_model(version: &Version, reader_name: &str) -> Model {
    let reader_lower = reader_name.to_lowercase();

    // YubiKey 5 series
    if version.major >= 5 {
        if reader_lower.contains("nano") || reader_lower.contains("5 nano") {
            if reader_lower.contains("5c") {
                return Model::YubiKey5CNano;
            }
            return Model::YubiKey5Nano;
        }
        if reader_lower.contains("5ci") {
            return Model::YubiKey5Ci;
        }
        if reader_lower.contains("5c") {
            return Model::YubiKey5C;
        }
        if reader_lower.contains("nfc") {
            return Model::YubiKey5NFC;
        }
        return Model::YubiKey5;
    }

    // YubiKey 4 series
    if version.major == 4 {
        if reader_lower.contains("nano") {
            return Model::YubiKey4Nano;
        }
        if reader_lower.contains("4c") {
            return Model::YubiKey4C;
        }
        return Model::YubiKey4;
    }

    // YubiKey NEO
    if version.major == 3 || reader_lower.contains("neo") {
        return Model::YubiKeyNeo;
    }

    Model::Unknown
}

fn detect_form_factor(model: &Model, reader_name: &str) -> FormFactor {
    let reader_lower = reader_name.to_lowercase();

    if reader_lower.contains("nano") {
        return FormFactor::Nano;
    }

    match model {
        Model::YubiKey5C | Model::YubiKey5Ci | Model::YubiKey5CNano | Model::YubiKey4C => {
            FormFactor::UsbC
        }
        Model::YubiKey5Nano | Model::YubiKey4Nano => FormFactor::Nano,
        _ => FormFactor::UsbA,
    }
}
