use anyhow::Result;
use std::io::Write;
use std::process::Command;

// ── Non-interactive key generation and import (Plan 04-03) ───────────────────

/// Key algorithm selection for the TUI key generation wizard.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyAlgorithm {
    /// Ed25519 primary key + Cv25519 encryption subkey (recommended)
    Ed25519,
    Rsa2048,
    Rsa4096,
}

impl std::fmt::Display for KeyAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyAlgorithm::Ed25519 => write!(f, "Ed25519/Cv25519 (recommended)"),
            KeyAlgorithm::Rsa2048 => write!(f, "RSA 2048"),
            KeyAlgorithm::Rsa4096 => write!(f, "RSA 4096"),
        }
    }
}

/// Parameters collected by the key generation wizard.
#[allow(dead_code)]
pub struct KeyGenParams {
    pub algorithm: KeyAlgorithm,
    /// "0" for no expiry, "1y", "2y", or "YYYY-MM-DD"
    pub expire_date: String,
    pub name: String,
    pub email: String,
    pub backup: bool,
    /// Only meaningful when backup=true
    pub backup_path: Option<String>,
}

/// Result of a programmatic key operation.
#[allow(dead_code)]
pub struct KeyOperationResult {
    pub success: bool,
    pub messages: Vec<String>,
    pub fingerprint: Option<String>,
}

/// Result of a programmatic key import operation.
#[allow(dead_code)]
pub struct ImportResult {
    pub sig_filled: bool,
    pub enc_filled: bool,
    pub aut_filled: bool,
    pub messages: Vec<String>,
}

impl ImportResult {
    /// Format slot fill status as "SIG ✓  ENC ✓  AUT —"
    #[allow(dead_code)]
    pub fn format_slots(&self) -> String {
        let check = "\u{2713}"; // ✓
        let dash = "\u{2014}";  // —
        format!(
            "SIG {}  ENC {}  AUT {}",
            if self.sig_filled { check } else { dash },
            if self.enc_filled { check } else { dash },
            if self.aut_filled { check } else { dash },
        )
    }
}

/// Information about a subkey extracted from `gpg --list-keys --with-colons`.
struct SubkeyInfo {
    /// 1-based index for gpg "key N" command
    index: usize,
    /// capability flags, e.g. "e", "s", "a", "se"
    capabilities: String,
}

/// Generate an OpenPGP key non-interactively using gpg --batch --gen-key.
///
/// Creates a temporary parameter file, spawns gpg, reads --status-fd output
/// for progress and the final KEY_CREATED fingerprint, then optionally exports
/// a backup copy. The card PIN is the protection, so %no-protection is used.
#[allow(dead_code)]
pub fn generate_key_batch(params: &KeyGenParams, _admin_pin: &str) -> Result<KeyOperationResult> {
    use crate::yubikey::gpg_status::{parse_status_line, GpgStatus};
    use std::io::BufRead;

    // Build the batch parameter file content
    let (key_type, key_length, key_curve, subkey_type, subkey_length, subkey_curve) =
        match params.algorithm {
            KeyAlgorithm::Ed25519 => (
                "EDDSA",
                "",
                "ed25519",
                "ECDH",
                "",
                "cv25519",
            ),
            KeyAlgorithm::Rsa2048 => ("RSA", "2048", "", "RSA", "2048", ""),
            KeyAlgorithm::Rsa4096 => ("RSA", "4096", "", "RSA", "4096", ""),
        };

    let mut batch = String::new();
    batch.push_str("%echo Generating OpenPGP key\n");
    batch.push_str(&format!("Key-Type: {}\n", key_type));
    if !key_length.is_empty() {
        batch.push_str(&format!("Key-Length: {}\n", key_length));
    }
    if !key_curve.is_empty() {
        batch.push_str(&format!("Key-Curve: {}\n", key_curve));
    }
    batch.push_str(&format!("Subkey-Type: {}\n", subkey_type));
    if !subkey_length.is_empty() {
        batch.push_str(&format!("Subkey-Length: {}\n", subkey_length));
    }
    if !subkey_curve.is_empty() {
        batch.push_str(&format!("Subkey-Curve: {}\n", subkey_curve));
    }
    batch.push_str(&format!("Name-Real: {}\n", params.name));
    batch.push_str(&format!("Name-Email: {}\n", params.email));
    batch.push_str(&format!("Expire-Date: {}\n", params.expire_date));
    batch.push_str("%no-protection\n");
    batch.push_str("%commit\n");
    batch.push_str("%echo done\n");

    // Write batch file to temp dir
    let tmp_path = {
        let mut p = std::env::temp_dir();
        p.push(format!("yubitui-keygen-{}.txt", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()));
        p
    };
    std::fs::write(&tmp_path, &batch)?;

    let mut child = Command::new("gpg")
        .arg("--batch")
        .arg("--status-fd")
        .arg("2")
        .arg("--gen-key")
        .arg(&tmp_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().expect("stderr piped");

    // Read stderr for status lines
    let mut messages: Vec<String> = Vec::new();
    let mut fingerprint: Option<String> = None;

    let reader = std::io::BufReader::new(stderr);
    for line in reader.lines().map_while(|l| l.ok()) {
        let status = parse_status_line(&line);
        match &status {
            GpgStatus::KeyCreated { key_type: kt, fingerprint: fp } => {
                if kt == "P" || kt == "B" {
                    // Primary key fingerprint
                    fingerprint = Some(fp.clone());
                }
                messages.push(format!("Key created ({}): {}", kt, fp));
            }
            _ => {
                let msg = crate::yubikey::gpg_status::status_to_message(&status);
                if !msg.is_empty() {
                    messages.push(msg);
                }
            }
        }
    }

    let exit_status = child.wait()?;

    // Clean up temp file
    let _ = std::fs::remove_file(&tmp_path);

    let success = exit_status.success();

    if !success {
        messages.push("Key generation failed. Check gpg is installed and working.".to_string());
        return Ok(KeyOperationResult { success: false, messages, fingerprint: None });
    }

    // Optionally export backup
    if params.backup {
        if let (Some(fp), Some(ref backup_path)) = (&fingerprint, &params.backup_path) {
            let backup_result = Command::new("gpg")
                .arg("--export-secret-keys")
                .arg("--armor")
                .arg("--output")
                .arg(backup_path)
                .arg("--")
                .arg(fp)
                .output();

            match backup_result {
                Ok(out) if out.status.success() => {
                    messages.push(format!("Backup exported to {}", backup_path));
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    messages.push(format!("Backup export failed: {}", err.trim()));
                }
                Err(e) => {
                    messages.push(format!("Backup export error: {}", e));
                }
            }
        }
    }

    Ok(KeyOperationResult { success, messages, fingerprint })
}

/// Import a key to the YubiKey non-interactively via gpg --edit-key with
/// --command-fd 0, auto-mapping subkeys by capability (S→SIG, E→ENC, A→AUT).
///
/// Per D-12 and D-13: no subkey picker shown to the user. Subkeys are mapped
/// by their capability flags in the colon-format key listing.
#[allow(dead_code)]
pub fn import_key_programmatic(key_id: &str, admin_pin: &str) -> Result<ImportResult> {
    use crate::yubikey::gpg_status::{parse_status_line, status_to_message, GpgStatus};
    use std::io::BufRead;
    use std::sync::mpsc;

    // Validate key_id
    if key_id.is_empty() {
        anyhow::bail!("key_id must not be empty");
    }
    if key_id.starts_with('-') {
        anyhow::bail!("Invalid key_id: must not start with '-'");
    }

    // Discover subkey capabilities before spawning gpg --edit-key
    let subkeys = parse_subkey_capabilities(key_id)?;

    // Build the slot mapping: capability → (subkey_index, card_slot)
    // gpg card slots: 1=SIG, 2=ENC, 3=AUT
    let mut sig_subkey: Option<usize> = None;
    let mut enc_subkey: Option<usize> = None;
    let mut aut_subkey: Option<usize> = None;

    for sk in &subkeys {
        let caps = sk.capabilities.to_ascii_lowercase();
        if caps.contains('s') && sig_subkey.is_none() {
            sig_subkey = Some(sk.index);
        }
        if caps.contains('e') && enc_subkey.is_none() {
            enc_subkey = Some(sk.index);
        }
        if caps.contains('a') && aut_subkey.is_none() {
            aut_subkey = Some(sk.index);
        }
    }

    let sig_filled = sig_subkey.is_some();
    let enc_filled = enc_subkey.is_some();
    let aut_filled = aut_subkey.is_some();

    // Build the ordered command sequence for gpg --edit-key.
    // For each subkey to move: "key N\nkeytocard\nSLOT_NUMBER\n"
    // gpg reads slot_number (1-3) from --command-fd when prompted.
    // After all moves: "save\n"
    let mut card_commands: Vec<String> = Vec::new();
    for (maybe_idx, slot) in [
        (sig_subkey, 1u8),
        (enc_subkey, 2u8),
        (aut_subkey, 3u8),
    ] {
        if let Some(idx) = maybe_idx {
            card_commands.push(format!("key {}", idx));
            card_commands.push("keytocard".to_string());
            card_commands.push(slot.to_string());
        }
    }
    card_commands.push("save".to_string());

    let mut child = Command::new("gpg")
        .arg("--edit-key")
        .arg("--pinentry-mode")
        .arg("loopback")
        .arg("--status-fd")
        .arg("2")
        .arg("--command-fd")
        .arg("0")
        .arg("--")
        .arg(key_id)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().expect("stderr piped");
    let mut stdin = child.stdin.take().expect("stdin piped");

    // Write all card-edit commands upfront; gpg buffers and processes them
    // in sequence, requesting PINs via --status-fd GET_HIDDEN when needed.
    for cmd in &card_commands {
        writeln!(stdin, "{}", cmd)?;
    }

    // Spawn a thread to drain stderr and forward lines via channel.
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines().map_while(|l| l.ok()) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    // Process status lines; respond to GET_HIDDEN with the admin PIN.
    let mut messages: Vec<String> = Vec::new();
    let admin_pin = admin_pin.to_string();

    for line in rx {
        let status = parse_status_line(&line);
        match &status {
            GpgStatus::GetHidden { .. } => {
                // gpg is asking for the admin PIN
                if writeln!(stdin, "{}", admin_pin).is_err() {
                    break;
                }
            }
            _ => {
                let msg = status_to_message(&status);
                if !msg.is_empty() {
                    messages.push(msg);
                }
            }
        }
    }

    // Drop stdin to signal EOF to gpg's --command-fd
    drop(stdin);

    let exit_status = child.wait()?;

    if !exit_status.success() && messages.is_empty() {
        messages.push("Import operation failed. Check key ID and admin PIN.".to_string());
    }

    Ok(ImportResult {
        sig_filled,
        enc_filled,
        aut_filled,
        messages,
    })
}

/// Parse subkey capability flags from `gpg --list-keys --with-colons` output.
///
/// Returns a list of SubkeyInfo with 1-based indices (for gpg "key N" command)
/// and capability flag strings extracted from colon-record field 12.
fn parse_subkey_capabilities(key_id: &str) -> Result<Vec<SubkeyInfo>> {
    let output = Command::new("gpg")
        .arg("--list-keys")
        .arg("--with-colons")
        .arg("--")
        .arg(key_id)
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Could not list key {}: {}", key_id, err.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut subkeys = Vec::new();
    let mut index = 1usize; // 1-based subkey index

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.is_empty() {
            continue;
        }
        if parts[0] == "sub" || parts[0] == "ssb" {
            // Field 12 (0-indexed: 11) contains capability flags
            let caps = parts.get(11).copied().unwrap_or("").to_string();
            subkeys.push(SubkeyInfo { index, capabilities: caps });
            index += 1;
        }
    }

    Ok(subkeys)
}

/// Parse subkey capabilities from colons output (pub for testing).
#[allow(dead_code)]
pub fn parse_subkey_capabilities_pub(output: &str) -> Vec<(usize, String)> {
    let mut result = Vec::new();
    let mut index = 1usize;
    for line in output.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.is_empty() {
            continue;
        }
        if parts[0] == "sub" || parts[0] == "ssb" {
            let caps = parts.get(11).copied().unwrap_or("").to_string();
            result.push((index, caps));
            index += 1;
        }
    }
    result
}

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

pub fn parse_ykman_openpgp_info(output: &str) -> Result<KeyAttributes> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ykman_openpgp_info_full() {
        let input = "\
OpenPGP version:            3.4\n\
Application version:        5.2.7\n\
PIN tries remaining:        3\n\
Reset code tries remaining: 0\n\
Admin PIN tries remaining:  3\n\
SIG key:\n\
  Fingerprint: ABCDEF0123456789\n\
  Algorithm:   ed25519\n\
ENC key:\n\
  Fingerprint: 1234567890ABCDEF\n\
  Algorithm:   cv25519\n\
AUT key:\n\
  Fingerprint: FEDCBA9876543210\n\
  Algorithm:   ed25519\n\
";
        let attrs = parse_ykman_openpgp_info(input).unwrap();
        let sig = attrs.signature.as_ref().unwrap();
        assert_eq!(sig.algorithm, "ed25519");
        assert_eq!(sig.fingerprint, "ABCDEF0123456789");
        let enc = attrs.encryption.as_ref().unwrap();
        assert_eq!(enc.algorithm, "cv25519");
        assert_eq!(enc.fingerprint, "1234567890ABCDEF");
        let aut = attrs.authentication.as_ref().unwrap();
        assert_eq!(aut.algorithm, "ed25519");
        assert_eq!(aut.fingerprint, "FEDCBA9876543210");
    }

    #[test]
    fn test_parse_ykman_openpgp_info_empty_slots() {
        let input = "SIG key:\nENC key:\nAUT key:\n";
        let attrs = parse_ykman_openpgp_info(input).unwrap();
        assert!(attrs.signature.is_none());
        assert!(attrs.encryption.is_none());
        assert!(attrs.authentication.is_none());
    }

    #[test]
    fn test_parse_ykman_openpgp_info_partial() {
        let input = "\
SIG key:\n\
  Fingerprint: AABBCCDD11223344\n\
  Algorithm:   rsa2048\n\
ENC key:\n\
AUT key:\n\
";
        let attrs = parse_ykman_openpgp_info(input).unwrap();
        assert!(attrs.signature.is_some());
        let sig = attrs.signature.as_ref().unwrap();
        assert_eq!(sig.algorithm, "rsa2048");
        assert!(attrs.encryption.is_none());
        assert!(attrs.authentication.is_none());
    }
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
