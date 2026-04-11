use textual_rs::{Widget, Header, Label, Footer, Button, ButtonVariant, Horizontal, Vertical};
use textual_rs::widget::context::AppContext;
use textual_rs::widget::EventPropagation;
use textual_rs::event::keybinding::KeyBinding;
use crossterm::event::{KeyCode, KeyModifiers, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::AppState;
use crate::diagnostics::Diagnostics;


#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum DashboardAction {
    None,
    Quit,
    NavigateTo(crate::model::Screen),
    OpenContextMenu,
    SwitchYubiKey,
    Refresh,
    SelectMenuItem(usize),
    CloseContextMenu,
    MenuUp,
    MenuDown,
    CycleTheme,
}

/// Dashboard — root screen.
///
/// Device status card at the top, 3x3 navigation grid below.
/// Uses Horizontal/Vertical layout containers and CSS classes for visual structure.
pub struct DashboardScreen {
    app_state: AppState,
    diagnostics: Diagnostics,
}

impl DashboardScreen {
    pub fn new(app_state: AppState, diagnostics: Diagnostics) -> Self {
        Self {
            app_state,
            diagnostics,
        }
    }
}

static DASHBOARD_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Char('1'),
        modifiers: KeyModifiers::NONE,
        action: "nav_1",
        description: "1-9 Navigate",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('2'),
        modifiers: KeyModifiers::NONE,
        action: "nav_2",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('3'),
        modifiers: KeyModifiers::NONE,
        action: "nav_3",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('4'),
        modifiers: KeyModifiers::NONE,
        action: "nav_4",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('5'),
        modifiers: KeyModifiers::NONE,
        action: "nav_5",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('6'),
        modifiers: KeyModifiers::NONE,
        action: "nav_6",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('7'),
        modifiers: KeyModifiers::NONE,
        action: "nav_7",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('8'),
        modifiers: KeyModifiers::NONE,
        action: "nav_8",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('9'),
        modifiers: KeyModifiers::NONE,
        action: "nav_9",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('?'),
        modifiers: KeyModifiers::NONE,
        action: "glossary",
        description: "? Glossary",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('r'),
        modifiers: KeyModifiers::NONE,
        action: "refresh",
        description: "R Refresh",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Tab,
        modifiers: KeyModifiers::NONE,
        action: "switch_key",
        description: "Tab Switch key",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('m'),
        modifiers: KeyModifiers::NONE,
        action: "open_menu",
        description: "M Menu",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "open_menu",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('w'),
        modifiers: KeyModifiers::NONE,
        action: "wizards",
        description: "W Wizards",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        action: "quit",
        description: "Q Quit",
        show: true,
    },
];

impl Widget for DashboardScreen {
    fn widget_type_name(&self) -> &'static str {
        "DashboardScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut children: Vec<Box<dyn Widget>> = Vec::new();

        children.push(Box::new(Header::new("yubitui")));

        // ── Device status card ──────────────────────────────────────────
        if let Some(yk) = self.app_state.yubikey_state() {
            let pin = &yk.pin_status;

            let mut status_lines: Vec<Box<dyn Widget>> = Vec::new();

            // Multi-key indicator
            if self.app_state.yubikey_count() > 1 {
                status_lines.push(Box::new(Label::new(format!(
                    "Key {}/{} (Tab to switch)",
                    self.app_state.selected_yubikey_idx + 1,
                    self.app_state.yubikey_count()
                ))));
            }

            status_lines.push(Box::new(Label::new(format!(
                "{} {} | FW {} | SN {}",
                yk.info.model, yk.info.form_factor, yk.info.version, yk.info.serial
            ))));

            // PIN status row
            let pin_user = if pin.user_pin_blocked {
                "BLOCKED"
            } else if pin.user_pin_retries <= 1 {
                "LOW"
            } else {
                "OK"
            };
            let pin_admin = if pin.admin_pin_blocked {
                "BLOCKED"
            } else if pin.admin_pin_retries <= 1 {
                "LOW"
            } else {
                "OK"
            };
            status_lines.push(Box::new(Label::new(format!(
                "PIN {}/3 [{}]  Admin {}/3 [{}]",
                pin.user_pin_retries, pin_user,
                pin.admin_pin_retries, pin_admin
            ))));

            // Key slot status
            if let Some(ref openpgp) = yk.openpgp {
                let sig = if openpgp.signature_key.is_some() { "SET" } else { "---" };
                let enc = if openpgp.encryption_key.is_some() { "SET" } else { "---" };
                let aut = if openpgp.authentication_key.is_some() { "SET" } else { "---" };
                status_lines.push(Box::new(Label::new(format!(
                    "Keys: Sign [{}]  Encrypt [{}]  Auth [{}]",
                    sig, enc, aut
                ))));
            }

            children.push(Box::new(
                Vertical::with_children(status_lines).with_class("status-card")
            ));
        } else {
            // No YubiKey detected
            let no_key: Vec<Box<dyn Widget>> = vec![
                Box::new(Label::new("No YubiKey detected")),
                Box::new(Label::new("Insert your YubiKey and press R to refresh")),
            ];
            children.push(Box::new(
                Vertical::with_children(no_key).with_class("status-card-error")
            ));
        }

        // ── Navigation grid (3 rows of 3 buttons) ─────────────────────
        children.push(Box::new(Label::new(""))); // spacer

        // Row 1: Core features
        children.push(Box::new(Horizontal::with_children(vec![
            Box::new(Button::new("[1] Keys").with_variant(ButtonVariant::Primary).with_action("nav_1")),
            Box::new(Button::new("[2] Diagnostics").with_action("nav_2")),
            Box::new(Button::new("[3] PIN").with_action("nav_3")),
        ]).with_class("nav-row")));

        // Row 2: Setup & certificates
        children.push(Box::new(Horizontal::with_children(vec![
            Box::new(Button::new("[4] SSH Setup").with_action("nav_4")),
            Box::new(Button::new("[5] PIV Certs").with_action("nav_5")),
            Box::new(Button::new("[6] Help").with_action("nav_6")),
        ]).with_class("nav-row")));

        // Row 3: Protocols
        children.push(Box::new(Horizontal::with_children(vec![
            Box::new(Button::new("[7] OATH").with_action("nav_7")),
            Box::new(Button::new("[8] FIDO2").with_action("nav_8")),
            Box::new(Button::new("[9] OTP").with_action("nav_9")),
        ]).with_class("nav-row")));

        // Wizards button (standalone, full-width)
        children.push(Box::new(Button::new("[W] Setup Wizards").with_variant(ButtonVariant::Success).with_action("wizards")));

        children.push(Box::new(Footer));
        children
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        DASHBOARD_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            for binding in self.key_bindings() {
                if binding.matches(key.code, key.modifiers) {
                    self.on_action(binding.action, ctx);
                    return EventPropagation::Stop;
                }
            }
        }
        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "nav_1" => {
                let yk = self.app_state.yubikey_state().cloned();
                ctx.push_screen_deferred(Box::new(crate::tui::keys::KeysScreen::new(yk)));
            }
            "nav_2" => {
                ctx.push_screen_deferred(Box::new(
                    crate::tui::diagnostics::DiagnosticsScreen::new(self.diagnostics.clone()),
                ));
            }
            "nav_3" => {
                let yk = self.app_state.yubikey_state().cloned();
                ctx.push_screen_deferred(Box::new(crate::tui::pin::PinManagementScreen::new(yk)));
            }
            "nav_4" => {
                ctx.push_screen_deferred(Box::new(
                    crate::tui::ssh::SshWizardScreen::new(crate::tui::ssh::SshState::default()),
                ));
            }
            "nav_5" => {
                let yk = self.app_state.yubikey_state().cloned();
                ctx.push_screen_deferred(Box::new(crate::tui::piv::PivScreen::new(yk)));
            }
            "nav_6" => {
                ctx.push_screen_deferred(Box::new(crate::tui::help::HelpScreen::new()));
            }
            "glossary" => {
                ctx.push_screen_deferred(Box::new(crate::tui::glossary::GlossaryScreen::new()));
            }
            "nav_7" => {
                let key_present = self.app_state.yubikey_state().is_some();
                let oath_state = crate::model::oath::get_oath_state().ok();
                let screen = if key_present {
                    crate::tui::oath::OathScreen::new_with_key(oath_state)
                } else {
                    crate::tui::oath::OathScreen::new(oath_state)
                };
                ctx.push_screen_deferred(Box::new(screen));
            }
            "nav_8" => {
                let key_present = self.app_state.yubikey_state().is_some();
                let fido2_result = crate::model::fido2::get_fido2_info();
                if let Err(ref e) = fido2_result {
                    tracing::warn!("FIDO2 fetch failed: {}", e);
                }
                let screen = if key_present {
                    crate::tui::fido2::Fido2Screen::new_with_key(fido2_result.ok())
                } else {
                    crate::tui::fido2::Fido2Screen::new(fido2_result.ok())
                };
                ctx.push_screen_deferred(Box::new(screen));
            }
            "nav_9" => {
                let key_present = self.app_state.yubikey_state().is_some();
                let otp_state = self.app_state.yubikey_state()
                    .and_then(|yk| yk.otp.clone());
                let screen = if key_present {
                    crate::tui::otp::OtpScreen::new_with_key(otp_state)
                } else {
                    crate::tui::otp::OtpScreen::new(otp_state)
                };
                ctx.push_screen_deferred(Box::new(screen));
            }
            "refresh" => {
                let fresh_states = crate::model::YubiKeyState::detect_all().unwrap_or_default();
                let mut fresh_app_state = self.app_state.clone();
                fresh_app_state.yubikey_states = fresh_states;
                ctx.pop_screen_deferred();
                ctx.push_screen_deferred(Box::new(DashboardScreen::new(
                    fresh_app_state,
                    self.diagnostics.clone(),
                )));
            }
            "switch_key" => {
                // Multi-key switching is an app-level side effect.
            }
            "open_menu" => {
                ctx.push_screen_deferred(Box::new(crate::tui::help::HelpScreen::new()));
            }
            "wizards" => {
                let yk = self.app_state.yubikey_state().cloned();
                ctx.push_screen_deferred(Box::new(
                    crate::tui::wizard::WizardMenuScreen::new(yk),
                ));
            }
            "quit" => ctx.quit(),
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {
        // Layout and child rendering handled by compose() children.
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use textual_rs::TestApp;
    use crossterm::event::KeyCode;

    fn make_app_state_with_key() -> AppState {
        AppState {
            yubikey_states: crate::model::mock::mock_yubikey_states(),
            mock_mode: true,
            ..AppState::default()
        }
    }

    #[tokio::test]
    async fn dashboard_default_populated() {
        let app_state = make_app_state_with_key();
        let diagnostics = Diagnostics::default();
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(DashboardScreen::new(app_state.clone(), diagnostics.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn dashboard_no_yubikey() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || {
            Box::new(DashboardScreen::new(AppState::default(), Diagnostics::default()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn dashboard_context_menu_open() {
        let app_state = make_app_state_with_key();
        let diagnostics = Diagnostics::default();
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(DashboardScreen::new(app_state.clone(), diagnostics.clone()))
        });
        let mut pilot = app.pilot();
        pilot.press(KeyCode::Char('m')).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_snapshot!(app.backend());
    }
}
