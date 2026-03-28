use std::cell::{Cell, RefCell};

use textual_rs::{Widget, Label, Button, ButtonVariant, Footer};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::widget::screen::ModalScreen;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::YubiKeyState;
use crate::tui::widgets::pin_input::PinInputWidget;
use crate::tui::widgets::popup::{ConfirmScreen, PopupScreen};

const PIN_HELP_TEXT: &str = "\
PIN Management\n\
\n\
Your YubiKey's OpenPGP applet uses two PINs:\n\
- User PIN (default: 123456) — required for signing and decryption\n\
- Admin PIN (default: 12345678) — required for key management operations\n\
\n\
After 3 wrong User PIN attempts, the PIN is blocked. Use the Admin PIN\n\
to unblock it, or set a Reset Code as a backup unblock method.\n\
\n\
Change both PINs from defaults immediately after setting up your key.";


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PinScreen {
    Main,
    ChangeUserPin,
    ChangeAdminPin,
    SetResetCode,
    #[allow(dead_code)]
    UnblockUserPin,
    // Wizard screens:
    UnblockWizardCheck,        // Shows retry counters, determines available path
    UnblockWizardWithReset,    // Confirm: use reset code to unblock
    UnblockWizardWithAdmin,    // Confirm: use admin PIN to unblock
    UnblockWizardFactoryReset, // WARNING: factory reset destroys all keys
    // Programmatic flow screens (Plan 04-02):
    PinInputActive,    // TUI PIN input form is active (collecting PINs)
    OperationRunning,  // gpg subprocess is executing
    OperationResult,   // showing success/failure result
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnblockPath {
    ResetCode,
    AdminPin,
    FactoryReset,
}

pub struct PinState {
    pub screen: PinScreen,
    pub message: Option<String>,
    pub unblock_path: Option<UnblockPath>,
    pub confirm_factory_reset: bool,
    pub operation_running: bool,
    pub operation_status: Option<String>,
    pub progress_tick: usize,
    pub pending_operation: Option<PinScreen>,
}

impl Default for PinState {
    fn default() -> Self {
        Self {
            screen: PinScreen::Main,
            message: None,
            unblock_path: None,
            confirm_factory_reset: false,
            operation_running: false,
            operation_status: None,
            progress_tick: 0,
            pending_operation: None,
        }
    }
}

/// PIN Management screen as a textual-rs Widget.
///
/// Renders the PIN status panel (retry counters) and action buttons.
/// PIN input sub-screens are pushed via `push_screen_deferred`.
pub struct PinManagementScreen {
    yubikey_state: Option<YubiKeyState>,
    state: RefCell<PinState>,
    /// Tracks the WidgetId for self (set on mount) so on_action can post messages.
    own_id: Cell<Option<textual_rs::WidgetId>>,
}

impl PinManagementScreen {
    pub fn new(yubikey_state: Option<YubiKeyState>) -> Self {
        Self {
            yubikey_state,
            state: RefCell::new(PinState::default()),
            own_id: Cell::new(None),
        }
    }
}

static PIN_MAIN_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Char('c'),
        modifiers: KeyModifiers::NONE,
        action: "change_user_pin",
        description: "Change User PIN",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('a'),
        modifiers: KeyModifiers::NONE,
        action: "change_admin_pin",
        description: "Change Admin PIN",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('r'),
        modifiers: KeyModifiers::NONE,
        action: "set_reset_code",
        description: "Set Reset Code",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('u'),
        modifiers: KeyModifiers::NONE,
        action: "unblock_pin",
        description: "Unblock PIN",
        show: true,
    },
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
        action: "help",
        description: "? Help",
        show: true,
    },
];

impl Widget for PinManagementScreen {
    fn widget_type_name(&self) -> &'static str {
        "PinManagementScreen"
    }

    fn on_mount(&self, id: textual_rs::WidgetId) {
        self.own_id.set(Some(id));
    }

    fn on_unmount(&self, _id: textual_rs::WidgetId) {
        self.own_id.set(None);
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut children: Vec<Box<dyn Widget>> = Vec::new();

        children.push(Box::new(textual_rs::Header::new("PIN Management")));

        // PIN status section
        if let Some(yk) = &self.yubikey_state {
            let pin = &yk.pin_status;

            let user_status = if pin.user_pin_blocked {
                "BLOCKED"
            } else if pin.user_pin_retries <= 1 {
                "DANGER"
            } else {
                "OK"
            };

            let admin_status = if pin.admin_pin_blocked {
                "BLOCKED"
            } else if pin.admin_pin_retries <= 1 {
                "DANGER"
            } else {
                "OK"
            };

            let reset_status = if pin.reset_code_retries > 0 {
                "Set"
            } else {
                "Not set"
            };

            children.push(Box::new(Label::new(format!(
                "User PIN: {}/3 retries [{}]",
                pin.user_pin_retries, user_status
            ))));
            children.push(Box::new(Label::new(format!(
                "Admin PIN: {}/3 retries [{}]",
                pin.admin_pin_retries, admin_status
            ))));
            children.push(Box::new(Label::new(format!(
                "Reset Code: {}",
                reset_status
            ))));
        } else {
            children.push(Box::new(Label::new("No YubiKey detected.")));
        }

        // Status message (operation result)
        {
            let state = self.state.borrow();
            if let Some(msg) = &state.message {
                children.push(Box::new(Label::new("")));
                children.push(Box::new(Label::new(format!("Status: {}", msg))));
            }
        }

        children.push(Box::new(Label::new("")));

        // Action buttons
        children.push(Box::new(Button::new("Change User PIN")));
        children.push(Box::new(Button::new("Change Admin PIN")));
        children.push(Box::new(Button::new("Set Reset Code")));
        children.push(Box::new(Button::new("Unblock PIN (Wizard)")));

        children.push(Box::new(Footer));
        children
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        PIN_MAIN_BINDINGS
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "change_user_pin" => {
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    PinInputWidget::new(
                        "Change User PIN",
                        &["Current PIN", "New PIN", "Confirm New PIN"],
                    ),
                ))));
            }
            "change_admin_pin" => {
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    PinInputWidget::new(
                        "Change Admin PIN",
                        &["Current Admin PIN", "New Admin PIN", "Confirm New Admin PIN"],
                    ),
                ))));
            }
            "set_reset_code" => {
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    PinInputWidget::new(
                        "Set Reset Code",
                        &["Admin PIN", "New Reset Code", "Confirm Reset Code"],
                    ),
                ))));
            }
            "unblock_pin" => {
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    UnblockWizardScreen::new(self.yubikey_state.clone()),
                ))));
            }
            "back" => {
                ctx.pop_screen_deferred();
            }
            "help" => {
                ctx.push_screen_deferred(Box::new(
                    ModalScreen::new(Box::new(
                        PopupScreen::new("PIN Management Help", PIN_HELP_TEXT)
                    ))
                ));
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {
        // Layout and child rendering handled by compose() children.
    }
}

// ---------------------------------------------------------------------------
// Unblock PIN Wizard Screen
// ---------------------------------------------------------------------------

/// Unblock wizard — determines which recovery path is available and offers
/// appropriate options (reset code, admin PIN, or factory reset).
pub struct UnblockWizardScreen {
    yubikey_state: Option<YubiKeyState>,
}

impl UnblockWizardScreen {
    pub fn new(yubikey_state: Option<YubiKeyState>) -> Self {
        Self { yubikey_state }
    }
}

static UNBLOCK_WIZARD_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Cancel",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "",
        show: false,
    },
];

impl Widget for UnblockWizardScreen {
    fn widget_type_name(&self) -> &'static str {
        "UnblockWizardScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut children: Vec<Box<dyn Widget>> = Vec::new();

        children.push(Box::new(textual_rs::Header::new("PIN Unblock Wizard")));

        if let Some(yk) = &self.yubikey_state {
            let pin = &yk.pin_status;

            children.push(Box::new(Label::new("Current PIN status:")));
            children.push(Box::new(Label::new(format!(
                "  User PIN retries:   {}/3",
                pin.user_pin_retries
            ))));
            children.push(Box::new(Label::new(format!(
                "  Admin PIN retries:  {}/3",
                pin.admin_pin_retries
            ))));
            children.push(Box::new(Label::new(format!(
                "  Reset Code retries: {}/3",
                pin.reset_code_retries
            ))));
            children.push(Box::new(Label::new("")));
            children.push(Box::new(Label::new("Recovery options:")));

            if pin.reset_code_retries > 0 {
                children.push(Box::new(
                    Button::new("[1] Unblock with Reset Code (recommended)")
                        .with_variant(ButtonVariant::Success),
                ));
            }
            if pin.admin_pin_retries > 0 {
                children.push(Box::new(Button::new("[2] Unblock with Admin PIN")));
            }
            if pin.admin_pin_retries == 0 {
                if pin.reset_code_retries == 0 {
                    children.push(Box::new(Label::new(
                        "No recovery paths available — only factory reset remains.",
                    )));
                } else {
                    children.push(Box::new(Label::new(
                        "Admin PIN is blocked — cannot be unblocked without factory reset.",
                    )));
                }
                children.push(Box::new(
                    Button::new("[3] Factory Reset (DESTROYS ALL KEYS)")
                        .with_variant(ButtonVariant::Error),
                ));
            }
        } else {
            children.push(Box::new(Label::new("No YubiKey detected.")));
        }

        children.push(Box::new(Footer));
        children
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        UNBLOCK_WIZARD_BINDINGS
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "back" => ctx.pop_screen_deferred(),
            "unblock_with_reset" => {
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    PinInputWidget::new(
                        "Unblock with Reset Code",
                        &["Reset Code", "New User PIN", "Confirm New PIN"],
                    ),
                ))));
            }
            "unblock_with_admin" => {
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    PinInputWidget::new(
                        "Unblock with Admin PIN",
                        &["Admin PIN", "New User PIN", "Confirm New PIN"],
                    ),
                ))));
            }
            "factory_reset" => {
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    ConfirmScreen::new(
                        "Confirm Factory Reset",
                        "THIS WILL PERMANENTLY DELETE all GPG keys, certificates, and cardholder data.\nDefault PINs will be restored (User: 123456, Admin: 12345678).\nAre you ABSOLUTELY sure?",
                        true, // destructive
                    ),
                ))));
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ---------------------------------------------------------------------------
// Factory Reset screen (standalone — pushed from unblock wizard)
// ---------------------------------------------------------------------------

/// Factory Reset confirmation screen — shown when all recovery paths are exhausted.
pub struct FactoryResetScreen;

impl FactoryResetScreen {
    pub fn new() -> Self {
        Self
    }
}

static FACTORY_RESET_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "cancel",
        description: "Cancel",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        action: "cancel",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('y'),
        modifiers: KeyModifiers::NONE,
        action: "confirm_reset",
        description: "Confirm reset",
        show: true,
    },
];

impl Widget for FactoryResetScreen {
    fn widget_type_name(&self) -> &'static str {
        "FactoryResetScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        vec![
            Box::new(textual_rs::Header::new("Factory Reset")),
            Box::new(Label::new("[WARNING] DESTRUCTIVE OPERATION")),
            Box::new(Label::new("")),
            Box::new(Label::new(
                "Both your Admin PIN and Reset Code are exhausted.",
            )),
            Box::new(Label::new(
                "The only way to recover this YubiKey is a full factory reset.",
            )),
            Box::new(Label::new("")),
            Box::new(Label::new("THIS WILL PERMANENTLY DELETE:")),
            Box::new(Label::new("  - All GPG keys stored on the card")),
            Box::new(Label::new("  - All certificates")),
            Box::new(Label::new("  - All cardholder data")),
            Box::new(Label::new("")),
            Box::new(Label::new("After reset, default PINs will be restored:")),
            Box::new(Label::new("  - User PIN:  123456")),
            Box::new(Label::new("  - Admin PIN: 12345678")),
            Box::new(Label::new("")),
            Box::new(
                Button::new("Confirm Factory Reset — Press Y to execute")
                    .with_variant(ButtonVariant::Error),
            ),
            Box::new(Button::new("Cancel")),
            Box::new(Footer),
        ]
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        FACTORY_RESET_BINDINGS
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "cancel" => ctx.pop_screen_deferred(),
            "confirm_reset" => {
                // Execution is handled by the parent via model layer operations.
                // Pop to signal completion to the caller.
                ctx.pop_screen_deferred();
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ---------------------------------------------------------------------------
// Result popup helper
// ---------------------------------------------------------------------------

/// Push an operation result popup (success or failure message).
pub fn push_result_popup(ctx: &AppContext, title: &str, message: String) {
    ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(PopupScreen::new(
        title, message,
    )))));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use textual_rs::TestApp;
    use crossterm::event::KeyCode;

    #[tokio::test]
    async fn pin_default_state() {
        let yubikey_states = crate::model::mock::mock_yubikey_states();
        let yk = yubikey_states.into_iter().next();
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(PinManagementScreen::new(yk))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn pin_no_yubikey() {
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(PinManagementScreen::new(None))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn pin_unblock_wizard() {
        let yubikey_states = crate::model::mock::mock_yubikey_states();
        let yk = yubikey_states.into_iter().next();
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(PinManagementScreen::new(yk))
        });
        let mut pilot = app.pilot();
        pilot.press(KeyCode::Char('u')).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_display_snapshot!(app.backend());
    }
}
