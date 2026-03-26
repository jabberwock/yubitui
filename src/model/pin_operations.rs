use anyhow::Result;
use std::process::Command;

/// Result of a programmatic PIN operation, containing success flag and
/// human-readable status messages collected from gpg's --status-fd output.
#[allow(dead_code)]
pub struct PinOperationResult {
    pub success: bool,
    pub messages: Vec<String>,
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
/// - Commands are split into pre-PIN (written immediately) and post-PIN (the
///   final "q", written after all PINs are dispatched).  Pre-writing "q"
///   causes a deadlock: gpg reads it as the first PIN response, fails, then
///   sits at the card-edit prompt waiting for more commands while our rx loop
///   waits for stderr to close — neither side makes progress.
/// - A background thread reads stderr (the --status-fd 2 stream) line-by-line,
///   sending each line over a channel.
/// - The main thread enters a PIN dispatch loop: for each GET_HIDDEN received
///   it writes the next PIN.  After the last PIN the post-PIN commands are
///   sent and stdin is dropped so gpg sees EOF on its command-fd and exits.
/// - On error (wrong PIN / SC_OP_FAILURE) stdin is also dropped immediately to
///   prevent a second deadlock where gpg waits for commands after the failure.
fn run_gpg_pin_operation(
    card_edit_commands: &[&str],
    pins: &[&str],
) -> Result<PinOperationResult> {
    use crate::model::gpg_status::{parse_status_line, status_to_message, GpgStatus};
    use std::io::{BufRead, BufReader, Write};
    use std::sync::mpsc;

    let mut child = Command::new("gpg")
        .arg("--no-tty") // prevent gpg from writing card status to the controlling terminal
        .arg("--card-edit")
        .arg("--pinentry-mode")
        .arg("loopback")
        .arg("--status-fd")
        .arg("2")
        .arg("--command-fd")
        .arg("0")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null()) // discard — gpg writes interactive menus here; piping without draining deadlocks
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().expect("stderr piped");
    let mut stdin = child.stdin.take().expect("stdin piped");

    // Split commands: everything except the last element is written upfront;
    // the last element ("q") is written after all PINs are dispatched so it
    // cannot be mistaken for a PIN response.
    let split = if pins.is_empty() {
        card_edit_commands.len()
    } else {
        card_edit_commands.len().saturating_sub(1)
    };
    let pre_pin_cmds = &card_edit_commands[..split];
    let post_pin_cmds = &card_edit_commands[split..];

    for cmd in pre_pin_cmds {
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
    // stdin is wrapped in Option so it can be dropped early (sending EOF) once
    // all PINs are dispatched or on error, causing gpg to exit cleanly.
    let mut pin_iter = pins.iter().peekable();
    let mut messages: Vec<String> = Vec::new();
    let mut success = true;
    let mut stdin_opt: Option<std::process::ChildStdin> = Some(stdin);

    // Drain the channel until the sender thread exits.
    for line in rx {
        let status = parse_status_line(&line);
        match &status {
            GpgStatus::GetHidden { .. } => {
                // gpg is asking for the next PIN — write it to stdin.
                if let Some(pin) = pin_iter.next() {
                    if let Some(ref mut s) = stdin_opt {
                        if let Err(e) = writeln!(s, "{}", pin) {
                            // stdin may have closed early (e.g. card removed)
                            messages.push(format!("Failed to send PIN: {}", e));
                            success = false;
                        }
                    }
                    // After the last PIN: send post-PIN commands then drop stdin.
                    // Dropping stdin sends EOF to gpg's --command-fd, letting it exit.
                    if pin_iter.peek().is_none() {
                        if let Some(ref mut s) = stdin_opt {
                            for cmd in post_pin_cmds {
                                let _ = writeln!(s, "{}", cmd);
                            }
                        }
                        stdin_opt.take();
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
                // Drop stdin on error: gpg may be waiting at a submenu for
                // more commands; EOF lets it exit so stderr closes and this
                // loop can end.
                stdin_opt.take();
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
            | GpgStatus::GotIt
            | GpgStatus::GetLine { .. } => {
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

    // Drop stdin so gpg sees EOF if it is still waiting (covers the no-PIN
    // case and any early-exit paths not handled inside the loop above).
    drop(stdin_opt);

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

/// Hardware factory reset of the OpenPGP application via direct PC/SC APDU.
///
/// Resets ONLY the OpenPGP application — GPG keys, cardholder data, and PINs
/// are wiped and restored to factory defaults.  PIV, FIDO2, and OTP are untouched.
///
/// Requires the Admin PIN to be fully blocked (0 retries remaining).
/// The UI enforces this: factory reset is only offered when admin_pin_retries == 0.
///
/// Sequence:
///   1. Kill scdaemon so it releases the card channel
///   2. SELECT OpenPGP AID (D2 76 00 01 24 01)
///   3. TERMINATE DF (00 E6 00 00) — puts app in terminated state; requires admin blocked
///   4. ACTIVATE FILE (00 44 00 00) — resets all data and PINs to factory defaults
pub fn factory_reset_openpgp() -> Result<String> {
    use pcsc::{Context, Protocols, Scope, ShareMode};

    // scdaemon holds the card channel; releasing it lets us connect exclusively.
    let _ = std::process::Command::new("gpgconf")
        .args(["--kill", "scdaemon"])
        .output();
    // 50ms grace period — scdaemon process termination is async; the OS may not
    // release the exclusive card lock until the process fully exits.
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User)
        .map_err(|e| anyhow::anyhow!("PC/SC error: {e}"))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = ctx
        .list_readers(&mut readers_buf)
        .map_err(|e| anyhow::anyhow!("No smart card readers found: {e}"))?
        .collect();

    if readers.is_empty() {
        anyhow::bail!("No smart card readers found");
    }

    for reader in readers {
        let card = match ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // SELECT OpenPGP application by AID
        let select = [
            0x00u8, 0xA4, 0x04, 0x00, 0x06, 0xD2, 0x76, 0x00, 0x01, 0x24, 0x01,
        ];
        let mut buf = [0u8; 256];
        let resp = match card.transmit(&select, &mut buf) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if apdu_sw(resp) != 0x9000 {
            continue;
        }

        // TERMINATE DF — security condition: Admin PIN blocked (0 retries).
        // Returns 9000 when the condition is met.
        // Returns 6982 if admin is not yet blocked — the caller must ensure
        // admin_pin_retries == 0 before invoking this function.
        let terminate = [0x00u8, 0xE6, 0x00, 0x00];
        let mut buf2 = [0u8; 16];
        let sw_terminate = card
            .transmit(&terminate, &mut buf2)
            .map(apdu_sw)
            .unwrap_or(0);
        if sw_terminate != 0x9000 {
            anyhow::bail!(
                "TERMINATE DF failed (SW {:04X}). \
                 Factory reset requires Admin PIN to be fully blocked (0 retries remaining).",
                sw_terminate
            );
        }

        // ACTIVATE FILE — resets all OpenPGP data and PINs to factory defaults.
        let activate = [0x00u8, 0x44, 0x00, 0x00];
        let mut buf3 = [0u8; 16];
        let resp3 = card
            .transmit(&activate, &mut buf3)
            .map_err(|e| anyhow::anyhow!("ACTIVATE FILE failed: {e}"))?;

        if apdu_sw(resp3) == 0x9000 {
            return Ok("OpenPGP application reset successfully.\n\
                All keys, cardholder data, and PINs have been wiped.\n\
                Default PINs restored:\n\
                  User PIN:  123456\n\
                  Admin PIN: 12345678\n\
                  Reset Code: not set"
                .to_string());
        }

        anyhow::bail!(
            "ACTIVATE FILE failed (SW {:04X})",
            apdu_sw(resp3)
        );
    }

    anyhow::bail!("No YubiKey with OpenPGP application found")
}

/// Extract the two-byte status word from an APDU response slice.
/// Delegates to the canonical implementation in card.rs.
fn apdu_sw(resp: &[u8]) -> u16 {
    super::card::apdu_sw(resp)
}

