use anyhow::Result;

#[derive(Debug, Clone)]
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
pub struct KeyInfo {
    pub fingerprint: String,
    pub created: Option<String>,
    pub key_attributes: String,
}

pub fn get_openpgp_state() -> Result<OpenPgpState> {
    // TODO: Implement by parsing `gpg --card-status` output
    Ok(OpenPgpState {
        card_present: false,
        version: String::new(),
        signature_key: None,
        encryption_key: None,
        authentication_key: None,
        cardholder_name: None,
        public_key_url: None,
    })
}
