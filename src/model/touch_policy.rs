use anyhow::Result;
use sha2::{Digest, Sha256};
use std::fmt;

/// Touch policy variants for OpenPGP slots.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[allow(dead_code)]
pub struct TouchPolicies {
    pub signature: TouchPolicy,
    pub encryption: TouchPolicy,
    pub authentication: TouchPolicy,
    pub attestation: TouchPolicy,
}

/// Parsed fields from OpenPGP DO 0xF9 (KDF-DO).
struct KdfDo {
    /// Hash algorithm: 0x08 = SHA-256
    hash_algo: u8,
    /// Iteration count for Iterated S2K (4-byte big-endian u32 from tag 0x83).
    iteration_count: u32,
    /// Salt for PW3 / Admin PIN (tag 0x86, 8 bytes).
    salt_pw3: Vec<u8>,
}

/// Parse the KDF-DO (DO 0xF9) BER-TLV structure.
///
/// Returns `None` if KDF is not active (empty data, or first byte == 0x00).
/// Returns `Err` if the TLV is malformed or required tags are missing.
fn parse_kdf_do(kdf_data: &[u8]) -> Result<Option<KdfDo>> {
    // KDF not active: empty data.
    if kdf_data.is_empty() {
        return Ok(None);
    }
    // KDF not active: bare 0x00 byte (old format, no TLV wrapper).
    if kdf_data == [0x00] {
        return Ok(None);
    }

    let mut hash_algo: Option<u8> = None;
    let mut iteration_count: Option<u32> = None;
    let mut salt_pw3: Option<Vec<u8>> = None;

    let mut i = 0usize;
    while i < kdf_data.len() {
        let tag = kdf_data[i];
        i += 1;
        if i >= kdf_data.len() {
            break;
        }
        // BER-TLV short-form length (single byte, values 0x00–0x7F).
        // Long-form is not expected in DO 0xF9 but handle gracefully.
        let len = if kdf_data[i] & 0x80 == 0 {
            let l = kdf_data[i] as usize;
            i += 1;
            l
        } else {
            let num_octets = (kdf_data[i] & 0x7F) as usize;
            i += 1;
            if i + num_octets > kdf_data.len() {
                anyhow::bail!("KDF-DO TLV length field truncated");
            }
            let mut l = 0usize;
            for _ in 0..num_octets {
                l = (l << 8) | (kdf_data[i] as usize);
                i += 1;
            }
            l
        };

        if i + len > kdf_data.len() {
            anyhow::bail!("KDF-DO TLV value truncated (tag={:#04X} len={})", tag, len);
        }
        let value = &kdf_data[i..i + len];
        i += len;

        match tag {
            // Tag 0x81: KDF algorithm (0x00 = none, 0x03 = Iterated S2K)
            0x81 => {
                // If algorithm == 0x00, KDF is not active — return None immediately.
                if value.first().copied().unwrap_or(0x00) == 0x00 {
                    return Ok(None);
                }
            }
            // Tag 0x82: Hash algorithm (0x08 = SHA-256)
            0x82 => {
                if let Some(&b) = value.first() {
                    hash_algo = Some(b);
                }
            }
            // Tag 0x83: Iteration count (4-byte big-endian u32)
            0x83 => {
                if value.len() >= 4 {
                    let count = u32::from_be_bytes([value[0], value[1], value[2], value[3]]);
                    iteration_count = Some(count);
                }
            }
            // Tag 0x86: Salt for PW3 / Admin PIN (8 bytes)
            0x86 => {
                salt_pw3 = Some(value.to_vec());
            }
            // Tags 0x84 (PW1 salt), 0x85 (PW2/reset salt) — not needed here
            _ => {}
        }
    }

    let hash_algo = hash_algo.ok_or_else(|| anyhow::anyhow!("KDF-DO missing hash algorithm (tag 0x82)"))?;
    let iteration_count = iteration_count.ok_or_else(|| anyhow::anyhow!("KDF-DO missing iteration count (tag 0x83)"))?;
    let salt_pw3 = salt_pw3.ok_or_else(|| anyhow::anyhow!("KDF-DO missing PW3 salt (tag 0x86)"))?;

    Ok(Some(KdfDo { hash_algo, iteration_count, salt_pw3 }))
}

/// Hash a PIN using Iterated S2K (OpenPGP KDF) for use in VERIFY when KDF is active.
///
/// Algorithm: hash( repeat(salt || pin) until `count` bytes ) using SHA-256.
/// This matches the computation performed by GnuPG and ykman when KDF-DO is set.
///
/// `kdf_data` is the raw bytes returned by GET DATA for DO 0xF9.
/// `pin` is the raw PIN bytes (ASCII for numeric PINs).
///
/// Returns the hashed bytes to use in place of the raw PIN in the VERIFY APDU.
pub fn kdf_hash_pin(kdf_data: &[u8], pin: &[u8]) -> Result<Vec<u8>> {
    let kdf = parse_kdf_do(kdf_data)?
        .ok_or_else(|| anyhow::anyhow!("kdf_hash_pin called but KDF is not active"))?;

    // Only SHA-256 (0x08) is supported. Reject other hash algorithms.
    if kdf.hash_algo != 0x08 {
        anyhow::bail!(
            "Unsupported KDF hash algorithm {:#04X} (only SHA-256 / 0x08 is supported)",
            kdf.hash_algo
        );
    }

    // Build the S2K input: (salt || pin) repeated until `count` bytes total.
    let unit: Vec<u8> = kdf.salt_pw3.iter().chain(pin.iter()).copied().collect();
    let count = kdf.iteration_count as usize;

    // Guard against pathological iteration counts that would OOM.
    const MAX_COUNT: usize = 65_011_712; // OpenPGP spec maximum (0xFA << (6 + (0xFF & 0x1F)))
    if count > MAX_COUNT {
        anyhow::bail!("KDF iteration count {} exceeds maximum {}", count, MAX_COUNT);
    }

    let mut data = Vec::with_capacity(count);
    while data.len() < count {
        let remaining = count - data.len();
        let chunk = &unit[..remaining.min(unit.len())];
        data.extend_from_slice(chunk);
    }

    let hash = Sha256::digest(&data);
    Ok(hash.to_vec())
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
///   2. KDF check via GET DATA 0xF9 — if KDF is active, hash PIN with Iterated S2K (SHA-256)
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

    tracing::debug!(
        "set_touch_policy: slot={} do_tag={:#04X} policy={:?} policy_byte={:#04X}",
        slot,
        do_tag,
        policy,
        policy.to_byte()
    );

    // KDF check: GET DATA 0xF9 — if KDF is active, hash the PIN with Iterated S2K before VERIFY.
    //
    // When KDF-DO (DO 0xF9) is set, the card requires VERIFY to carry SHA-256(salt||pin) instead
    // of the raw PIN bytes. We parse the DO 0xF9 TLV to extract salt_pw3, iteration_count, and
    // hash_algo, then compute the S2K hash natively without any external tools.
    let pin_to_verify: Vec<u8> = match super::card::get_data(&card, 0x00, 0xF9) {
        Ok(kdf_data) => {
            tracing::debug!(
                "set_touch_policy: KDF DO 0xF9 = {:02X?} (len={})",
                kdf_data,
                kdf_data.len()
            );
            match parse_kdf_do(&kdf_data)? {
                Some(_) => {
                    tracing::debug!("set_touch_policy: KDF active — hashing Admin PIN with Iterated S2K");
                    kdf_hash_pin(&kdf_data, admin_pin.as_bytes())?
                }
                None => {
                    tracing::debug!("set_touch_policy: KDF inactive (algorithm=0x00) — using raw PIN");
                    admin_pin.as_bytes().to_vec()
                }
            }
        }
        Err(_) => {
            tracing::debug!("set_touch_policy: KDF DO 0xF9 not found (older YubiKey — no KDF) — using raw PIN");
            admin_pin.as_bytes().to_vec()
        }
    };

    // VERIFY Admin PIN: [CLA=00, INS=20, P1=00, P2=83 (Admin PIN), Lc, ...PIN bytes]
    // `pin_to_verify` is either the raw PIN (no KDF) or the S2K-hashed PIN (KDF active).
    let pin_len = pin_to_verify.len() as u8;
    let mut verify_apdu = vec![0x00u8, 0x20, 0x00, 0x83, pin_len];
    verify_apdu.extend_from_slice(&pin_to_verify);

    tracing::debug!(
        "set_touch_policy: VERIFY APDU [00 20 00 83 {:02X} <{}-byte PIN>]",
        pin_len,
        pin_len
    );

    let mut buf = [0u8; 256];
    let resp = card
        .transmit(&verify_apdu, &mut buf)
        .map_err(|e| anyhow::anyhow!("VERIFY transmit error: {e}"))?;
    let verify_sw = super::card::apdu_sw(resp);
    tracing::debug!("set_touch_policy: VERIFY SW={:#06X}", verify_sw);
    if verify_sw != 0x9000 {
        anyhow::bail!(
            "{}",
            super::card::apdu_error_message(verify_sw, "verifying Admin PIN")
        );
    }

    // PUT DATA: [CLA=00, INS=DA, P1=00, P2=DO, Lc=02, policy_byte, 0x20]
    let put_apdu = [0x00u8, 0xDA, 0x00, do_tag, 0x02, policy.to_byte(), 0x20];
    tracing::debug!(
        "set_touch_policy: PUT DATA APDU [{:02X?}]",
        put_apdu
    );
    let resp = card
        .transmit(&put_apdu, &mut buf)
        .map_err(|e| anyhow::anyhow!("PUT DATA transmit error: {e}"))?;
    let put_sw = super::card::apdu_sw(resp);
    tracing::debug!("set_touch_policy: PUT DATA SW={:#06X}", put_sw);
    if put_sw != 0x9000 {
        anyhow::bail!(
            "{}",
            super::card::apdu_error_message(put_sw, &format!("setting touch policy for {}", slot))
        );
    }

    // Read back the UIF DO in the same session to confirm the write was accepted.
    // This is a within-session check — if the card returns the new value here, the
    // write reached the card's working state (whether buffered or EEPROM).
    let readback_tag_p2: u8 = do_tag; // same tag: 0xD6-0xD9
    let readback_byte = match super::card::get_data(&card, 0x00, readback_tag_p2) {
        Ok(data) => {
            let rb = data.first().copied().unwrap_or(0xFF);
            tracing::debug!(
                "set_touch_policy: same-session readback DO {:#04X} = {:02X?} (first_byte={:#04X})",
                readback_tag_p2,
                data,
                rb
            );
            if rb != policy.to_byte() {
                tracing::debug!(
                    "set_touch_policy: WARNING — readback {:#04X} != expected {:#04X}; \
                     write may not have taken effect",
                    rb,
                    policy.to_byte()
                );
            }
            rb
        }
        Err(e) => {
            tracing::debug!("set_touch_policy: readback GET DATA failed: {}", e);
            0xFF
        }
    };

    // Disconnect with LeaveCard (not ResetCard) so the YubiKey can commit the EEPROM write.
    //
    // pcsc::Card::drop uses SCardDisconnect(SCARD_RESET_CARD) by default. On YubiKey firmware
    // the card uses a write-back model: PUT DATA writes to a session buffer and returns SW 9000
    // immediately, then commits to EEPROM before the next card reset. SCARD_RESET_CARD clears
    // that buffer before the commit, so the touch policy appears to succeed but doesn't persist.
    //
    // LeaveCard disconnects without issuing a card reset, allowing the EEPROM write to complete.
    // If disconnect fails (error ignored), the card is reset by Drop as a fallback.
    let disconnect_result = card.disconnect(pcsc::Disposition::LeaveCard);
    match &disconnect_result {
        Ok(()) => tracing::debug!("set_touch_policy: disconnect(LeaveCard) succeeded"),
        Err((_, e)) => tracing::debug!(
            "set_touch_policy: disconnect(LeaveCard) FAILED: {} — Drop will use RESET_CARD",
            e
        ),
    }
    // Discard result — if Err, the Card inside the tuple is dropped here with RESET_CARD.
    drop(disconnect_result);

    Ok(format!(
        "Touch policy set to {} for slot {} [VERIFY={:#06X} PUT={:#06X} readback={:#04X}]",
        policy, slot, verify_sw, put_sw, readback_byte
    ))
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
        assert_eq!(
            TouchPolicy::from_str("cached-fixed"),
            TouchPolicy::CachedFixed
        );
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
        assert_eq!(
            TouchPolicy::from_byte(0xFF),
            TouchPolicy::Unknown("0xFF".to_string())
        );
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

    /// Build a minimal DO 0xF9 BER-TLV payload for tests.
    ///
    /// Tags included: 0x81 (algo=0x03), 0x82 (hash=0x08), 0x83 (count as 4-byte BE), 0x86 (salt_pw3).
    fn make_kdf_do(count: u32, salt_pw3: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        // Tag 0x81: KDF algorithm = 0x03 (Iterated S2K)
        out.extend_from_slice(&[0x81, 0x01, 0x03]);
        // Tag 0x82: Hash algo = 0x08 (SHA-256)
        out.extend_from_slice(&[0x82, 0x01, 0x08]);
        // Tag 0x83: Iteration count (4-byte big-endian)
        let count_bytes = count.to_be_bytes();
        out.extend_from_slice(&[0x83, 0x04]);
        out.extend_from_slice(&count_bytes);
        // Tag 0x86: Salt for PW3 (Admin PIN)
        out.push(0x86);
        out.push(salt_pw3.len() as u8);
        out.extend_from_slice(salt_pw3);
        out
    }

    #[test]
    fn test_parse_kdf_do_inactive_empty() {
        // Empty data → KDF not active
        let result = parse_kdf_do(&[]).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_kdf_do_inactive_zero_byte() {
        // First byte 0x00 → KDF not active
        let result = parse_kdf_do(&[0x00]).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_kdf_do_active() {
        let salt = [0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let kdf_data = make_kdf_do(1024, &salt);
        let kdf = parse_kdf_do(&kdf_data).unwrap().expect("should be active");
        assert_eq!(kdf.hash_algo, 0x08);
        assert_eq!(kdf.iteration_count, 1024);
        assert_eq!(kdf.salt_pw3, salt);
    }

    #[test]
    fn test_kdf_hash_pin_known_vector() {
        // Construct a simple KDF-DO and verify the S2K output matches manual computation.
        // salt = [0xAA; 8], count = 16, pin = b"12345678"
        // unit = salt || pin = 16 bytes; count = 16 → data = unit (exactly 16 bytes)
        // SHA-256([0xAA * 8 || b"12345678"])
        let salt = [0xAAu8; 8];
        let pin = b"12345678";
        let count: u32 = 16; // exactly one unit length (8 + 8 = 16)
        let kdf_data = make_kdf_do(count, &salt);

        let hash = kdf_hash_pin(&kdf_data, pin).unwrap();
        assert_eq!(hash.len(), 32, "SHA-256 output must be 32 bytes");

        // Compute expected: SHA-256(salt || pin) since count == len(salt||pin)
        use sha2::{Digest, Sha256};
        let mut input = Vec::new();
        input.extend_from_slice(&salt);
        input.extend_from_slice(pin);
        let expected = Sha256::digest(&input).to_vec();
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_kdf_hash_pin_repeated_input() {
        // count = 32 with unit of 16 bytes → data = unit repeated twice
        let salt = [0xBBu8; 8];
        let pin = b"adminpin";
        let count: u32 = 32;
        let kdf_data = make_kdf_do(count, &salt);

        let hash = kdf_hash_pin(&kdf_data, pin).unwrap();
        assert_eq!(hash.len(), 32);

        use sha2::{Digest, Sha256};
        let unit: Vec<u8> = salt.iter().chain(pin.iter()).copied().collect();
        let data: Vec<u8> = unit.iter().chain(unit.iter()).copied().collect(); // unit twice
        let expected = Sha256::digest(&data).to_vec();
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_kdf_hash_pin_partial_last_chunk() {
        // count = 20 with unit of 16 bytes → first 16 bytes = unit, last 4 = first 4 of unit
        let salt = [0xCCu8; 8];
        let pin = b"shortpin";
        let count: u32 = 20;
        let kdf_data = make_kdf_do(count, &salt);

        let hash = kdf_hash_pin(&kdf_data, pin).unwrap();
        assert_eq!(hash.len(), 32);

        use sha2::{Digest, Sha256};
        let unit: Vec<u8> = salt.iter().chain(pin.iter()).copied().collect();
        let data: Vec<u8> = unit.iter().chain(unit[..4].iter()).copied().collect(); // 16 + 4 = 20
        let expected = Sha256::digest(&data).to_vec();
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_kdf_hash_pin_unsupported_algo() {
        // Build KDF-DO with hash algo = 0x09 (SHA-384, not supported)
        let salt = [0x01u8; 8];
        let mut kdf_data = make_kdf_do(16, &salt);
        // Patch byte at offset 4 (tag 0x82, len 0x01, value): find tag 0x82 and update value
        if let Some(pos) = kdf_data.windows(2).position(|w| w == [0x82, 0x01]) {
            kdf_data[pos + 2] = 0x09; // SHA-384
        }
        let result = kdf_hash_pin(&kdf_data, b"pin");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Unsupported"), "expected unsupported algo error, got: {msg}");
    }
}
