use std::cell::{Cell, RefCell};

use textual_rs::{Widget, Footer, Header, Label, Button, ButtonVariant, DataTable, ColumnDef, ProgressBar, Markdown, Vertical, Horizontal};
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
        key: KeyCode::Char('u'),
        modifiers: KeyModifiers::NONE,
        action: "import_uri",
        description: "U Import URI",
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
        key: KeyCode::Char('p'),
        modifiers: KeyModifiers::NONE,
        action: "password_mgmt",
        description: "P Password",
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
                    widgets.push(Box::new(Markdown::new(
                        "## OATH Credentials Not Loaded\n\nPress **R** to load credentials from your YubiKey.",
                    )));
                } else {
                    widgets.push(Box::new(Markdown::new(
                        "## No YubiKey Detected\n\nInsert your YubiKey and press **R** to refresh.",
                    )));
                }
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Button::new("Refresh").with_action("refresh")));
            }
            Some(state) if state.password_required => {
                widgets.push(Box::new(Markdown::new(
                    "## Password Required\n\nThis YubiKey's OATH applet is protected by a password.\n\nPress **P** to enter the password and unlock your credentials.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Button::new("Unlock with Password").with_action("password_mgmt")));
            }
            Some(state) if state.credentials.is_empty() => {
                widgets.push(Box::new(Markdown::new(
                    "## No Accounts Stored\n\nYour YubiKey has no OATH credentials yet.\n\nAdd an account to store TOTP or HOTP codes securely on hardware.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Button::new("Add Account").with_action("add_account")));
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
                        OathType::Totp => "TOTP",
                        OathType::Hotp => "HOTP",
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
                widgets.push(Box::new(Button::new("Add Account").with_action("add_account")));
                widgets.push(Box::new(Button::new("Delete Account").with_variant(ButtonVariant::Warning).with_action("delete_account")));
                widgets.push(Box::new(Button::new("Refresh").with_action("refresh")));
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
            "import_uri" => {
                ctx.push_screen_deferred(Box::new(ImportUriScreen::new()));
            }
            "password_mgmt" => {
                let password_required = self.oath_state.borrow()
                    .as_ref().map(|s| s.password_required).unwrap_or(false);
                if password_required {
                    ctx.push_screen_deferred(Box::new(OathUnlockScreen::new()));
                } else {
                    ctx.push_screen_deferred(Box::new(OathPasswordMgmtScreen::new()));
                }
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
        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
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
            Box::new(Label::new(format!("Step {}/5: {}", step_num, step_name)).with_class("section-title")),
            Box::new(Label::new("")),
        ];

        match state.step {
            AddAccountStep::Issuer => {
                widgets.push(Box::new(Vertical::with_children(vec![
                    Box::new(Label::new("Enter issuer name (e.g., GitHub, Google):")),
                    Box::new(Label::new(format!("> {}_", *input))),
                ]).with_class("status-card")));
            }
            AddAccountStep::AccountName => {
                widgets.push(Box::new(Vertical::with_children(vec![
                    Box::new(Label::new("Enter account name (e.g., user@example.com):")),
                    Box::new(Label::new(format!("> {}_", *input))),
                ]).with_class("status-card")));
            }
            AddAccountStep::Secret => {
                let masked = "●".repeat(input.len());
                widgets.push(Box::new(Vertical::with_children(vec![
                    Box::new(Label::new("Enter Base32 secret key:")),
                    Box::new(Label::new(format!("> {}_", masked))),
                ]).with_class("status-card")));
            }
            AddAccountStep::TypeSelect => {
                let totp_marker = if state.oath_type == OathType::Totp { ">" } else { " " };
                let hotp_marker = if state.oath_type == OathType::Hotp { ">" } else { " " };
                widgets.push(Box::new(Vertical::with_children(vec![
                    Box::new(Label::new("Select type:")),
                    Box::new(Label::new(format!(" {} TOTP (time-based, default)", totp_marker))),
                    Box::new(Label::new(format!(" {} HOTP (counter-based)", hotp_marker))),
                ]).with_class("status-card")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Press T or H to select, Enter to confirm.")));
            }
            AddAccountStep::Confirm => {
                let cred_name = if state.issuer.is_empty() {
                    state.account_name.clone()
                } else {
                    format!("{}:{}", state.issuer, state.account_name)
                };
                widgets.push(Box::new(Vertical::with_children(vec![
                    Box::new(Label::new("Review:")),
                    Box::new(Label::new(format!("  Name:   {}", cred_name))),
                    Box::new(Label::new(format!("  Type:   {}", state.oath_type))),
                    Box::new(Label::new(format!("  Secret: {}", "●".repeat(state.secret_b32.len())))),
                ]).with_class("status-card")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Enter to save, Esc to cancel.")));
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
// ImportUriScreen — paste an otpauth:// URI to skip the 5-step wizard
// ============================================================================

/// Two-step state for ImportUriScreen.
#[derive(Clone, PartialEq)]
enum ImportUriStep {
    /// Step 1: user pastes URI
    Paste,
    /// Step 2: parsed fields shown for confirmation
    Confirm(ParsedOtpAuth),
}

/// Parsed fields from an otpauth:// URI, shown to the user before committing.
#[derive(Clone, PartialEq)]
struct ParsedOtpAuth {
    /// Full credential name in "Issuer:account" form (stored on YubiKey)
    name: String,
    /// Human-readable issuer (may be empty if not in URI)
    issuer: String,
    /// Account portion of the label
    account: String,
    /// Base32-encoded secret (shown masked)
    secret: String,
    /// HMAC algorithm from `algorithm=` param, defaults to SHA-1
    algorithm: OathAlgorithm,
    /// TOTP or HOTP
    oath_type: OathType,
}

/// Two-step import screen.
///
/// Step 1 — Paste: user types/pastes an otpauth:// URI and presses Enter to parse.
/// Step 2 — Confirm: parsed fields (issuer, account, masked secret, algorithm, type)
///   are shown. Enter commits; Esc returns to Step 1 for editing.
pub struct ImportUriScreen {
    input: RefCell<String>,
    step: RefCell<ImportUriStep>,
    error: RefCell<Option<String>>,
    own_id: Cell<Option<textual_rs::WidgetId>>,
}

impl ImportUriScreen {
    pub fn new() -> Self {
        Self {
            input: RefCell::new(String::new()),
            step: RefCell::new(ImportUriStep::Paste),
            error: RefCell::new(None),
            own_id: Cell::new(None),
        }
    }

    /// Advance from Paste → Confirm by parsing the URI, or show an error.
    fn parse_and_preview(&self, ctx: &AppContext) {
        let uri = self.input.borrow().trim().to_string();
        match parse_otpauth_uri(&uri) {
            Err(e) => {
                *self.error.borrow_mut() = Some(e);
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
            Ok(parsed) => {
                *self.error.borrow_mut() = None;
                *self.step.borrow_mut() = ImportUriStep::Confirm(parsed);
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
        }
    }

    /// Commit the parsed credential to the YubiKey.
    fn commit(&self, ctx: &AppContext, parsed: ParsedOtpAuth) {
        match crate::model::oath::put_credential(
            &parsed.name,
            &parsed.secret,
            parsed.oath_type,
            parsed.algorithm,
            6,
        ) {
            Ok(()) => {
                ctx.pop_screen_deferred();
                ctx.push_screen_deferred(Box::new(
                    PopupScreen::new("Imported", format!("Account '{}' added.", parsed.name)),
                ));
            }
            Err(e) => {
                *self.step.borrow_mut() = ImportUriStep::Paste;
                *self.error.borrow_mut() = Some(format!("Add failed: {}", e));
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
        }
    }

    /// Mask a Base32 secret: show first 4 chars then asterisks.
    fn mask_secret(secret: &str) -> String {
        if secret.len() <= 4 {
            return "●".repeat(secret.len());
        }
        format!("{}{}",  &secret[..4], "●".repeat(secret.len() - 4))
    }
}

static IMPORT_URI_PASTE_BINDINGS: &[KeyBinding] = &[
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
        action: "parse",
        description: "Enter Preview",
        show: true,
    },
];

static IMPORT_URI_CONFIRM_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Esc Edit",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "commit",
        description: "Enter Add",
        show: true,
    },
];

impl Widget for ImportUriScreen {
    fn widget_type_name(&self) -> &'static str {
        "ImportUriScreen"
    }

    fn on_mount(&self, id: textual_rs::WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _id: textual_rs::WidgetId) { self.own_id.set(None); }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let step = self.step.borrow().clone();
        let error = self.error.borrow().clone();

        match step {
            ImportUriStep::Paste => {
                let input = self.input.borrow().clone();
                let mut widgets: Vec<Box<dyn Widget>> = vec![
                    Box::new(Header::new("Import OATH URI")),
                    Box::new(Label::new("")),
                    Box::new(Vertical::with_children(vec![
                        Box::new(Label::new("Paste an otpauth:// URI then press Enter.")),
                        Box::new(Label::new("")),
                        Box::new(Label::new("Example: otpauth://totp/GitHub:user?secret=BASE32")),
                        Box::new(Label::new("")),
                        Box::new(Label::new(format!("> {}_", input))),
                    ]).with_class("status-card")),
                ];
                if let Some(err) = error {
                    widgets.push(Box::new(Label::new(format!("Error: {}", err))));
                }
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Footer));
                widgets
            }
            ImportUriStep::Confirm(ref parsed) => {
                let masked = Self::mask_secret(&parsed.secret);
                let issuer_display = if parsed.issuer.is_empty() { "(none)" } else { &parsed.issuer };
                let mut widgets: Vec<Box<dyn Widget>> = vec![
                    Box::new(Header::new("Import OATH URI — Confirm")),
                    Box::new(Label::new("")),
                    Box::new(Vertical::with_children(vec![
                        Box::new(Label::new("Review before adding to YubiKey:").with_class("section-title")),
                        Box::new(Label::new(format!("  Issuer:    {}", issuer_display))),
                        Box::new(Label::new(format!("  Account:   {}", parsed.account))),
                        Box::new(Label::new(format!("  Secret:    {}", masked))),
                        Box::new(Label::new(format!("  Algorithm: {}", parsed.algorithm))),
                        Box::new(Label::new(format!("  Type:      {}", parsed.oath_type))),
                    ]).with_class("status-card")),
                ];
                if let Some(err) = error {
                    widgets.push(Box::new(Label::new(format!("Error: {}", err))));
                }
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Enter to save, Esc to edit.")));
                widgets.push(Box::new(Footer));
                widgets
            }
        }
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        match *self.step.borrow() {
            ImportUriStep::Paste => IMPORT_URI_PASTE_BINDINGS,
            ImportUriStep::Confirm(_) => IMPORT_URI_CONFIRM_BINDINGS,
        }
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            let step = self.step.borrow().clone();
            match step {
                ImportUriStep::Paste => match key.code {
                    KeyCode::Esc => {
                        ctx.pop_screen_deferred();
                        return EventPropagation::Stop;
                    }
                    KeyCode::Enter => {
                        self.parse_and_preview(ctx);
                        return EventPropagation::Stop;
                    }
                    KeyCode::Backspace => {
                        self.input.borrow_mut().pop();
                        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                        return EventPropagation::Stop;
                    }
                    KeyCode::Char(c) => {
                        self.input.borrow_mut().push(c);
                        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                        return EventPropagation::Stop;
                    }
                    _ => {}
                },
                ImportUriStep::Confirm(parsed) => match key.code {
                    KeyCode::Esc => {
                        *self.step.borrow_mut() = ImportUriStep::Paste;
                        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                        return EventPropagation::Stop;
                    }
                    KeyCode::Enter => {
                        self.commit(ctx, parsed);
                        return EventPropagation::Stop;
                    }
                    _ => {}
                },
            }
        }
        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        let step = self.step.borrow().clone();
        match action {
            "cancel" => ctx.pop_screen_deferred(),
            "parse" => self.parse_and_preview(ctx),
            "back" => {
                *self.step.borrow_mut() = ImportUriStep::Paste;
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
            "commit" => {
                if let ImportUriStep::Confirm(parsed) = step {
                    self.commit(ctx, parsed);
                }
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

/// Parse an `otpauth://` URI into a [`ParsedOtpAuth`] struct.
///
/// Supports the standard format used by Google Authenticator, 1Password, etc.:
///   `otpauth://TYPE/LABEL?secret=BASE32&issuer=ISSUER&algorithm=SHA256&...`
///
/// The `issuer` query parameter takes precedence over an issuer prefix in LABEL
/// (standard per RFC 6030 / Google key URI format spec).
/// The `algorithm` parameter is parsed; defaults to SHA-1 if absent.
fn parse_otpauth_uri(uri: &str) -> Result<ParsedOtpAuth, String> {
    let uri = uri.trim();

    // Must start with otpauth://
    let rest = uri
        .strip_prefix("otpauth://")
        .ok_or_else(|| "URI must start with otpauth://".to_string())?;

    // TYPE/LABEL?QUERY
    let (type_and_label, query) = rest.split_once('?').unwrap_or((rest, ""));

    let (type_str, label_encoded) = type_and_label
        .split_once('/')
        .ok_or_else(|| "URI must contain a label after the type".to_string())?;

    let oath_type = match type_str.to_lowercase().as_str() {
        "totp" => OathType::Totp,
        "hotp" => OathType::Hotp,
        other => return Err(format!("Unknown OATH type '{}' (expected totp or hotp)", other)),
    };

    let label = percent_decode(label_encoded);

    // Parse query parameters
    let mut secret: Option<String> = None;
    let mut issuer_param: Option<String> = None;
    let mut algorithm = OathAlgorithm::Sha1;

    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = key.to_lowercase();
            let value = percent_decode(value);
            match key.as_str() {
                "secret" => secret = Some(value.to_uppercase()),
                "issuer" => issuer_param = Some(value),
                "algorithm" => {
                    algorithm = match value.to_uppercase().as_str() {
                        "SHA256" | "SHA-256" => OathAlgorithm::Sha256,
                        "SHA512" | "SHA-512" => OathAlgorithm::Sha512,
                        _ => OathAlgorithm::Sha1,
                    };
                }
                _ => {} // digits, period, counter — accepted and ignored
            }
        }
    }

    let secret = secret.ok_or_else(|| "URI missing 'secret' parameter".to_string())?;
    if secret.is_empty() {
        return Err("Secret key cannot be empty".to_string());
    }

    // Validate secret is Base32 (subset check)
    if secret.chars().any(|c| !matches!(c, 'A'..='Z' | '2'..='7' | '=')) {
        return Err("Secret must be Base32 encoded (A-Z, 2-7, =)".to_string());
    }

    // Split issuer and account
    let (issuer, account) = if let Some(ref iss) = issuer_param {
        // Label may be "Issuer:account" or just "account" when issuer= is present
        let acc = if let Some((_, acc)) = label.split_once(':') {
            acc.trim().to_string()
        } else {
            label.trim().to_string()
        };
        (iss.clone(), acc)
    } else if let Some((iss, acc)) = label.split_once(':') {
        (iss.trim().to_string(), acc.trim().to_string())
    } else {
        (String::new(), label.trim().to_string())
    };

    // Build credential name stored on YubiKey
    let name = if issuer.is_empty() {
        account.clone()
    } else if account.is_empty() {
        issuer.clone()
    } else {
        format!("{}:{}", issuer, account)
    };

    if name.is_empty() {
        return Err("Could not determine credential name from URI".to_string());
    }

    Ok(ParsedOtpAuth { name, issuer, account, secret, algorithm, oath_type })
}

/// Minimal percent-decode: replaces `%XX` sequences with the decoded character.
/// Only decodes printable ASCII; leaves non-ASCII as-is (YubiKey OATH names are ASCII).
fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(h), Some(l)) = (hi, lo) {
                if let (Some(hi_d), Some(lo_d)) = (h.to_digit(16), l.to_digit(16)) {
                    let byte = ((hi_d << 4) | lo_d) as u8;
                    if byte.is_ascii() {
                        out.push(byte as char);
                        continue;
                    }
                }
            }
            // Failed to decode — emit literal %
            out.push('%');
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
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
// OathUnlockScreen — enter password to access a password-protected OATH applet
// ============================================================================

/// Prompts for the OATH application password, validates against the card,
/// and (on success) pops itself and pushes a fresh OathScreen with credentials.
pub struct OathUnlockScreen {
    input: RefCell<String>,
    error: RefCell<Option<String>>,
    own_id: Cell<Option<textual_rs::WidgetId>>,
}

impl OathUnlockScreen {
    pub fn new() -> Self {
        Self {
            input: RefCell::new(String::new()),
            error: RefCell::new(None),
            own_id: Cell::new(None),
        }
    }

    fn attempt_unlock(&self, ctx: &AppContext) {
        let password = self.input.borrow().clone();
        if password.is_empty() {
            *self.error.borrow_mut() = Some("Password cannot be empty".to_string());
            if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            return;
        }

        match crate::model::oath::get_oath_state_with_password(&password) {
            Ok(state) => {
                ctx.pop_screen_deferred();
                ctx.push_screen_deferred(Box::new(OathScreen::new(Some(state))));
            }
            Err(e) => {
                *self.error.borrow_mut() = Some(e.to_string());
                *self.input.borrow_mut() = String::new();
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
        }
    }
}

static OATH_UNLOCK_BINDINGS: &[KeyBinding] = &[
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
        action: "unlock",
        description: "Enter Unlock",
        show: true,
    },
];

impl Widget for OathUnlockScreen {
    fn widget_type_name(&self) -> &'static str { "OathUnlockScreen" }

    fn on_mount(&self, id: textual_rs::WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _: textual_rs::WidgetId) { self.own_id.set(None); }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let masked = "●".repeat(self.input.borrow().len());
        let error = self.error.borrow().clone();
        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("OATH Password")),
            Box::new(Label::new("")),
            Box::new(Vertical::with_children(vec![
                Box::new(Label::new("Enter the OATH application password:")),
                Box::new(Label::new(format!("> {}_", masked))),
            ]).with_class("status-card")),
        ];
        if let Some(err) = error {
            widgets.push(Box::new(Label::new(format!("Error: {}", err))));
        }
        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] { OATH_UNLOCK_BINDINGS }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            match key.code {
                KeyCode::Esc => { ctx.pop_screen_deferred(); return EventPropagation::Stop; }
                KeyCode::Enter => { self.attempt_unlock(ctx); return EventPropagation::Stop; }
                KeyCode::Backspace => {
                    self.input.borrow_mut().pop();
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c) => {
                    self.input.borrow_mut().push(c);
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
            "unlock" => self.attempt_unlock(ctx),
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ============================================================================
// OathPasswordMgmtScreen — set, change, or remove OATH application password
// ============================================================================

/// Password management menu: set, change, or remove the OATH applet password.
pub struct OathPasswordMgmtScreen;

impl OathPasswordMgmtScreen {
    pub fn new() -> Self { Self }
}

static OATH_PASSWD_MENU_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Esc Back",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('s'),
        modifiers: KeyModifiers::NONE,
        action: "set_password",
        description: "S Set password",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('c'),
        modifiers: KeyModifiers::NONE,
        action: "change_password",
        description: "C Change password",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('x'),
        modifiers: KeyModifiers::NONE,
        action: "remove_password",
        description: "X Remove password",
        show: true,
    },
];

impl Widget for OathPasswordMgmtScreen {
    fn widget_type_name(&self) -> &'static str { "OathPasswordMgmtScreen" }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        vec![
            Box::new(Header::new("OATH Password Management")),
            Box::new(Label::new("")),
            Box::new(Label::new("Manage the OATH applet application password.")),
            Box::new(Label::new("")),
            Box::new(Horizontal::with_children(vec![
                Box::new(Button::new("Set New Password").with_action("set_password")),
                Box::new(Button::new("Change Password").with_action("change_password")),
                Box::new(Button::new("Remove Password").with_variant(ButtonVariant::Warning).with_action("remove_password")),
            ]).with_class("button-bar")),
            Box::new(Label::new("")),
            Box::new(Footer),
        ]
    }

    fn key_bindings(&self) -> &[KeyBinding] { OATH_PASSWD_MENU_BINDINGS }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "back" => ctx.pop_screen_deferred(),
            "set_password" => ctx.push_screen_deferred(Box::new(OathSetPasswordScreen::new())),
            "change_password" => ctx.push_screen_deferred(Box::new(OathChangePasswordScreen::new())),
            "remove_password" => ctx.push_screen_deferred(Box::new(OathRemovePasswordScreen::new())),
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ============================================================================
// OathSetPasswordScreen — OATH-08: set password when none is configured
// ============================================================================

/// Two-field form: new password + confirm. Calls `set_oath_password()` on submit.
pub struct OathSetPasswordScreen {
    new_pw: RefCell<String>,
    confirm_pw: RefCell<String>,
    active_field: Cell<u8>, // 0 = new, 1 = confirm
    error: RefCell<Option<String>>,
    own_id: Cell<Option<textual_rs::WidgetId>>,
}

impl OathSetPasswordScreen {
    pub fn new() -> Self {
        Self {
            new_pw: RefCell::new(String::new()),
            confirm_pw: RefCell::new(String::new()),
            active_field: Cell::new(0),
            error: RefCell::new(None),
            own_id: Cell::new(None),
        }
    }

    fn submit(&self, ctx: &AppContext) {
        let new_pw = self.new_pw.borrow().clone();
        let confirm = self.confirm_pw.borrow().clone();
        if new_pw.is_empty() {
            *self.error.borrow_mut() = Some("Password cannot be empty".to_string());
        } else if new_pw != confirm {
            *self.error.borrow_mut() = Some("Passwords do not match".to_string());
            self.confirm_pw.borrow_mut().clear();
            self.active_field.set(1);
        } else {
            match crate::model::oath::set_oath_password(&new_pw) {
                Ok(()) => {
                    ctx.pop_screen_deferred(); // pop OathSetPasswordScreen
                    ctx.pop_screen_deferred(); // pop OathPasswordMgmtScreen
                    ctx.push_screen_deferred(Box::new(
                        PopupScreen::new("Password Set", "OATH application password has been set."),
                    ));
                    return;
                }
                Err(e) => { *self.error.borrow_mut() = Some(e.to_string()); }
            }
        }
        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
    }
}

static OATH_SET_PW_BINDINGS: &[KeyBinding] = &[
    KeyBinding { key: KeyCode::Esc, modifiers: KeyModifiers::NONE, action: "cancel", description: "Esc Cancel", show: true },
    KeyBinding { key: KeyCode::Tab, modifiers: KeyModifiers::NONE, action: "next_field", description: "Tab Next", show: true },
    KeyBinding { key: KeyCode::Enter, modifiers: KeyModifiers::NONE, action: "submit", description: "Enter Submit", show: true },
];

impl Widget for OathSetPasswordScreen {
    fn widget_type_name(&self) -> &'static str { "OathSetPasswordScreen" }
    fn on_mount(&self, id: textual_rs::WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _: textual_rs::WidgetId) { self.own_id.set(None); }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let f = self.active_field.get();
        let m1 = "●".repeat(self.new_pw.borrow().len());
        let m2 = "●".repeat(self.confirm_pw.borrow().len());
        let err = self.error.borrow().clone();
        let c1 = if f == 0 { "_" } else { "" };
        let c2 = if f == 1 { "_" } else { "" };
        let mk1 = if f == 0 { ">" } else { " " };
        let mk2 = if f == 1 { ">" } else { " " };
        let mut w: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("Set OATH Password")),
            Box::new(Label::new("")),
            Box::new(Vertical::with_children(vec![
                Box::new(Label::new(format!(" {} New password:", mk1))),
                Box::new(Label::new(format!("   {}{}", m1, c1))),
                Box::new(Label::new(format!(" {} Confirm password:", mk2))),
                Box::new(Label::new(format!("   {}{}", m2, c2))),
            ]).with_class("status-card")),
        ];
        if let Some(e) = err {
            w.push(Box::new(Label::new("")));
            w.push(Box::new(Label::new(format!("Error: {}", e))));
        }
        w.push(Box::new(Label::new("")));
        w.push(Box::new(Footer));
        w
    }

    fn key_bindings(&self) -> &[KeyBinding] { OATH_SET_PW_BINDINGS }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            match key.code {
                KeyCode::Esc => { ctx.pop_screen_deferred(); return EventPropagation::Stop; }
                KeyCode::Enter | KeyCode::Tab => {
                    let f = self.active_field.get();
                    if key.code == KeyCode::Enter && f == 1 {
                        self.submit(ctx);
                    } else {
                        self.active_field.set(1 - f);
                        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    }
                    return EventPropagation::Stop;
                }
                KeyCode::Backspace => {
                    let f = self.active_field.get();
                    if f == 0 { self.new_pw.borrow_mut().pop(); }
                    else { self.confirm_pw.borrow_mut().pop(); }
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c) => {
                    let f = self.active_field.get();
                    if f == 0 { self.new_pw.borrow_mut().push(c); }
                    else { self.confirm_pw.borrow_mut().push(c); }
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
            "submit" => self.submit(ctx),
            "next_field" => {
                self.active_field.set(1 - self.active_field.get());
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
// OathChangePasswordScreen — OATH-09: change existing password
// ============================================================================

/// Three-field form: current password, new password, confirm.
pub struct OathChangePasswordScreen {
    current_pw: RefCell<String>,
    new_pw: RefCell<String>,
    confirm_pw: RefCell<String>,
    active_field: Cell<u8>, // 0/1/2
    error: RefCell<Option<String>>,
    own_id: Cell<Option<textual_rs::WidgetId>>,
}

impl OathChangePasswordScreen {
    pub fn new() -> Self {
        Self {
            current_pw: RefCell::new(String::new()),
            new_pw: RefCell::new(String::new()),
            confirm_pw: RefCell::new(String::new()),
            active_field: Cell::new(0),
            error: RefCell::new(None),
            own_id: Cell::new(None),
        }
    }

    fn submit(&self, ctx: &AppContext) {
        let current = self.current_pw.borrow().clone();
        let new_pw = self.new_pw.borrow().clone();
        let confirm = self.confirm_pw.borrow().clone();
        if current.is_empty() || new_pw.is_empty() {
            *self.error.borrow_mut() = Some("All fields required".to_string());
        } else if new_pw != confirm {
            *self.error.borrow_mut() = Some("New passwords do not match".to_string());
            self.confirm_pw.borrow_mut().clear();
            self.active_field.set(2);
        } else {
            match crate::model::oath::change_oath_password(&current, &new_pw) {
                Ok(()) => {
                    ctx.pop_screen_deferred();
                    ctx.pop_screen_deferred();
                    ctx.push_screen_deferred(Box::new(
                        PopupScreen::new("Password Changed", "OATH application password has been changed."),
                    ));
                    return;
                }
                Err(e) => {
                    *self.error.borrow_mut() = Some(e.to_string());
                    self.current_pw.borrow_mut().clear();
                    self.active_field.set(0);
                }
            }
        }
        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
    }
}

static OATH_CHANGE_PW_BINDINGS: &[KeyBinding] = &[
    KeyBinding { key: KeyCode::Esc, modifiers: KeyModifiers::NONE, action: "cancel", description: "Esc Cancel", show: true },
    KeyBinding { key: KeyCode::Tab, modifiers: KeyModifiers::NONE, action: "next_field", description: "Tab Next", show: true },
    KeyBinding { key: KeyCode::Enter, modifiers: KeyModifiers::NONE, action: "submit", description: "Enter Submit", show: true },
];

impl Widget for OathChangePasswordScreen {
    fn widget_type_name(&self) -> &'static str { "OathChangePasswordScreen" }
    fn on_mount(&self, id: textual_rs::WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _: textual_rs::WidgetId) { self.own_id.set(None); }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let f = self.active_field.get();
        let labels = ["Current password:", "New password:", "Confirm new password:"];
        let values = [
            "●".repeat(self.current_pw.borrow().len()),
            "●".repeat(self.new_pw.borrow().len()),
            "●".repeat(self.confirm_pw.borrow().len()),
        ];
        let err = self.error.borrow().clone();

        let mut card_lines: Vec<Box<dyn Widget>> = Vec::new();
        for (i, (label, value)) in labels.iter().zip(values.iter()).enumerate() {
            let mk = if i as u8 == f { ">" } else { " " };
            let cur = if i as u8 == f { "_" } else { "" };
            card_lines.push(Box::new(Label::new(format!(" {} {}:", mk, label))));
            card_lines.push(Box::new(Label::new(format!("   {}{}", value, cur))));
        }
        let mut w: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("Change OATH Password")),
            Box::new(Label::new("")),
            Box::new(Vertical::with_children(card_lines).with_class("status-card")),
        ];
        if let Some(e) = err {
            w.push(Box::new(Label::new(format!("Error: {}", e))));
            w.push(Box::new(Label::new("")));
        }
        w.push(Box::new(Footer));
        w
    }

    fn key_bindings(&self) -> &[KeyBinding] { OATH_CHANGE_PW_BINDINGS }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            match key.code {
                KeyCode::Esc => { ctx.pop_screen_deferred(); return EventPropagation::Stop; }
                KeyCode::Enter | KeyCode::Tab => {
                    let f = self.active_field.get();
                    if key.code == KeyCode::Enter && f == 2 {
                        self.submit(ctx);
                    } else if f < 2 {
                        self.active_field.set(f + 1);
                        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    }
                    return EventPropagation::Stop;
                }
                KeyCode::Backspace => {
                    match self.active_field.get() {
                        0 => { self.current_pw.borrow_mut().pop(); }
                        1 => { self.new_pw.borrow_mut().pop(); }
                        _ => { self.confirm_pw.borrow_mut().pop(); }
                    }
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c) => {
                    match self.active_field.get() {
                        0 => { self.current_pw.borrow_mut().push(c); }
                        1 => { self.new_pw.borrow_mut().push(c); }
                        _ => { self.confirm_pw.borrow_mut().push(c); }
                    }
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
            "submit" => self.submit(ctx),
            "next_field" => {
                let f = self.active_field.get();
                if f < 2 { self.active_field.set(f + 1); }
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
// OathRemovePasswordScreen — OATH-10: remove existing password
// ============================================================================

/// Single-field form: current password. Calls `remove_oath_password()` on submit.
pub struct OathRemovePasswordScreen {
    current_pw: RefCell<String>,
    error: RefCell<Option<String>>,
    own_id: Cell<Option<textual_rs::WidgetId>>,
}

impl OathRemovePasswordScreen {
    pub fn new() -> Self {
        Self {
            current_pw: RefCell::new(String::new()),
            error: RefCell::new(None),
            own_id: Cell::new(None),
        }
    }

    fn submit(&self, ctx: &AppContext) {
        let pw = self.current_pw.borrow().clone();
        if pw.is_empty() {
            *self.error.borrow_mut() = Some("Password required".to_string());
            if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            return;
        }
        match crate::model::oath::remove_oath_password(&pw) {
            Ok(()) => {
                ctx.pop_screen_deferred();
                ctx.pop_screen_deferred();
                ctx.push_screen_deferred(Box::new(
                    PopupScreen::new("Password Removed", "OATH application password has been removed."),
                ));
            }
            Err(e) => {
                *self.error.borrow_mut() = Some(e.to_string());
                self.current_pw.borrow_mut().clear();
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
        }
    }
}

static OATH_REMOVE_PW_BINDINGS: &[KeyBinding] = &[
    KeyBinding { key: KeyCode::Esc, modifiers: KeyModifiers::NONE, action: "cancel", description: "Esc Cancel", show: true },
    KeyBinding { key: KeyCode::Enter, modifiers: KeyModifiers::NONE, action: "submit", description: "Enter Remove", show: true },
];

impl Widget for OathRemovePasswordScreen {
    fn widget_type_name(&self) -> &'static str { "OathRemovePasswordScreen" }
    fn on_mount(&self, id: textual_rs::WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _: textual_rs::WidgetId) { self.own_id.set(None); }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let masked = "●".repeat(self.current_pw.borrow().len());
        let err = self.error.borrow().clone();
        let mut w: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("Remove OATH Password")),
            Box::new(Label::new("")),
            Box::new(Vertical::with_children(vec![
                Box::new(Label::new("Enter current password to confirm removal:")),
                Box::new(Label::new(format!("> {}_", masked))),
            ]).with_class("status-card")),
        ];
        if let Some(e) = err {
            w.push(Box::new(Label::new("")));
            w.push(Box::new(Label::new(format!("Error: {}", e))));
        }
        w.push(Box::new(Label::new("")));
        w.push(Box::new(Footer));
        w
    }

    fn key_bindings(&self) -> &[KeyBinding] { OATH_REMOVE_PW_BINDINGS }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            match key.code {
                KeyCode::Esc => { ctx.pop_screen_deferred(); return EventPropagation::Stop; }
                KeyCode::Enter => { self.submit(ctx); return EventPropagation::Stop; }
                KeyCode::Backspace => {
                    self.current_pw.borrow_mut().pop();
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c) => {
                    self.current_pw.borrow_mut().push(c);
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
            "submit" => self.submit(ctx),
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
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(OathScreen::new(oath.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(stable_snapshot(&app.backend()));
    }

    #[tokio::test]
    async fn oath_screen_no_yubikey() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || {
            Box::new(OathScreen::new(None))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_screen_empty_credentials() {
        let oath = Some(OathState {
            credentials: vec![],
            password_required: false,
        });
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(OathScreen::new(oath.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_screen_password_required() {
        let oath = Some(OathState {
            credentials: vec![],
            password_required: true,
        });
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(OathScreen::new(oath.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn add_account_screen_initial() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || {
            Box::new(AddAccountScreen::new())
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn add_account_screen_step_navigation() {
        use crossterm::event::KeyCode;
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || {
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
        insta::assert_snapshot!(app.backend());
    }

    // -----------------------------------------------------------------------
    // Phase 09-04: Pilot snapshot tests using mock_yubikey_states
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn oath_default_state() {
        let states = crate::model::mock::mock_yubikey_states();
        let oath_state = states.first().and_then(|yk| yk.oath.clone());
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
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
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(OathScreen::new(oath_state.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_password_protected() {
        let oath_state = Some(OathState {
            credentials: vec![],
            password_required: true,
        });
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(OathScreen::new(oath_state.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_navigate_down() {
        use crossterm::event::KeyCode;
        let states = crate::model::mock::mock_yubikey_states();
        let oath_state = states.first().and_then(|yk| yk.oath.clone());
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(OathScreen::new(oath_state.clone()))
        });
        let mut pilot = app.pilot();
        pilot.settle().await;
        pilot.press(KeyCode::Down).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_snapshot!(stable_snapshot(&app.backend()));
    }

    #[tokio::test]
    async fn import_uri_screen_initial() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || {
            Box::new(ImportUriScreen::new())
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    // ---- parse_otpauth_uri unit tests ----

    #[test]
    fn parse_uri_totp_full() {
        let uri = "otpauth://totp/GitHub:user%40example.com?secret=JBSWY3DPEHPK3PXP&issuer=GitHub&algorithm=SHA1&digits=6&period=30";
        let p = parse_otpauth_uri(uri).unwrap();
        assert_eq!(p.name, "GitHub:user@example.com");
        assert_eq!(p.secret, "JBSWY3DPEHPK3PXP");
        assert_eq!(p.oath_type, OathType::Totp);
        assert_eq!(p.algorithm, OathAlgorithm::Sha1);
    }

    #[test]
    fn parse_uri_sha256_algorithm() {
        let uri = "otpauth://totp/Acme:alice?secret=JBSWY3DPEHPK3PXP&algorithm=SHA256";
        let p = parse_otpauth_uri(uri).unwrap();
        assert_eq!(p.algorithm, OathAlgorithm::Sha256);
    }

    #[test]
    fn parse_uri_sha512_algorithm() {
        let uri = "otpauth://totp/Acme:alice?secret=JBSWY3DPEHPK3PXP&algorithm=SHA512";
        let p = parse_otpauth_uri(uri).unwrap();
        assert_eq!(p.algorithm, OathAlgorithm::Sha512);
    }

    #[test]
    fn parse_uri_hotp() {
        let uri = "otpauth://hotp/Example?secret=JBSWY3DPEHPK3PXP";
        let p = parse_otpauth_uri(uri).unwrap();
        assert_eq!(p.name, "Example");
        assert_eq!(p.oath_type, OathType::Hotp);
    }

    #[test]
    fn parse_uri_label_only_no_issuer_param() {
        let uri = "otpauth://totp/Acme:alice?secret=JBSWY3DPEHPK3PXP";
        let p = parse_otpauth_uri(uri).unwrap();
        assert_eq!(p.name, "Acme:alice");
        assert_eq!(p.issuer, "Acme");
        assert_eq!(p.account, "alice");
    }

    #[test]
    fn parse_uri_missing_secret() {
        let uri = "otpauth://totp/GitHub?issuer=GitHub";
        assert!(parse_otpauth_uri(uri).is_err());
    }

    #[test]
    fn parse_uri_bad_prefix() {
        let uri = "otpauth2://totp/GitHub?secret=JBSWY3DPEHPK3PXP";
        assert!(parse_otpauth_uri(uri).is_err());
    }

    #[test]
    fn parse_uri_unknown_type() {
        let uri = "otpauth://steam/GitHub?secret=JBSWY3DPEHPK3PXP";
        assert!(parse_otpauth_uri(uri).is_err());
    }

    #[test]
    fn parse_uri_lowercase_secret_uppercased() {
        let uri = "otpauth://totp/X?secret=jbswy3dpehpk3pxp";
        let p = parse_otpauth_uri(uri).unwrap();
        assert_eq!(p.secret, "JBSWY3DPEHPK3PXP");
    }

    #[tokio::test]
    async fn oath_unlock_screen() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || Box::new(OathUnlockScreen::new()));
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_password_mgmt_screen() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || Box::new(OathPasswordMgmtScreen::new()));
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_set_password_screen() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || Box::new(OathSetPasswordScreen::new()));
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_change_password_screen() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || Box::new(OathChangePasswordScreen::new()));
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_remove_password_screen() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || Box::new(OathRemovePasswordScreen::new()));
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }
}
