use textual_rs::{Widget, Footer, Header, Label};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;


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
            Box::new(Label::new(" Global Keybindings")),
            Box::new(Label::new("  1-9          Switch screen (Keys / Diagnostics / PIN / SSH / PIV / Help / OATH / FIDO2 / OTP)")),
            Box::new(Label::new("  r            Refresh YubiKey status and diagnostics")),
            Box::new(Label::new("  ?            Toggle this help screen")),
            Box::new(Label::new("  q / Esc      Quit (from Dashboard) or go back")),
            Box::new(Label::new("  m / Enter    Open navigation menu (Dashboard)")),
            Box::new(Label::new("")),
            Box::new(Label::new(" Key Management (Screen 1)")),
            Box::new(Label::new("  v            View full card status")),
            Box::new(Label::new("  i            Import existing key to card")),
            Box::new(Label::new("  g            Generate new key on card")),
            Box::new(Label::new("  e            Export SSH public key")),
            Box::new(Label::new("  Up/Down      Select key (in import view)")),
            Box::new(Label::new("  Enter        Execute selected operation")),
            Box::new(Label::new("")),
            Box::new(Label::new(" PIN Management (Screen 3)")),
            Box::new(Label::new("  c            Change user PIN")),
            Box::new(Label::new("  a            Change admin PIN")),
            Box::new(Label::new("  r            Set reset code")),
            Box::new(Label::new("  u            Unblock user PIN")),
            Box::new(Label::new("  Enter        Execute selected operation")),
            Box::new(Label::new("")),
            Box::new(Label::new(" SSH Wizard (Screen 4)")),
            Box::new(Label::new("  1-5          Select wizard step")),
            Box::new(Label::new("  r            Refresh SSH status")),
            Box::new(Label::new("  Enter        Execute selected step")),
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
    async fn help_screen() {
        let mut app = TestApp::new_styled(80, 24, "", || Box::new(HelpScreen::new()));
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }
}
