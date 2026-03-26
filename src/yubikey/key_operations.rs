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
    /// True when this key/subkey lives on the card (sec>/ssb> in --list-secret-keys)
    on_card: bool,
    /// Keygrip hex string (from `grp` record in --list-secret-keys --with-colons).
    /// Used to clear stale gpg-agent shadow stubs before card transfer.
    keygrip: String,
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
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null()) // discard — piping without draining deadlocks when buffer fills
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
pub fn import_key_programmatic(key_id: &str, key_passphrase: &str, admin_pin: &str) -> Result<ImportResult> {

    // Validate key_id
    if key_id.is_empty() {
        anyhow::bail!("key_id must not be empty");
    }
    if key_id.starts_with('-') {
        anyhow::bail!("Invalid key_id: must not start with '-'");
    }

    // Discover subkey capabilities before spawning gpg --edit-key
    let subkeys = parse_subkey_capabilities(key_id)?;

    // Build the slot mapping: capability → (subkey_index, on_card flag, card_slot)
    // gpg card slots: 1=SIG, 2=ENC, 3=AUT
    let mut sig_subkey: Option<usize> = None;
    let mut enc_subkey: Option<usize> = None;
    let mut aut_subkey: Option<usize> = None;
    let mut sig_on_card = false;
    let mut enc_on_card = false;
    let mut aut_on_card = false;
    let mut sig_keygrip = String::new();
    let mut enc_keygrip = String::new();
    let mut aut_keygrip = String::new();

    for sk in &subkeys {
        let caps = sk.capabilities.to_ascii_lowercase();
        if caps.contains('s') && sig_subkey.is_none() {
            sig_subkey = Some(sk.index);
            sig_on_card = sk.on_card;
            sig_keygrip = sk.keygrip.clone();
        }
        if caps.contains('e') && enc_subkey.is_none() {
            enc_subkey = Some(sk.index);
            enc_on_card = sk.on_card;
            enc_keygrip = sk.keygrip.clone();
        }
        if caps.contains('a') && aut_subkey.is_none() {
            aut_subkey = Some(sk.index);
            aut_on_card = sk.on_card;
            aut_keygrip = sk.keygrip.clone();
        }
    }

    // Ensure scdaemon is running before spawning gpg sessions. detect_all() may
    // have killed it for exclusive PC/SC access; gpgconf --launch is a no-op if
    // scdaemon is already running, and starts it otherwise so gpg doesn't need
    // to cold-start it mid-operation.
    let _ = Command::new("gpgconf")
        .args(["--launch", "scdaemon"])
        .output();

    // Run one gpg --edit-key process per slot. Combining all keytocard operations
    // into a single session causes scdaemon to drop the card after the first
    // operation, leaving subsequent slots with "Card removed" errors.
    let mut sig_filled = false;
    let mut enc_filled = false;
    let mut aut_filled = false;
    let mut all_messages: Vec<String> = Vec::new();

    for (maybe_idx, maybe_on_card, keygrip, slot, label) in [
        (sig_subkey, sig_on_card, sig_keygrip.as_str(), 1u8, "SIG"),
        (enc_subkey, enc_on_card, enc_keygrip.as_str(), 2u8, "ENC"),
        (aut_subkey, aut_on_card, aut_keygrip.as_str(), 3u8, "AUT"),
    ] {
        let subkey_idx = match maybe_idx {
            Some(i) => i,
            None => continue,
        };

        // Skip slots where the local key is already a confirmed card stub.
        if maybe_on_card {
            all_messages.push(format!(
                "{} key is already on the card — slot skipped",
                label
            ));
            continue;
        }

        // Clear any stale gpg-agent shadow stub for this keygrip before attempting
        // card transfer. If a previous failed keytocard left the agent believing
        // the key lives on the card, it will refuse to export with "Unusable secret
        // key". Only call DELETE_KEY when the key file is a shadow stub (contains
        // "(shadowed") — never when it holds real private material, since
        // DELETE_KEY --force would delete that material from disk before we import it.
        if !keygrip.is_empty() {
            let is_stub = crate::utils::config::gnupg_home()
                .ok()
                .map(|h| h.join("private-keys-v1.d").join(format!("{}.key", keygrip)))
                .and_then(|p| std::fs::read_to_string(p).ok())
                .map(|s| s.contains("(shadowed"))
                .unwrap_or(false);
            if is_stub {
                let _ = Command::new("gpg-connect-agent")
                    .arg(format!("DELETE_KEY --force {}", keygrip))
                    .arg("/bye")
                    .output();
            }
        }

        let ok = run_keytocard_session(key_id, key_passphrase, admin_pin, subkey_idx, slot, &mut all_messages)?;
        match (label, ok) {
            ("SIG", true) => sig_filled = true,
            ("ENC", true) => enc_filled = true,
            ("AUT", true) => aut_filled = true,
            (lbl, false) => all_messages.push(format!("{} slot import failed", lbl)),
            _ => {}
        }
    }

    Ok(ImportResult {
        sig_filled,
        enc_filled,
        aut_filled,
        messages: all_messages,
    })
}

/// Parse subkey capability flags from `gpg --list-keys --with-colons` output.
///
/// Returns a list of SubkeyInfo with 1-based indices (for gpg "key N" command)
/// and capability flag strings extracted from colon-record field 12.
/// Run a single gpg --edit-key session that moves one subkey to one card slot.
/// Returns true if SC_OP_SUCCESS was observed, false otherwise.
fn run_keytocard_session(
    key_id: &str,
    key_passphrase: &str,
    admin_pin: &str,
    subkey_idx: usize,
    slot: u8,
    messages: &mut Vec<String>,
) -> Result<bool> {
    use crate::yubikey::gpg_status::{parse_status_line, GpgStatus};
    use std::io::BufRead;
    use std::sync::mpsc;

    let mut child = Command::new("gpg")
        .arg("--no-tty")
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
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().expect("stderr piped");
    let mut stdin = child.stdin.take().expect("stdin piped");

    // Do NOT pre-buffer any commands. gpg --edit-key drives the conversation
    // via GET_LINE keyedit.prompt: each time it issues that prompt it expects
    // exactly one command. Pre-buffering causes commands to be consumed by the
    // wrong prompt (e.g. the slot number consumed by a GET_BOOL confirm prompt).
    // We use a state machine to respond to each prompt in order.

    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines().map_while(|l| l.ok()) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    // Debug log: capture every raw status line and our response.
    let log_path = std::path::PathBuf::from(format!("/tmp/yubitui-keytocard-{}-slot{}.log", key_id, slot));
    let mut log = std::fs::OpenOptions::new().create(true).truncate(true).write(true).open(&log_path).ok();
    macro_rules! dbg_log {
        ($($arg:tt)*) => {
            if let Some(ref mut f) = log {
                use std::io::Write;
                let _ = writeln!(f, $($arg)*);
            }
        };
    }
    dbg_log!("=== keytocard subkey_idx={} slot={} ===", subkey_idx, slot);

    // State machine for the keyedit.prompt conversation.
    //
    // gpg --edit-key issues GET_LINE keyedit.prompt for every command it needs.
    // The sequence we drive:
    //   State 0 (SelectKey):  if subkey_idx > 0, send "key N"; else skip to State 1
    //   State 1 (SendKeytocard): send "keytocard"
    //   State 2 (WaitForResult): do NOT respond to keyedit.prompt here; wait for
    //     SC_OP_SUCCESS / SC_OP_FAILURE / keytocard.where prompts
    //   State 3 (SendSave): SC_OP_SUCCESS seen, send "save" on next keyedit.prompt
    //   State 4 (SendQuit): after save, send "quit" if gpg returns to main prompt
    //
    // States 0-1 happen at GET_LINE keyedit.prompt.
    // States 2 onward depend on intervening SC_OP_SUCCESS / SC_OP_FAILURE events
    // and then a final keyedit.prompt (or gpg just exits after save).
    #[derive(PartialEq)]
    enum KtcState {
        SelectKey,    // 0: send "key N" if subkey_idx > 0
        SendKeytocard, // 1: send "keytocard"
        WaitForResult, // 2: keytocard issued, waiting for card op to complete
        SendSave,     // 3: SC_OP_SUCCESS seen, next keyedit.prompt → "save"
        Done,         // 4: save sent, done
    }

    let initial_state = if subkey_idx > 0 {
        KtcState::SelectKey
    } else {
        KtcState::SendKeytocard
    };
    let mut key_passphrase_sent = false;
    let mut state = initial_state;
    let mut success = false;

    for line in rx {
        dbg_log!("RECV: {}", line);
        let status = parse_status_line(&line);
        match &status {
            GpgStatus::GetLine { prompt } if prompt == "keyedit.prompt" => {
                match state {
                    KtcState::SelectKey => {
                        dbg_log!("SEND: key {} (select subkey)", subkey_idx);
                        let _ = writeln!(stdin, "key {}", subkey_idx);
                        state = KtcState::SendKeytocard;
                    }
                    KtcState::SendKeytocard => {
                        dbg_log!("SEND: keytocard");
                        let _ = writeln!(stdin, "keytocard");
                        state = KtcState::WaitForResult;
                    }
                    KtcState::WaitForResult => {
                        // gpg 2.4.9 does not emit SC_OP_SUCCESS — it returns to keyedit.prompt
                        // after a successful card write. Reaching keyedit.prompt here without
                        // SC_OP_FAILURE means the card write succeeded. Mark success and save.
                        dbg_log!("SEND: save (keyedit.prompt in WaitForResult — gpg 2.4.9 no SC_OP_SUCCESS)");
                        let _ = writeln!(stdin, "save");
                        success = true;
                        state = KtcState::Done;
                    }
                    KtcState::SendSave => {
                        dbg_log!("SEND: save");
                        let _ = writeln!(stdin, "save");
                        state = KtcState::Done;
                        // After save gpg typically exits; if it re-prompts we'll
                        // fall through to the Done arm below.
                    }
                    KtcState::Done => {
                        dbg_log!("SEND: quit (keyedit.prompt after Done)");
                        let _ = writeln!(stdin, "quit");
                    }
                }
            }
            GpgStatus::GetLine { prompt } if prompt == "keytocard.where" => {
                dbg_log!("SEND: {} (slot)", slot);
                let _ = writeln!(stdin, "{}", slot);
            }
            GpgStatus::GetLine { prompt } if prompt == "cardedit.genkeys.storekeytype" => {
                // gpg asks which card slot to use (1=SIG, 2=ENC, 3=AUT) — send the target slot.
                dbg_log!("SEND: {} (cardedit.genkeys.storekeytype = slot)", slot);
                let _ = writeln!(stdin, "{}", slot);
            }
            GpgStatus::GetLine { prompt } => {
                // Unknown GET_LINE — send quit to exit gracefully.
                dbg_log!("UNEXPECTED GET_LINE prompt={:?} — sending quit", prompt);
                let _ = writeln!(stdin, "quit");
            }
            GpgStatus::GetHidden { prompt } if prompt == "passphrase.enter" => {
                if !key_passphrase_sent {
                    // First passphrase.enter: decrypt the local key.
                    dbg_log!("SEND: <key_passphrase>");
                    let _ = writeln!(stdin, "{}", key_passphrase);
                    key_passphrase_sent = true;
                } else {
                    // Second passphrase.enter: card admin PIN via loopback.
                    dbg_log!("SEND: <admin_pin> (second passphrase.enter = card PIN)");
                    let _ = writeln!(stdin, "{}", admin_pin);
                }
            }
            GpgStatus::GetHidden { .. } => {
                // Any other hidden prompt is the card admin PIN.
                dbg_log!("SEND: <admin_pin>");
                let _ = writeln!(stdin, "{}", admin_pin);
            }
            GpgStatus::GetBool { prompt } if prompt == "keyedit.save.okay" => {
                // gpg asks to save after `save` or `quit`. Only treat as success
                // if we're in SendSave or Done-after-WaitForResult — i.e. we
                // actually reached the save commit path. If state is Done because
                // SC_OP_FAILURE fired and we sent 'quit', this is a no-op save
                // confirmation and we must NOT mark success.
                if matches!(state, KtcState::SendSave | KtcState::Done) && !success {
                    // SendSave → save confirmed → success.
                    // Done with success already set (SC_OP_SUCCESS path) — keep it.
                    // Done without success (SC_OP_FAILURE path) — do not set it.
                    // Only mark success here when coming from SendSave (gpg 2.4.9
                    // path that skips SC_OP_SUCCESS and goes straight to save.okay).
                    if matches!(state, KtcState::SendSave) {
                        dbg_log!("SEND: y (keyedit.save.okay in SendSave — marking success)");
                        success = true;
                    } else {
                        dbg_log!("SEND: y (keyedit.save.okay in Done — no-op, not marking success)");
                    }
                } else {
                    dbg_log!("SEND: y (keyedit.save.okay — success already={} state=Done)", success);
                }
                let _ = writeln!(stdin, "y");
            }
            GpgStatus::GetBool { prompt } if prompt == "cardedit.genkeys.replace_key" => {
                // gpg found this key fingerprint already in the target card slot.
                // Answer "n" — key is already there, nothing to do. Mark success.
                dbg_log!("SEND: n (replace_key — key already on card, marking success)");
                success = true;
                state = KtcState::Done;
                let _ = writeln!(stdin, "n");
            }
            GpgStatus::GetBool { .. } => {
                dbg_log!("SEND: y (bool confirm)");
                let _ = writeln!(stdin, "y");
            }
            GpgStatus::ScOpSuccess => {
                success = true;
                // Transition to SendSave. gpg will issue keyedit.prompt next,
                // at which point we send "save". Do NOT write save here — gpg
                // may not be ready to receive stdin yet.
                dbg_log!("SC_OP_SUCCESS — transitioning to SendSave state");
                state = KtcState::SendSave;
            }
            GpgStatus::ScOpFailure(_) => {
                let msg = crate::yubikey::gpg_status::status_to_message(&status);
                if !msg.is_empty() {
                    messages.push(msg);
                }
                dbg_log!("SEND: quit (sc_op_failure)");
                let _ = writeln!(stdin, "quit");
                state = KtcState::Done;
            }
            // CardCtrl events (card inserted/removed) are scdaemon connection noise
            // between sessions. Suppress them — the operation result speaks for itself.
            crate::yubikey::gpg_status::GpgStatus::CardCtrl(_) => {}
            _ => {
                let msg = crate::yubikey::gpg_status::status_to_message(&status);
                if !msg.is_empty()
                    && msg != "Enter value"
                    && msg != "PIN accepted"
                {
                    messages.push(msg);
                }
            }
        }
    }
    dbg_log!("=== session done success={} ===", success);

    drop(stdin);
    let exit_status = child.wait()?;

    // If gpg was killed by a signal (e.g. user kill -9 or external kill), do not
    // silently return Ok(false) — that would cause the caller to continue importing
    // subsequent slots, spawning a new gpg process for each one. Instead, propagate
    // an error so the entire import aborts. scdaemon may still complete the pending
    // card operation internally (touch policy), but we cannot know the outcome.
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = exit_status.signal() {
            dbg_log!("gpg killed by signal {} — aborting import", sig);
            anyhow::bail!(
                "gpg was killed by signal {} during slot {} import. \
                 If your YubiKey is flashing, touch it to complete any pending card operation, \
                 then retry the import.",
                sig, slot
            );
        }
    }

    Ok(success)
}

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
    let mut index = 1usize; // 1-based subkey index for `key N` command

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.is_empty() {
            continue;
        }
        match parts[0] {
            // Index 0 = primary key. gpg keytocard uses it for SIG slot without
            // any `key N` selection — just `keytocard` directly.
            // Only keep lowercase letters: uppercase letters in the pub/sec field
            // are summary flags for the overall key (e.g. 'E' means a subkey can
            // encrypt) and must not be matched as primary-key direct capabilities.
            "pub" | "sec" => {
                let caps: String = parts.get(11).copied().unwrap_or("")
                    .chars().filter(|c| c.is_lowercase()).collect();
                subkeys.push(SubkeyInfo { index: 0, capabilities: caps, on_card: false, keygrip: String::new() });
            }
            "sub" | "ssb" => {
                let caps = parts.get(11).copied().unwrap_or("").to_string();
                subkeys.push(SubkeyInfo { index, capabilities: caps, on_card: false, keygrip: String::new() });
                index += 1;
            }
            _ => {}
        }
    }

    // Cross-reference with --list-secret-keys to mark any subkeys that live on a
    // smartcard. In gpg --list-secret-keys --with-colons output the record type
    // for an on-card key is "sec>" (primary) or "ssb>" (subkey). We match by
    // sequential position (same order as pub/sub records above) rather than by
    // key ID to avoid relying on fingerprint field availability.
    let sec_output = Command::new("gpg")
        .arg("--list-secret-keys")
        .arg("--with-colons")
        .arg("--")
        .arg(key_id)
        .output()
        .ok();

    if let Some(sec_out) = sec_output {
        let sec_stdout = String::from_utf8_lossy(&sec_out.stdout);
        let mut sk_iter = subkeys.iter_mut();
        let mut last_sk: Option<&mut SubkeyInfo> = None;
        for line in sec_stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            let rec = parts[0];
            match rec {
                "sec>" | "sec#" => {
                    last_sk = sk_iter.next();
                    if let Some(ref mut sk) = last_sk { sk.on_card = true; }
                }
                "sec" | "ssb" => {
                    last_sk = sk_iter.next();
                }
                "ssb>" | "ssb#" => {
                    last_sk = sk_iter.next();
                    if let Some(ref mut sk) = last_sk { sk.on_card = true; }
                }
                "grp" => {
                    // grp record immediately follows sec/ssb — capture keygrip
                    if let Some(ref mut sk) = last_sk {
                        if let Some(grip) = parts.get(9) {
                            if !grip.is_empty() {
                                sk.keygrip = grip.to_string();
                            }
                        }
                    }
                }
                _ => {}
            }
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

/// Fetch key attributes via native PC/SC GET DATA flat DOs.
///
/// Uses GET DATA 0xC5 (fingerprints, 60 bytes) and 0xC1/0xC2/0xC3 (algorithm
/// attributes per slot) instead of the nested 0x6E/0x73 TLV approach, which
/// fails on real hardware when the YubiKey includes the outer 0x6E tag in the
/// response and tlv_find scans past the content.
/// No ykman binary required.
pub fn get_key_attributes() -> Result<KeyAttributes> {
    let (card, _aid) = crate::yubikey::card::connect_to_openpgp_card()?;

    // GET DATA 0x00C5 — Fingerprints: 60 bytes = SIG(20) | ENC(20) | AUT(20).
    let c5 = crate::yubikey::card::get_data(&card, 0x00, 0xC5).ok();
    let sig_fp = c5.as_deref().and_then(|b| if b.len() >= 20 { Some(b[..20].to_vec()) } else { None });
    let enc_fp = c5.as_deref().and_then(|b| if b.len() >= 40 { Some(b[20..40].to_vec()) } else { None });
    let aut_fp = c5.as_deref().and_then(|b| if b.len() >= 60 { Some(b[40..60].to_vec()) } else { None });

    // GET DATA 0x00C1/C2/C3 — Algorithm attributes per slot.
    let sig_algo = crate::yubikey::card::get_data(&card, 0x00, 0xC1).ok();
    let enc_algo = crate::yubikey::card::get_data(&card, 0x00, 0xC2).ok();
    let aut_algo = crate::yubikey::card::get_data(&card, 0x00, 0xC3).ok();

    let signature = build_slot_info(sig_fp.as_deref(), sig_algo.as_deref());
    let encryption = build_slot_info(enc_fp.as_deref(), enc_algo.as_deref());
    let authentication = build_slot_info(aut_fp.as_deref(), aut_algo.as_deref());

    // connect_to_openpgp_card() killed scdaemon to get exclusive access.
    // Restart it now so subsequent gpg operations (import, PIN change) don't
    // have to cold-start scdaemon mid-operation.
    drop(card);
    let _ = Command::new("gpgconf")
        .args(["--launch", "scdaemon"])
        .output();

    Ok(KeyAttributes { signature, encryption, authentication })
}

/// Build SlotInfo from raw fingerprint and algorithm attribute bytes.
/// Returns None if the fingerprint is all-zeros (no key in slot) or absent.
fn build_slot_info(fp_bytes: Option<&[u8]>, algo_bytes: Option<&[u8]>) -> Option<SlotInfo> {
    let fp_bytes = fp_bytes?;
    if fp_bytes.iter().all(|&b| b == 0) {
        return None;
    }
    let fingerprint = crate::yubikey::detection::format_fingerprint(fp_bytes);
    if fingerprint.is_empty() {
        return None;
    }
    let algorithm = algo_bytes
        .map(crate::yubikey::detection::parse_algorithm_attributes)
        .unwrap_or_else(|| "Unknown".to_string());
    Some(SlotInfo { algorithm, fingerprint })
}

#[allow(dead_code)]
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
///
/// Reads the authentication key fingerprint via native PC/SC GET DATA (no gpg --card-status),
/// then exports the SSH public key from the GPG keyring using gpg --export-ssh-key.
pub fn get_ssh_public_key_text() -> Result<String> {
    // Get the authentication key fingerprint from the card via native PC/SC.
    let states = crate::yubikey::YubiKeyState::detect_all()
        .map_err(|e| anyhow::anyhow!("Could not read card state: {e}"))?;

    let auth_fp = states
        .into_iter()
        .find_map(|s| {
            s.openpgp
                .as_ref()
                .and_then(|o| o.authentication_key.as_ref())
                .map(|k| k.fingerprint.clone())
        })
        .filter(|fp| !fp.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("No authentication key found on card. Import or generate a key first.")
        })?;

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

/// View card status — reads card state via native PC/SC APDUs and formats it as
/// human-readable text. No gpg --card-status subprocess call.
pub fn view_card_status() -> Result<String> {
    let states = crate::yubikey::YubiKeyState::detect_all()
        .map_err(|e| anyhow::anyhow!("Could not read card state: {e}"))?;

    if states.is_empty() {
        anyhow::bail!("No YubiKey detected. Make sure your YubiKey is inserted.");
    }

    let mut lines = Vec::new();
    for (i, s) in states.iter().enumerate() {
        if i > 0 {
            lines.push(String::new());
        }
        if states.len() > 1 {
            lines.push(format!("YubiKey {} of {}:", i + 1, states.len()));
        }
        lines.push(format!("Serial number: {}", s.info.serial));
        lines.push(format!(
            "Firmware:      {}.{}.{}",
            s.info.version.major, s.info.version.minor, s.info.version.patch
        ));
        lines.push(format!("Model:         {:?}", s.info.model));
        lines.push(format!(
            "PIN retries:   User={} Admin={} Reset={}",
            s.pin_status.user_pin_retries,
            s.pin_status.admin_pin_retries,
            s.pin_status.reset_code_retries,
        ));
        if let Some(ref openpgp) = s.openpgp {
            let fp_or = |k: &Option<crate::yubikey::openpgp::KeyInfo>| {
                k.as_ref()
                    .map(|i| i.fingerprint.as_str())
                    .unwrap_or("[none]")
                    .to_string()
            };
            lines.push(format!("Sig key:       {}", fp_or(&openpgp.signature_key)));
            lines.push(format!("Enc key:       {}", fp_or(&openpgp.encryption_key)));
            lines.push(format!("Aut key:       {}", fp_or(&openpgp.authentication_key)));
            if let Some(ref name) = openpgp.cardholder_name {
                if !name.is_empty() {
                    lines.push(format!("Name:          {}", name));
                }
            }
        }
    }

    Ok(lines.join("\n"))
}

/// List available GPG primary keys that can be imported.
///
/// Returns one entry per importable primary key. Each entry is the full
/// 40-character fingerprint from the `fpr` record following the `sec` line.
/// The fingerprint is a valid gpg key selector (passed directly to
/// `import_key_programmatic` as key_id) and is displayed as-is in the UI.
///
/// Card-stub primary keys (`sec>`) and unavailable keys (`sec#`) are excluded
/// because their local key material has been replaced by a card reference and
/// cannot be re-exported to the card.
pub fn list_gpg_keys() -> Result<Vec<String>> {
    let output = Command::new("gpg")
        .arg("--list-secret-keys")
        .arg("--with-colons")
        .output()?;

    let mut keys = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut is_importable = false;
    let mut captured_fpr = false;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "sec" => {
                is_importable = true;
                captured_fpr = false;
            }
            // sec> = key on card (stub), sec# = dummy/unavailable — not importable
            "sec>" | "sec#" => {
                is_importable = false;
                captured_fpr = false;
            }
            "fpr" if is_importable && !captured_fpr => {
                // First fpr record after an importable sec is the primary key fingerprint
                if let Some(fp) = parts.get(9) {
                    if !fp.is_empty() {
                        keys.push(fp.to_string());
                        captured_fpr = true;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(keys)
}
