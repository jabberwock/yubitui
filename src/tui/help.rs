use textual_rs::{Widget, Footer, Header, Markdown};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

const HELP_MARKDOWN: &str = "\
# yubitui -- YubiKey Management TUI

## Global Keybindings

- `1-9` Switch screen (Keys / Diagnostics / PIN / SSH / PIV / Help / OATH / FIDO2 / OTP)
- `r` Refresh YubiKey status and diagnostics
- `?` Toggle help / glossary
- `q` / `Esc` Quit (from Dashboard) or go back
- `m` / `Enter` Open navigation menu (Dashboard)

## Key Management (Screen 1)

- `v` View full card status
- `i` Import existing key to card
- `g` Generate new key on card
- `e` Export SSH public key
- `t` Set touch policy for a slot
- `Up` / `Down` Navigate slots
- `Enter` Execute selected operation

## PIN Management (Screen 3)

- `c` Change user PIN
- `a` Change admin PIN
- `r` Set reset code
- `u` Unblock user PIN with reset code

## SSH Setup (Screen 4)

- `1-5` Select wizard step
- `r` Refresh SSH status
- `Enter` Execute selected step

## OATH / Authenticator (Screen 7)

- `a` Add new TOTP / HOTP account
- `Del` Delete selected account
- `Enter` Show current OTP code

## FIDO2 / Security Key (Screen 8)

- `s` Set FIDO2 PIN
- `r` Reset FIDO2 applet (destructive)

## OTP Slots (Screen 9)

- `1` / `2` Select OTP slot
- `p` Program selected slot
- `d` Delete / reset selected slot
";

/// Help screen — displays all keybindings grouped by context.
///
/// This is the first screen migrated to the textual-rs Widget pattern (D-01).
/// No sidebar — full-width content area with Header and Footer (D-07/D-15).
pub struct HelpScreen;

impl HelpScreen {
    pub fn new() -> Self {
        HelpScreen
    }
}

impl Widget for HelpScreen {
    fn widget_type_name(&self) -> &'static str {
        "HelpScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        vec![
            Box::new(Header::new("yubitui -- YubiKey Management TUI")),
            Box::new(Markdown::new(HELP_MARKDOWN)),
            Box::new(Footer),
        ]
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        &[
            KeyBinding {
                key: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                action: "back",
                description: "Back",
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
                description: "Close Help",
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
    async fn help_screen() {
        // Use the same CSS as app.rs so Markdown fills remaining vertical space.
        let css = "HelpScreen Markdown { flex-grow: 1; }";
        let mut app = TestApp::new_styled(80, 24, css, || Box::new(HelpScreen::new()));
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

}
