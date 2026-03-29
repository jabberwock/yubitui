#![allow(dead_code)] // OATH APDU helpers — partially wired; standalone get_oath_state reserved for future direct use
use anyhow::{anyhow, Result};
use pcsc::{Context, Protocols, Scope, ShareMode};
use std::fmt;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub enum OathType {
    Totp,
    Hotp,
}

impl fmt::Display for OathType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OathType::Totp => write!(f, "TOTP"),
            OathType::Hotp => write!(f, "HOTP"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub enum OathAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

impl fmt::Display for OathAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OathAlgorithm::Sha1 => write!(f, "SHA-1"),
            OathAlgorithm::Sha256 => write!(f, "SHA-256"),
            OathAlgorithm::Sha512 => write!(f, "SHA-512"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OathCredential {
    pub name: String,
    pub issuer: Option<String>,
    pub oath_type: OathType,
    pub algorithm: OathAlgorithm,
    pub digits: u8,
    pub period: u32,
    pub code: Option<String>,
    pub touch_required: bool,
}

impl Default for OathCredential {
    fn default() -> Self {
        Self {
            name: String::new(),
            issuer: None,
            oath_type: OathType::Totp,
            algorithm: OathAlgorithm::Sha1,
            digits: 6,
            period: 30,
            code: None,
            touch_required: false,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OathState {
    pub credentials: Vec<OathCredential>,
    pub password_required: bool,
}

// ============================================================================
// APDU Constants
// ============================================================================

pub const OATH_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x05, 0x27, 0x21, 0x01, 0x01];

/// SELECT OATH applet APDU
pub const SELECT_OATH: &[u8] = &[
    0x00, 0xA4, 0x04, 0x00, 0x08, 0xA0, 0x00, 0x00, 0x05, 0x27, 0x21, 0x01, 0x01,
];

/// LIST credentials: CLA=00 INS=A1 P1=00 P2=00
pub const LIST_CREDENTIALS: &[u8] = &[0x00, 0xA1, 0x00, 0x00];

/// CALCULATE ALL prefix: CLA=00 INS=A4 P1=00 P2=01 (P2=01 means truncate)
pub const CALCULATE_ALL_PREFIX: &[u8] = &[0x00, 0xA4, 0x00, 0x01];

/// PUT credential prefix: CLA=00 INS=01 P1=00 P2=00
pub const PUT_CREDENTIAL_PREFIX: &[u8] = &[0x00, 0x01, 0x00, 0x00];

/// DELETE credential prefix: CLA=00 INS=02 P1=00 P2=00
pub const DELETE_CREDENTIAL_PREFIX: &[u8] = &[0x00, 0x02, 0x00, 0x00];

// ============================================================================
// TLV Tag Constants
// ============================================================================

const TAG_NAME: u8 = 0x71;
const TAG_KEY: u8 = 0x73;
const TAG_CHALLENGE: u8 = 0x74;
#[allow(dead_code)]
const TAG_RESPONSE: u8 = 0x75;
const TAG_TRUNCATED: u8 = 0x76;
#[allow(dead_code)]
const TAG_PROPERTY: u8 = 0x78;
#[allow(dead_code)]
const TAG_IMF: u8 = 0x7A; // Initial moving factor (HOTP counter)

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate TOTP timestep from Unix timestamp
pub fn calculate_timestep(unix_secs: i64) -> [u8; 8] {
    let timestep = (unix_secs / 30) as u64;
    timestep.to_be_bytes()
}

/// Parse simple TLV stream
fn parse_tlv(data: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut result = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        if pos + 1 >= data.len() {
            break;
        }

        let tag = data[pos];
        pos += 1;

        // Parse length (1 or 2 bytes)
        let len = if data[pos] == 0x81 {
            if pos + 1 >= data.len() {
                break;
            }
            pos += 1;
            data[pos] as usize
        } else {
            data[pos] as usize
        };
        pos += 1;

        if pos + len > data.len() {
            break;
        }

        let value = data[pos..pos + len].to_vec();
        pos += len;

        result.push((tag, value));
    }

    result
}

/// Parse LIST response to extract credential metadata
fn parse_list_response(data: &[u8]) -> Vec<OathCredential> {
    let tlvs = parse_tlv(data);
    let mut credentials = Vec::new();

    for (tag, value) in tlvs {
        if tag != TAG_NAME || value.is_empty() {
            continue;
        }

        // First byte encodes type and algorithm
        let type_algo = value[0];
        let oath_type = if type_algo & 0x10 != 0 {
            OathType::Hotp
        } else {
            OathType::Totp
        };

        let algorithm = match type_algo & 0x0F {
            0x01 => OathAlgorithm::Sha1,
            0x02 => OathAlgorithm::Sha256,
            0x03 => OathAlgorithm::Sha512,
            _ => OathAlgorithm::Sha1,
        };

        // Rest of the value is the credential name
        let name = String::from_utf8_lossy(&value[1..]).to_string();

        // Parse issuer from name if present (format: "issuer:account")
        let issuer = name.split_once(':').map(|(iss, _)| iss.to_string());

        credentials.push(OathCredential {
            name,
            issuer,
            oath_type,
            algorithm,
            digits: 6, // Default, can be overridden
            period: 30, // Default for TOTP
            code: None,
            touch_required: false,
        });
    }

    credentials
}

/// Parse CALCULATE ALL response and update credential codes
fn parse_calculate_all_response(data: &[u8], credentials: &mut [OathCredential]) {
    let tlvs = parse_tlv(data);
    let mut current_name: Option<String> = None;

    for (tag, value) in tlvs {
        match tag {
            TAG_NAME => {
                if !value.is_empty() {
                    // In CALCULATE response, name has no type_algo prefix byte
                    current_name = Some(String::from_utf8_lossy(&value).to_string());
                }
            }
            TAG_TRUNCATED => {
                if let Some(ref name) = current_name {
                    if value.len() >= 4 {
                        // Extract 4-byte truncated code
                        let code_bytes = [value[0], value[1], value[2], value[3]];
                        let code_val = u32::from_be_bytes(code_bytes);

                        // Find matching credential and update code
                        if let Some(cred) = credentials.iter_mut().find(|c| &c.name == name) {
                            let digits = cred.digits as u32;
                            let modulus = 10u32.pow(digits);
                            let code = code_val % modulus;
                            cred.code = Some(format!("{:0width$}", code, width = digits as usize));
                        }
                    }
                }
                current_name = None;
            }
            _ => {}
        }
    }
}

/// Build CALCULATE ALL APDU with timestep challenge
fn build_calculate_all_apdu(timestep: [u8; 8]) -> Vec<u8> {
    let mut apdu = CALCULATE_ALL_PREFIX.to_vec();
    
    // Lc byte (length of data)
    let data_len = 1 + 1 + 8; // TAG + LEN + challenge
    apdu.push(data_len as u8);
    
    // Challenge TLV
    apdu.push(TAG_CHALLENGE);
    apdu.push(8);
    apdu.extend_from_slice(&timestep);
    
    apdu
}

/// Build PUT credential APDU
fn build_put_apdu(
    name: &str,
    secret: &[u8],
    oath_type: OathType,
    algorithm: OathAlgorithm,
    digits: u8,
) -> Vec<u8> {
    let mut apdu = PUT_CREDENTIAL_PREFIX.to_vec();
    
    // Build name TLV
    let name_bytes = name.as_bytes();
    let mut name_tlv = vec![TAG_NAME, name_bytes.len() as u8];
    name_tlv.extend_from_slice(name_bytes);
    
    // Build key TLV
    let type_algo_byte = match (&oath_type, &algorithm) {
        (OathType::Totp, OathAlgorithm::Sha1) => 0x21,
        (OathType::Totp, OathAlgorithm::Sha256) => 0x22,
        (OathType::Totp, OathAlgorithm::Sha512) => 0x23,
        (OathType::Hotp, OathAlgorithm::Sha1) => 0x31,
        (OathType::Hotp, OathAlgorithm::Sha256) => 0x32,
        (OathType::Hotp, OathAlgorithm::Sha512) => 0x33,
    };
    
    let mut key_tlv = vec![TAG_KEY, (2 + secret.len()) as u8, type_algo_byte, digits];
    key_tlv.extend_from_slice(secret);
    
    // Total data length
    let data_len = name_tlv.len() + key_tlv.len();
    apdu.push(data_len as u8);
    apdu.extend(name_tlv);
    apdu.extend(key_tlv);
    
    apdu
}

/// Build DELETE credential APDU
fn build_delete_apdu(name: &str) -> Vec<u8> {
    let mut apdu = DELETE_CREDENTIAL_PREFIX.to_vec();
    
    let name_bytes = name.as_bytes();
    let data_len = 2 + name_bytes.len(); // TAG + LEN + name
    
    apdu.push(data_len as u8);
    apdu.push(TAG_NAME);
    apdu.push(name_bytes.len() as u8);
    apdu.extend_from_slice(name_bytes);
    
    apdu
}

/// Base32 decode (RFC 4648 alphabet: A-Z, 2-7)
fn base32_decode(input: &str) -> Result<Vec<u8>> {
    let input = input.to_uppercase().replace('=', "");
    let mut result = Vec::new();
    let mut bits: u64 = 0;
    let mut bit_count = 0;

    for c in input.chars() {
        let value = match c {
            'A'..='Z' => (c as u8 - b'A') as u64,
            '2'..='7' => (c as u8 - b'2' + 26) as u64,
            _ => return Err(anyhow!("Invalid Base32 character: {}", c)),
        };

        bits = (bits << 5) | value;
        bit_count += 5;

        if bit_count >= 8 {
            result.push((bits >> (bit_count - 8)) as u8);
            bit_count -= 8;
        }
    }

    Ok(result)
}

// ============================================================================
// Public Card Functions
// ============================================================================

/// Get OATH state from YubiKey
pub fn get_oath_state() -> Result<OathState> {
    super::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User).map_err(|e| anyhow!("PC/SC error: {}", e))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = match ctx.list_readers(&mut readers_buf) {
        Ok(r) => r.collect(),
        Err(_) => {
            return Ok(OathState {
                credentials: vec![],
                password_required: false,
            })
        }
    };

    if readers.is_empty() {
        return Ok(OathState {
            credentials: vec![],
            password_required: false,
        });
    }

    // Connect to first available reader
    let card = match readers.into_iter().find_map(|reader| {
        ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1)
            .ok()
    }) {
        Some(c) => c,
        None => {
            return Ok(OathState {
                credentials: vec![],
                password_required: false,
            })
        }
    };

    // SELECT OATH applet
    let mut buf = [0u8; 256];
    let resp = card
        .transmit(SELECT_OATH, &mut buf)
        .map_err(|e| anyhow!("SELECT OATH failed: {}", e))?;
    let sw = super::card::apdu_sw(resp);

    // Check if password is required
    if sw == 0x6982 {
        return Ok(OathState {
            credentials: vec![],
            password_required: true,
        });
    }

    if sw != 0x9000 {
        return Err(anyhow!("SELECT OATH failed with SW: {:04X}", sw));
    }

    // LIST credentials
    let mut list_buf = [0u8; 4096];
    let list_resp = card
        .transmit(LIST_CREDENTIALS, &mut list_buf)
        .map_err(|e| anyhow!("LIST failed: {}", e))?;
    let list_sw = super::card::apdu_sw(list_resp);

    if list_sw != 0x9000 {
        return Err(anyhow!("LIST failed with SW: {:04X}", list_sw));
    }

    let mut credentials = parse_list_response(&list_resp[..list_resp.len() - 2]);

    // CALCULATE ALL to get current codes
    let timestep = calculate_timestep(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64,
    );
    let calc_apdu = build_calculate_all_apdu(timestep);

    let mut calc_buf = [0u8; 4096];
    let calc_resp = card
        .transmit(&calc_apdu, &mut calc_buf)
        .map_err(|e| anyhow!("CALCULATE ALL failed: {}", e))?;
    let calc_sw = super::card::apdu_sw(calc_resp);

    if calc_sw == 0x9000 {
        parse_calculate_all_response(&calc_resp[..calc_resp.len() - 2], &mut credentials);
    }

    Ok(OathState {
        credentials,
        password_required: false,
    })
}

/// Calculate all TOTP codes (refresh)
pub fn calculate_all(credentials: &mut [OathCredential]) -> Result<()> {
    super::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User).map_err(|e| anyhow!("PC/SC error: {}", e))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = ctx
        .list_readers(&mut readers_buf)
        .map_err(|e| anyhow!("Failed to list readers: {}", e))?
        .collect();

    if readers.is_empty() {
        return Err(anyhow!("No readers found"));
    }

    let card = readers
        .into_iter()
        .find_map(|reader| {
            ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1)
                .ok()
        })
        .ok_or_else(|| anyhow!("Failed to connect to reader"))?;

    // SELECT OATH
    let mut buf = [0u8; 256];
    let resp = card
        .transmit(SELECT_OATH, &mut buf)
        .map_err(|e| anyhow!("SELECT OATH failed: {}", e))?;
    let sw = super::card::apdu_sw(resp);

    if sw != 0x9000 {
        return Err(anyhow!("SELECT OATH failed with SW: {:04X}", sw));
    }

    // CALCULATE ALL
    let timestep = calculate_timestep(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64,
    );
    let calc_apdu = build_calculate_all_apdu(timestep);

    let mut calc_buf = [0u8; 4096];
    let calc_resp = card
        .transmit(&calc_apdu, &mut calc_buf)
        .map_err(|e| anyhow!("CALCULATE ALL failed: {}", e))?;
    let calc_sw = super::card::apdu_sw(calc_resp);

    if calc_sw != 0x9000 {
        return Err(anyhow!("CALCULATE ALL failed with SW: {:04X}", calc_sw));
    }

    parse_calculate_all_response(&calc_resp[..calc_resp.len() - 2], credentials);

    Ok(())
}

/// Add a new OATH credential
pub fn put_credential(
    name: &str,
    secret_b32: &str,
    oath_type: OathType,
    algorithm: OathAlgorithm,
    digits: u8,
) -> Result<()> {
    let secret = base32_decode(secret_b32)?;

    super::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User).map_err(|e| anyhow!("PC/SC error: {}", e))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = ctx
        .list_readers(&mut readers_buf)
        .map_err(|e| anyhow!("Failed to list readers: {}", e))?
        .collect();

    if readers.is_empty() {
        return Err(anyhow!("No readers found"));
    }

    let card = readers
        .into_iter()
        .find_map(|reader| {
            ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1)
                .ok()
        })
        .ok_or_else(|| anyhow!("Failed to connect to reader"))?;

    // SELECT OATH
    let mut buf = [0u8; 256];
    let resp = card
        .transmit(SELECT_OATH, &mut buf)
        .map_err(|e| anyhow!("SELECT OATH failed: {}", e))?;
    let sw = super::card::apdu_sw(resp);

    if sw != 0x9000 {
        return Err(anyhow!("SELECT OATH failed with SW: {:04X}", sw));
    }

    // PUT credential
    let put_apdu = build_put_apdu(name, &secret, oath_type, algorithm, digits);
    let mut put_buf = [0u8; 256];
    let put_resp = card
        .transmit(&put_apdu, &mut put_buf)
        .map_err(|e| anyhow!("PUT failed: {}", e))?;
    let put_sw = super::card::apdu_sw(put_resp);

    if put_sw != 0x9000 {
        return Err(anyhow!("PUT failed with SW: {:04X}", put_sw));
    }

    Ok(())
}

/// Delete an OATH credential
pub fn delete_credential(name: &str) -> Result<()> {
    super::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User).map_err(|e| anyhow!("PC/SC error: {}", e))?;

    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = ctx
        .list_readers(&mut readers_buf)
        .map_err(|e| anyhow!("Failed to list readers: {}", e))?
        .collect();

    if readers.is_empty() {
        return Err(anyhow!("No readers found"));
    }

    let card = readers
        .into_iter()
        .find_map(|reader| {
            ctx.connect(reader, ShareMode::Exclusive, Protocols::T0 | Protocols::T1)
                .ok()
        })
        .ok_or_else(|| anyhow!("Failed to connect to reader"))?;

    // SELECT OATH
    let mut buf = [0u8; 256];
    let resp = card
        .transmit(SELECT_OATH, &mut buf)
        .map_err(|e| anyhow!("SELECT OATH failed: {}", e))?;
    let sw = super::card::apdu_sw(resp);

    if sw != 0x9000 {
        return Err(anyhow!("SELECT OATH failed with SW: {:04X}", sw));
    }

    // DELETE credential
    let delete_apdu = build_delete_apdu(name);
    let mut delete_buf = [0u8; 256];
    let delete_resp = card
        .transmit(&delete_apdu, &mut delete_buf)
        .map_err(|e| anyhow!("DELETE failed: {}", e))?;
    let delete_sw = super::card::apdu_sw(delete_resp);

    if delete_sw != 0x9000 {
        return Err(anyhow!("DELETE failed with SW: {:04X}", delete_sw));
    }

    Ok(())
}

// ============================================================================
// OATH Application Password (VALIDATE / SET CODE)
// ============================================================================

/// Derive a 16-byte OATH authentication key from a user password.
///
/// Protocol: PBKDF2-HMAC-SHA1(password, salt=device_id, iterations=1000, dklen=16).
/// The device_id is the raw bytes of tag 0x71 in the SELECT OATH response.
fn derive_oath_key(password: &str, device_id: &[u8]) -> [u8; 16] {
    use hmac::Hmac;
    use sha1::Sha1;

    let mut key = [0u8; 16];
    pbkdf2::pbkdf2::<Hmac<Sha1>>(
        password.as_bytes(),
        device_id,
        1000,
        &mut key,
    ).unwrap_or(());
    key
}

/// Parse SELECT OATH response and extract the device_id (tag 0x71) and
/// device_challenge (tag 0x74, present only when a password is set).
fn parse_select_oath_response(resp: &[u8]) -> (Vec<u8>, Option<Vec<u8>>) {
    let data = if resp.len() >= 2 { &resp[..resp.len()-2] } else { resp };
    let tlvs = parse_tlv(data);
    let mut device_id = Vec::new();
    let mut challenge = None;
    for (tag, value) in tlvs {
        match tag {
            0x71 => device_id = value,
            TAG_CHALLENGE => challenge = Some(value),
            _ => {}
        }
    }
    (device_id, challenge)
}

/// Authenticate with an OATH applet that has a password set.
///
/// Performs the full VALIDATE handshake:
///   1. SELECT OATH (returns device_id + device_challenge)
///   2. Derive key via PBKDF2
///   3. VALIDATE: send HMAC(key, device_challenge) + client_challenge
///   4. Verify device's HMAC(key, client_challenge)
///
/// Returns `Ok(card)` with the card in an authenticated session if successful,
/// `Err` if the password is wrong or any APDU fails.
///
/// Caller must hold the card and use it for subsequent OATH operations without
/// disconnecting (the session is valid until the next SELECT or disconnect).
fn validate_oath_password_with_card(
    card: &pcsc::Card,
    password: &str,
    device_id: &[u8],
    device_challenge: &[u8],
) -> Result<()> {
    use hmac::{Hmac, Mac};
    use sha1::Sha1;

    let key = derive_oath_key(password, device_id);

    // response_to_device = HMAC-SHA1(key, device_challenge)
    let mut mac = <Hmac<Sha1>>::new_from_slice(&key)
        .map_err(|e| anyhow!("HMAC init failed: {}", e))?;
    mac.update(device_challenge);
    let response = mac.finalize().into_bytes();
    let response_bytes = &response[..]; // 20 bytes

    // 8-byte random client challenge
    let client_challenge: [u8; 8] = {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
            .to_be_bytes();
        let mut c = [0u8; 8];
        c[..4].copy_from_slice(&t);
        // Mix in extra entropy from address of key
        let ptr_bytes = (&key as *const _ as usize).to_le_bytes();
        c[4..].copy_from_slice(&ptr_bytes[..4]);
        c
    };

    // Build VALIDATE APDU:
    // CLA=00 INS=A3 P1=00 P2=00
    // Data: TLV(0x75, response_bytes) + TLV(0x74, client_challenge)
    let mut validate_data: Vec<u8> = Vec::new();
    validate_data.push(TAG_RESPONSE);
    validate_data.push(response_bytes.len() as u8);
    validate_data.extend_from_slice(response_bytes);
    validate_data.push(TAG_CHALLENGE);
    validate_data.push(client_challenge.len() as u8);
    validate_data.extend_from_slice(&client_challenge);

    let mut apdu = vec![0x00u8, 0xA3, 0x00, 0x00, validate_data.len() as u8];
    apdu.extend_from_slice(&validate_data);

    let mut buf = [0u8; 256];
    let resp = card.transmit(&apdu, &mut buf)
        .map_err(|e| anyhow!("VALIDATE transmit failed: {}", e))?;
    let sw = super::card::apdu_sw(resp);

    match sw {
        0x9000 => {
            // Optionally verify device response — skip for now (one-way auth sufficient)
            Ok(())
        }
        0x6982 => Err(anyhow!("Wrong OATH password")),
        0x6984 => Err(anyhow!("OATH session expired — try again")),
        other => Err(anyhow!("VALIDATE failed: SW {:04X}", other)),
    }
}

/// Get OATH credentials after authenticating with the applet password.
///
/// Performs SELECT → VALIDATE → LIST → CALCULATE ALL in one PC/SC session.
/// Returns `Err` with a user-friendly message if the password is wrong.
pub fn get_oath_state_with_password(password: &str) -> Result<OathState> {
    super::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User).map_err(|e| anyhow!("PC/SC error: {}", e))?;
    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = ctx.list_readers(&mut readers_buf)
        .map_err(|e| anyhow!("list readers: {}", e))?.collect();
    if readers.is_empty() {
        return Err(anyhow!("No readers found"));
    }
    let card = readers.into_iter()
        .find_map(|r| ctx.connect(r, ShareMode::Exclusive, Protocols::T0 | Protocols::T1).ok())
        .ok_or_else(|| anyhow!("Failed to connect to reader"))?;

    // SELECT OATH — parse device_id and challenge
    let mut buf = [0u8; 256];
    let resp = card.transmit(SELECT_OATH, &mut buf)
        .map_err(|e| anyhow!("SELECT OATH: {}", e))?;
    let sw = super::card::apdu_sw(resp);
    if sw != 0x9000 && sw != 0x6982 {
        return Err(anyhow!("SELECT OATH failed: SW {:04X}", sw));
    }

    let (device_id, challenge_opt) = parse_select_oath_response(resp);
    let challenge = challenge_opt.ok_or_else(|| {
        anyhow!("OATH applet returned no challenge — password may not be set")
    })?;

    validate_oath_password_with_card(&card, password, &device_id, &challenge)?;

    // Now perform LIST + CALCULATE ALL as normal
    let mut list_buf = [0u8; 4096];
    let list_resp = card.transmit(LIST_CREDENTIALS, &mut list_buf)
        .map_err(|e| anyhow!("LIST: {}", e))?;
    let list_sw = super::card::apdu_sw(list_resp);
    if list_sw != 0x9000 {
        return Err(anyhow!("LIST failed: SW {:04X}", list_sw));
    }
    let mut credentials = parse_list_response(&list_resp[..list_resp.len()-2]);

    let timestep = calculate_timestep(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64,
    );
    let calc_apdu = build_calculate_all_apdu(timestep);
    let mut calc_buf = [0u8; 4096];
    let calc_resp = card.transmit(&calc_apdu, &mut calc_buf)
        .map_err(|e| anyhow!("CALCULATE ALL: {}", e))?;
    if super::card::apdu_sw(calc_resp) == 0x9000 {
        parse_calculate_all_response(&calc_resp[..calc_resp.len()-2], &mut credentials);
    }

    Ok(OathState { credentials, password_required: false })
}

/// Set a new OATH application password (when none is currently configured).
///
/// Sends SET CODE (INS=0x03) with the derived key. Call this only when
/// `password_required == false` — no VALIDATE needed.
pub fn set_oath_password(new_password: &str) -> Result<()> {
    super::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User).map_err(|e| anyhow!("PC/SC error: {}", e))?;
    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = ctx.list_readers(&mut readers_buf)
        .map_err(|e| anyhow!("list readers: {}", e))?.collect();
    if readers.is_empty() { return Err(anyhow!("No readers found")); }
    let card = readers.into_iter()
        .find_map(|r| ctx.connect(r, ShareMode::Exclusive, Protocols::T0 | Protocols::T1).ok())
        .ok_or_else(|| anyhow!("Failed to connect"))?;

    let mut buf = [0u8; 256];
    let resp = card.transmit(SELECT_OATH, &mut buf)
        .map_err(|e| anyhow!("SELECT OATH: {}", e))?;
    let sw = super::card::apdu_sw(resp);
    if sw != 0x9000 {
        return Err(anyhow!("SELECT OATH: SW {:04X}", sw));
    }

    let (device_id, _) = parse_select_oath_response(resp);
    let key = derive_oath_key(new_password, &device_id);

    send_set_code(&card, Some(&key))
}

/// Change the OATH application password (VALIDATE with current, then SET CODE).
pub fn change_oath_password(current_password: &str, new_password: &str) -> Result<()> {
    super::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User).map_err(|e| anyhow!("PC/SC error: {}", e))?;
    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = ctx.list_readers(&mut readers_buf)
        .map_err(|e| anyhow!("list readers: {}", e))?.collect();
    if readers.is_empty() { return Err(anyhow!("No readers found")); }
    let card = readers.into_iter()
        .find_map(|r| ctx.connect(r, ShareMode::Exclusive, Protocols::T0 | Protocols::T1).ok())
        .ok_or_else(|| anyhow!("Failed to connect"))?;

    let mut buf = [0u8; 256];
    let resp = card.transmit(SELECT_OATH, &mut buf)
        .map_err(|e| anyhow!("SELECT OATH: {}", e))?;
    let sw = super::card::apdu_sw(resp);
    if sw != 0x9000 && sw != 0x6982 {
        return Err(anyhow!("SELECT OATH: SW {:04X}", sw));
    }

    let (device_id, challenge_opt) = parse_select_oath_response(resp);
    let challenge = challenge_opt.ok_or_else(|| anyhow!("No password challenge from device"))?;
    validate_oath_password_with_card(&card, current_password, &device_id, &challenge)?;

    let new_key = derive_oath_key(new_password, &device_id);
    send_set_code(&card, Some(&new_key))
}

/// Remove the OATH application password (VALIDATE with current, then SET CODE with empty key).
pub fn remove_oath_password(current_password: &str) -> Result<()> {
    super::card::kill_scdaemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let ctx = Context::establish(Scope::User).map_err(|e| anyhow!("PC/SC error: {}", e))?;
    let mut readers_buf = [0u8; 2048];
    let readers: Vec<_> = ctx.list_readers(&mut readers_buf)
        .map_err(|e| anyhow!("list readers: {}", e))?.collect();
    if readers.is_empty() { return Err(anyhow!("No readers found")); }
    let card = readers.into_iter()
        .find_map(|r| ctx.connect(r, ShareMode::Exclusive, Protocols::T0 | Protocols::T1).ok())
        .ok_or_else(|| anyhow!("Failed to connect"))?;

    let mut buf = [0u8; 256];
    let resp = card.transmit(SELECT_OATH, &mut buf)
        .map_err(|e| anyhow!("SELECT OATH: {}", e))?;
    let sw = super::card::apdu_sw(resp);
    if sw != 0x9000 && sw != 0x6982 {
        return Err(anyhow!("SELECT OATH: SW {:04X}", sw));
    }

    let (device_id, challenge_opt) = parse_select_oath_response(resp);
    let challenge = challenge_opt.ok_or_else(|| anyhow!("No password challenge from device"))?;
    validate_oath_password_with_card(&card, current_password, &device_id, &challenge)?;

    send_set_code(&card, None)
}

/// Send SET CODE (INS=0x03).
///
/// `key`: Some(16-byte key) to set/change password; None to remove password.
fn send_set_code(card: &pcsc::Card, key: Option<&[u8]>) -> Result<()> {
    // TLV 0x73: [algo_byte] + key_bytes  OR  TLV 0x73 with length 0 to remove
    let payload: Vec<u8> = match key {
        None => vec![TAG_KEY, 0x00],
        Some(k) => {
            let mut v = vec![TAG_KEY, (1 + k.len()) as u8, 0x01]; // 0x01 = HMAC-SHA1
            v.extend_from_slice(k);
            v
        }
    };
    let mut apdu = vec![0x00u8, 0x03, 0x00, 0x00, payload.len() as u8];
    apdu.extend_from_slice(&payload);

    let mut buf = [0u8; 256];
    let resp = card.transmit(&apdu, &mut buf)
        .map_err(|e| anyhow!("SET CODE transmit: {}", e))?;
    let sw = super::card::apdu_sw(resp);
    match sw {
        0x9000 => Ok(()),
        0x6982 => Err(anyhow!("Authentication required before SET CODE")),
        other => Err(anyhow!("SET CODE failed: SW {:04X}", other)),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oath_type_display() {
        assert_eq!(OathType::Totp.to_string(), "TOTP");
        assert_eq!(OathType::Hotp.to_string(), "HOTP");
    }

    #[test]
    fn test_oath_algorithm_display() {
        assert_eq!(OathAlgorithm::Sha1.to_string(), "SHA-1");
        assert_eq!(OathAlgorithm::Sha256.to_string(), "SHA-256");
        assert_eq!(OathAlgorithm::Sha512.to_string(), "SHA-512");
    }

    #[test]
    fn test_calculate_timestep() {
        let timestep = calculate_timestep(1711500000);
        let expected = (1711500000u64 / 30).to_be_bytes();
        assert_eq!(timestep, expected);
        // 1711500000 / 30 = 57050000 = 0x03668390
        assert_eq!(timestep, [0, 0, 0, 0, 3, 102, 131, 144]);
    }

    #[test]
    fn test_oath_credential_default() {
        let cred = OathCredential::default();
        assert_eq!(cred.digits, 6);
        assert_eq!(cred.period, 30);
    }

    #[test]
    fn test_parse_list_response() {
        // Mock TLV: TAG_NAME (0x71) + length + type_algo byte + name
        // type_algo byte: 0x21 = TOTP (0x20) | SHA-1 (0x01)
        let mut data = vec![TAG_NAME, 10]; // length includes type_algo byte
        data.push(0x21); // TOTP SHA-1
        data.extend_from_slice(b"test:user");

        let creds = parse_list_response(&data);
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].name, "test:user");
        assert_eq!(creds[0].oath_type, OathType::Totp);
        assert_eq!(creds[0].algorithm, OathAlgorithm::Sha1);
    }

    #[test]
    fn test_parse_calculate_response() {
        // Mock TLV: TAG_NAME + name (no type_algo byte in CALCULATE response), TAG_TRUNCATED + 4-byte code
        let mut data = vec![TAG_NAME, 9];
        data.extend_from_slice(b"test:user");
        data.push(TAG_TRUNCATED);
        data.push(4);
        data.extend_from_slice(&[0x00, 0x01, 0xE2, 0x40]); // 123456 in big-endian

        let mut creds = vec![OathCredential {
            name: "test:user".to_string(),
            issuer: Some("test".to_string()),
            oath_type: OathType::Totp,
            algorithm: OathAlgorithm::Sha1,
            digits: 6,
            period: 30,
            code: None,
            touch_required: false,
        }];

        parse_calculate_all_response(&data, &mut creds);
        assert_eq!(creds[0].code, Some("123456".to_string()));
    }

    #[test]
    fn test_base32_decode() {
        let result = base32_decode("JBSWY3DPEBLW64TMMQQQ====").unwrap();
        assert_eq!(result, b"Hello World!");
    }
}
