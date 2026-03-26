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

/// Read PIN retry counters directly from the card via GET DATA 0xC4 (PW Status Bytes).
///
/// PW Status Bytes layout (7 bytes):
///   byte 0: PIN format flags
///   byte 1: max User PIN length
///   byte 2: max Reset Code length
///   byte 3: max Admin PIN length
///   byte 4: PW1 (User PIN) remaining tries
///   byte 5: RC  (Reset Code) remaining tries
///   byte 6: PW3 (Admin PIN) remaining tries
#[allow(dead_code)]
pub fn get_pin_status() -> Result<PinStatus> {
    let (card, _aid) = super::card::connect_to_openpgp_card()?;
    let data = super::card::get_data(&card, 0x00, 0xC4)?;
    if data.len() < 7 {
        anyhow::bail!("Unexpected PW Status Bytes length: {}", data.len());
    }
    Ok(PinStatus {
        user_pin_retries: data[4],
        admin_pin_retries: data[6],
        reset_code_retries: data[5],
        user_pin_blocked: data[4] == 0,
        admin_pin_blocked: data[6] == 0,
    })
}

#[allow(dead_code)]
pub fn parse_pin_status(output: &str) -> Result<PinStatus> {
    let mut user_pin_retries = 3;
    let mut admin_pin_retries = 3;
    let mut reset_code_retries = 0;

    for line in output.lines() {
        let line = line.trim();

        // Look for "PIN retry counter : 3 0 3"
        // gpg output order: user_pin (PW1), reset_code (RC), admin_pin (PW3)
        if line.starts_with("PIN retry counter :") {
            if let Some(counters) = line.split(':').nth(1) {
                let parts: Vec<&str> = counters.split_whitespace().collect();
                if parts.len() >= 3 {
                    user_pin_retries = parts[0].parse().unwrap_or(3);
                    reset_code_retries = parts[1].parse().unwrap_or(0);
                    admin_pin_retries = parts[2].parse().unwrap_or(3);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pin_status_normal() {
        // gpg order: user RC admin → "3 0 3" means user=3, rc=0, admin=3
        let status = parse_pin_status("PIN retry counter : 3 0 3\n").unwrap();
        assert_eq!(status.user_pin_retries, 3);
        assert_eq!(status.admin_pin_retries, 3);
        assert_eq!(status.reset_code_retries, 0);
        assert!(!status.user_pin_blocked);
        assert!(!status.admin_pin_blocked);
    }

    #[test]
    fn test_parse_pin_status_user_blocked() {
        // gpg order: user RC admin → "0 0 3" means user=0, rc=0, admin=3
        let status = parse_pin_status("PIN retry counter : 0 0 3\n").unwrap();
        assert_eq!(status.user_pin_retries, 0);
        assert!(status.user_pin_blocked);
        assert!(!status.admin_pin_blocked);
    }

    #[test]
    fn test_parse_pin_status_admin_blocked() {
        let status = parse_pin_status("PIN retry counter : 3 0 0\n").unwrap();
        assert_eq!(status.admin_pin_retries, 0);
        assert!(!status.user_pin_blocked);
        assert!(status.admin_pin_blocked);
    }

    #[test]
    fn test_parse_pin_status_all_blocked() {
        let status = parse_pin_status("PIN retry counter : 0 0 0\n").unwrap();
        assert!(status.user_pin_blocked);
        assert!(status.admin_pin_blocked);
    }

    #[test]
    fn test_parse_pin_status_no_match() {
        let status = parse_pin_status("some unrelated output\n").unwrap();
        // Defaults: user=3, admin=3, reset=0
        assert_eq!(status.user_pin_retries, 3);
        assert_eq!(status.admin_pin_retries, 3);
        assert_eq!(status.reset_code_retries, 0);
        assert!(!status.user_pin_blocked);
        assert!(!status.admin_pin_blocked);
    }

    #[test]
    fn test_is_healthy() {
        let status = PinStatus {
            user_pin_retries: 3,
            admin_pin_retries: 3,
            reset_code_retries: 0,
            user_pin_blocked: false,
            admin_pin_blocked: false,
        };
        assert!(status.is_healthy());
    }

    #[test]
    fn test_needs_attention() {
        let status = PinStatus {
            user_pin_retries: 1,
            admin_pin_retries: 3,
            reset_code_retries: 0,
            user_pin_blocked: false,
            admin_pin_blocked: false,
        };
        assert!(status.needs_attention());
    }
}
