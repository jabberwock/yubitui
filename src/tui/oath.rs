use textual_rs::{Widget, Footer, Header, Label};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::reactive::Reactive;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::oath::{OathState, OathType};

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
                    "OATH application is password-protected. Password unlock not yet implemented.",
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
                    // For now, show a descriptive message via a no-op.
                    let _ = ctx;
                }
            }
            "add_account" => {
                // Add account wizard wired in Plan 03.
                let _ = ctx;
            }
            "delete_account" => {
                // Delete account confirmation wired in Plan 03.
                let _ = ctx;
            }
            "refresh" => {
                // Re-CALCULATE ALL from card; no-op in mock mode.
                // Full wiring in Plan 03.
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
}
