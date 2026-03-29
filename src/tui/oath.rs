use std::cell::{Cell, RefCell};

use textual_rs::{Widget, Footer, Header, Label, Button, DataTable, ColumnDef, ProgressBar};
use textual_rs::widget::context::AppContext;
use textual_rs::widget::{EventPropagation, WidgetId};
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::reactive::Reactive;
use textual_rs::worker::WorkerResult;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::oath::{OathState, OathType, OathAlgorithm};
use crate::tui::widgets::popup::{ConfirmScreen, PopupScreen};

const OATH_HELP_TEXT: &str = "\
OATH / Authenticator\n\
\n\
OATH manages TOTP and HOTP one-time password credentials on your YubiKey.\n\
These are the same 6-digit codes you would see in Google Authenticator or\n\
Authy, but stored securely on hardware instead of a phone.\n\
\n\
TOTP codes change every 30 seconds. HOTP codes advance on each use.\n\
\n\
You can add new accounts, delete existing ones, and see live codes.\n\
Touch-required credentials need a physical key touch to reveal the code.";

// ============================================================================
// TUI State
// ============================================================================

#[derive(Default, Clone, PartialEq)]
pub struct OathTuiState {
    pub selected_index: usize,
    pub scroll_offset: usize,
}

// ============================================================================
// Key Bindings
// ============================================================================

static OATH_BINDINGS: &[KeyBinding] = &[
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
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "generate_hotp",
        description: "Enter Generate",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('a'),
        modifiers: KeyModifiers::NONE,
        action: "add_account",
        description: "A Add",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('d'),
        modifiers: KeyModifiers::NONE,
        action: "delete_account",
        description: "D Delete",
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
        key: KeyCode::Char('?'),
        modifiers: KeyModifiers::NONE,
        action: "help",
        description: "? Help",
        show: true,
    },
];

// ============================================================================
// OathScreen Widget
// ============================================================================

/// OATH Credentials screen — shows TOTP/HOTP credentials with live countdown timer.
///
/// Follows the textual-rs Widget pattern (D-01, D-07, D-15):
/// - Header("OATH Credentials")
/// - Credential list as Labels (name, code, type badge per row)
/// - Countdown bar showing seconds remaining in 30s TOTP window
/// - Footer with keybindings
///
/// The countdown is computed on each render from chrono::Utc::now() — no timer
/// thread needed since textual-rs re-renders on any key event.
///
/// HOTP credentials show "[press Enter]" until explicitly triggered via Enter key.
pub struct OathScreen {
    oath_state: RefCell<Option<OathState>>,
    key_present: bool,
    loading: Cell<bool>,
    state: Reactive<OathTuiState>,
    own_id: Cell<Option<WidgetId>>,
}

impl OathScreen {
    pub fn new(oath_state: Option<OathState>) -> Self {
        Self {
            oath_state: RefCell::new(oath_state),
            key_present: false,
            loading: Cell::new(false),
            state: Reactive::new(OathTuiState::default()),
            own_id: Cell::new(None),
        }
    }

    pub fn new_with_key(oath_state: Option<OathState>) -> Self {
        Self {
            oath_state: RefCell::new(oath_state),
            key_present: true,
            loading: Cell::new(false),
            state: Reactive::new(OathTuiState::default()),
            own_id: Cell::new(None),
        }
    }

}

impl Widget for OathScreen {
    fn widget_type_name(&self) -> &'static str {
        "OathScreen"
    }

    fn on_mount(&self, id: WidgetId) {
        self.own_id.set(Some(id));
    }

    fn on_unmount(&self, _id: WidgetId) {
        self.own_id.set(None);
        self.loading.set(false);
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(result) = event.downcast_ref::<WorkerResult<anyhow::Result<OathState>>>() {
            self.loading.set(false);
            match &result.value {
                Ok(state) => {
                    *self.oath_state.borrow_mut() = Some(state.clone());
                }
                Err(_) => {
                    // Leave oath_state as None; compose will show the no-yubikey message
                }
            }
            if let Some(id) = self.own_id.get() {
                ctx.request_recompose(id);
            }
            return EventPropagation::Stop;
        }
        EventPropagation::Continue
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("OATH Credentials")),
        ];

        let oath_state = self.oath_state.borrow();
        match &*oath_state {
            None => {
                widgets.push(Box::new(Label::new("")));
                if self.loading.get() {
                    widgets.push(Box::new(Label::new("Loading OATH credentials...")));
                } else if self.key_present {
                    widgets.push(Box::new(Label::new(
                        "OATH credentials not loaded. Press R to load.",
                    )));
                } else {
                    widgets.push(Box::new(Label::new(
                        "No YubiKey detected. Insert your YubiKey and press R to refresh.",
                    )));
                }
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Button::new("Refresh (R)")));
            }
            Some(state) if state.password_required => {
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "OATH applet is password-protected.",
                )));
                widgets.push(Box::new(Label::new(
                    "Password management is not yet supported (deferred to v2).",
                )));
                widgets.push(Box::new(Label::new(
                    "Use the yubikey manager CLI to remove the password, then retry.",
                )));
            }
            Some(state) if state.credentials.is_empty() => {
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "No OATH credentials stored.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Button::new("Add Account (A)")));
            }
            Some(state) => {
                let selected = self.state.get_untracked().selected_index;

                widgets.push(Box::new(Label::new("")));

                // Credential list as DataTable
                let columns = vec![
                    ColumnDef::new("").with_width(2),
                    ColumnDef::new("Name").with_width(30),
                    ColumnDef::new("Code").with_width(14),
                    ColumnDef::new("Type").with_width(8),
                ];
                let mut table = DataTable::new(columns);

                for (idx, cred) in state.credentials.iter().enumerate() {
                    let cursor = if idx == selected { ">" } else { " " };

                    let display_name = cred.issuer.as_deref().unwrap_or(&cred.name);
                    let name_col = if display_name.len() > 30 {
                        display_name[..30].to_string()
                    } else {
                        display_name.to_string()
                    };

                    let code_col = match &cred.oath_type {
                        OathType::Hotp => match &cred.code {
                            None => "[Enter]".to_string(),
                            Some(c) => c.clone(),
                        },
                        OathType::Totp => match &cred.code {
                            None => "------".to_string(),
                            Some(c) => c.clone(),
                        },
                    };

                    let type_col = match &cred.oath_type {
                        OathType::Totp => "[TOTP]",
                        OathType::Hotp => "[HOTP]",
                    };

                    table.add_row(vec![
                        cursor.to_string(),
                        name_col,
                        code_col,
                        type_col.to_string(),
                    ]);
                }

                widgets.push(Box::new(table));

                // TOTP countdown as ProgressBar
                let now_secs = chrono::Utc::now().timestamp();
                let secs_remaining = 30 - (now_secs % 30);
                let progress = secs_remaining as f64 / 30.0;

                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(format!(
                    "TOTP refreshes in {}s",
                    secs_remaining
                ))));
                widgets.push(Box::new(ProgressBar::new(progress)));

                // Action Buttons
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Button::new("Add Account (A)")));
                widgets.push(Box::new(Button::new("Delete Account (D)")));
                widgets.push(Box::new(Button::new("Refresh (R)")));
            }
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        OATH_BINDINGS
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "back" => ctx.pop_screen_deferred(),
            "up" => {
                let current = self.state.get_untracked().selected_index;
                if current > 0 {
                    self.state.update(|s| s.selected_index = current - 1);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                }
            }
            "down" => {
                let cred_count = self
                    .oath_state
                    .borrow()
                    .as_ref()
                    .map(|s| s.credentials.len())
                    .unwrap_or(0);
                if cred_count > 0 {
                    let current = self.state.get_untracked().selected_index;
                    if current + 1 < cred_count {
                        self.state.update(|s| s.selected_index = current + 1);
                        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    }
                }
            }
            "generate_hotp" => {
                // Check if selected credential is HOTP
                let is_hotp = self
                    .oath_state
                    .borrow()
                    .as_ref()
                    .and_then(|s| s.credentials.get(self.state.get_untracked().selected_index))
                    .map(|c| matches!(c.oath_type, OathType::Hotp))
                    .unwrap_or(false);

                if is_hotp {
                    // Full HOTP generation (CALCULATE with counter increment) is wired in Plan 03.
                    let _ = ctx;
                }
            }
            "add_account" => {
                ctx.push_screen_deferred(Box::new(AddAccountScreen::new()));
            }
            "delete_account" => {
                let selected_idx = self.state.get_untracked().selected_index;
                let oath = self.oath_state.borrow();
                let cred_opt = oath
                    .as_ref()
                    .and_then(|s| s.credentials.get(selected_idx));

                if let Some(cred) = cred_opt {
                    let name = cred.name.clone();
                    let display_name = cred.issuer.as_deref().unwrap_or(&cred.name).to_string();
                    ctx.push_screen_deferred(Box::new(DeleteConfirmScreen::new(
                        name,
                        display_name,
                    )));
                }
            }
            "refresh" => {
                // On-demand OATH fetch from card (detection.rs skips OATH as expensive)
                let fresh_oath = crate::model::oath::get_oath_state().ok();
                ctx.pop_screen_deferred();
                ctx.push_screen_deferred(Box::new(OathScreen::new(fresh_oath)));
            }
            "help" => {
                ctx.push_screen_deferred(Box::new(
                    PopupScreen::new("OATH Help", OATH_HELP_TEXT)
                ));
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ============================================================================
// AddAccountScreen Widget — 5-step sequential wizard
// ============================================================================

/// Wizard step for adding a new OATH credential.
#[derive(Default, Clone, Copy, PartialEq)]
pub enum AddAccountStep {
    #[default]
    Issuer,      // Step 1: Enter issuer (e.g., "GitHub")
    AccountName, // Step 2: Enter account name (e.g., "user@github.com")
    Secret,      // Step 3: Enter Base32 secret
    TypeSelect,  // Step 4: Select TOTP or HOTP (default TOTP)
    Confirm,     // Step 5: Review and confirm
}

/// Mutable state for the AddAccountScreen wizard.
#[derive(Clone, PartialEq)]
pub struct AddAccountState {
    pub step: AddAccountStep,
    pub issuer: String,
    pub account_name: String,
    pub secret_b32: String,
    pub oath_type: OathType,
    pub error_message: Option<String>,
}

impl Default for AddAccountState {
    fn default() -> Self {
        Self {
            step: AddAccountStep::default(),
            issuer: String::new(),
            account_name: String::new(),
            secret_b32: String::new(),
            oath_type: OathType::Totp,
            error_message: None,
        }
    }
}

/// 5-step wizard for adding a new OATH credential to the YubiKey.
pub struct AddAccountScreen {
    state: RefCell<AddAccountState>,
    input_buffer: RefCell<String>,
    own_id: Cell<Option<textual_rs::WidgetId>>,
}

impl AddAccountScreen {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(AddAccountState::default()),
            input_buffer: RefCell::new(String::new()),
            own_id: Cell::new(None),
        }
    }

    fn advance_step(&self, ctx: &AppContext) {
        let step = self.state.borrow().step;
        match step {
            AddAccountStep::Issuer => {
                let input = self.input_buffer.borrow().clone();
                self.state.borrow_mut().issuer = input;
                self.input_buffer.borrow_mut().clear();
                self.state.borrow_mut().step = AddAccountStep::AccountName;
            }
            AddAccountStep::AccountName => {
                let input = self.input_buffer.borrow().clone();
                if input.is_empty() {
                    self.state.borrow_mut().error_message =
                        Some("Account name cannot be empty.".to_string());
                    return;
                }
                self.state.borrow_mut().error_message = None;
                self.state.borrow_mut().account_name = input;
                self.input_buffer.borrow_mut().clear();
                self.state.borrow_mut().step = AddAccountStep::Secret;
            }
            AddAccountStep::Secret => {
                let input = self.input_buffer.borrow().clone();
                if input.is_empty() {
                    self.state.borrow_mut().error_message =
                        Some("Secret key cannot be empty.".to_string());
                    return;
                }
                self.state.borrow_mut().error_message = None;
                self.state.borrow_mut().secret_b32 = input;
                self.input_buffer.borrow_mut().clear();
                self.state.borrow_mut().step = AddAccountStep::TypeSelect;
            }
            AddAccountStep::TypeSelect => {
                // Type already selected via 't'/'h' keys; move to confirm
                self.state.borrow_mut().step = AddAccountStep::Confirm;
            }
            AddAccountStep::Confirm => {
                // Build credential name
                let (name, secret, oath_type) = {
                    let s = self.state.borrow();
                    let cred_name = if s.issuer.is_empty() {
                        s.account_name.clone()
                    } else {
                        format!("{}:{}", s.issuer, s.account_name)
                    };
                    (cred_name, s.secret_b32.clone(), s.oath_type.clone())
                };

                match crate::model::oath::put_credential(
                    &name,
                    &secret,
                    oath_type,
                    OathAlgorithm::Sha1,
                    6,
                ) {
                    Ok(()) => {
                        ctx.pop_screen_deferred();
                        ctx.push_screen_deferred(Box::new(
                            PopupScreen::new(
                                "Success",
                                format!("Account '{}' added successfully.", name),
                            ),
                        ));
                    }
                    Err(e) => {
                        self.state.borrow_mut().error_message = Some(e.to_string());
                        if let Some(id) = self.own_id.get() {
                            ctx.request_recompose(id);
                        }
                    }
                }
            }
        }
    }
}

static ADD_ACCOUNT_BINDINGS: &[KeyBinding] = &[
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

impl Widget for AddAccountScreen {
    fn widget_type_name(&self) -> &'static str {
        "AddAccountScreen"
    }

    fn on_mount(&self, id: textual_rs::WidgetId) {
        self.own_id.set(Some(id));
    }

    fn on_unmount(&self, _id: textual_rs::WidgetId) {
        self.own_id.set(None);
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let state = self.state.borrow();
        let input = self.input_buffer.borrow();

        let step_num = match state.step {
            AddAccountStep::Issuer => 1,
            AddAccountStep::AccountName => 2,
            AddAccountStep::Secret => 3,
            AddAccountStep::TypeSelect => 4,
            AddAccountStep::Confirm => 5,
        };
        let step_name = match state.step {
            AddAccountStep::Issuer => "Issuer",
            AddAccountStep::AccountName => "Account Name",
            AddAccountStep::Secret => "Secret Key",
            AddAccountStep::TypeSelect => "Type",
            AddAccountStep::Confirm => "Confirm",
        };

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("Add OATH Account")),
            Box::new(Label::new(format!("Step {}/5: {}", step_num, step_name))),
            Box::new(Label::new("")),
        ];

        match state.step {
            AddAccountStep::Issuer => {
                widgets.push(Box::new(Label::new("Enter issuer name (e.g., GitHub, Google):")));
                widgets.push(Box::new(Label::new(format!("> {}_", *input))));
            }
            AddAccountStep::AccountName => {
                widgets.push(Box::new(Label::new("Enter account name (e.g., user@example.com):")));
                widgets.push(Box::new(Label::new(format!("> {}_", *input))));
            }
            AddAccountStep::Secret => {
                widgets.push(Box::new(Label::new("Enter Base32 secret key:")));
                let masked = "*".repeat(input.len());
                widgets.push(Box::new(Label::new(format!("> {}_", masked))));
            }
            AddAccountStep::TypeSelect => {
                let totp_marker = if state.oath_type == OathType::Totp { ">" } else { " " };
                let hotp_marker = if state.oath_type == OathType::Hotp { ">" } else { " " };
                widgets.push(Box::new(Label::new("Select type:")));
                widgets.push(Box::new(Label::new(format!("{} [T] TOTP (time-based, default)", totp_marker))));
                widgets.push(Box::new(Label::new(format!("{} [H] HOTP (counter-based)", hotp_marker))));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Press T or H to select, Enter to confirm.")));
            }
            AddAccountStep::Confirm => {
                let cred_name = if state.issuer.is_empty() {
                    state.account_name.clone()
                } else {
                    format!("{}:{}", state.issuer, state.account_name)
                };
                widgets.push(Box::new(Label::new("Review your new OATH credential:")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(format!("  Credential name: {}", cred_name))));
                widgets.push(Box::new(Label::new(format!("  Type:            {}", state.oath_type))));
                widgets.push(Box::new(Label::new(format!(
                    "  Secret:          {}",
                    "*".repeat(state.secret_b32.len())
                ))));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Press Enter to save, Esc to cancel.")));
            }
        }

        if let Some(ref err) = state.error_message {
            widgets.push(Box::new(Label::new("")));
            widgets.push(Box::new(Label::new(format!("Error: {}", err))));
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        ADD_ACCOUNT_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            let step = self.state.borrow().step;

            match key.code {
                KeyCode::Esc => {
                    ctx.pop_screen_deferred();
                    return EventPropagation::Stop;
                }
                KeyCode::Backspace => {
                    self.input_buffer.borrow_mut().pop();
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return EventPropagation::Stop;
                }
                KeyCode::Enter => {
                    self.advance_step(ctx);
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c) if step == AddAccountStep::TypeSelect => {
                    match c {
                        't' | 'T' => {
                            self.state.borrow_mut().oath_type = OathType::Totp;
                        }
                        'h' | 'H' => {
                            self.state.borrow_mut().oath_type = OathType::Hotp;
                        }
                        _ => {}
                    }
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c)
                    if step == AddAccountStep::Issuer
                        || step == AddAccountStep::AccountName
                        || step == AddAccountStep::Secret =>
                {
                    self.input_buffer.borrow_mut().push(c);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
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
                self.advance_step(ctx);
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ============================================================================
// DeleteConfirmScreen — wraps ConfirmScreen to delete a specific credential
// ============================================================================

/// Pushed screen that shows a ConfirmScreen and deletes the credential on confirm.
///
/// Uses push_screen_deferred pattern: OathScreen pushes this; this handles
/// the "confirm" / "cancel" actions and calls delete_credential on confirm.
pub struct DeleteConfirmScreen {
    credential_name: String,
    display_name: String,
    inner: ConfirmScreen,
}

impl DeleteConfirmScreen {
    pub fn new(credential_name: String, display_name: String) -> Self {
        let body = format!(
            "Permanently delete '{}'?\n\nThis cannot be undone. The credential will be removed from the YubiKey.",
            display_name
        );
        Self {
            credential_name,
            display_name,
            inner: ConfirmScreen::new("Delete Credential", body, true),
        }
    }
}

impl Widget for DeleteConfirmScreen {
    fn widget_type_name(&self) -> &'static str {
        "DeleteConfirmScreen"
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
                match crate::model::oath::delete_credential(&self.credential_name) {
                    Ok(()) => {
                        ctx.pop_screen_deferred();
                        ctx.push_screen_deferred(Box::new(
                            PopupScreen::new(
                                "Success",
                                format!("'{}' deleted from YubiKey.", self.display_name),
                            ),
                        ));
                    }
                    Err(e) => {
                        ctx.pop_screen_deferred();
                        ctx.push_screen_deferred(Box::new(
                            PopupScreen::new("Error", format!("Delete failed: {}", e)),
                        ));
                    }
                }
            }
            "cancel" => ctx.pop_screen_deferred(),
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use textual_rs::TestApp;
    use crate::model::oath::{OathCredential, OathAlgorithm};

    /// Strip the time-varying TOTP countdown line (and following ProgressBar line) from a
    /// snapshot string so that snapshot tests are not flaky across 30-second TOTP windows.
    fn stable_snapshot(s: &impl std::fmt::Display) -> String {
        let raw = s.to_string();
        let mut result = Vec::new();
        let mut skip_next = false;
        for l in raw.lines() {
            if skip_next {
                // Replace the ProgressBar render line that follows the countdown label
                result.push("\"<ProgressBar>                                                                   \"".to_string());
                skip_next = false;
                continue;
            }
            let content = l.trim_start_matches('"').trim_start();
            if content.starts_with("TOTP refreshes in") {
                result.push("\"TOTP refreshes in <countdown>                                                   \"".to_string());
                skip_next = true;
            } else {
                result.push(l.to_string());
            }
        }
        result.join("\n")
    }

    fn mock_oath_state() -> OathState {
        OathState {
            credentials: vec![
                OathCredential {
                    name: "GitHub".to_string(),
                    issuer: Some("GitHub".to_string()),
                    oath_type: OathType::Totp,
                    algorithm: OathAlgorithm::Sha1,
                    digits: 6,
                    period: 30,
                    code: Some("123456".to_string()),
                    touch_required: false,
                },
                OathCredential {
                    name: "AWS".to_string(),
                    issuer: Some("Amazon Web Services".to_string()),
                    oath_type: OathType::Hotp,
                    algorithm: OathAlgorithm::Sha1,
                    digits: 6,
                    period: 30,
                    code: None,
                    touch_required: false,
                },
            ],
            password_required: false,
        }
    }

    #[tokio::test]
    async fn oath_screen_with_credentials() {
        let oath = Some(mock_oath_state());
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(OathScreen::new(oath.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(stable_snapshot(&app.backend()));
    }

    #[tokio::test]
    async fn oath_screen_no_yubikey() {
        let mut app = TestApp::new_styled(80, 24, "", || {
            Box::new(OathScreen::new(None))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_screen_empty_credentials() {
        let oath = Some(OathState {
            credentials: vec![],
            password_required: false,
        });
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(OathScreen::new(oath.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_screen_password_required() {
        let oath = Some(OathState {
            credentials: vec![],
            password_required: true,
        });
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(OathScreen::new(oath.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn add_account_screen_initial() {
        let mut app = TestApp::new_styled(80, 24, "", || {
            Box::new(AddAccountScreen::new())
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn add_account_screen_step_navigation() {
        use crossterm::event::KeyCode;
        let mut app = TestApp::new_styled(80, 24, "", || {
            Box::new(AddAccountScreen::new())
        });
        let mut pilot = app.pilot();
        // Type issuer and press Enter to advance to step 2
        pilot.press(KeyCode::Char('G')).await;
        pilot.press(KeyCode::Char('i')).await;
        pilot.press(KeyCode::Char('t')).await;
        pilot.press(KeyCode::Enter).await;
        pilot.settle().await;
        drop(pilot);
        // Should now be on step 2 (Account Name)
        insta::assert_display_snapshot!(app.backend());
    }

    // -----------------------------------------------------------------------
    // Phase 09-04: Pilot snapshot tests using mock_yubikey_states
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn oath_default_state() {
        let states = crate::model::mock::mock_yubikey_states();
        let oath_state = states.first().and_then(|yk| yk.oath.clone());
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(OathScreen::new(oath_state.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(stable_snapshot(&app.backend()));
    }

    #[tokio::test]
    async fn oath_no_credentials() {
        let oath_state = Some(OathState {
            credentials: vec![],
            password_required: false,
        });
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(OathScreen::new(oath_state.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_password_protected() {
        let oath_state = Some(OathState {
            credentials: vec![],
            password_required: true,
        });
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(OathScreen::new(oath_state.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_navigate_down() {
        use crossterm::event::KeyCode;
        let states = crate::model::mock::mock_yubikey_states();
        let oath_state = states.first().and_then(|yk| yk.oath.clone());
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(OathScreen::new(oath_state.clone()))
        });
        let mut pilot = app.pilot();
        pilot.settle().await;
        pilot.press(KeyCode::Down).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_snapshot!(stable_snapshot(&app.backend()));
    }
}
