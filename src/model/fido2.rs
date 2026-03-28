use anyhow::Result;
use ctap_hid_fido2::{FidoKeyHidFactory, LibCfg};

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
    let pin_retry_count = device.get_pin_retries()?;

    let pin_is_set = info
        .options
        .iter()
        .find(|(k, _)| k == "clientPin")
        .map(|(_, v)| *v)
        .unwrap_or(false);

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
