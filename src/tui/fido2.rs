use std::cell::{Cell, RefCell};

use textual_rs::{Widget, Footer, Header, Label, WidgetId};
use textual_rs::widget::context::AppContext;
use textual_rs::widget::EventPropagation;
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::reactive::Reactive;
use textual_rs::widget::screen::ModalScreen;
use textual_rs::worker::{WorkerProgress, WorkerResult};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::fido2::{Fido2State, Fido2Credential};
use crate::tui::widgets::popup::{ConfirmScreen, PopupScreen};

// ============================================================================
// TUI State
// ============================================================================

#[derive(Default, Clone, PartialEq)]
pub struct Fido2TuiState {
    pub selected_index: usize,
    pub pin_authenticated: bool,
    pub cached_pin: Option<String>,
}

// ============================================================================
// Key Bindings
// ============================================================================

static FIDO2_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Esc Back",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Up,
        modifiers: KeyModifiers::NONE,
        action: "up",
        description: "Up",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Down,
        modifiers: KeyModifiers::NONE,
        action: "down",
        description: "Down",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('j'),
        modifiers: KeyModifiers::NONE,
        action: "down",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('k'),
        modifiers: KeyModifiers::NONE,
        action: "up",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('s'),
        modifiers: KeyModifiers::NONE,
        action: "set_pin",
        description: "S PIN",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('d'),
        modifiers: KeyModifiers::NONE,
        action: "delete_credential",
        description: "D Delete",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('r'),
        modifiers: KeyModifiers::NONE,
        action: "reset",
        description: "R Reset",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('p'),
        modifiers: KeyModifiers::NONE,
        action: "authenticate_pin",
        description: "P Unlock",
        show: true,
    },
];

// ============================================================================
// Fido2Screen Widget
// ============================================================================

/// FIDO2 / Security Key screen — shows device info, credential list, and PIN management.
///
/// Follows the textual-rs Widget pattern (D-06):
/// - Header("FIDO2 / Security Key")
/// - Info section: firmware, algorithms, PIN status (always visible — no PIN needed)
/// - Passkeys section: conditional on PIN state and credential management support
/// - Footer with keybindings
///
/// Three-state credential display (from 10-01 model layer):
/// - credentials: None = locked (PIN required but not provided)
/// - credentials: Some([]) = no credentials stored
/// - credentials: Some(creds) = populated list
pub struct Fido2Screen {
    fido2_state: Option<Fido2State>,
    state: Reactive<Fido2TuiState>,
}

impl Fido2Screen {
    pub fn new(fido2_state: Option<Fido2State>) -> Self {
        Self {
            fido2_state,
            state: Reactive::new(Fido2TuiState::default()),
        }
    }
}

impl Widget for Fido2Screen {
    fn widget_type_name(&self) -> &'static str {
        "Fido2Screen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("FIDO2 / Security Key")),
        ];

        match &self.fido2_state {
            None => {
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "No YubiKey detected. Insert your YubiKey and press Esc to return.",
                )));
            }
            Some(state) => {
                let selected_index = self.state.get().selected_index;

                // --- Info section (always visible, no PIN required per D-03) ---
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(format!(
                    "Firmware: {}",
                    state.firmware_version.as_deref().unwrap_or("Unknown")
                ))));

                let alg_str = if state.algorithms.is_empty() {
                    "None reported".to_string()
                } else {
                    state.algorithms.join(", ")
                };
                widgets.push(Box::new(Label::new(format!("Algorithms: {}", alg_str))));

                let pin_status = if state.pin_is_set {
                    format!("PIN: Set ({} retries remaining)", state.pin_retry_count)
                } else {
                    "PIN: Not set".to_string()
                };
                widgets.push(Box::new(Label::new(pin_status)));
                widgets.push(Box::new(Label::new("")));

                // --- Passkeys section ---
                if !state.pin_is_set {
                    // D-04: No PIN configured — prompt to set one
                    widgets.push(Box::new(Label::new(
                        "No PIN configured -- press S to set one.",
                    )));
                } else if state.credentials.is_none() {
                    // D-05: Credentials locked — need PIN auth
                    widgets.push(Box::new(Label::new(
                        "Credentials locked -- press P to authenticate",
                    )));
                } else if !state.supports_cred_mgmt {
                    widgets.push(Box::new(Label::new(
                        "Passkey management requires CTAP 2.1 (not supported by this device)",
                    )));
                } else {
                    match &state.credentials {
                        Some(creds) if creds.is_empty() => {
                            widgets.push(Box::new(Label::new("No passkeys stored on this device.")));
                        }
                        Some(creds) => {
                            widgets.push(Box::new(Label::new(format!("Passkeys ({})", creds.len()))));
                            for (idx, cred) in creds.iter().enumerate() {
                                let marker = if idx == selected_index { ">" } else { " " };
                                widgets.push(Box::new(Label::new(format!(
                                    "  {} {:<30}  {}",
                                    marker, cred.rp_id, cred.user_name
                                ))));
                            }
                        }
                        None => {
                            // Already handled above — cannot reach here
                        }
                    }
                }
            }
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        FIDO2_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            // Match key against our bindings to dispatch on_action
            for binding in FIDO2_BINDINGS {
                if binding.key == key.code && binding.modifiers == key.modifiers {
                    self.on_action(binding.action, ctx);
                    return EventPropagation::Stop;
                }
            }
        }
        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "back" => ctx.pop_screen_deferred(),
            "up" => {
                let current = self.state.get().selected_index;
                if current > 0 {
                    self.state.update(|s| s.selected_index = current - 1);
                }
            }
            "down" => {
                let cred_count = self
                    .fido2_state
                    .as_ref()
                    .and_then(|s| s.credentials.as_ref())
                    .map(|c| c.len())
                    .unwrap_or(0);
                if cred_count > 0 {
                    let current = self.state.get().selected_index;
                    if current + 1 < cred_count {
                        self.state.update(|s| s.selected_index = current + 1);
                    }
                }
            }
            "set_pin" => {
                let pin_is_set = self
                    .fido2_state
                    .as_ref()
                    .map(|s| s.pin_is_set)
                    .unwrap_or(false);
                if pin_is_set {
                    ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                        PinChangeScreen::new(),
                    ))));
                } else {
                    ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                        PinSetScreen::new(),
                    ))));
                }
            }
            "delete_credential" => {
                let selected_idx = self.state.get().selected_index;
                let cached_pin = self.state.get().cached_pin.clone();
                let cred_opt = self
                    .fido2_state
                    .as_ref()
                    .and_then(|s| s.credentials.as_ref())
                    .and_then(|c| c.get(selected_idx))
                    .cloned();

                if let Some(cred) = cred_opt {
                    ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                        DeleteCredentialScreen::new(
                            cred.rp_id,
                            cred.user_name,
                            cred.credential_id,
                            cached_pin,
                        ),
                    ))));
                }
            }
            "authenticate_pin" => {
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    PinAuthScreen::new(),
                ))));
            }
            "reset" => {
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    ResetConfirmScreen::new(),
                ))));
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {
        // Rendering handled by compose() — leaf rendering not needed for container screens.
    }
}

// ============================================================================
// PinSetScreen — Set a new FIDO2 PIN (no PIN currently configured)
// ============================================================================

#[derive(Default, Clone, Copy, PartialEq)]
pub enum PinSetStep {
    #[default]
    EnterNew,
    ConfirmNew,
}

/// Pushed screen for setting a new FIDO2 PIN when no PIN is currently configured.
pub struct PinSetScreen {
    new_pin: RefCell<String>,
    confirm_pin: RefCell<String>,
    step: RefCell<PinSetStep>,
    error_message: RefCell<Option<String>>,
}

impl PinSetScreen {
    pub fn new() -> Self {
        Self {
            new_pin: RefCell::new(String::new()),
            confirm_pin: RefCell::new(String::new()),
            step: RefCell::new(PinSetStep::default()),
            error_message: RefCell::new(None),
        }
    }
}

static PIN_SET_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "cancel",
        description: "Esc Cancel",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "next_step",
        description: "Enter Next",
        show: true,
    },
];

impl Widget for PinSetScreen {
    fn widget_type_name(&self) -> &'static str {
        "PinSetScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let step = *self.step.borrow();
        let error = self.error_message.borrow().clone();

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("Set FIDO2 PIN")),
            Box::new(Label::new("")),
        ];

        match step {
            PinSetStep::EnterNew => {
                widgets.push(Box::new(Label::new("Enter new PIN (min 4 characters):")));
                let masked = "*".repeat(self.new_pin.borrow().len());
                widgets.push(Box::new(Label::new(format!("> {}_", masked))));
            }
            PinSetStep::ConfirmNew => {
                widgets.push(Box::new(Label::new("Confirm new PIN:")));
                let masked = "*".repeat(self.confirm_pin.borrow().len());
                widgets.push(Box::new(Label::new(format!("> {}_", masked))));
            }
        }

        if let Some(err) = error {
            widgets.push(Box::new(Label::new("")));
            widgets.push(Box::new(Label::new(format!("Error: {}", err))));
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        PIN_SET_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            match key.code {
                KeyCode::Esc => {
                    ctx.pop_screen_deferred();
                    return EventPropagation::Stop;
                }
                KeyCode::Backspace => {
                    let step = *self.step.borrow();
                    match step {
                        PinSetStep::EnterNew => {
                            self.new_pin.borrow_mut().pop();
                        }
                        PinSetStep::ConfirmNew => {
                            self.confirm_pin.borrow_mut().pop();
                        }
                    }
                    return EventPropagation::Stop;
                }
                KeyCode::Enter => {
                    self.on_action("next_step", ctx);
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c) => {
                    let step = *self.step.borrow();
                    match step {
                        PinSetStep::EnterNew => {
                            self.new_pin.borrow_mut().push(c);
                        }
                        PinSetStep::ConfirmNew => {
                            self.confirm_pin.borrow_mut().push(c);
                        }
                    }
                    return EventPropagation::Stop;
                }
                _ => {}
            }
        }
        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "cancel" => ctx.pop_screen_deferred(),
            "next_step" => {
                let step = *self.step.borrow();
                match step {
                    PinSetStep::EnterNew => {
                        let pin = self.new_pin.borrow().clone();
                        if pin.len() < 4 {
                            *self.error_message.borrow_mut() =
                                Some("PIN must be at least 4 characters".to_string());
                            return;
                        }
                        *self.error_message.borrow_mut() = None;
                        *self.step.borrow_mut() = PinSetStep::ConfirmNew;
                    }
                    PinSetStep::ConfirmNew => {
                        let new_pin = self.new_pin.borrow().clone();
                        let confirm = self.confirm_pin.borrow().clone();
                        if new_pin != confirm {
                            *self.error_message.borrow_mut() =
                                Some("PINs do not match. Try again.".to_string());
                            self.confirm_pin.borrow_mut().clear();
                            return;
                        }
                        match crate::model::fido2::set_pin(&new_pin) {
                            Ok(()) => {
                                ctx.pop_screen_deferred();
                                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                                    PopupScreen::new("Success", "PIN set successfully."),
                                ))));
                            }
                            Err(e) => {
                                *self.error_message.borrow_mut() = Some(e.to_string());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ============================================================================
// PinChangeScreen — Change an existing FIDO2 PIN
// ============================================================================

#[derive(Default, Clone, Copy, PartialEq)]
pub enum PinChangeStep {
    #[default]
    EnterCurrent,
    EnterNew,
    ConfirmNew,
}

/// Pushed screen for changing an existing FIDO2 PIN.
pub struct PinChangeScreen {
    current_pin: RefCell<String>,
    new_pin: RefCell<String>,
    confirm_pin: RefCell<String>,
    step: RefCell<PinChangeStep>,
    error_message: RefCell<Option<String>>,
}

impl PinChangeScreen {
    pub fn new() -> Self {
        Self {
            current_pin: RefCell::new(String::new()),
            new_pin: RefCell::new(String::new()),
            confirm_pin: RefCell::new(String::new()),
            step: RefCell::new(PinChangeStep::default()),
            error_message: RefCell::new(None),
        }
    }
}

static PIN_CHANGE_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "cancel",
        description: "Esc Cancel",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "next_step",
        description: "Enter Next",
        show: true,
    },
];

impl Widget for PinChangeScreen {
    fn widget_type_name(&self) -> &'static str {
        "PinChangeScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let step = *self.step.borrow();
        let error = self.error_message.borrow().clone();

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("Change FIDO2 PIN")),
            Box::new(Label::new("")),
        ];

        match step {
            PinChangeStep::EnterCurrent => {
                widgets.push(Box::new(Label::new("Enter current PIN:")));
                let masked = "*".repeat(self.current_pin.borrow().len());
                widgets.push(Box::new(Label::new(format!("> {}_", masked))));
            }
            PinChangeStep::EnterNew => {
                widgets.push(Box::new(Label::new("Enter new PIN (min 4 characters):")));
                let masked = "*".repeat(self.new_pin.borrow().len());
                widgets.push(Box::new(Label::new(format!("> {}_", masked))));
            }
            PinChangeStep::ConfirmNew => {
                widgets.push(Box::new(Label::new("Confirm new PIN:")));
                let masked = "*".repeat(self.confirm_pin.borrow().len());
                widgets.push(Box::new(Label::new(format!("> {}_", masked))));
            }
        }

        if let Some(err) = error {
            widgets.push(Box::new(Label::new("")));
            widgets.push(Box::new(Label::new(format!("Error: {}", err))));
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        PIN_CHANGE_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            match key.code {
                KeyCode::Esc => {
                    ctx.pop_screen_deferred();
                    return EventPropagation::Stop;
                }
                KeyCode::Backspace => {
                    let step = *self.step.borrow();
                    match step {
                        PinChangeStep::EnterCurrent => {
                            self.current_pin.borrow_mut().pop();
                        }
                        PinChangeStep::EnterNew => {
                            self.new_pin.borrow_mut().pop();
                        }
                        PinChangeStep::ConfirmNew => {
                            self.confirm_pin.borrow_mut().pop();
                        }
                    }
                    return EventPropagation::Stop;
                }
                KeyCode::Enter => {
                    self.on_action("next_step", ctx);
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c) => {
                    let step = *self.step.borrow();
                    match step {
                        PinChangeStep::EnterCurrent => {
                            self.current_pin.borrow_mut().push(c);
                        }
                        PinChangeStep::EnterNew => {
                            self.new_pin.borrow_mut().push(c);
                        }
                        PinChangeStep::ConfirmNew => {
                            self.confirm_pin.borrow_mut().push(c);
                        }
                    }
                    return EventPropagation::Stop;
                }
                _ => {}
            }
        }
        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "cancel" => ctx.pop_screen_deferred(),
            "next_step" => {
                let step = *self.step.borrow();
                match step {
                    PinChangeStep::EnterCurrent => {
                        *self.error_message.borrow_mut() = None;
                        *self.step.borrow_mut() = PinChangeStep::EnterNew;
                    }
                    PinChangeStep::EnterNew => {
                        let pin = self.new_pin.borrow().clone();
                        if pin.len() < 4 {
                            *self.error_message.borrow_mut() =
                                Some("PIN must be at least 4 characters".to_string());
                            return;
                        }
                        *self.error_message.borrow_mut() = None;
                        *self.step.borrow_mut() = PinChangeStep::ConfirmNew;
                    }
                    PinChangeStep::ConfirmNew => {
                        let current = self.current_pin.borrow().clone();
                        let new_pin = self.new_pin.borrow().clone();
                        let confirm = self.confirm_pin.borrow().clone();
                        if new_pin != confirm {
                            *self.error_message.borrow_mut() =
                                Some("PINs do not match. Try again.".to_string());
                            self.confirm_pin.borrow_mut().clear();
                            return;
                        }
                        match crate::model::fido2::change_pin(&current, &new_pin) {
                            Ok(()) => {
                                ctx.pop_screen_deferred();
                                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                                    PopupScreen::new("Success", "PIN changed successfully."),
                                ))));
                            }
                            Err(e) => {
                                *self.error_message.borrow_mut() = Some(e.to_string());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ============================================================================
// PinAuthScreen — Authenticate with PIN to unlock credential list
// ============================================================================

/// Pushed screen for authenticating with a PIN to unlock the FIDO2 credential list.
///
/// On success: pops self and the parent Fido2Screen, then pushes a new Fido2Screen
/// with the credentials populated (fresh fido2_info merged with enumerated credentials).
pub struct PinAuthScreen {
    pin_input: RefCell<String>,
    error_message: RefCell<Option<String>>,
}

impl PinAuthScreen {
    pub fn new() -> Self {
        Self {
            pin_input: RefCell::new(String::new()),
            error_message: RefCell::new(None),
        }
    }
}

static PIN_AUTH_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "cancel",
        description: "Esc Cancel",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "authenticate",
        description: "Enter Unlock",
        show: true,
    },
];

impl Widget for PinAuthScreen {
    fn widget_type_name(&self) -> &'static str {
        "PinAuthScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let error = self.error_message.borrow().clone();
        let masked = "*".repeat(self.pin_input.borrow().len());

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("Authenticate FIDO2 PIN")),
            Box::new(Label::new("")),
            Box::new(Label::new("Enter FIDO2 PIN:")),
            Box::new(Label::new(format!("> {}_", masked))),
        ];

        if let Some(err) = error {
            widgets.push(Box::new(Label::new("")));
            widgets.push(Box::new(Label::new(format!("Error: {}", err))));
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        PIN_AUTH_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            match key.code {
                KeyCode::Esc => {
                    ctx.pop_screen_deferred();
                    return EventPropagation::Stop;
                }
                KeyCode::Backspace => {
                    self.pin_input.borrow_mut().pop();
                    return EventPropagation::Stop;
                }
                KeyCode::Enter => {
                    self.on_action("authenticate", ctx);
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c) => {
                    self.pin_input.borrow_mut().push(c);
                    return EventPropagation::Stop;
                }
                _ => {}
            }
        }
        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "cancel" => ctx.pop_screen_deferred(),
            "authenticate" => {
                let pin = self.pin_input.borrow().clone();
                match crate::model::fido2::enumerate_credentials(&pin) {
                    Ok(credentials) => {
                        // Get fresh device info and merge with enumerated credentials
                        let new_state = match crate::model::fido2::get_fido2_info() {
                            Ok(mut info) => {
                                info.credentials = Some(credentials);
                                Some(info)
                            }
                            Err(_) => {
                                // Fallback: create a minimal state with just the credentials
                                None
                            }
                        };
                        // Pop PinAuthScreen, pop parent Fido2Screen, push new Fido2Screen
                        ctx.pop_screen_deferred();
                        ctx.pop_screen_deferred();
                        ctx.push_screen_deferred(Box::new(Fido2Screen::new(new_state)));
                    }
                    Err(e) => {
                        *self.error_message.borrow_mut() = Some(format!("Invalid PIN: {}", e));
                        self.pin_input.borrow_mut().clear();
                    }
                }
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ============================================================================
// DeleteCredentialScreen — Confirm and delete a FIDO2 credential
// ============================================================================

/// Pushed screen wrapping ConfirmScreen to delete a specific FIDO2 credential.
///
/// Follows the OathScreen DeleteConfirmScreen pattern exactly.
pub struct DeleteCredentialScreen {
    rp_id: String,
    user_name: String,
    credential_id: Vec<u8>,
    cached_pin: Option<String>,
    inner: ConfirmScreen,
}

impl DeleteCredentialScreen {
    pub fn new(
        rp_id: String,
        user_name: String,
        credential_id: Vec<u8>,
        cached_pin: Option<String>,
    ) -> Self {
        let body = format!(
            "Permanently delete passkey for '{}'?\n\nUser: {}\n\nThis cannot be undone.",
            rp_id, user_name
        );
        Self {
            rp_id,
            user_name,
            credential_id,
            cached_pin,
            inner: ConfirmScreen::new("Delete Passkey", body, true),
        }
    }
}

impl Widget for DeleteCredentialScreen {
    fn widget_type_name(&self) -> &'static str {
        "DeleteCredentialScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        self.inner.compose()
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        self.inner.key_bindings()
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "confirm" => {
                if let Some(ref pin) = self.cached_pin {
                    match crate::model::fido2::delete_credential(pin, &self.credential_id) {
                        Ok(()) => {
                            ctx.pop_screen_deferred();
                            ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                                PopupScreen::new(
                                    "Success",
                                    format!("'{}' credential deleted.", self.rp_id),
                                ),
                            ))));
                        }
                        Err(e) => {
                            ctx.pop_screen_deferred();
                            ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                                PopupScreen::new("Error", format!("Delete failed: {}", e)),
                            ))));
                        }
                    }
                } else {
                    // No cached PIN — need to authenticate first
                    // Push PinAuthScreen; user must authenticate before deleting
                    ctx.pop_screen_deferred();
                    ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                        PinAuthScreen::new(),
                    ))));
                }
            }
            "cancel" => ctx.pop_screen_deferred(),
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ============================================================================
// ResetConfirmScreen — Irreversibility warning before reset proceeds (D-10)
// ============================================================================

/// Confirmation dialog warning the user that FIDO2 reset permanently destroys
/// all passkeys and the PIN. Pops self and pushes ResetGuidanceScreen on confirm.
pub struct ResetConfirmScreen {
    inner: ConfirmScreen,
}

impl ResetConfirmScreen {
    pub fn new() -> Self {
        let body = "WARNING: This will permanently delete ALL passkeys and the FIDO2 PIN.\n\n\
            All FIDO2 credentials stored on this YubiKey will be destroyed.\n\
            This action cannot be undone.\n\n\
            Are you sure you want to proceed?";
        Self {
            inner: ConfirmScreen::new("Reset FIDO2 Applet", body, true),
        }
    }
}

impl Widget for ResetConfirmScreen {
    fn widget_type_name(&self) -> &'static str {
        "ResetConfirmScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        self.inner.compose()
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        self.inner.key_bindings()
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "confirm" => {
                ctx.pop_screen_deferred();
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    ResetGuidanceScreen::new(),
                ))));
            }
            "cancel" => ctx.pop_screen_deferred(),
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ============================================================================
// ResetGuidanceScreen — Countdown timer + replug detection + outcome (D-10, D-11)
// ============================================================================

/// Tracks the phase of the FIDO2 reset guided workflow.
#[derive(Clone, PartialEq)]
pub enum ResetPhase {
    /// Step 1: instruct user to unplug the device first
    WaitingForUnplug,
    /// Step 2: countdown with seconds_remaining; poll for device reconnect
    WaitingForReplug(u8),
    /// Device detected — sending reset command now
    Resetting,
    /// Reset completed successfully
    Success,
    /// 10-second window expired — device was not replugged in time
    Expired,
    /// Reset command failed with an error message
    Error(String),
}

/// Result type delivered from the countdown worker task.
#[derive(Clone)]
pub enum ResetWorkerResult {
    /// Device reconnected within the window — caller should now call reset_fido2()
    DeviceFound,
    /// 10-second window elapsed without device reconnection
    Expired,
}

/// Guided reset screen: instructs user to unplug/replug with a 10-second countdown
/// and device polling. Explains the 10-second timing requirement (D-11).
pub struct ResetGuidanceScreen {
    phase: Reactive<ResetPhase>,
    own_id: Cell<Option<WidgetId>>,
}

impl ResetGuidanceScreen {
    pub fn new() -> Self {
        Self {
            phase: Reactive::new(ResetPhase::WaitingForUnplug),
            own_id: Cell::new(None),
        }
    }
}

static RESET_GUIDANCE_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "cancel",
        description: "Esc Cancel",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "next",
        description: "Enter",
        show: true,
    },
];

impl Widget for ResetGuidanceScreen {
    fn widget_type_name(&self) -> &'static str {
        "ResetGuidanceScreen"
    }

    fn on_mount(&self, id: WidgetId) {
        self.own_id.set(Some(id));
    }

    fn on_unmount(&self, _id: WidgetId) {
        self.own_id.set(None);
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let phase = self.phase.get();
        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("FIDO2 Reset")),
            Box::new(Label::new("")),
        ];

        match &phase {
            ResetPhase::WaitingForUnplug => {
                widgets.push(Box::new(Label::new(
                    "FIDO2 protocol requires the device to receive the reset command",
                )));
                widgets.push(Box::new(Label::new(
                    "within 10 seconds of being plugged in.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Step 1: Unplug your YubiKey now.")));
                widgets.push(Box::new(Label::new("Step 2: Wait for the countdown to start.")));
                widgets.push(Box::new(Label::new("Step 3: Replug your YubiKey when prompted.")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Press Enter when your YubiKey is unplugged, or Esc to cancel.",
                )));
            }
            ResetPhase::WaitingForReplug(secs) => {
                widgets.push(Box::new(Label::new(
                    "FIDO2 protocol requires reset within 10s of power-on (USB insertion).",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(">> Plug in your YubiKey NOW <<")));
                widgets.push(Box::new(Label::new("")));
                // Countdown bar: 20 chars wide, filled proportional to time remaining
                let filled = ((*secs as f32 / 10.0) * 20.0).round() as usize;
                let empty = 20usize.saturating_sub(filled);
                let bar = format!(
                    "Time remaining: {}s  [{}{}]",
                    secs,
                    "#".repeat(filled),
                    " ".repeat(empty)
                );
                widgets.push(Box::new(Label::new(bar)));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Waiting for device...")));
            }
            ResetPhase::Resetting => {
                widgets.push(Box::new(Label::new(
                    "Device detected! Sending reset command...",
                )));
            }
            ResetPhase::Success => {
                widgets.push(Box::new(Label::new(
                    "FIDO2 applet has been reset to factory defaults.",
                )));
                widgets.push(Box::new(Label::new(
                    "All passkeys and the FIDO2 PIN have been removed.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Press Enter or Esc to return.")));
            }
            ResetPhase::Expired => {
                widgets.push(Box::new(Label::new(
                    "Window expired -- the 10-second timing window has passed.",
                )));
                widgets.push(Box::new(Label::new(
                    "The device was not replugged in time.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Press Enter to try again, or Esc to cancel.",
                )));
            }
            ResetPhase::Error(msg) => {
                widgets.push(Box::new(Label::new(format!("Reset failed: {}", msg))));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Press Esc to return.")));
            }
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        RESET_GUIDANCE_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        // Handle WorkerProgress<u8> — countdown tick
        if let Some(progress) = event.downcast_ref::<WorkerProgress<u8>>() {
            if progress.source_id == self.own_id.get().unwrap_or(progress.source_id) {
                self.phase.update(|p| *p = ResetPhase::WaitingForReplug(progress.progress));
                return EventPropagation::Stop;
            }
        }

        // Handle WorkerResult<ResetWorkerResult> — countdown finished
        if let Some(result) = event.downcast_ref::<WorkerResult<ResetWorkerResult>>() {
            if result.source_id == self.own_id.get().unwrap_or(result.source_id) {
                match &result.value {
                    ResetWorkerResult::DeviceFound => {
                        self.phase.update(|p| *p = ResetPhase::Resetting);
                        // Device is present — send reset command synchronously (single HID frame)
                        match crate::model::fido2::reset_fido2() {
                            Ok(()) => {
                                self.phase.update(|p| *p = ResetPhase::Success);
                            }
                            Err(e) => {
                                let msg = e.to_string();
                                self.phase.update(|p| *p = ResetPhase::Error(msg));
                            }
                        }
                    }
                    ResetWorkerResult::Expired => {
                        self.phase.update(|p| *p = ResetPhase::Expired);
                    }
                }
                return EventPropagation::Stop;
            }
        }

        // Handle keyboard events
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            for binding in RESET_GUIDANCE_BINDINGS {
                if binding.key == key.code && binding.modifiers == key.modifiers {
                    self.on_action(binding.action, ctx);
                    return EventPropagation::Stop;
                }
            }
        }

        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        let phase = self.phase.get();
        match action {
            "cancel" => ctx.pop_screen_deferred(),
            "next" => {
                match &phase {
                    ResetPhase::WaitingForUnplug => {
                        // Start the countdown worker
                        if let Some(own_id) = self.own_id.get() {
                            self.phase.update(|p| *p = ResetPhase::WaitingForReplug(10));
                            ctx.run_worker_with_progress(own_id, |progress_tx| {
                                Box::pin(async move {
                                    for secs_remaining in (0u8..=10u8).rev() {
                                        let _ = progress_tx.send(secs_remaining);
                                        // Check if device is present
                                        if secs_remaining > 0
                                            && crate::model::fido2::is_fido_device_present()
                                        {
                                            return ResetWorkerResult::DeviceFound;
                                        }
                                        if secs_remaining == 0 {
                                            break;
                                        }
                                        tokio::time::sleep(
                                            std::time::Duration::from_secs(1),
                                        )
                                        .await;
                                    }
                                    ResetWorkerResult::Expired
                                })
                            });
                        }
                    }
                    ResetPhase::Success => {
                        ctx.pop_screen_deferred();
                    }
                    ResetPhase::Expired => {
                        // Retry: go back to WaitingForUnplug
                        self.phase.update(|p| *p = ResetPhase::WaitingForUnplug);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use textual_rs::TestApp;
    use crate::model::fido2::{Fido2State, Fido2Credential};

    fn mock_fido2_state() -> Fido2State {
        Fido2State {
            firmware_version: Some("5.4.3".to_string()),
            algorithms: vec!["ES256".to_string(), "EdDSA".to_string()],
            pin_is_set: true,
            pin_retry_count: 8,
            credentials: Some(vec![
                Fido2Credential {
                    rp_id: "github.com".to_string(),
                    rp_name: Some("GitHub".to_string()),
                    user_name: "user@example.com".to_string(),
                    credential_id: vec![0x01, 0x02, 0x03, 0x04],
                },
                Fido2Credential {
                    rp_id: "google.com".to_string(),
                    rp_name: Some("Google".to_string()),
                    user_name: "user@gmail.com".to_string(),
                    credential_id: vec![0x05, 0x06, 0x07, 0x08],
                },
            ]),
            supports_cred_mgmt: true,
        }
    }

    #[tokio::test]
    async fn fido2_default_state() {
        let state = Some(mock_fido2_state());
        let mut app = TestApp::new(80, 24, move || {
            Box::new(Fido2Screen::new(state.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn fido2_no_yubikey() {
        let mut app = TestApp::new(80, 24, || {
            Box::new(Fido2Screen::new(None))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn fido2_no_pin() {
        let mut state = mock_fido2_state();
        state.pin_is_set = false;
        state.credentials = Some(vec![]);
        let state = Some(state);
        let mut app = TestApp::new(80, 24, move || {
            Box::new(Fido2Screen::new(state.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn fido2_credentials_locked() {
        let mut state = mock_fido2_state();
        state.credentials = None; // locked
        let state = Some(state);
        let mut app = TestApp::new(80, 24, move || {
            Box::new(Fido2Screen::new(state.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn fido2_navigate_down() {
        let state = Some(mock_fido2_state());
        let mut app = TestApp::new(80, 24, move || {
            Box::new(Fido2Screen::new(state.clone()))
        });
        let mut pilot = app.pilot();
        pilot.settle().await;
        pilot.press(KeyCode::Down).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn fido2_reset_confirm_screen() {
        let mut app = TestApp::new(80, 24, || {
            Box::new(ResetConfirmScreen::new())
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn fido2_reset_guidance_waiting() {
        let mut app = TestApp::new(80, 24, || {
            Box::new(ResetGuidanceScreen::new())
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn fido2_from_mock() {
        let states = crate::model::mock::mock_yubikey_states();
        let fido2_state = states.first().and_then(|yk| yk.fido2.clone());
        let mut app = TestApp::new(80, 24, move || {
            Box::new(Fido2Screen::new(fido2_state.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }
}
