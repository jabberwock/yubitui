use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

/// Change the User PIN interactively
pub fn change_user_pin() -> Result<String> {
    execute_gpg_card_edit(&["admin", "passwd", "1", "q"])
}

/// Change the Admin PIN interactively
pub fn change_admin_pin() -> Result<String> {
    execute_gpg_card_edit(&["admin", "passwd", "3", "q"])
}

/// Set the Reset Code
pub fn set_reset_code() -> Result<String> {
    execute_gpg_card_edit(&["admin", "passwd", "4", "q"])
}

/// Unblock the User PIN
pub fn unblock_user_pin() -> Result<String> {
    execute_gpg_card_edit(&["admin", "passwd", "2", "q"])
}

/// Find ykman binary. Tries PATH first, then well-known Windows location.
pub fn find_ykman() -> Result<PathBuf> {
    // Try PATH first -- spawn a simple version check
    if let Ok(output) = Command::new("ykman").arg("--version").output() {
        if output.status.success() {
            return Ok(PathBuf::from("ykman"));
        }
    }

    // Well-known Windows location
    #[cfg(target_os = "windows")]
    {
        let path = PathBuf::from(r"C:\Program Files\Yubico\YubiKey Manager\ykman.exe");
        if path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "ykman not found. Install from https://www.yubico.com/support/download/yubikey-manager/"
    )
}

/// Check if ykman is available (does not run any YubiKey operations).
pub fn is_ykman_available() -> bool {
    find_ykman().is_ok()
}

/// Factory reset the OpenPGP application on the YubiKey.
/// WARNING: This destroys all stored keys, certificates, and cardholder data.
/// Requires ykman to be installed.
pub fn factory_reset_openpgp() -> Result<String> {
    let ykman_path = find_ykman()?;

    let output = Command::new(ykman_path)
        .arg("openpgp")
        .arg("reset")
        .arg("--force")
        .output()?;

    if output.status.success() {
        Ok("OpenPGP application reset successfully.\n\
            Default PINs restored:\n\
              User PIN:  123456\n\
              Admin PIN: 12345678\n\
              Reset Code: not set"
            .to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Factory reset failed: {}", stderr)
    }
}

/// Execute gpg --card-edit interactively in the terminal
fn execute_gpg_card_edit(commands: &[&str]) -> Result<String> {
    use std::io::Write;

    let mut child = Command::new("gpg")
        .arg("--card-edit")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        for cmd in commands {
            writeln!(stdin, "{}", cmd)?;
        }
    }

    let output = child.wait()?;

    if output.success() {
        Ok("Operation completed successfully".to_string())
    } else {
        Ok("Operation cancelled or failed".to_string())
    }
}
