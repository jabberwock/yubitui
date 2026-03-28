use std::cell::RefCell;

use textual_rs::{Widget, Footer, Header, Label};
use textual_rs::widget::context::AppContext;
use textual_rs::widget::EventPropagation;
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::reactive::Reactive;
use textual_rs::widget::screen::ModalScreen;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::oath::{OathState, OathType, OathAlgorithm};
use crate::tui::widgets::popup::{ConfirmScreen, PopupScreen};

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
    oath_state: Option<OathState>,
    state: Reactive<OathTuiState>,
}

impl OathScreen {
    pub fn new(oath_state: Option<OathState>) -> Self {
        Self {
            oath_state,
            state: Reactive::new(OathTuiState::default()),
        }
    }
}

impl Widget for OathScreen {
    fn widget_type_name(&self) -> &'static str {
        "OathScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("OATH Credentials")),
        ];

        match &self.oath_state {
            None => {
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "No YubiKey detected. Insert your YubiKey and press R to refresh.",
                )));
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
                    "Use 'ykman oath access change' to remove the password, then retry.",
                )));
            }
            Some(state) if state.credentials.is_empty() => {
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "No OATH credentials stored. Press 'a' to add one.",
                )));
            }
            Some(state) => {
                let selected = self.state.get().selected_index;

                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "  Name                              Code      Type  ",
                )));
                widgets.push(Box::new(Label::new(
                    "  ──────────────────────────────  ────────  ──────",
                )));

                for (idx, cred) in state.credentials.iter().enumerate() {
                    let marker = if idx == selected { ">" } else { " " };

                    let display_name = cred.issuer.as_deref().unwrap_or(&cred.name);

                    // Truncate display name to 30 chars for alignment
                    let name_field = if display_name.len() > 30 {
                        format!("{:.30}", display_name)
                    } else {
                        format!("{:<30}", display_name)
                    };

                    let code_field = match &cred.oath_type {
                        OathType::Hotp => {
                            // HOTP: show [press Enter] if no code yet, or the generated code
                            match &cred.code {
                                None => format!("{:>8}", "[Enter]"),
                                Some(c) => format!("{:>8}", c),
                            }
                        }
                        OathType::Totp => {
                            match &cred.code {
                                None => format!("{:>8}", "------"),
                                Some(c) => format!("{:>8}", c),
                            }
                        }
                    };

                    let type_badge = match &cred.oath_type {
                        OathType::Totp => "[TOTP]",
                        OathType::Hotp => "[HOTP]",
                    };

                    // For HOTP with no code, show [press Enter] in code column
                    let display_code = if matches!(cred.oath_type, OathType::Hotp) && cred.code.is_none() {
                        format!("{:>8}", "[press Enter]")
                    } else {
                        code_field
                    };

                    widgets.push(Box::new(Label::new(format!(
                        "  {} {:<30}  {}  {}",
                        marker, name_field, display_code, type_badge
                    ))));
                }

                // Countdown bar for TOTP
                widgets.push(Box::new(Label::new("")));

                let now_secs = chrono::Utc::now().timestamp();
                let secs_remaining = 30 - (now_secs % 30);
                let filled = ((secs_remaining as f32 / 30.0) * 20.0).round() as usize;
                let empty = 20usize.saturating_sub(filled);
                let bar = format!("[{}{}]", "=".repeat(filled), " ".repeat(empty));

                widgets.push(Box::new(Label::new(format!(
                    "  TOTP refreshes in {}s  {}",
                    secs_remaining, bar
                ))));
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
                let current = self.state.get().selected_index;
                if current > 0 {
                    self.state.update(|s| s.selected_index = current - 1);
                }
            }
            "down" => {
                let cred_count = self
                    .oath_state
                    .as_ref()
                    .map(|s| s.credentials.len())
                    .unwrap_or(0);
                if cred_count > 0 {
                    let current = self.state.get().selected_index;
                    if current + 1 < cred_count {
                        self.state.update(|s| s.selected_index = current + 1);
                    }
                }
            }
            "generate_hotp" => {
                // Check if selected credential is HOTP
                let is_hotp = self
                    .oath_state
                    .as_ref()
                    .and_then(|s| s.credentials.get(self.state.get().selected_index))
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
                let selected_idx = self.state.get().selected_index;
                let cred_opt = self
                    .oath_state
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
                // Re-CALCULATE ALL from card; no-op in mock mode.
                let _ = ctx;
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {
        // Rendering handled by compose() — leaf rendering not needed for container screens.
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
}

impl AddAccountScreen {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(AddAccountState::default()),
            input_buffer: RefCell::new(String::new()),
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
                        ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                            PopupScreen::new(
                                "Success",
                                format!("Account '{}' added successfully.", name),
                            ),
                        ))));
                    }
                    Err(e) => {
                        self.state.borrow_mut().error_message = Some(e.to_string());
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
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c)
                    if step == AddAccountStep::Issuer
                        || step == AddAccountStep::AccountName
                        || step == AddAccountStep::Secret =>
                {
                    self.input_buffer.borrow_mut().push(c);
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
            "next_step" => self.advance_step(ctx),
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
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
                        ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                            PopupScreen::new(
                                "Success",
                                format!("'{}' deleted from YubiKey.", self.display_name),
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
            }
            "cancel" => ctx.pop_screen_deferred(),
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
    use crate::model::oath::{OathCredential, OathAlgorithm};

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
        let mut app = TestApp::new(80, 24, move || {
            Box::new(OathScreen::new(oath.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn oath_screen_no_yubikey() {
        let mut app = TestApp::new(80, 24, || {
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
        let mut app = TestApp::new(80, 24, move || {
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
        let mut app = TestApp::new(80, 24, move || {
            Box::new(OathScreen::new(oath.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn add_account_screen_initial() {
        let mut app = TestApp::new(80, 24, || {
            Box::new(AddAccountScreen::new())
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn add_account_screen_step_navigation() {
        use crossterm::event::KeyCode;
        let mut app = TestApp::new(80, 24, || {
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
}
