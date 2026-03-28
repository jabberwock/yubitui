use textual_rs::{Widget, Footer, Header, Label};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// Protocol Glossary screen — explains all 8 YubiKey protocols in plain language.
///
/// Accessible from the Dashboard via the ? keybinding. Provides new users with
/// a reference for PIV, FIDO2, FIDO U2F, OpenPGP, SSH, TOTP, HOTP, and OTP.
pub struct GlossaryScreen;

impl GlossaryScreen {
    pub fn new() -> Self {
        GlossaryScreen
    }
}

impl Widget for GlossaryScreen {
    fn widget_type_name(&self) -> &'static str {
        "GlossaryScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        vec![
            Box::new(Header::new("Protocol Glossary")),
            Box::new(Label::new("")),
            Box::new(Label::new(" PIV (Personal Identity Verification)")),
            Box::new(Label::new("   Smart card standard for X.509 certificates and key storage. Used for")),
            Box::new(Label::new("   code signing, VPN auth, and Windows smart card login.")),
            Box::new(Label::new("")),
            Box::new(Label::new(" FIDO2 / WebAuthn")),
            Box::new(Label::new("   Hardware passkey standard. Phishing-resistant — the key verifies the")),
            Box::new(Label::new("   website's identity before responding. Replaces passwords entirely.")),
            Box::new(Label::new("")),
            Box::new(Label::new(" FIDO U2F (Universal 2nd Factor)")),
            Box::new(Label::new("   Original FIDO standard. Adds a hardware second factor to password login.")),
            Box::new(Label::new("   Predecessor to FIDO2; still widely supported.")),
            Box::new(Label::new("")),
            Box::new(Label::new(" OpenPGP / PGP")),
            Box::new(Label::new("   Encryption and signing standard. Your YubiKey stores private keys for")),
            Box::new(Label::new("   email encryption, git commit signing, and SSH authentication.")),
            Box::new(Label::new("")),
            Box::new(Label::new(" SSH (Secure Shell)")),
            Box::new(Label::new("   Remote server access. YubiKey can hold your SSH private key via the")),
            Box::new(Label::new("   OpenPGP authentication subkey or PIV certificate.")),
            Box::new(Label::new("")),
            Box::new(Label::new(" TOTP (Time-Based One-Time Password)")),
            Box::new(Label::new("   6-digit codes that change every 30 seconds. The standard behind Google")),
            Box::new(Label::new("   Authenticator, Authy, etc. YubiKey stores secrets on hardware.")),
            Box::new(Label::new("")),
            Box::new(Label::new(" HOTP (HMAC-Based One-Time Password)")),
            Box::new(Label::new("   Counter-based codes. Each press generates the next code in sequence.")),
            Box::new(Label::new("   Less common than TOTP; used by some banking systems.")),
            Box::new(Label::new("")),
            Box::new(Label::new(" OTP / Yubico OTP")),
            Box::new(Label::new("   Yubico's proprietary one-time password. The key types a 44-character")),
            Box::new(Label::new("   string validated by Yubico's cloud service (or self-hosted).")),
            Box::new(Footer),
        ]
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        &[
            KeyBinding {
                key: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                action: "back",
                description: "Esc Back",
                show: true,
            },
            KeyBinding {
                key: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                action: "back",
                description: "",
                show: false,
            },
            KeyBinding {
                key: KeyCode::Char('?'),
                modifiers: KeyModifiers::NONE,
                action: "back",
                description: "Close Glossary",
                show: true,
            },
        ]
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "back" => ctx.pop_screen_deferred(),
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textual_rs::TestApp;

    #[tokio::test]
    async fn glossary_screen() {
        let mut app = TestApp::new_styled(80, 24, "", || Box::new(GlossaryScreen::new()));
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }
}
