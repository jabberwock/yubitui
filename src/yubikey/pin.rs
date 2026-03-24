use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct PinStatus {
    pub user_pin_retries: u8,
    pub admin_pin_retries: u8,
    pub reset_code_retries: u8,
    pub user_pin_blocked: bool,
    pub admin_pin_blocked: bool,
}

impl PinStatus {
    pub fn is_healthy(&self) -> bool {
        !self.user_pin_blocked && !self.admin_pin_blocked && self.user_pin_retries >= 2
    }

    pub fn needs_attention(&self) -> bool {
        self.user_pin_blocked || self.admin_pin_blocked || self.user_pin_retries <= 1
    }
}

pub fn get_pin_status() -> Result<PinStatus> {
    let output = Command::new("gpg")
        .arg("--card-status")
        .output()?;

    if !output.status.success() {
        anyhow::bail!("gpg --card-status failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pin_status(&stdout)
}

fn parse_pin_status(output: &str) -> Result<PinStatus> {
    let mut user_pin_retries = 3;
    let mut admin_pin_retries = 3;
    let mut reset_code_retries = 0;

    for line in output.lines() {
        let line = line.trim();
        
        // Look for "PIN retry counter : 3 0 3"
        // Format: user_pin admin_pin reset_code
        if line.starts_with("PIN retry counter :") {
            if let Some(counters) = line.split(':').nth(1) {
                let parts: Vec<&str> = counters.trim().split_whitespace().collect();
                if parts.len() >= 3 {
                    user_pin_retries = parts[0].parse().unwrap_or(3);
                    admin_pin_retries = parts[1].parse().unwrap_or(3);
                    reset_code_retries = parts[2].parse().unwrap_or(0);
                }
            }
            break;
        }
    }

    Ok(PinStatus {
        user_pin_retries,
        admin_pin_retries,
        reset_code_retries,
        user_pin_blocked: user_pin_retries == 0,
        admin_pin_blocked: admin_pin_retries == 0,
    })
}
