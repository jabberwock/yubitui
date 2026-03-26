use anyhow::Result;
use std::fmt;

/// Touch policy variants for OpenPGP slots.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TouchPolicy {
    #[default]
    Off,
    On,
    Fixed,
    Cached,
    CachedFixed,
    Unknown(String),
}

#[allow(dead_code)]
impl TouchPolicy {
    /// Parse a touch policy string from ykman output.
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "off" => TouchPolicy::Off,
            "on" => TouchPolicy::On,
            "fixed" => TouchPolicy::Fixed,
            "cached" => TouchPolicy::Cached,
            "cached-fixed" => TouchPolicy::CachedFixed,
            other => TouchPolicy::Unknown(other.to_string()),
        }
    }

    /// Parse a touch policy from the raw UIF data object byte.
    ///
    /// Per OpenPGP card spec and YubiKey UIF DOs (0xD6-0xD9):
    ///   0x00 = Off, 0x01 = On, 0x02 = Fixed, 0x03 = Cached, 0x04 = Cached-Fixed
    pub fn from_byte(b: u8) -> Self {
        match b {
            0x00 => TouchPolicy::Off,
            0x01 => TouchPolicy::On,
            0x02 => TouchPolicy::Fixed,
            0x03 => TouchPolicy::Cached,
            0x04 => TouchPolicy::CachedFixed,
            other => TouchPolicy::Unknown(format!("0x{:02X}", other)),
        }
    }

    /// Return the raw UIF byte for this policy.
    pub fn to_byte(&self) -> u8 {
        match self {
            TouchPolicy::Off => 0x00,
            TouchPolicy::On => 0x01,
            TouchPolicy::Fixed => 0x02,
            TouchPolicy::Cached => 0x03,
            TouchPolicy::CachedFixed => 0x04,
            TouchPolicy::Unknown(_) => 0x00, // fallback to off
        }
    }

    /// Returns true if this policy cannot be changed back without factory reset.
    pub fn is_irreversible(&self) -> bool {
        matches!(self, Self::Fixed | Self::CachedFixed)
    }

    /// Returns the ykman CLI argument string for this policy.
    pub fn as_ykman_arg(&self) -> &str {
        match self {
            TouchPolicy::Off => "off",
            TouchPolicy::On => "on",
            TouchPolicy::Fixed => "fixed",
            TouchPolicy::Cached => "cached",
            TouchPolicy::CachedFixed => "cached-fixed",
            TouchPolicy::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for TouchPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TouchPolicy::Off => write!(f, "Off"),
            TouchPolicy::On => write!(f, "On"),
            TouchPolicy::Fixed => write!(f, "Fixed (IRREVERSIBLE)"),
            TouchPolicy::Cached => write!(f, "Cached"),
            TouchPolicy::CachedFixed => write!(f, "Cached-Fixed (IRREVERSIBLE)"),
            TouchPolicy::Unknown(s) => write!(f, "Unknown({s})"),
        }
    }
}

/// Touch policies for all four OpenPGP slots.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub struct TouchPolicies {
    pub signature: TouchPolicy,
    pub encryption: TouchPolicy,
    pub authentication: TouchPolicy,
    pub attestation: TouchPolicy,
}

/// Read touch policies for all four OpenPGP slots via native GET DATA APDUs.
///
/// DOs: 0xD6 (sig), 0xD7 (enc), 0xD8 (aut), 0xD9 (att).
/// Each DO returns [policy_byte, 0x20] on success; first byte is the policy.
/// On failure (older cards that do not support UIF), defaults to TouchPolicy::Off.
#[allow(dead_code)]
pub fn get_touch_policies_native(card: &pcsc::Card) -> Result<TouchPolicies> {
    let sig = super::card::get_data(card, 0x00, 0xD6)
        .ok()
        .and_then(|d| d.first().copied())
        .map(TouchPolicy::from_byte)
        .unwrap_or(TouchPolicy::Off);

    let enc = super::card::get_data(card, 0x00, 0xD7)
        .ok()
        .and_then(|d| d.first().copied())
        .map(TouchPolicy::from_byte)
        .unwrap_or(TouchPolicy::Off);

    let aut = super::card::get_data(card, 0x00, 0xD8)
        .ok()
        .and_then(|d| d.first().copied())
        .map(TouchPolicy::from_byte)
        .unwrap_or(TouchPolicy::Off);

    let att = super::card::get_data(card, 0x00, 0xD9)
        .ok()
        .and_then(|d| d.first().copied())
        .map(TouchPolicy::from_byte)
        .unwrap_or(TouchPolicy::Off);

    Ok(TouchPolicies {
        signature: sig,
        encryption: enc,
        authentication: aut,
        attestation: att,
    })
}

/// Set the touch policy for a given OpenPGP slot using native PC/SC APDUs.
///
/// Steps:
///   1. Map slot string to UIF DO: sig->0xD6, enc->0xD7, aut->0xD8, att->0xD9
///   2. KDF check via GET DATA 0xF9 — if KDF is enabled, bail with clear error
///   3. VERIFY Admin PIN (APDU [0x00, 0x20, 0x00, 0x83, len, ...pin_bytes])
///   4. PUT DATA [0x00, 0xDA, 0x00, DO, 0x02, policy_byte, 0x20]
///
/// `serial` is kept for API compatibility (was used by old ykman path to select device).
///
/// Valid slots: "sig", "enc", "aut", "att"
pub fn set_touch_policy(
    slot: &str,
    policy: &TouchPolicy,
    _serial: Option<u32>,
    admin_pin: &str,
) -> Result<String> {
    let do_tag: u8 = match slot {
        "sig" => 0xD6,
        "enc" => 0xD7,
        "aut" => 0xD8,
        "att" => 0xD9,
        other => anyhow::bail!(
            "Invalid slot '{}'. Must be one of: sig, enc, aut, att",
            other
        ),
    };

    let (card, _aid) = super::card::connect_to_openpgp_card()?;

    // KDF check (Pitfall 5): GET DATA 0xF9 — if non-empty and first byte != 0x00, KDF is active
    if let Ok(kdf_data) = super::card::get_data(&card, 0x00, 0xF9) {
        if !kdf_data.is_empty() && kdf_data[0] != 0x00 {
            anyhow::bail!(
                "This YubiKey uses KDF PIN hashing. \
                 Touch policy changes require ykman on this device."
            );
        }
    }

    // VERIFY Admin PIN: [CLA=00, INS=20, P1=00, P2=83 (Admin PIN), Lc, ...PIN bytes]
    let pin_bytes = admin_pin.as_bytes();
    let pin_len = pin_bytes.len() as u8;
    let mut verify_apdu = vec![0x00u8, 0x20, 0x00, 0x83, pin_len];
    verify_apdu.extend_from_slice(pin_bytes);
    let mut buf = [0u8; 256];
    let resp = card
        .transmit(&verify_apdu, &mut buf)
        .map_err(|e| anyhow::anyhow!("VERIFY transmit error: {e}"))?;
    let sw = super::card::apdu_sw(resp);
    if sw != 0x9000 {
        anyhow::bail!(
            "{}",
            super::card::apdu_error_message(sw, "verifying Admin PIN")
        );
    }

    // PUT DATA: [CLA=00, INS=DA, P1=00, P2=DO, Lc=02, policy_byte, 0x20]
    let put_apdu = [0x00u8, 0xDA, 0x00, do_tag, 0x02, policy.to_byte(), 0x20];
    let resp = card
        .transmit(&put_apdu, &mut buf)
        .map_err(|e| anyhow::anyhow!("PUT DATA transmit error: {e}"))?;
    let sw = super::card::apdu_sw(resp);
    if sw != 0x9000 {
        anyhow::bail!(
            "{}",
            super::card::apdu_error_message(sw, &format!("setting touch policy for {}", slot))
        );
    }

    Ok(format!("Touch policy updated to {} for slot {}", policy, slot))
}

/// Parse touch policies from `ykman openpgp info` output.
///
/// Kept with `#[allow(dead_code)]` so existing unit tests remain valid.
///
/// Looks for a "Touch policies:" section and parses the four slot lines below it.
/// Returns all-Off (default) if the section is absent or output is empty.
#[allow(dead_code)]
pub fn parse_touch_policies(output: &str) -> TouchPolicies {
    let mut policies = TouchPolicies::default();
    let mut in_touch_section = false;
    let mut found_content = false;

    for line in output.lines() {
        if line.trim() == "Touch policies:" || line.trim().starts_with("Touch policies:") {
            in_touch_section = true;
            continue;
        }

        if !in_touch_section {
            continue;
        }

        let trimmed = line.trim();

        // Exit section on empty line after we've found content, or on a
        // non-indented line (another top-level section).
        if trimmed.is_empty() {
            if found_content {
                break;
            }
            continue;
        }

        // If the line has no colon but is non-empty and starts without leading
        // whitespace, we've left the section.
        if !line.starts_with(' ') && !line.starts_with('\t') && !trimmed.contains(':') {
            break;
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            let policy = TouchPolicy::from_str(value.trim());
            match key.trim() {
                "Signature key" => {
                    policies.signature = policy;
                    found_content = true;
                }
                "Encryption key" => {
                    policies.encryption = policy;
                    found_content = true;
                }
                "Authentication key" => {
                    policies.authentication = policy;
                    found_content = true;
                }
                "Attestation key" => {
                    policies.attestation = policy;
                    found_content = true;
                }
                _ => {}
            }
        }
    }

    policies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_touch_policies_all_off() {
        let input = "Touch policies:\n  Signature key:      Off\n  Encryption key:     Off\n  Authentication key: Off\n  Attestation key:    Off\n";
        let p = parse_touch_policies(input);
        assert_eq!(p.signature, TouchPolicy::Off);
        assert_eq!(p.encryption, TouchPolicy::Off);
        assert_eq!(p.authentication, TouchPolicy::Off);
        assert_eq!(p.attestation, TouchPolicy::Off);
    }

    #[test]
    fn test_parse_touch_policies_mixed() {
        let input = "Touch policies:\n  Signature key:      Fixed\n  Encryption key:     On\n  Authentication key: Cached\n  Attestation key:    Off\n";
        let p = parse_touch_policies(input);
        assert_eq!(p.signature, TouchPolicy::Fixed);
        assert_eq!(p.encryption, TouchPolicy::On);
        assert_eq!(p.authentication, TouchPolicy::Cached);
        assert_eq!(p.attestation, TouchPolicy::Off);
    }

    #[test]
    fn test_parse_touch_policies_empty_string() {
        let p = parse_touch_policies("");
        assert_eq!(p, TouchPolicies::default());
    }

    #[test]
    fn test_parse_touch_policies_no_section() {
        let input = "OpenPGP version: 3.4\n";
        let p = parse_touch_policies(input);
        assert_eq!(p, TouchPolicies::default());
    }

    #[test]
    fn test_touch_policy_from_str() {
        assert_eq!(TouchPolicy::from_str("off"), TouchPolicy::Off);
        assert_eq!(TouchPolicy::from_str("on"), TouchPolicy::On);
        assert_eq!(TouchPolicy::from_str("fixed"), TouchPolicy::Fixed);
        assert_eq!(TouchPolicy::from_str("cached"), TouchPolicy::Cached);
        assert_eq!(TouchPolicy::from_str("cached-fixed"), TouchPolicy::CachedFixed);
        // trimming
        assert_eq!(TouchPolicy::from_str("  Off  "), TouchPolicy::Off);
        // unknown
        assert_eq!(
            TouchPolicy::from_str("garbage"),
            TouchPolicy::Unknown("garbage".to_string())
        );
    }

    #[test]
    fn test_touch_policy_irreversible() {
        assert!(TouchPolicy::Fixed.is_irreversible());
        assert!(TouchPolicy::CachedFixed.is_irreversible());
        assert!(!TouchPolicy::On.is_irreversible());
        assert!(!TouchPolicy::Off.is_irreversible());
        assert!(!TouchPolicy::Cached.is_irreversible());
    }

    #[test]
    fn test_touch_policy_as_ykman_arg() {
        assert_eq!(TouchPolicy::Off.as_ykman_arg(), "off");
        assert_eq!(TouchPolicy::On.as_ykman_arg(), "on");
        assert_eq!(TouchPolicy::Fixed.as_ykman_arg(), "fixed");
        assert_eq!(TouchPolicy::Cached.as_ykman_arg(), "cached");
        assert_eq!(TouchPolicy::CachedFixed.as_ykman_arg(), "cached-fixed");
    }

    #[test]
    fn test_from_byte_all_variants() {
        assert_eq!(TouchPolicy::from_byte(0x00), TouchPolicy::Off);
        assert_eq!(TouchPolicy::from_byte(0x01), TouchPolicy::On);
        assert_eq!(TouchPolicy::from_byte(0x02), TouchPolicy::Fixed);
        assert_eq!(TouchPolicy::from_byte(0x03), TouchPolicy::Cached);
        assert_eq!(TouchPolicy::from_byte(0x04), TouchPolicy::CachedFixed);
        assert_eq!(TouchPolicy::from_byte(0xFF), TouchPolicy::Unknown("0xFF".to_string()));
    }

    #[test]
    fn test_to_byte_roundtrip() {
        assert_eq!(TouchPolicy::Off.to_byte(), 0x00);
        assert_eq!(TouchPolicy::On.to_byte(), 0x01);
        assert_eq!(TouchPolicy::Fixed.to_byte(), 0x02);
        assert_eq!(TouchPolicy::Cached.to_byte(), 0x03);
        assert_eq!(TouchPolicy::CachedFixed.to_byte(), 0x04);
        // Roundtrip: from_byte(to_byte(x)) == x for known variants
        for b in [0x00u8, 0x01, 0x02, 0x03, 0x04] {
            assert_eq!(TouchPolicy::from_byte(b).to_byte(), b);
        }
    }
}
