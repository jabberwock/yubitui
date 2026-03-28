pub mod app_state;
pub mod card;
pub mod gpg_status;
pub mod detection;
pub mod fido2;
pub mod mock;
pub mod key_operations;
pub mod oath;
pub mod openpgp;
pub mod pin;
pub mod pin_operations;
pub mod piv;
pub mod ssh;
pub mod attestation;
pub mod ssh_operations;
pub mod touch_policy;

pub use app_state::{AppState, Screen};

// YubiKey detection and management

use anyhow::Result;
use std::fmt;

#[derive(Debug, Clone, serde::Serialize)]
pub struct YubiKeyInfo {
    pub serial: u32,
    pub version: Version,
    pub model: Model,
    pub form_factor: FormFactor,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[allow(dead_code)]
pub enum Model {
    YubiKey5,
    YubiKey5C,
    YubiKey5CNFC,
    YubiKey5Ci,
    YubiKey5CNano,
    YubiKey5Nano,
    YubiKey5NFC,
    YubiKey4,
    YubiKey4C,
    YubiKey4Nano,
    YubiKeyNeo,
    Unknown,
}

impl Model {
    #[allow(dead_code)]
    pub fn supports_openpgp(&self) -> bool {
        !matches!(self, Model::Unknown)
    }

    #[allow(dead_code)]
    pub fn supports_piv(&self) -> bool {
        !matches!(self, Model::Unknown)
    }

    #[allow(dead_code)]
    pub fn supports_fido2(&self) -> bool {
        matches!(
            self,
            Model::YubiKey5
                | Model::YubiKey5C
                | Model::YubiKey5CNFC
                | Model::YubiKey5Ci
                | Model::YubiKey5CNano
                | Model::YubiKey5Nano
                | Model::YubiKey5NFC
        )
    }

    #[allow(dead_code)]
    pub fn max_rsa_bits(&self) -> u32 {
        match self {
            Model::YubiKey5
            | Model::YubiKey5C
            | Model::YubiKey5CNFC
            | Model::YubiKey5Ci
            | Model::YubiKey5CNano
            | Model::YubiKey5Nano
            | Model::YubiKey5NFC => 4096,
            Model::YubiKey4 | Model::YubiKey4C | Model::YubiKey4Nano => 4096,
            Model::YubiKeyNeo => 2048,
            Model::Unknown => 2048,
        }
    }
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Model::YubiKey5 => write!(f, "YubiKey 5"),
            Model::YubiKey5C => write!(f, "YubiKey 5C"),
            Model::YubiKey5CNFC => write!(f, "YubiKey 5C NFC"),
            Model::YubiKey5Ci => write!(f, "YubiKey 5Ci"),
            Model::YubiKey5CNano => write!(f, "YubiKey 5C Nano"),
            Model::YubiKey5Nano => write!(f, "YubiKey 5 Nano"),
            Model::YubiKey5NFC => write!(f, "YubiKey 5 NFC"),
            Model::YubiKey4 => write!(f, "YubiKey 4"),
            Model::YubiKey4C => write!(f, "YubiKey 4C"),
            Model::YubiKey4Nano => write!(f, "YubiKey 4 Nano"),
            Model::YubiKeyNeo => write!(f, "YubiKey NEO"),
            Model::Unknown => write!(f, "Unknown YubiKey"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[allow(dead_code)]
pub enum FormFactor {
    UsbA,
    UsbC,
    Nano,
    Unknown,
}

impl fmt::Display for FormFactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormFactor::UsbA => write!(f, "USB-A"),
            FormFactor::UsbC => write!(f, "USB-C"),
            FormFactor::Nano => write!(f, "Nano"),
            FormFactor::Unknown => write!(f, "Unknown"),
        }
    }
}

impl fmt::Display for YubiKeyInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (SN: {}, FW: {}, Form: {})",
            self.model, self.serial, self.version, self.form_factor
        )
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct YubiKeyState {
    pub info: YubiKeyInfo,
    pub openpgp: Option<openpgp::OpenPgpState>,
    pub oath: Option<oath::OathState>,
    #[allow(dead_code)]
    pub piv: Option<piv::PivState>,
    pub fido2: Option<fido2::Fido2State>,
    pub pin_status: pin::PinStatus,
    pub touch_policies: Option<touch_policy::TouchPolicies>,
}

impl YubiKeyState {
    #[allow(dead_code)]
    pub fn detect() -> Result<Option<Self>> {
        detection::detect_yubikey_state()
    }

    pub fn detect_all() -> Result<Vec<Self>> {
        detection::detect_all_yubikey_states()
    }
}
