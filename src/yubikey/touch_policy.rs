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

/// Parse touch policies from `ykman openpgp info` output.
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

/// Set the touch policy for a given OpenPGP slot non-interactively.
///
/// Spawns `ykman openpgp keys set-touch <slot> <policy> --force` with
/// piped IO (no terminal escape). The `--force` flag suppresses the Admin
/// PIN prompt — the caller must ensure ykman has stored credentials or that
/// the device does not require PIN confirmation for this operation.
/// If `serial` is provided, prepends `--device <serial>` to select a specific key.
///
/// Valid slots: "sig", "enc", "aut", "att"
#[allow(dead_code)]
pub fn set_touch_policy(
    slot: &str,
    policy: &TouchPolicy,
    serial: Option<u32>,
) -> Result<String> {
    match slot {
        "sig" | "enc" | "aut" | "att" => {}
        other => anyhow::bail!(
            "Invalid slot '{}'. Must be one of: sig, enc, aut, att",
            other
        ),
    }

    let ykman = crate::yubikey::pin_operations::find_ykman()?;
    let mut cmd = std::process::Command::new(&ykman);

    if let Some(s) = serial {
        cmd.args(["--device", &s.to_string()]);
    }

    cmd.args([
        "openpgp",
        "keys",
        "set-touch",
        slot,
        policy.as_ykman_arg(),
        "--force",
    ]);

    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output = cmd.output()?;

    if output.status.success() {
        Ok(format!("Touch policy set to {} for {}", policy, slot))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to set touch policy: {}", stderr.trim())
    }
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
}
