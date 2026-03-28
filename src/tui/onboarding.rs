use textual_rs::{Widget, Header, Label, Footer};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::YubiKeyState;

/// Onboarding checklist screen — shown on first launch when a factory-default YubiKey
/// is detected.
///
/// Shows a 4-item [x]/[ ] checklist of configurable features based on current state.
/// Informational only — no interactive wizards here.
///
/// When used as the app root (startup mode), dismiss pushes DashboardScreen so it sits
/// on top. When pushed from elsewhere, dismiss pops normally.
pub struct OnboardingScreen {
    pub yk: YubiKeyState,
    /// Startup mode: Some((app_state, diagnostics)) → dismiss pushes DashboardScreen.
    startup: Option<(crate::model::AppState, crate::diagnostics::Diagnostics)>,
}

impl OnboardingScreen {
    /// Standard constructor — dismiss pops this screen.
    pub fn new(yk: YubiKeyState) -> Self {
        Self { yk, startup: None }
    }

    /// Startup constructor — dismiss pushes DashboardScreen (used when this IS the root).
    pub fn new_startup(
        yk: YubiKeyState,
        app_state: crate::model::AppState,
        diagnostics: crate::diagnostics::Diagnostics,
    ) -> Self {
        Self { yk, startup: Some((app_state, diagnostics)) }
    }
}

impl Widget for OnboardingScreen {
    fn widget_type_name(&self) -> &'static str {
        "OnboardingScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("Welcome to yubitui")),
            Box::new(Label::new("")),
            Box::new(Label::new("Your YubiKey appears to be in factory-default state.")),
            Box::new(Label::new("Here's what you can configure:")),
            Box::new(Label::new("")),
        ];

        // FIDO2 PIN
        let fido2_set = self.yk.fido2.as_ref().map(|f| f.pin_is_set).unwrap_or(false);
        if fido2_set {
            widgets.push(Box::new(Label::new("  [x] FIDO2 PIN is set")));
        } else {
            widgets.push(Box::new(Label::new("  [ ] Set a FIDO2 PIN — required for passkey (WebAuthn) login")));
        }

        // OATH
        let has_oath = self.yk.oath.as_ref().map(|o| !o.credentials.is_empty()).unwrap_or(false);
        if has_oath {
            widgets.push(Box::new(Label::new("  [x] OATH accounts configured")));
        } else {
            widgets.push(Box::new(Label::new("  [ ] Add OATH accounts — store TOTP/HOTP codes on hardware")));
        }

        // PIV
        let has_piv = self.yk.piv.as_ref().map(|p| !p.slots.is_empty()).unwrap_or(false);
        if has_piv {
            widgets.push(Box::new(Label::new("  [x] PIV certificates present")));
        } else {
            widgets.push(Box::new(Label::new("  [ ] Configure PIV — smart card certificates for login/VPN")));
        }

        // OpenPGP
        let has_openpgp = self.yk.openpgp.as_ref().map(|o| {
            o.signature_key.is_some() || o.encryption_key.is_some() || o.authentication_key.is_some()
        }).unwrap_or(false);
        if has_openpgp {
            widgets.push(Box::new(Label::new("  [x] OpenPGP keys configured")));
        } else {
            widgets.push(Box::new(Label::new("  [ ] Set up OpenPGP keys — git signing, email encryption, SSH")));
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Label::new("Use the numbered keys from the dashboard to access each feature.")));
        widgets.push(Box::new(Label::new("Press ? on any screen for a protocol explanation.")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        &[
            KeyBinding {
                key: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                action: "dismiss",
                description: "Esc Continue to Dashboard",
                show: true,
            },
            KeyBinding {
                key: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                action: "dismiss",
                description: "",
                show: false,
            },
        ]
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        if action == "dismiss" {
            if let Some((app_state, diagnostics)) = &self.startup {
                ctx.push_screen_deferred(Box::new(
                    crate::tui::dashboard::DashboardScreen::new(
                        app_state.clone(),
                        diagnostics.clone(),
                    ),
                ));
            } else {
                ctx.pop_screen_deferred();
            }
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use textual_rs::TestApp;
    use crate::model::{YubiKeyInfo, Version, Model, FormFactor};

    fn factory_default_yk() -> YubiKeyState {
        YubiKeyState {
            info: YubiKeyInfo {
                serial: 1234,
                version: Version { major: 5, minor: 4, patch: 3 },
                model: Model::YubiKey5NFC,
                form_factor: FormFactor::UsbA,
            },
            openpgp: None,
            oath: Some(crate::model::oath::OathState {
                credentials: vec![],
                password_required: false,
            }),
            piv: Some(crate::model::piv::PivState { slots: vec![] }),
            fido2: Some(crate::model::fido2::Fido2State {
                firmware_version: None,
                algorithms: vec![],
                pin_is_set: false,
                pin_retry_count: 8,
                supports_cred_mgmt: false,
                credentials: Some(vec![]),
            }),
            otp: None,
            pin_status: crate::model::pin::PinStatus {
                user_pin_retries: 3,
                admin_pin_retries: 3,
                reset_code_retries: 3,
                user_pin_blocked: false,
                admin_pin_blocked: false,
            },
            touch_policies: None,
        }
    }

    #[tokio::test]
    async fn onboarding_factory_default() {
        let yk = factory_default_yk();
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(OnboardingScreen::new(yk.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }
}
