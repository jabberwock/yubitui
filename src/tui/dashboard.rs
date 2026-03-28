use textual_rs::{Widget, Header, Label, Footer};
use textual_rs::widget::button::{Button, messages as btn_messages};
use textual_rs::widget::context::AppContext;
use textual_rs::widget::EventPropagation;
use textual_rs::event::keybinding::KeyBinding;
use crossterm::event::{KeyCode, KeyModifiers, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::AppState;
use crate::diagnostics::Diagnostics;

#[derive(Default, Clone, PartialEq)]
pub struct DashboardState {
    pub show_context_menu: bool,
    pub menu_selected_index: usize,
}

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
/// Sidebar-style status block at the top, then navigation buttons for all 6 screens.
///
/// Follows textual-rs Widget pattern (D-01, D-06, D-07):
/// - Header("yubitui -- YubiKey Management")
/// - Device status Labels
/// - 6 navigation Buttons (all navigable elements per D-06)
/// - Footer with keybindings always visible (D-07, D-15)
/// - No hardcoded Color:: values
///
/// Ctrl+T theme cycling is handled globally by the textual-rs App runner.
/// 'q' / Esc quit is handled globally by the textual-rs App runner.
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
];

impl Widget for DashboardScreen {
    fn widget_type_name(&self) -> &'static str {
        "DashboardScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut children: Vec<Box<dyn Widget>> = Vec::new();

        children.push(Box::new(Header::new("yubitui -- YubiKey Management")));

        // Device status block (sidebar role — top status display)
        if let Some(yk) = self.app_state.yubikey_state() {
            let pin = &yk.pin_status;

            // Multi-key indicator
            if self.app_state.yubikey_count() > 1 {
                children.push(Box::new(Label::new(format!(
                    "Key {}/{} (Tab to switch)",
                    self.app_state.selected_yubikey_idx + 1,
                    self.app_state.yubikey_count()
                ))));
            }

            children.push(Box::new(Label::new(format!(
                "Device: {} {} | Firmware: {} | Serial: {}",
                yk.info.model, yk.info.form_factor, yk.info.version, yk.info.serial
            ))));

            let pin_user_status = if pin.user_pin_blocked {
                "BLOCKED"
            } else if pin.user_pin_retries <= 1 {
                "LOW"
            } else {
                "OK"
            };
            let pin_admin_status = if pin.admin_pin_blocked {
                "BLOCKED"
            } else if pin.admin_pin_retries <= 1 {
                "LOW"
            } else {
                "OK"
            };
            children.push(Box::new(Label::new(format!(
                "PIN: {}/3 retries [{}]  Admin: {}/3 retries [{}]",
                pin.user_pin_retries,
                pin_user_status,
                pin.admin_pin_retries,
                pin_admin_status
            ))));

            if let Some(ref openpgp) = yk.openpgp {
                let sig_status = if openpgp.signature_key.is_some() {
                    "Set"
                } else {
                    "Empty"
                };
                let enc_status = if openpgp.encryption_key.is_some() {
                    "Set"
                } else {
                    "Empty"
                };
                let aut_status = if openpgp.authentication_key.is_some() {
                    "Set"
                } else {
                    "Empty"
                };
                children.push(Box::new(Label::new(format!(
                    "Keys: Sign={} Encrypt={} Auth={}",
                    sig_status, enc_status, aut_status
                ))));
            }

            children.push(Box::new(Label::new("Device ready")));
        } else {
            children.push(Box::new(Label::new("No YubiKey Detected")));
            children.push(Box::new(Label::new(
                "Insert your YubiKey and press R to refresh. Check the USB connection or run Diagnostics.",
            )));
        }

        children.push(Box::new(Label::new("")));

        // Navigation buttons (D-06: all navigable elements are Buttons)
        children.push(Box::new(Button::new("[1] Open Keys")));
        children.push(Box::new(Button::new("[2] Diagnostics")));
        children.push(Box::new(Button::new("[3] PIN Management")));
        children.push(Box::new(Button::new("[4] SSH Setup")));
        children.push(Box::new(Button::new("[5] PIV Certificates")));
        children.push(Box::new(Button::new("[6] Help")));
        children.push(Box::new(Button::new("[7] OATH / Authenticator")));
        children.push(Box::new(Button::new("[8] FIDO2 / Security Key")));
        children.push(Box::new(Button::new("[9] OTP Slots")));

        children.push(Box::new(Footer));
        children
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        DASHBOARD_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(pressed) = event.downcast_ref::<btn_messages::Pressed>() {
            let action = match pressed.label.as_str() {
                "[1] Open Keys"        => "nav_1",
                "[2] Diagnostics"      => "nav_2",
                "[3] PIN Management"   => "nav_3",
                "[4] SSH Setup"        => "nav_4",
                "[5] PIV Certificates"    => "nav_5",
                "[6] Help"                => "nav_6",
                "[7] OATH / Authenticator" => "nav_7",
                "[8] FIDO2 / Security Key" => "nav_8",
                "[9] OTP Slots"           => "nav_9",
                _ => return EventPropagation::Continue,
            };
            self.on_action(action, ctx);
            return EventPropagation::Stop;
        }
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
            "nav_7" => {
                let oath_state = self.app_state.yubikey_state()
                    .and_then(|yk| yk.oath.clone());
                ctx.push_screen_deferred(Box::new(crate::tui::oath::OathScreen::new(oath_state)));
            }
            "nav_8" => {
                let fido2_state = self.app_state.yubikey_state()
                    .and_then(|yk| yk.fido2.clone());
                ctx.push_screen_deferred(Box::new(crate::tui::fido2::Fido2Screen::new(fido2_state)));
            }
            "nav_9" => {
                let otp_state = self.app_state.yubikey_state()
                    .and_then(|yk| yk.otp.clone());
                ctx.push_screen_deferred(Box::new(crate::tui::otp::OtpScreen::new(otp_state)));
            }
            "refresh" => {
                // Refresh is an app-level side effect — no-op in widget scope.
                // In 08-06, the root screen will re-build DashboardScreen with fresh state.
            }
            "switch_key" => {
                // Multi-key switching is an app-level side effect — no-op in widget scope.
            }
            "open_menu" => {
                // Push Help as context menu placeholder — full context menu in 08-06.
                ctx.push_screen_deferred(Box::new(crate::tui::help::HelpScreen::new()));
            }
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
        let mut app = TestApp::new(80, 24, move || {
            Box::new(DashboardScreen::new(app_state.clone(), diagnostics.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn dashboard_no_yubikey() {
        let mut app = TestApp::new(80, 24, || {
            Box::new(DashboardScreen::new(AppState::default(), Diagnostics::default()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn dashboard_context_menu_open() {
        let app_state = make_app_state_with_key();
        let diagnostics = Diagnostics::default();
        let mut app = TestApp::new(80, 24, move || {
            Box::new(DashboardScreen::new(app_state.clone(), diagnostics.clone()))
        });
        let mut pilot = app.pilot();
        pilot.press(KeyCode::Char('m')).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_display_snapshot!(app.backend());
    }
}
