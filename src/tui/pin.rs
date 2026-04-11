use std::cell::{Cell, RefCell};

use textual_rs::{Widget, Label, Button, ButtonVariant, Footer, Vertical, Horizontal};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub enum UnblockPath {
    ResetCode,
    AdminPin,
    FactoryReset,
}

#[allow(dead_code)]
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

            let user_label = match user_status {
                "BLOCKED" => "User PIN: Blocked — use Admin PIN to unblock".to_string(),
                "DANGER" => format!("User PIN: Warning — only {}/3 attempts remaining", pin.user_pin_retries),
                _ => format!("User PIN: Working ({}/3 attempts remaining)", pin.user_pin_retries),
            };
            let admin_label = match admin_status {
                "BLOCKED" => "Admin PIN: Blocked — factory reset required".to_string(),
                "DANGER" => format!("Admin PIN: Warning — only {}/3 attempts remaining", pin.admin_pin_retries),
                _ => format!("Admin PIN: Working ({}/3 attempts remaining)", pin.admin_pin_retries),
            };
            let card_class = if pin.user_pin_blocked || pin.admin_pin_blocked {
                "status-card-error"
            } else if pin.user_pin_retries <= 1 || pin.admin_pin_retries <= 1 {
                "status-card-warn"
            } else {
                "status-card"
            };
            children.push(Box::new(Vertical::with_children(vec![
                Box::new(Label::new(user_label)),
                Box::new(Label::new(admin_label)),
                Box::new(Label::new(format!("Reset Code: {}", reset_status))),
            ]).with_class(card_class)));
        } else {
            children.push(Box::new(Vertical::with_children(vec![
                Box::new(Label::new("No YubiKey detected.")),
                Box::new(Label::new("Insert your YubiKey and press R to refresh.")),
            ]).with_class("status-card-error")));
        }

        // Status message (operation result)
        {
            let state = self.state.borrow();
            if let Some(msg) = &state.message {
                children.push(Box::new(Label::new(format!("Status: {}", msg))));
            }
        }

        children.push(Box::new(Label::new("")));

        // Action buttons in two rows
        children.push(Box::new(Horizontal::with_children(vec![
            Box::new(Button::new("Change User PIN").with_action("change_user_pin")),
            Box::new(Button::new("Change Admin PIN").with_action("change_admin_pin")),
        ]).with_class("button-bar")));
        children.push(Box::new(Horizontal::with_children(vec![
            Box::new(Button::new("Set Reset Code").with_action("set_reset_code")),
            Box::new(Button::new("Unblock PIN (Wizard)").with_action("unblock_pin")),
        ]).with_class("button-bar")));

        children.push(Box::new(Footer));
        children
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        PIN_MAIN_BINDINGS
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "change_user_pin" => {
                ctx.push_screen_deferred(Box::new(PinOperationScreen::change_user_pin()));
            }
            "change_admin_pin" => {
                ctx.push_screen_deferred(Box::new(PinOperationScreen::change_admin_pin()));
            }
            "set_reset_code" => {
                ctx.push_screen_deferred(Box::new(PinOperationScreen::set_reset_code()));
            }
            "unblock_pin" => {
                ctx.push_screen_deferred(Box::new(
                    UnblockWizardScreen::new(self.yubikey_state.clone()),
                ));
            }
            "back" => {
                ctx.pop_screen_deferred();
            }
            "help" => {
                ctx.push_screen_deferred(Box::new(
                    PopupScreen::new("PIN Management Help", PIN_HELP_TEXT)
                ));
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
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

            children.push(Box::new(Label::new("")));
            children.push(Box::new(Vertical::with_children(vec![
                Box::new(Label::new("Current PIN Status").with_class("section-title")),
                Box::new(Label::new(format!("  User PIN retries:   {}/3", pin.user_pin_retries))),
                Box::new(Label::new(format!("  Admin PIN retries:  {}/3", pin.admin_pin_retries))),
                Box::new(Label::new(format!("  Reset Code retries: {}/3", pin.reset_code_retries))),
            ]).with_class("status-card-warn")));
            children.push(Box::new(Label::new("")));
            children.push(Box::new(Label::new("Recovery options:").with_class("section-title")));

            if pin.reset_code_retries > 0 {
                children.push(Box::new(
                    Button::new("[1] Unblock with Reset Code (recommended)")
                        .with_variant(ButtonVariant::Success)
                        .with_action("unblock_with_reset"),
                ));
            }
            if pin.admin_pin_retries > 0 {
                children.push(Box::new(Button::new("[2] Unblock with Admin PIN").with_action("unblock_with_admin")));
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
                        .with_variant(ButtonVariant::Error)
                        .with_action("factory_reset"),
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
                ctx.push_screen_deferred(Box::new(
                    PinInputWidget::new(
                        "Unblock with Reset Code",
                        &["Reset Code", "New User PIN", "Confirm New PIN"],
                    ),
                ));
            }
            "unblock_with_admin" => {
                ctx.push_screen_deferred(Box::new(
                    PinInputWidget::new(
                        "Unblock with Admin PIN",
                        &["Admin PIN", "New User PIN", "Confirm New PIN"],
                    ),
                ));
            }
            "factory_reset" => {
                ctx.push_screen_deferred(Box::new(
                    ConfirmScreen::new(
                        "Confirm Factory Reset",
                        "THIS WILL PERMANENTLY DELETE all GPG keys, certificates, and cardholder data.\nDefault PINs will be restored (User: 123456, Admin: 12345678).\nAre you ABSOLUTELY sure?",
                        true, // destructive
                    ),
                ));
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ---------------------------------------------------------------------------
// Factory Reset screen (standalone — pushed from unblock wizard)
// ---------------------------------------------------------------------------

/// Factory Reset confirmation screen — shown when all recovery paths are exhausted.
#[allow(dead_code)]
pub struct FactoryResetScreen;

impl FactoryResetScreen {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }
}

#[allow(dead_code)]
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
            Box::new(Label::new("")),
            Box::new(Vertical::with_children(vec![
                Box::new(Label::new("Both your Admin PIN and Reset Code are exhausted.")),
                Box::new(Label::new("The only way to recover this YubiKey is a full factory reset.")),
                Box::new(Label::new("")),
                Box::new(Label::new("THIS WILL PERMANENTLY DELETE:")),
                Box::new(Label::new("  - All GPG keys stored on the card")),
                Box::new(Label::new("  - All certificates")),
                Box::new(Label::new("  - All cardholder data")),
                Box::new(Label::new("")),
                Box::new(Label::new("After reset, default PINs:")),
                Box::new(Label::new("  User: 123456  Admin: 12345678")),
            ]).with_class("status-card-error")),
            Box::new(Label::new("")),
            Box::new(Horizontal::with_children(vec![
                Box::new(Button::new("Cancel").with_action("cancel")),
                Box::new(Button::new("Confirm Factory Reset")
                    .with_variant(ButtonVariant::Error)
                    .with_action("confirm_reset")),
            ]).with_class("button-bar")),
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

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ---------------------------------------------------------------------------
// Dedicated PIN operation screens
// ---------------------------------------------------------------------------

/// Which PIN operation this screen performs.
#[derive(Clone)]
enum PinOperation {
    ChangeUserPin,
    ChangeAdminPin,
    SetResetCode,
}

/// Screen that collects PIN fields and executes a PIN operation on submit.
pub struct PinOperationScreen {
    operation: PinOperation,
    fields: Vec<RefCell<String>>,
    field_labels: Vec<&'static str>,
    active_field: Cell<usize>,
    error_message: RefCell<Option<String>>,
    own_id: Cell<Option<textual_rs::WidgetId>>,
}

impl PinOperationScreen {
    pub fn change_user_pin() -> Self {
        Self {
            operation: PinOperation::ChangeUserPin,
            fields: vec![RefCell::new(String::new()), RefCell::new(String::new()), RefCell::new(String::new())],
            field_labels: vec!["Current PIN", "New PIN", "Confirm New PIN"],
            active_field: Cell::new(0),
            error_message: RefCell::new(None),
            own_id: Cell::new(None),
        }
    }
    pub fn change_admin_pin() -> Self {
        Self {
            operation: PinOperation::ChangeAdminPin,
            fields: vec![RefCell::new(String::new()), RefCell::new(String::new()), RefCell::new(String::new())],
            field_labels: vec!["Current Admin PIN", "New Admin PIN", "Confirm Admin PIN"],
            active_field: Cell::new(0),
            error_message: RefCell::new(None),
            own_id: Cell::new(None),
        }
    }
    pub fn set_reset_code() -> Self {
        Self {
            operation: PinOperation::SetResetCode,
            fields: vec![RefCell::new(String::new()), RefCell::new(String::new()), RefCell::new(String::new())],
            field_labels: vec!["Admin PIN", "New Reset Code", "Confirm Reset Code"],
            active_field: Cell::new(0),
            error_message: RefCell::new(None),
            own_id: Cell::new(None),
        }
    }
    fn recompose(&self, ctx: &AppContext) {
        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
    }
    fn title(&self) -> &'static str {
        match self.operation {
            PinOperation::ChangeUserPin => "Change User PIN",
            PinOperation::ChangeAdminPin => "Change Admin PIN",
            PinOperation::SetResetCode => "Set Reset Code",
        }
    }
}

impl Widget for PinOperationScreen {
    fn widget_type_name(&self) -> &'static str { "PinOperationScreen" }
    fn on_mount(&self, id: textual_rs::WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _id: textual_rs::WidgetId) { self.own_id.set(None); }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let active = self.active_field.get();
        let error = self.error_message.borrow().clone();

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(textual_rs::Header::new(self.title())),
            Box::new(Label::new("")),
        ];

        let mut card_lines: Vec<Box<dyn Widget>> = Vec::new();
        for (i, label) in self.field_labels.iter().enumerate() {
            let marker = if i == active { ">" } else { " " };
            let masked = "●".repeat(self.fields[i].borrow().len());
            let cursor = if i == active { "_" } else { "" };
            card_lines.push(Box::new(Label::new(format!(" {} {}:", marker, label))));
            card_lines.push(Box::new(Label::new(format!("   {}{}", masked, cursor))));
        }
        widgets.push(Box::new(Vertical::with_children(card_lines).with_class("status-card")));

        if let Some(err) = error {
            widgets.push(Box::new(Label::new("")));
            widgets.push(Box::new(Label::new(format!("Error: {}", err))));
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Label::new("Tab to switch fields, Enter to submit, Esc to cancel.")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[textual_rs::event::keybinding::KeyBinding] {
        &[
            textual_rs::event::keybinding::KeyBinding { key: KeyCode::Esc, modifiers: KeyModifiers::NONE, action: "cancel", description: "Esc Cancel", show: true },
            textual_rs::event::keybinding::KeyBinding { key: KeyCode::Tab, modifiers: KeyModifiers::NONE, action: "next_field", description: "Tab Next", show: true },
            textual_rs::event::keybinding::KeyBinding { key: KeyCode::Enter, modifiers: KeyModifiers::NONE, action: "submit", description: "Enter Submit", show: true },
        ]
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> textual_rs::widget::EventPropagation {
        use textual_rs::widget::EventPropagation;
        if let Some(key) = event.downcast_ref::<crossterm::event::KeyEvent>() {
            match key.code {
                KeyCode::Esc => { ctx.pop_screen_deferred(); return EventPropagation::Stop; }
                KeyCode::Tab => {
                    let next = (self.active_field.get() + 1) % self.fields.len();
                    self.active_field.set(next);
                    self.recompose(ctx);
                    return EventPropagation::Stop;
                }
                KeyCode::BackTab => {
                    let cur = self.active_field.get();
                    let prev = if cur == 0 { self.fields.len() - 1 } else { cur - 1 };
                    self.active_field.set(prev);
                    self.recompose(ctx);
                    return EventPropagation::Stop;
                }
                KeyCode::Backspace => {
                    self.fields[self.active_field.get()].borrow_mut().pop();
                    self.recompose(ctx);
                    return EventPropagation::Stop;
                }
                KeyCode::Enter => {
                    self.on_action("submit", ctx);
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c) => {
                    self.fields[self.active_field.get()].borrow_mut().push(c);
                    self.recompose(ctx);
                    return EventPropagation::Stop;
                }
                _ => {}
            }
        }
        textual_rs::widget::EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "cancel" => ctx.pop_screen_deferred(),
            "next_field" => {
                let next = (self.active_field.get() + 1) % self.fields.len();
                self.active_field.set(next);
                self.recompose(ctx);
            }
            "submit" => {
                let vals: Vec<String> = self.fields.iter().map(|f| f.borrow().clone()).collect();

                // Validate: all fields must be non-empty
                if vals.iter().any(|v| v.is_empty()) {
                    *self.error_message.borrow_mut() = Some("All fields are required.".to_string());
                    self.recompose(ctx);
                    return;
                }

                // Validate: new == confirm (fields[1] == fields[2])
                if vals[1] != vals[2] {
                    *self.error_message.borrow_mut() = Some("New values do not match. Try again.".to_string());
                    self.fields[2].borrow_mut().clear();
                    self.recompose(ctx);
                    return;
                }

                let result = match self.operation {
                    PinOperation::ChangeUserPin => {
                        crate::model::pin_operations::change_user_pin_programmatic(&vals[0], &vals[1])
                    }
                    PinOperation::ChangeAdminPin => {
                        crate::model::pin_operations::change_admin_pin_programmatic(&vals[0], &vals[1])
                    }
                    PinOperation::SetResetCode => {
                        crate::model::pin_operations::set_reset_code_programmatic(&vals[0], &vals[1])
                    }
                };

                match result {
                    Ok(result) => {
                        if result.success {
                            ctx.pop_screen_deferred();
                            ctx.push_screen_deferred(Box::new(PopupScreen::new(
                                "Success",
                                result.messages.join("\n"),
                            )));
                        } else {
                            *self.error_message.borrow_mut() = Some(result.messages.join("; "));
                            for f in &self.fields { f.borrow_mut().clear(); }
                            self.active_field.set(0);
                            self.recompose(ctx);
                        }
                    }
                    Err(e) => {
                        *self.error_message.borrow_mut() = Some(e.to_string());
                        for f in &self.fields { f.borrow_mut().clear(); }
                        self.active_field.set(0);
                        self.recompose(ctx);
                    }
                }
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
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

    #[tokio::test]
    async fn pin_default_state() {
        let yubikey_states = crate::model::mock::mock_yubikey_states();
        let yk = yubikey_states.into_iter().next();
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(PinManagementScreen::new(yk))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn pin_no_yubikey() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(PinManagementScreen::new(None))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn pin_unblock_wizard() {
        let yubikey_states = crate::model::mock::mock_yubikey_states();
        let yk = yubikey_states.into_iter().next();
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(PinManagementScreen::new(yk))
        });
        let mut pilot = app.pilot();
        pilot.press(KeyCode::Char('u')).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn pin_change_user_pin_form() {
        let yubikey_states = crate::model::mock::mock_yubikey_states();
        let yk = yubikey_states.into_iter().next();
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(PinManagementScreen::new(yk))
        });
        let mut pilot = app.pilot();
        pilot.press(KeyCode::Char('c')).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_snapshot!(app.backend());
    }
}
