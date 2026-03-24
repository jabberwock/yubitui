use anyhow::Result;
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
