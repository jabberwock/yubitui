use textual_rs::{Widget, Footer, Header, Markdown};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

const GLOSSARY_MARKDOWN: &str = "\
# Protocol Glossary

## PIV (Personal Identity Verification)
Smart card standard for X.509 certificates and key storage. Used for
code signing, VPN auth, and Windows smart card login.

## FIDO2 / WebAuthn
Hardware passkey standard. Phishing-resistant -- the key verifies the
website's identity before responding. Replaces passwords entirely.

## FIDO U2F (Universal 2nd Factor)
Original FIDO standard. Adds a hardware second factor to password login.
Predecessor to FIDO2; still widely supported.

## OpenPGP / PGP
Encryption and signing standard. Your YubiKey stores private keys for
email encryption, git commit signing, and SSH authentication.

## SSH (Secure Shell)
Remote server access. YubiKey can hold your SSH private key via the
OpenPGP authentication subkey or PIV certificate.

## TOTP (Time-Based One-Time Password)
6-digit codes that change every 30 seconds. The standard behind Google
Authenticator, Authy, etc. YubiKey stores secrets on hardware.

## HOTP (HMAC-Based One-Time Password)
Counter-based codes. Each press generates the next code in sequence.
Less common than TOTP; used by some banking systems.

## OTP / Yubico OTP
Yubico's proprietary one-time password. The key types a 44-character
string validated by Yubico's cloud service (or self-hosted).
";

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
            Box::new(Markdown::new(GLOSSARY_MARKDOWN)),
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
        if action == "back" {
            ctx.pop_screen_deferred();
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
