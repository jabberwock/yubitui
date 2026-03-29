use crate::model::YubiKeyState;

/// Heuristic: returns true if the YubiKey appears to be in factory-default state.
///
/// Uses model data only — no extra PC/SC calls at startup.
///
/// Criteria (all must be true):
/// - FIDO2 PIN not set
/// - Zero OATH credentials
/// - No PIV slots occupied
///
/// NOTE: EDU-04's PIV management key AUTHENTICATE check is replaced by the slot-empty
/// heuristic per research Pitfall 5 (double scdaemon kill at startup). Full AUTHENTICATE
/// check deferred to v2.
///
/// If any field is None (e.g., FIDO2 not available on this key model), that criterion
/// returns false — conservative: we don't show onboarding unless we can confirm all
/// three conditions.
pub fn is_factory_default(yk: &YubiKeyState) -> bool {
    let no_fido2_pin = yk.fido2.as_ref().map(|f| !f.pin_is_set).unwrap_or(false);
    let zero_oath = yk.oath.as_ref().map(|o| o.credentials.is_empty()).unwrap_or(false);
    let piv_empty = yk.piv.as_ref().map(|p| p.slots.is_empty()).unwrap_or(false);
    no_fido2_pin && zero_oath && piv_empty
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{YubiKeyInfo, Version, Model, FormFactor};
    use crate::model::fido2::Fido2State;
    use crate::model::oath::{OathState, OathCredential, OathType, OathAlgorithm};
    use crate::model::piv::{PivState, SlotInfo};

    fn base_yk() -> YubiKeyState {
        YubiKeyState {
            info: YubiKeyInfo {
                serial: 1234,
                version: Version { major: 5, minor: 4, patch: 3 },
                model: Model::YubiKey5NFC,
                form_factor: FormFactor::UsbA,
            },
            openpgp: None,
            oath: None,
            piv: None,
            fido2: None,
            otp: None,
            pin_status: pin_status(),
            touch_policies: None,
        }
    }

    fn pin_status() -> crate::model::pin::PinStatus {
        crate::model::pin::PinStatus {
            user_pin_retries: 3,
            admin_pin_retries: 3,
            reset_code_retries: 3,
            user_pin_blocked: false,
            admin_pin_blocked: false,
        }
    }

    fn empty_fido2() -> Fido2State {
        Fido2State {
            firmware_version: None,
            algorithms: vec![],
            pin_is_set: false,
            pin_retry_count: 8,
            supports_cred_mgmt: false,
            credentials: Some(vec![]),
        }
    }

    fn empty_oath() -> OathState {
        OathState { credentials: vec![], password_required: false }
    }

    fn empty_piv() -> PivState {
        PivState { slots: vec![] }
    }

    fn occupied_oath() -> OathState {
        OathState {
            password_required: false,
            credentials: vec![OathCredential {
                name: "test".to_string(),
                issuer: None,
                oath_type: OathType::Totp,
                algorithm: OathAlgorithm::Sha1,
                digits: 6,
                period: 30,
                touch_required: false,
                code: None,
            }],
        }
    }

    fn occupied_piv() -> PivState {
        PivState {
            slots: vec![SlotInfo::occupied("9a")],
        }
    }

    #[test]
    fn test_factory_default_all_empty() {
        let mut yk = base_yk();
        yk.fido2 = Some(empty_fido2());
        yk.oath = Some(empty_oath());
        yk.piv = Some(empty_piv());
        assert!(is_factory_default(&yk));
    }

    #[test]
    fn test_not_factory_default_pin_set() {
        let mut yk = base_yk();
        let mut fido2 = empty_fido2();
        fido2.pin_is_set = true;
        yk.fido2 = Some(fido2);
        yk.oath = Some(empty_oath());
        yk.piv = Some(empty_piv());
        assert!(!is_factory_default(&yk));
    }

    #[test]
    fn test_not_factory_default_has_creds() {
        let mut yk = base_yk();
        yk.fido2 = Some(empty_fido2());
        yk.oath = Some(occupied_oath());
        yk.piv = Some(empty_piv());
        assert!(!is_factory_default(&yk));
    }

    #[test]
    fn test_not_factory_default_piv_occupied() {
        let mut yk = base_yk();
        yk.fido2 = Some(empty_fido2());
        yk.oath = Some(empty_oath());
        yk.piv = Some(occupied_piv());
        assert!(!is_factory_default(&yk));
    }

    #[test]
    fn test_none_fields_not_default() {
        // fido2=None → conservative → not factory default
        let yk = base_yk();
        assert!(!is_factory_default(&yk));
    }
}
