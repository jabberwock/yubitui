//! GPG status-fd token parser and human message translator.
//!
//! Parses `[GNUPG:] TOKEN [args...]` lines emitted on the status file descriptor
//! when gpg is invoked with `--status-fd 1 --pinentry-mode loopback`.

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum GpgStatus {
    /// gpg needs a passphrase/PIN (legacy token)
    NeedPassphrase,
    /// gpg asking for hidden (masked) input like a PIN
    GetHidden { prompt: String },
    /// gpg asking for visible input
    GetLine { prompt: String },
    /// gpg asking a yes/no question (e.g. keytocard.replace_key)
    GetBool { prompt: String },
    /// gpg received the input we sent
    GotIt,
    /// Operation error with operation name and error code
    Error { operation: String, code: u64 },
    /// Card control event: 1=inserted, 3=removed, 4=card-check
    CardCtrl(u32),
    /// Key was created: key_type is P (primary) / B (backup) / S (subkey), fingerprint is hex
    KeyCreated { key_type: String, fingerprint: String },
    /// Smartcard operation succeeded
    ScOpSuccess,
    /// Smartcard operation failed with reason code
    ScOpFailure(u32),
    /// Pinentry was launched (we suppress this since we handle PIN in-TUI)
    PinentryLaunched,
    /// Key fingerprint being considered
    KeyConsidered { fingerprint: String },
    /// Progress report: what operation, current step, total steps
    Progress { what: String, cur: u32, total: u32 },
    /// Any token not explicitly handled
    Unknown(String),
}

/// Parse a single line from gpg's `--status-fd` output into a typed `GpgStatus`.
///
/// Lines have the form: `[GNUPG:] TOKEN [args...]`
/// Lines without the `[GNUPG:] ` prefix are returned as `GpgStatus::Unknown`.
#[allow(dead_code)]
pub fn parse_status_line(line: &str) -> GpgStatus {
    let prefix = "[GNUPG:] ";
    let Some(rest) = line.strip_prefix(prefix) else {
        return GpgStatus::Unknown(line.to_string());
    };

    let mut parts = rest.splitn(4, ' ');
    let token = parts.next().unwrap_or("");
    let arg1 = parts.next().unwrap_or("");
    let arg2 = parts.next().unwrap_or("");
    let arg3 = parts.next().unwrap_or("");

    match token {
        "NEED_PASSPHRASE" => GpgStatus::NeedPassphrase,
        "GET_HIDDEN" => GpgStatus::GetHidden {
            prompt: arg1.to_string(),
        },
        "GET_LINE" => GpgStatus::GetLine {
            prompt: arg1.to_string(),
        },
        "GET_BOOL" => GpgStatus::GetBool {
            prompt: arg1.to_string(),
        },
        "GOT_IT" => GpgStatus::GotIt,
        "ERROR" => {
            let code: u64 = arg2.parse().unwrap_or(0);
            GpgStatus::Error {
                operation: arg1.to_string(),
                code,
            }
        }
        "CARDCTRL" => {
            let code: u32 = arg1.parse().unwrap_or(0);
            GpgStatus::CardCtrl(code)
        }
        "KEY_CREATED" => GpgStatus::KeyCreated {
            key_type: arg1.to_string(),
            fingerprint: arg2.to_string(),
        },
        "SC_OP_SUCCESS" => GpgStatus::ScOpSuccess,
        "SC_OP_FAILURE" => {
            let code: u32 = arg1.parse().unwrap_or(0);
            GpgStatus::ScOpFailure(code)
        }
        "PINENTRY_LAUNCHED" => GpgStatus::PinentryLaunched,
        "KEY_CONSIDERED" => GpgStatus::KeyConsidered {
            fingerprint: arg1.to_string(),
        },
        "PROGRESS" => {
            // PROGRESS <what> <type> <cur> <total>
            // arg1=what, arg2=type_char, arg3="cur total" — but we used splitn(4)
            // so arg1=what, arg2=type_char, arg3=rest
            // Re-parse to extract cur and total from remaining tokens
            let mut num_parts = arg3.splitn(2, ' ');
            let cur: u32 = num_parts.next().unwrap_or("0").parse().unwrap_or(0);
            let total: u32 = num_parts.next().unwrap_or("0").parse().unwrap_or(0);
            GpgStatus::Progress {
                what: arg1.to_string(),
                cur,
                total,
            }
        }
        _ => GpgStatus::Unknown(rest.to_string()),
    }
}

/// Translate a `GpgStatus` variant into a human-readable message suitable for
/// display in the TUI status line. Returns an empty string for variants that
/// should be silently ignored (e.g. `PinentryLaunched`, `Unknown`).
#[allow(dead_code)]
pub fn status_to_message(status: &GpgStatus) -> String {
    match status {
        GpgStatus::NeedPassphrase => "PIN required...".to_string(),
        GpgStatus::GetHidden { prompt } => match prompt.as_str() {
            "passphrase.pin" => "Enter User PIN".to_string(),
            "passphrase.admin_pin" => "Enter Admin PIN".to_string(),
            _ => "Enter passphrase".to_string(),
        },
        GpgStatus::GetLine { .. } => "Enter value".to_string(),
        GpgStatus::GetBool { .. } => String::new(),
        GpgStatus::GotIt => "PIN accepted".to_string(),
        GpgStatus::Error { operation, code } => match (operation.as_str(), *code) {
            ("change_passwd", 67108949) => "Incorrect PIN (check remaining attempts)".to_string(),
            ("change_passwd", _) => "PIN change failed".to_string(),
            (_, 67108949) => "Authentication failed".to_string(),
            _ => format!("Operation failed (error {})", code),
        },
        GpgStatus::CardCtrl(1) => "Card detected".to_string(),
        GpgStatus::CardCtrl(3) => "Card removed -- reinsert and retry".to_string(),
        GpgStatus::CardCtrl(_) => "Card event".to_string(),
        GpgStatus::KeyCreated { .. } => "Key generated successfully".to_string(),
        GpgStatus::ScOpSuccess => "Operation completed successfully".to_string(),
        GpgStatus::ScOpFailure(2) => "Wrong PIN".to_string(),
        GpgStatus::ScOpFailure(6) => "Wrong Admin PIN".to_string(),
        GpgStatus::ScOpFailure(_) => "Smartcard operation failed".to_string(),
        GpgStatus::PinentryLaunched => String::new(),
        GpgStatus::KeyConsidered { .. } => String::new(),
        GpgStatus::Progress { what, cur, total } => format!("{}: {}/{}", what, cur, total),
        GpgStatus::Unknown(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_status_line tests ---

    #[test]
    fn test_parse_need_passphrase() {
        assert_eq!(
            parse_status_line("[GNUPG:] NEED_PASSPHRASE"),
            GpgStatus::NeedPassphrase
        );
    }

    #[test]
    fn test_parse_get_hidden() {
        assert_eq!(
            parse_status_line("[GNUPG:] GET_HIDDEN passphrase.pin"),
            GpgStatus::GetHidden {
                prompt: "passphrase.pin".to_string()
            }
        );
    }

    #[test]
    fn test_parse_got_it() {
        assert_eq!(
            parse_status_line("[GNUPG:] GOT_IT"),
            GpgStatus::GotIt
        );
    }

    #[test]
    fn test_parse_error() {
        assert_eq!(
            parse_status_line("[GNUPG:] ERROR change_passwd 67108949"),
            GpgStatus::Error {
                operation: "change_passwd".to_string(),
                code: 67108949,
            }
        );
    }

    #[test]
    fn test_parse_cardctrl() {
        assert_eq!(
            parse_status_line("[GNUPG:] CARDCTRL 3"),
            GpgStatus::CardCtrl(3)
        );
    }

    #[test]
    fn test_parse_key_created() {
        assert_eq!(
            parse_status_line("[GNUPG:] KEY_CREATED P ABCDEF1234567890"),
            GpgStatus::KeyCreated {
                key_type: "P".to_string(),
                fingerprint: "ABCDEF1234567890".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_sc_op_success() {
        assert_eq!(
            parse_status_line("[GNUPG:] SC_OP_SUCCESS"),
            GpgStatus::ScOpSuccess
        );
    }

    #[test]
    fn test_parse_sc_op_failure() {
        assert_eq!(
            parse_status_line("[GNUPG:] SC_OP_FAILURE 2"),
            GpgStatus::ScOpFailure(2)
        );
    }

    #[test]
    fn test_parse_unknown() {
        assert_eq!(
            parse_status_line("not a status line"),
            GpgStatus::Unknown("not a status line".to_string())
        );
    }

    #[test]
    fn test_parse_pinentry_launched() {
        assert_eq!(
            parse_status_line("[GNUPG:] PINENTRY_LAUNCHED 12345 some extra args"),
            GpgStatus::PinentryLaunched
        );
    }

    #[test]
    fn test_parse_progress() {
        let status = parse_status_line("[GNUPG:] PROGRESS primegen X 23 100");
        assert_eq!(
            status,
            GpgStatus::Progress {
                what: "primegen".to_string(),
                cur: 23,
                total: 100,
            }
        );
    }

    #[test]
    fn test_parse_get_line() {
        assert_eq!(
            parse_status_line("[GNUPG:] GET_LINE keygen.uid"),
            GpgStatus::GetLine {
                prompt: "keygen.uid".to_string()
            }
        );
    }

    // --- status_to_message tests ---

    #[test]
    fn test_message_error_change_passwd_wrong_pin() {
        let status = GpgStatus::Error {
            operation: "change_passwd".to_string(),
            code: 67108949,
        };
        assert_eq!(
            status_to_message(&status),
            "Incorrect PIN (check remaining attempts)"
        );
    }

    #[test]
    fn test_message_sc_op_success() {
        assert_eq!(
            status_to_message(&GpgStatus::ScOpSuccess),
            "Operation completed successfully"
        );
    }

    #[test]
    fn test_message_cardctrl_removed() {
        assert_eq!(
            status_to_message(&GpgStatus::CardCtrl(3)),
            "Card removed -- reinsert and retry"
        );
    }

    #[test]
    fn test_message_key_created() {
        let status = GpgStatus::KeyCreated {
            key_type: "P".to_string(),
            fingerprint: "ABCDEF".to_string(),
        };
        assert_eq!(status_to_message(&status), "Key generated successfully");
    }

    #[test]
    fn test_message_need_passphrase() {
        assert_eq!(
            status_to_message(&GpgStatus::NeedPassphrase),
            "PIN required..."
        );
    }

    #[test]
    fn test_message_get_hidden_user_pin() {
        let status = GpgStatus::GetHidden {
            prompt: "passphrase.pin".to_string(),
        };
        assert_eq!(status_to_message(&status), "Enter User PIN");
    }

    #[test]
    fn test_message_get_hidden_admin_pin() {
        let status = GpgStatus::GetHidden {
            prompt: "passphrase.admin_pin".to_string(),
        };
        assert_eq!(status_to_message(&status), "Enter Admin PIN");
    }

    #[test]
    fn test_message_pinentry_launched_is_empty() {
        assert_eq!(status_to_message(&GpgStatus::PinentryLaunched), "");
    }

    #[test]
    fn test_message_unknown_is_empty() {
        assert_eq!(
            status_to_message(&GpgStatus::Unknown("garbage".to_string())),
            ""
        );
    }

    #[test]
    fn test_message_sc_op_failure_wrong_admin_pin() {
        assert_eq!(
            status_to_message(&GpgStatus::ScOpFailure(6)),
            "Wrong Admin PIN"
        );
    }

    #[test]
    fn test_message_sc_op_failure_generic() {
        assert_eq!(
            status_to_message(&GpgStatus::ScOpFailure(99)),
            "Smartcard operation failed"
        );
    }
}
