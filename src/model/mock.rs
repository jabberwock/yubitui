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
        oath: Some(oath::OathState {
            credentials: vec![
                oath::OathCredential {
                    name: "GitHub:mockuser@github.com".to_string(),
                    issuer: Some("GitHub".to_string()),
                    oath_type: oath::OathType::Totp,
                    algorithm: oath::OathAlgorithm::Sha1,
                    digits: 6,
                    period: 30,
                    code: Some("123456".to_string()),
                    touch_required: false,
                },
                oath::OathCredential {
                    name: "Google:mock@gmail.com".to_string(),
                    issuer: Some("Google".to_string()),
                    oath_type: oath::OathType::Totp,
                    algorithm: oath::OathAlgorithm::Sha256,
                    digits: 6,
                    period: 30,
                    code: Some("789012".to_string()),
                    touch_required: false,
                },
                oath::OathCredential {
                    name: "AWS:mock-iam-user".to_string(),
                    issuer: Some("AWS".to_string()),
                    oath_type: oath::OathType::Hotp,
                    algorithm: oath::OathAlgorithm::Sha1,
                    digits: 6,
                    period: 0,
                    code: None,
                    touch_required: false,
                },
            ],
            password_required: false,
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
        fido2: Some(fido2::Fido2State {
            firmware_version: Some("5.4.3".to_string()),
            algorithms: vec!["ES256".to_string(), "EdDSA".to_string()],
            pin_is_set: true,
            pin_retry_count: 8,
            credentials: Some(vec![
                fido2::Fido2Credential {
                    rp_id: "github.com".to_string(),
                    rp_name: Some("GitHub".to_string()),
                    user_name: "user@example.com".to_string(),
                    credential_id: vec![0x01, 0x02, 0x03, 0x04],
                },
                fido2::Fido2Credential {
                    rp_id: "google.com".to_string(),
                    rp_name: Some("Google".to_string()),
                    user_name: "user@gmail.com".to_string(),
                    credential_id: vec![0x05, 0x06, 0x07, 0x08],
                },
            ]),
            supports_cred_mgmt: true,
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
