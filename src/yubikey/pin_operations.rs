use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

/// Result of a programmatic PIN operation, containing success flag and
/// human-readable status messages collected from gpg's --status-fd output.
#[allow(dead_code)]
pub struct PinOperationResult {
    pub success: bool,
    pub messages: Vec<String>,
}

/// Change the User PIN interactively.
/// TODO(04-04): Remove — superseded by change_user_pin_programmatic.
pub fn change_user_pin() -> Result<String> {
    execute_gpg_card_edit(&["admin", "passwd", "1", "q"])
}

/// Change the Admin PIN interactively.
/// TODO(04-04): Remove — superseded by change_admin_pin_programmatic.
pub fn change_admin_pin() -> Result<String> {
    execute_gpg_card_edit(&["admin", "passwd", "3", "q"])
}

/// Set the Reset Code.
/// TODO(04-04): Remove — superseded by set_reset_code_programmatic.
pub fn set_reset_code() -> Result<String> {
    execute_gpg_card_edit(&["admin", "passwd", "4", "q"])
}

/// Unblock the User PIN.
/// TODO(04-04): Remove — superseded by unblock_user_pin_programmatic.
pub fn unblock_user_pin() -> Result<String> {
    execute_gpg_card_edit(&["admin", "passwd", "2", "q"])
}

/// Change User PIN non-interactively via gpg --pinentry-mode loopback.
///
/// Collects current PIN, new PIN, and confirmation from the caller (TUI).
/// Status messages translated from gpg's --status-fd output are returned.
#[allow(dead_code)]
pub fn change_user_pin_programmatic(
    current_pin: &str,
    new_pin: &str,
) -> Result<PinOperationResult> {
    run_gpg_pin_operation(
        &["admin", "passwd", "1", "q"],
        &[current_pin, new_pin, new_pin],
    )
}

/// Change Admin PIN non-interactively via gpg --pinentry-mode loopback.
#[allow(dead_code)]
pub fn change_admin_pin_programmatic(
    current_pin: &str,
    new_pin: &str,
) -> Result<PinOperationResult> {
    run_gpg_pin_operation(
        &["admin", "passwd", "3", "q"],
        &[current_pin, new_pin, new_pin],
    )
}

/// Set Reset Code non-interactively via gpg --pinentry-mode loopback.
///
/// `admin_pin` authenticates the operation; `reset_code` is the new value
/// (written twice for confirmation).
#[allow(dead_code)]
pub fn set_reset_code_programmatic(
    admin_pin: &str,
    reset_code: &str,
) -> Result<PinOperationResult> {
    run_gpg_pin_operation(
        &["admin", "passwd", "4", "q"],
        &[admin_pin, reset_code, reset_code],
    )
}

/// Unblock User PIN non-interactively via gpg --pinentry-mode loopback.
///
/// `reset_code_or_admin` is either the Reset Code or Admin PIN depending on
/// which unblock path is taken; `new_pin` is set twice for confirmation.
#[allow(dead_code)]
pub fn unblock_user_pin_programmatic(
    reset_code_or_admin: &str,
    new_pin: &str,
) -> Result<PinOperationResult> {
    run_gpg_pin_operation(
        &["admin", "passwd", "2", "q"],
        &[reset_code_or_admin, new_pin, new_pin],
    )
}

/// Core helper: spawn `gpg --card-edit --pinentry-mode loopback --status-fd 2
/// --command-fd 0`, feed card-edit commands via stdin, respond to GET_HIDDEN
/// prompts with PINs from `pins`, and collect status messages.
///
/// Strategy:
/// - A background thread reads stderr (the --status-fd 2 stream) line-by-line,
///   sending each line over a channel.
/// - The main thread writes card-edit commands to stdin, then enters a PIN
///   dispatch loop: for each GET_HIDDEN status received on the channel it
///   writes the next PIN to stdin.
/// - After all PINs are dispatched stdin is dropped so gpg sees EOF on its
///   command-fd, allowing the process to finish.
fn run_gpg_pin_operation(
    card_edit_commands: &[&str],
    pins: &[&str],
) -> Result<PinOperationResult> {
    use crate::yubikey::gpg_status::{parse_status_line, status_to_message, GpgStatus};
    use std::io::{BufRead, BufReader, Write};
    use std::sync::mpsc;

    let mut child = Command::new("gpg")
        .arg("--card-edit")
        .arg("--pinentry-mode")
        .arg("loopback")
        .arg("--status-fd")
        .arg("2")
        .arg("--command-fd")
        .arg("0")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().expect("stderr piped");
    let mut stdin = child.stdin.take().expect("stdin piped");

    // Write all card-edit commands to stdin first.  gpg buffers them and
    // processes them in sequence, prompting for PINs via --status-fd when
    // needed.
    for cmd in card_edit_commands {
        writeln!(stdin, "{}", cmd)?;
    }

    // Spawn a thread to drain stderr and forward lines via channel.
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(|l| l.ok()) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    // Respond to GET_HIDDEN prompts with PINs in order.
    let mut pin_iter = pins.iter();
    let mut messages: Vec<String> = Vec::new();
    let mut success = true;

    // Drain the channel until the sender thread exits.
    for line in rx {
        let status = parse_status_line(&line);
        match &status {
            GpgStatus::GetHidden { .. } => {
                // gpg is asking for the next PIN — write it to stdin.
                if let Some(pin) = pin_iter.next() {
                    if let Err(e) = writeln!(stdin, "{}", pin) {
                        // stdin may have closed early (e.g. card removed)
                        messages.push(format!("Failed to send PIN: {}", e));
                        success = false;
                    }
                }
                // Don't push a visible message for the prompt itself.
            }
            GpgStatus::Error { .. } | GpgStatus::ScOpFailure(_) => {
                let msg = status_to_message(&status);
                if !msg.is_empty() {
                    messages.push(msg);
                }
                success = false;
            }
            GpgStatus::ScOpSuccess => {
                let msg = status_to_message(&status);
                if !msg.is_empty() {
                    messages.push(msg);
                }
                // success stays true unless an Error follows
            }
            GpgStatus::PinentryLaunched
            | GpgStatus::KeyConsidered { .. }
            | GpgStatus::Unknown(_)
            | GpgStatus::GotIt => {
                // Silent — don't clutter message list
            }
            _ => {
                let msg = status_to_message(&status);
                if !msg.is_empty() {
                    messages.push(msg);
                }
            }
        }
    }

    // Drop stdin so gpg sees EOF if it is still waiting.
    drop(stdin);

    // Wait for the child process to finish.
    let exit_status = child.wait()?;
    if !exit_status.success() && success {
        // Process exited non-zero but no explicit Error status was seen.
        success = false;
        if messages.is_empty() {
            messages.push("Operation failed (gpg exited with non-zero status)".to_string());
        }
    }

    if success && messages.is_empty() {
        messages.push("Operation completed successfully".to_string());
    }

    Ok(PinOperationResult { success, messages })
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
