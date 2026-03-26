use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OpenPgpState {
    pub card_present: bool,
    pub version: String,
    pub signature_key: Option<KeyInfo>,
    pub encryption_key: Option<KeyInfo>,
    pub authentication_key: Option<KeyInfo>,
    pub cardholder_name: Option<String>,
    pub public_key_url: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct KeyInfo {
    pub fingerprint: String,
    pub created: Option<String>,
    pub key_attributes: String,
}

pub fn get_openpgp_state() -> Result<OpenPgpState> {
    let output = Command::new("gpg").arg("--no-tty").arg("--batch").arg("--card-status").output()?;

    if !output.status.success() {
        return Ok(OpenPgpState {
            card_present: false,
            version: String::new(),
            signature_key: None,
            encryption_key: None,
            authentication_key: None,
            cardholder_name: None,
            public_key_url: None,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_card_status(&stdout)
}

pub fn parse_card_status(output: &str) -> Result<OpenPgpState> {
    let mut signature_key = None;
    let mut encryption_key = None;
    let mut authentication_key = None;
    let mut cardholder_name = None;
    let mut public_key_url = None;
    let mut version = String::new();
    let mut key_attributes = "rsa2048".to_string(); // default

    for line in output.lines() {
        let line = line.trim();

        if line.starts_with("Version ..........:") {
            version = line
                .split(':')
                .nth(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
        } else if line.starts_with("Signature key .....:") {
            let key = line.split(':').nth(1).map(|s| s.trim());
            if let Some(key_str) = key {
                if key_str != "[none]" && !key_str.is_empty() {
                    signature_key = Some(KeyInfo {
                        fingerprint: key_str.to_string(),
                        created: None,
                        key_attributes: key_attributes.clone(),
                    });
                }
            }
        } else if line.starts_with("Encryption key.....:") {
            let key = line.split(':').nth(1).map(|s| s.trim());
            if let Some(key_str) = key {
                if key_str != "[none]" && !key_str.is_empty() {
                    encryption_key = Some(KeyInfo {
                        fingerprint: key_str.to_string(),
                        created: None,
                        key_attributes: key_attributes.clone(),
                    });
                }
            }
        } else if line.starts_with("Authentication key:") {
            let key = line.split(':').nth(1).map(|s| s.trim());
            if let Some(key_str) = key {
                if key_str != "[none]" && !key_str.is_empty() {
                    authentication_key = Some(KeyInfo {
                        fingerprint: key_str.to_string(),
                        created: None,
                        key_attributes: key_attributes.clone(),
                    });
                }
            }
        } else if line.starts_with("Name of cardholder:") {
            let name = line.split(':').nth(1).map(|s| s.trim());
            if let Some(name_str) = name {
                if name_str != "[not set]" && !name_str.is_empty() {
                    cardholder_name = Some(name_str.to_string());
                }
            }
        } else if line.starts_with("URL of public key :") {
            // Use split_once so that the URL itself (which contains ':') is kept intact
            let url_str = line.split_once(':').map(|(_label, rest)| rest.trim());
            if let Some(u) = url_str {
                if u != "[not set]" && !u.is_empty() {
                    public_key_url = Some(u.to_string());
                }
            }
        } else if line.starts_with("Key attributes ...:") {
            // Parse something like "rsa2048 rsa2048 rsa2048" or "ed25519 cv25519 ed25519"
            key_attributes = line
                .split(':')
                .nth(1)
                .map(|s| s.split_whitespace().next().unwrap_or("rsa2048").to_string())
                .unwrap_or_else(|| "rsa2048".to_string());
        }
    }

    Ok(OpenPgpState {
        card_present: true,
        version,
        signature_key,
        encryption_key,
        authentication_key,
        cardholder_name,
        public_key_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_card_status_full() {
        let input = "\
Version ..........: 3.4\n\
Name of cardholder: Test User\n\
URL of public key : https://example.com/key.asc\n\
Key attributes ...: ed25519 cv25519 ed25519\n\
Signature key .....: ABCD 1234 5678 9012 3456  7890 ABCD EF01 2345 6789\n\
Encryption key.....: 1111 2222 3333 4444 5555  6666 7777 8888 9999 0000\n\
Authentication key: AAAA BBBB CCCC DDDD EEEE  FFFF 0000 1111 2222 3333\n\
";
        let state = parse_card_status(input).unwrap();
        assert!(state.card_present);
        assert_eq!(state.version, "3.4");
        assert_eq!(state.cardholder_name.as_deref(), Some("Test User"));
        assert_eq!(
            state.public_key_url.as_deref(),
            Some("https://example.com/key.asc")
        );
        assert!(state.signature_key.is_some());
        assert!(state.encryption_key.is_some());
        assert!(state.authentication_key.is_some());
        let sig = state.signature_key.unwrap();
        assert!(sig.fingerprint.contains("ABCD"));
        assert_eq!(sig.key_attributes, "ed25519");
    }

    #[test]
    fn test_parse_card_status_empty() {
        let state = parse_card_status("").unwrap();
        assert!(state.card_present);
        assert!(state.signature_key.is_none());
        assert!(state.encryption_key.is_none());
        assert!(state.authentication_key.is_none());
        assert!(state.cardholder_name.is_none());
    }

    #[test]
    fn test_parse_card_status_none_keys() {
        let input = "\
Signature key .....: [none]\n\
Encryption key.....: [none]\n\
Authentication key: [none]\n\
";
        let state = parse_card_status(input).unwrap();
        assert!(state.signature_key.is_none());
        assert!(state.encryption_key.is_none());
        assert!(state.authentication_key.is_none());
    }

    #[test]
    fn test_parse_card_status_rsa() {
        let input = "Key attributes ...: rsa4096 rsa4096 rsa4096\n";
        let state = parse_card_status(input).unwrap();
        // key_attributes is updated but no key lines present so keys are None
        // The key_attributes default before this line is rsa2048, after it becomes rsa4096
        // We verify the state parses without error
        assert!(state.card_present);
        assert!(state.signature_key.is_none());
    }
}
