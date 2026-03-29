use anyhow::Result;
use ctap_hid_fido2::{FidoKeyHidFactory, LibCfg};

// ============================================================================
// CTAP HID Constants for raw reset frame
// ============================================================================

/// CTAPHID command for channel allocation (CTAPHID_INIT)
const CTAPHID_INIT: u8 = 0x06;
/// CTAPHID command for CTAP2 CBOR tunneling (CTAPHID_CBOR)
const CTAPHID_CBOR: u8 = 0x10;
/// Broadcast channel ID — used before a real channel is allocated
const BROADCAST_CID: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
/// FIDO HID usage page (0xF1D0 per CTAP HID spec)
const FIDO_USAGE_PAGE: u16 = 0xF1D0;
/// FIDO HID usage (0x01 per CTAP HID spec)
const FIDO_USAGE: u16 = 0x01;

/// CTAP2 status: success
const CTAP2_OK: u8 = 0x00;
/// CTAP2 error: not allowed (device was not freshly plugged in — 10s window expired)
const CTAP2_ERR_NOT_ALLOWED: u8 = 0x2E;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct Fido2State {
    /// Firmware version formatted as "major.minor.patch" (e.g. "5.4.3").
    /// None if the device reports firmware_version as 0 (unknown).
    pub firmware_version: Option<String>,
    /// Supported algorithms (e.g. ["ES256", "EdDSA"])
    pub algorithms: Vec<String>,
    /// Whether a FIDO2 PIN has been configured (from options["clientPin"])
    pub pin_is_set: bool,
    /// Remaining PIN retry count from get_pin_retries()
    pub pin_retry_count: u8,
    /// Resident credentials — None means locked (PIN required but not provided)
    pub credentials: Option<Vec<Fido2Credential>>,
    /// Whether credentialManagement extension is supported (credMgmt or credentialMgmtPreview)
    pub supports_cred_mgmt: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Fido2Credential {
    /// Relying party ID (e.g. "github.com")
    pub rp_id: String,
    /// Relying party display name (e.g. "GitHub") — empty string becomes None
    pub rp_name: Option<String>,
    /// User display name (falls back to display_name, then empty string)
    pub user_name: String,
    /// Raw credential ID bytes — needed for delete operation
    pub credential_id: Vec<u8>,
}

// ============================================================================
// Device Access
// ============================================================================

/// Open a fresh FIDO2 HID connection.
///
/// On Windows, HID access requires administrator privileges. If the error
/// message contains "access" or "permission", a hint is appended.
pub fn get_fido2_device() -> Result<ctap_hid_fido2::FidoKeyHid> {
    FidoKeyHidFactory::create(&LibCfg::init()).map_err(|e| {
        let msg = e.to_string();
        let lower = msg.to_lowercase();
        if lower.contains("access") || lower.contains("permission") {
            anyhow::anyhow!(
                "FIDO2 device error: {} — on Windows, run yubitui as Administrator \
                (FIDO2/HID access requires elevated privileges)",
                msg
            )
        } else {
            anyhow::anyhow!("FIDO2 device error: {}", msg)
        }
    })
}

// ============================================================================
// Operations
// ============================================================================

/// Fetch FIDO2 device info without credentials (no PIN required).
///
/// Returns Fido2State with credentials: None — use enumerate_credentials() to
/// load the passkey list after authenticating with a PIN.
pub fn get_fido2_info() -> Result<Fido2State> {
    let device = get_fido2_device()?;
    let info = device.get_info()?;

    let pin_is_set = info
        .options
        .iter()
        .find(|(k, _)| k == "clientPin")
        .map(|(_, v)| *v)
        .unwrap_or(false);

    // get_pin_retries() returns CTAP2_ERR_PIN_NOT_SET (0x35) when no PIN is configured.
    // Treat that as 0 retries rather than propagating the error.
    let pin_retry_count = if pin_is_set {
        device.get_pin_retries().unwrap_or(0)
    } else {
        0
    };

    let supports_cred_mgmt = info
        .options
        .iter()
        .any(|(k, v)| (k == "credMgmt" || k == "credentialMgmtPreview") && *v);

    // algorithms is Vec<(type_str, alg_str)> — extract just the algorithm name
    let algorithms: Vec<String> = info.algorithms.iter().map(|(_, alg)| alg.clone()).collect();

    // firmware_version is u32; 0 means not reported
    let firmware_version = if info.firmware_version == 0 {
        None
    } else {
        let v = info.firmware_version;
        let major = (v >> 16) & 0xFF;
        let minor = (v >> 8) & 0xFF;
        let patch = v & 0xFF;
        Some(format!("{}.{}.{}", major, minor, patch))
    };

    Ok(Fido2State {
        firmware_version,
        algorithms,
        pin_is_set,
        pin_retry_count: pin_retry_count as u8,
        credentials: None,
        supports_cred_mgmt,
    })
}

/// Enumerate all resident credentials using the provided PIN.
///
/// Performs the two-step enumerate_rps → enumerate_credentials protocol
/// required by CTAP 2.1 credentialManagement.
pub fn enumerate_credentials(pin: &str) -> Result<Vec<Fido2Credential>> {
    let device = get_fido2_device()?;
    let rps = device.credential_management_enumerate_rps(Some(pin))?;

    let mut all_credentials = Vec::new();
    for rp in &rps {
        let rp_creds =
            device.credential_management_enumerate_credentials(Some(pin), &rp.rpid_hash)?;
        for cred in rp_creds {
            let rp_id = rp.public_key_credential_rp_entity.id.clone();
            let rp_name = {
                let n = &rp.public_key_credential_rp_entity.name;
                if n.is_empty() {
                    None
                } else {
                    Some(n.clone())
                }
            };
            // user_entity.name and display_name are String (not Option<String>)
            let user_name = {
                let name = &cred.public_key_credential_user_entity.name;
                if !name.is_empty() {
                    name.clone()
                } else {
                    cred.public_key_credential_user_entity.display_name.clone()
                }
            };
            let credential_id = cred.public_key_credential_descriptor.id.clone();
            all_credentials.push(Fido2Credential {
                rp_id,
                rp_name,
                user_name,
                credential_id,
            });
        }
    }

    Ok(all_credentials)
}

/// Delete a resident credential identified by its raw credential ID bytes.
///
/// Requires the FIDO2 PIN. The credential_id must match exactly the bytes
/// returned from enumerate_credentials().
pub fn delete_credential(pin: &str, credential_id: &[u8]) -> Result<()> {
    use ctap_hid_fido2::public_key_credential_descriptor::PublicKeyCredentialDescriptor;
    let device = get_fido2_device()?;
    let pkcd = PublicKeyCredentialDescriptor {
        id: credential_id.to_vec(),
        ctype: "public-key".to_string(),
    };
    device.credential_management_delete_credential(Some(pin), pkcd)?;
    Ok(())
}

/// Set a new FIDO2 PIN when no PIN is currently configured.
pub fn set_pin(new_pin: &str) -> Result<()> {
    let device = get_fido2_device()?;
    device.set_new_pin(new_pin)?;
    Ok(())
}

/// Change an existing FIDO2 PIN.
pub fn change_pin(current_pin: &str, new_pin: &str) -> Result<()> {
    let device = get_fido2_device()?;
    device.change_pin(current_pin, new_pin)?;
    Ok(())
}

// ============================================================================
// FIDO2 Reset — raw HID authenticatorReset (command 0x07)
// ============================================================================

/// Find the HID path of the first FIDO2 device (usage_page=0xF1D0, usage=0x01).
///
/// Returns the device path as a CString-safe string for use with hidapi::open_path().
pub fn find_fido_hid_device_path() -> Result<std::ffi::CString> {
    let api = hidapi::HidApi::new()
        .map_err(|e| anyhow::anyhow!("Failed to open HID API: {}", e))?;
    for device_info in api.device_list() {
        if device_info.usage_page() == FIDO_USAGE_PAGE && device_info.usage() == FIDO_USAGE {
            let path = device_info.path().to_owned();
            return Ok(path);
        }
    }
    Err(anyhow::anyhow!("No FIDO HID device found"))
}

/// Returns true if a FIDO2 HID device is currently present (plugged in).
pub fn is_fido_device_present() -> bool {
    find_fido_hid_device_path().is_ok()
}

/// Send authenticatorReset (CTAP2 command 0x07) via raw CTAPHID frames.
///
/// The FIDO2 spec requires the reset command to arrive within 10 seconds
/// of the device being powered on (USB insertion). This function:
/// 1. Sends CTAPHID_INIT on broadcast channel to get a channel ID
/// 2. Sends CTAPHID_CBOR with payload 0x07 (authenticatorReset) on that channel
/// 3. Parses the response status byte
///
/// Returns:
/// - Ok(()) on success (status 0x00)
/// - Err with "Reset not allowed" message if outside the 10-second window (status 0x2E)
/// - Err with status code for any other error
pub fn reset_fido2() -> Result<()> {
    let path = find_fido_hid_device_path()?;
    let api = hidapi::HidApi::new()
        .map_err(|e| anyhow::anyhow!("Failed to open HID API: {}", e))?;
    let device = api
        .open_path(&path)
        .map_err(|e| anyhow::anyhow!("Failed to open FIDO HID device: {}", e))?;

    // --- Step 1: CTAPHID_INIT on broadcast channel ---
    // Packet: [broadcast_cid(4), 0x80|CTAPHID_INIT(1), bcnt_hi(1), bcnt_lo(1), nonce(8), pad]
    // Total: 64 bytes (standard CTAPHID report size)
    // nonce is 8 bytes — not security-critical; use fixed value
    let nonce: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let mut init_packet = [0u8; 65]; // 65 bytes: first byte is report ID (0x00 for HID)
    init_packet[0] = 0x00; // HID report ID
    init_packet[1] = BROADCAST_CID[0];
    init_packet[2] = BROADCAST_CID[1];
    init_packet[3] = BROADCAST_CID[2];
    init_packet[4] = BROADCAST_CID[3];
    init_packet[5] = 0x80 | CTAPHID_INIT;
    init_packet[6] = 0x00; // bcnt_hi
    init_packet[7] = 0x08; // bcnt_lo = 8 (nonce length)
    init_packet[8..16].copy_from_slice(&nonce);
    // rest is zero padding

    device
        .write(&init_packet)
        .map_err(|e| anyhow::anyhow!("CTAPHID_INIT write failed: {}", e))?;

    let mut response = [0u8; 64];
    let n = device
        .read_timeout(&mut response, 1000)
        .map_err(|e| anyhow::anyhow!("CTAPHID_INIT read failed: {}", e))?;
    if n < 17 {
        return Err(anyhow::anyhow!(
            "CTAPHID_INIT response too short: {} bytes",
            n
        ));
    }

    // Response layout: [cid(4), cmd(1), bcnt_hi(1), bcnt_lo(1), nonce_echo(8), new_cid(4), ...]
    // new_cid is at bytes [15..19] of the 64-byte response
    let channel_id: [u8; 4] = [response[15], response[16], response[17], response[18]];

    // --- Step 2: CTAPHID_CBOR with authenticatorReset (0x07) ---
    // Packet: [channel_id(4), 0x80|CTAPHID_CBOR(1), 0x00, 0x01, 0x07, pad to 64]
    // bcnt = 1 (one byte payload), payload = 0x07 = authenticatorReset command
    let mut cbor_packet = [0u8; 65];
    cbor_packet[0] = 0x00; // HID report ID
    cbor_packet[1] = channel_id[0];
    cbor_packet[2] = channel_id[1];
    cbor_packet[3] = channel_id[2];
    cbor_packet[4] = channel_id[3];
    cbor_packet[5] = 0x80 | CTAPHID_CBOR;
    cbor_packet[6] = 0x00; // bcnt_hi
    cbor_packet[7] = 0x01; // bcnt_lo = 1 (single CTAP2 command byte)
    cbor_packet[8] = 0x07; // authenticatorReset = command 0x07
    // rest is zero padding

    device
        .write(&cbor_packet)
        .map_err(|e| anyhow::anyhow!("CTAPHID_CBOR reset write failed: {}", e))?;

    // Read response — timeout 30s to allow user presence check on some devices
    let mut cbor_response = [0u8; 64];
    let m = device
        .read_timeout(&mut cbor_response, 30_000)
        .map_err(|e| anyhow::anyhow!("CTAPHID_CBOR reset read failed: {}", e))?;
    if m < 8 {
        return Err(anyhow::anyhow!(
            "CTAPHID_CBOR response too short: {} bytes",
            m
        ));
    }

    // Response layout: [cid(4), cmd(1), bcnt_hi(1), bcnt_lo(1), status(1), ...]
    // Status byte is at index 7 of the 64-byte response buffer
    let status = cbor_response[7];
    match status {
        CTAP2_OK => Ok(()),
        CTAP2_ERR_NOT_ALLOWED => Err(anyhow::anyhow!(
            "Reset not allowed — device must be freshly plugged in (within 10 seconds of USB insertion)"
        )),
        other => Err(anyhow::anyhow!(
            "Authenticator reset failed with CTAP2 error: 0x{:02X}",
            other
        )),
    }
}
