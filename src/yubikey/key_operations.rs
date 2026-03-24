use anyhow::Result;
use std::io::Write;
use std::process::Command;

/// Key attributes for each OpenPGP slot, parsed from ykman openpgp info.
#[derive(Debug, Clone, Default)]
pub struct KeyAttributes {
    pub signature: Option<SlotInfo>,
    pub encryption: Option<SlotInfo>,
    pub authentication: Option<SlotInfo>,
}

#[derive(Debug, Clone)]
pub struct SlotInfo {
    pub algorithm: String,   // e.g., "RSA2048", "ed25519", "NIST P-256"
    pub fingerprint: String, // short hex fingerprint
}

/// Fetch key attributes from ykman openpgp info.
/// Returns structured info about algorithm type per slot.
/// Requires ykman to be installed.
pub fn get_key_attributes() -> Result<KeyAttributes> {
    let ykman_path = crate::yubikey::pin_operations::find_ykman()?;

    let output = Command::new(ykman_path)
        .arg("openpgp")
        .arg("info")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ykman openpgp info failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_ykman_openpgp_info(&stdout)
}

fn parse_ykman_openpgp_info(output: &str) -> Result<KeyAttributes> {
    let mut attrs = KeyAttributes::default();

    // ykman openpgp info output format (example):
    // OpenPGP version:            3.4
    // Application version:        5.2.7
    // PIN tries remaining:        3
    // Reset code tries remaining: 0
    // Admin PIN tries remaining:  3
    // ...
    // SIG key:
    //   Fingerprint: ABCD...
    //   Algorithm:   ed25519
    // ENC key:
    //   Fingerprint: 1234...
    //   Algorithm:   cv25519
    // AUT key:
    //   Fingerprint: 5678...
    //   Algorithm:   ed25519

    let mut current_slot: Option<&str> = None;
    let mut current_algo = String::new();
    let mut current_fp = String::new();

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("SIG key") {
            if let Some(slot) = current_slot {
                save_slot(&mut attrs, slot, &current_algo, &current_fp);
            }
            current_slot = Some("SIG");
            current_algo.clear();
            current_fp.clear();
        } else if trimmed.starts_with("ENC key") {
            if let Some(slot) = current_slot {
                save_slot(&mut attrs, slot, &current_algo, &current_fp);
            }
            current_slot = Some("ENC");
            current_algo.clear();
            current_fp.clear();
        } else if trimmed.starts_with("AUT key") {
            if let Some(slot) = current_slot {
                save_slot(&mut attrs, slot, &current_algo, &current_fp);
            }
            current_slot = Some("AUT");
            current_algo.clear();
            current_fp.clear();
        } else if current_slot.is_some() {
            if trimmed.starts_with("Algorithm:") {
                current_algo = trimmed.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if trimmed.starts_with("Fingerprint:") {
                current_fp = trimmed.split(':').nth(1).unwrap_or("").trim().to_string();
            }
        }
    }
    // Save last slot
    if let Some(slot) = current_slot {
        save_slot(&mut attrs, slot, &current_algo, &current_fp);
    }

    Ok(attrs)
}

fn save_slot(attrs: &mut KeyAttributes, slot: &str, algo: &str, fp: &str) {
    if algo.is_empty() && fp.is_empty() {
        return; // No key in this slot
    }
    let info = SlotInfo {
        algorithm: if algo.is_empty() {
            "Unknown".to_string()
        } else {
            algo.to_string()
        },
        fingerprint: if fp.is_empty() {
            "N/A".to_string()
        } else {
            fp.to_string()
        },
    };
    match slot {
        "SIG" => attrs.signature = Some(info),
        "ENC" => attrs.encryption = Some(info),
        "AUT" => attrs.authentication = Some(info),
        _ => {}
    }
}

/// Get SSH public key as text without interactive terminal output.
/// Uses gpg --export-ssh-key to get the key in authorized_keys format.
pub fn get_ssh_public_key_text() -> Result<String> {
    // First get the authentication key fingerprint from card status
    let card_output = Command::new("gpg").arg("--card-status").output()?;

    if !card_output.status.success() {
        anyhow::bail!("Could not read card status");
    }

    let stdout = String::from_utf8_lossy(&card_output.stdout);

    // Find authentication key fingerprint
    // Look for line after "Authentication key" containing fingerprint
    let mut found_auth = false;
    let mut auth_fp = String::new();
    for line in stdout.lines() {
        if line.contains("Authentication key") {
            // The fingerprint is on the next non-empty line or same line
            // Format varies: "Authentication key ....: XXXX XXXX XXXX..."
            if let Some(fp_part) = line.split(':').nth(1) {
                auth_fp = fp_part.trim().replace(' ', "");
                if !auth_fp.is_empty() {
                    found_auth = true;
                }
            }
        }
    }

    if !found_auth || auth_fp.is_empty() {
        anyhow::bail!("No authentication key found on card. Import or generate a key first.");
    }

    // Export as SSH key using the fingerprint.
    // `--` prevents the fingerprint from being interpreted as a flag.
    let ssh_output = Command::new("gpg")
        .arg("--export-ssh-key")
        .arg("--")
        .arg(&auth_fp)
        .output()?;

    if !ssh_output.status.success() {
        let stderr = String::from_utf8_lossy(&ssh_output.stderr);
        anyhow::bail!("Could not export SSH key: {}", stderr);
    }

    let key = String::from_utf8_lossy(&ssh_output.stdout)
        .trim()
        .to_string();
    if key.is_empty() {
        anyhow::bail!("SSH key export returned empty result");
    }

    Ok(key)
}

/// View card status
pub fn view_card_status() -> Result<String> {
    let output = Command::new("gpg").arg("--card-status").output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Import a key to YubiKey (interactive)
pub fn import_key_to_card(key_id: &str) -> Result<String> {
    // Validate key_id: must be non-empty and must not start with '-' (GPG flag injection)
    if key_id.is_empty() {
        anyhow::bail!("key_id must not be empty");
    }
    if key_id.starts_with('-') {
        anyhow::bail!("Invalid key_id: must not start with '-'");
    }

    let mut child = Command::new("gpg")
        .arg("--edit-key")
        .arg("--")
        .arg(key_id)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "keytocard")?;
        writeln!(stdin, "save")?;
    }

    let output = child.wait()?;

    if output.success() {
        Ok("Key imported to card successfully".to_string())
    } else {
        Ok("Operation cancelled or failed".to_string())
    }
}

/// Generate a key on the YubiKey (interactive)
pub fn generate_key_on_card() -> Result<String> {
    let mut child = Command::new("gpg")
        .arg("--card-edit")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "admin")?;
        writeln!(stdin, "generate")?;
        writeln!(stdin, "quit")?;
    }

    let output = child.wait()?;

    if output.success() {
        Ok("Key generated on card successfully".to_string())
    } else {
        Ok("Operation cancelled or failed".to_string())
    }
}

/// Export SSH public key from authentication slot
pub fn export_ssh_public_key() -> Result<String> {
    // First, get the authentication key fingerprint from card status
    let card_status = Command::new("gpg").arg("--card-status").output()?;

    let status_text = String::from_utf8_lossy(&card_status.stdout);

    // Look for the authentication key fingerprint
    let mut auth_keygrip = None;
    for line in status_text.lines() {
        if line.trim().starts_with("Authentication key:") {
            if let Some(fpr) = line.split(':').nth(1) {
                let fpr_clean = fpr.trim();
                if fpr_clean != "[none]" && !fpr_clean.is_empty() {
                    auth_keygrip = Some(fpr_clean.to_string());
                    break;
                }
            }
        }
    }

    if let Some(keygrip) = auth_keygrip {
        // Export the SSH public key; `--` prevents a leading `-` in the
        // fingerprint from being interpreted as a GPG flag (defence-in-depth —
        // fingerprints are hex, but we guard the boundary regardless).
        let output = Command::new("gpg")
            .arg("--export-ssh-key")
            .arg("--")
            .arg(&keygrip)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            // Try alternative method
            let output = Command::new("ssh-add").arg("-L").output()?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                anyhow::bail!("No authentication key found or SSH agent not configured")
            }
        }
    } else {
        anyhow::bail!("No authentication key on card")
    }
}

/// Delete/reset a key slot (interactive)
#[allow(dead_code)]
pub fn reset_key_slot() -> Result<String> {
    let mut child = Command::new("gpg")
        .arg("--card-edit")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "admin")?;
        writeln!(stdin, "factory-reset")?;
        writeln!(stdin, "quit")?;
    }

    let output = child.wait()?;

    if output.success() {
        Ok("Card reset successfully".to_string())
    } else {
        Ok("Operation cancelled or failed".to_string())
    }
}

/// List available GPG keys that can be imported
pub fn list_gpg_keys() -> Result<Vec<String>> {
    let output = Command::new("gpg")
        .arg("--list-secret-keys")
        .arg("--with-colons")
        .output()?;

    let mut keys = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.is_empty() {
            continue;
        }

        if parts[0] == "sec" || parts[0] == "ssb" {
            // Extract key ID and user ID
            if parts.len() > 4 {
                let key_id = parts[4].to_string();
                keys.push(key_id);
            }
        }
    }

    Ok(keys)
}
