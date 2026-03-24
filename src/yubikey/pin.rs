use anyhow::Result;

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
    // For now, return a placeholder
    // TODO: Implement actual PIN status reading via gpg --card-status parsing
    Ok(PinStatus {
        user_pin_retries: 3,
        admin_pin_retries: 3,
        reset_code_retries: 3,
        user_pin_blocked: false,
        admin_pin_blocked: false,
    })
}
