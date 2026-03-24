use anyhow::Result;
use std::process::Command;
use std::io::Write;

/// View card status
pub fn view_card_status() -> Result<String> {
    let output = Command::new("gpg")
        .arg("--card-status")
        .output()?;
    
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
    let card_status = Command::new("gpg")
        .arg("--card-status")
        .output()?;
    
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
            let output = Command::new("ssh-add")
                .arg("-L")
                .output()?;
            
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
