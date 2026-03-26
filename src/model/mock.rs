use super::*;

/// Returns mock YubiKey state for `--mock` mode.
/// Represents a fully configured YubiKey 5 NFC — all OpenPGP slots occupied,
/// SSH configured, PINs at default retries. Gives E2E tests real content
/// to exercise on every screen.
pub fn mock_yubikey_states() -> Vec<YubiKeyState> {
    vec![YubiKeyState {
        info: YubiKeyInfo {
            serial: 12345678,
            version: Version {
                major: 5,
                minor: 4,
                patch: 3,
            },
            model: Model::YubiKey5NFC,
            form_factor: FormFactor::UsbA,
        },
        openpgp: Some(openpgp::OpenPgpState {
            card_present: true,
            version: "3.4".to_string(),
            signature_key: Some(openpgp::KeyInfo {
                fingerprint: "A1B2C3D4E5F6A7B8C9D0E1F2A3B4C5D6E7F8A9B0".to_string(),
                created: Some("2024-01-15".to_string()),
                key_attributes: "EdDSA (Ed25519)".to_string(),
            }),
            encryption_key: Some(openpgp::KeyInfo {
                fingerprint: "B2C3D4E5F6A7B8C9D0E1F2A3B4C5D6E7F8A9B0A1".to_string(),
                created: Some("2024-01-15".to_string()),
                key_attributes: "ECDH (Cv25519)".to_string(),
            }),
            authentication_key: Some(openpgp::KeyInfo {
                fingerprint: "C3D4E5F6A7B8C9D0E1F2A3B4C5D6E7F8A9B0A1B2".to_string(),
                created: Some("2024-01-15".to_string()),
                key_attributes: "EdDSA (Ed25519)".to_string(),
            }),
            cardholder_name: Some("Mock User".to_string()),
            public_key_url: None,
        }),
        piv: Some(piv::PivState {
            slots: vec![
                piv::SlotInfo {
                    slot: "9a".to_string(),
                    algorithm: Some("ECCP256".to_string()),
                    subject: Some("Mock PIV Auth".to_string()),
                },
            ],
        }),
        pin_status: pin::PinStatus {
            user_pin_retries: 3,
            admin_pin_retries: 3,
            reset_code_retries: 0,
            user_pin_blocked: false,
            admin_pin_blocked: false,
        },
        touch_policies: Some(touch_policy::TouchPolicies {
            signature: touch_policy::TouchPolicy::On,
            encryption: touch_policy::TouchPolicy::Off,
            authentication: touch_policy::TouchPolicy::On,
            attestation: touch_policy::TouchPolicy::Off,
        }),
    }]
}
