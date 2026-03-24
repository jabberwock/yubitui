use anyhow::Result;
use std::process::Command;

/// Execute a GPG command and return stdout
pub fn gpg_command(args: &[&str]) -> Result<String> {
    let output = Command::new("gpg")
        .args(args)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("GPG command failed: {}", stderr);
    }

    Ok(String::from_utf8(output.stdout)?)
}

/// Get card status from gpg --card-status
pub fn get_card_status() -> Result<String> {
    gpg_command(&["--card-status"])
}

/// Run gpg --card-edit with commands
pub fn card_edit(commands: &[&str]) -> Result<String> {
    let mut child = Command::new("gpg")
        .arg("--card-edit")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        for cmd in commands {
            writeln!(stdin, "{}", cmd)?;
        }
        writeln!(stdin, "quit")?;
    }

    let output = child.wait_with_output()?;
    Ok(String::from_utf8(output.stdout)?)
}

/// Parse PIN retry counters from card status
pub fn parse_pin_retries(card_status: &str) -> Option<(u8, u8, u8)> {
    for line in card_status.lines() {
        if line.contains("PIN retry counter") {
            // Format: "PIN retry counter : 3 0 3"
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 2 {
                let numbers: Vec<u8> = parts[1]
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                
                if numbers.len() == 3 {
                    return Some((numbers[0], numbers[1], numbers[2]));
                }
            }
        }
    }
    None
}
