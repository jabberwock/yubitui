use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::tui::widgets::popup;
use crate::tui::widgets::pin_input::{render_pin_input, PinInputState};
use crate::tui::widgets::progress::render_progress_popup;
use crate::model::YubiKeyState;

pub enum PinAction {
    None,
    NavigateTo(crate::model::Screen),
    ExecutePinOperation,
    ExecuteFactoryReset,
}

/// Handle key events for the PIN Management screen.
/// Sub-screen navigation is handled internally. Only hardware calls are returned as actions.
pub fn handle_key(
    state: &mut PinState,
    key: KeyEvent,
    yubikey_state: Option<&YubiKeyState>,
) -> PinAction {
    match state.screen {
        PinScreen::Main => match key.code {
            KeyCode::Char('c') => {
                state.pending_operation = Some(PinScreen::ChangeUserPin);
                state.pin_input = Some(PinInputState::new(
                    "Change User PIN",
                    &["Current PIN", "New PIN", "Confirm New PIN"],
                ));
                state.screen = PinScreen::PinInputActive;
                PinAction::None
            }
            KeyCode::Char('a') => {
                state.pending_operation = Some(PinScreen::ChangeAdminPin);
                state.pin_input = Some(PinInputState::new(
                    "Change Admin PIN",
                    &[
                        "Current Admin PIN",
                        "New Admin PIN",
                        "Confirm New Admin PIN",
                    ],
                ));
                state.screen = PinScreen::PinInputActive;
                PinAction::None
            }
            KeyCode::Char('r') => {
                state.pending_operation = Some(PinScreen::SetResetCode);
                state.pin_input = Some(PinInputState::new(
                    "Set Reset Code",
                    &["Admin PIN", "New Reset Code", "Confirm Reset Code"],
                ));
                state.screen = PinScreen::PinInputActive;
                PinAction::None
            }
            KeyCode::Char('u') => {
                state.screen = PinScreen::UnblockWizardCheck;
                PinAction::None
            }
            KeyCode::Esc => PinAction::NavigateTo(crate::model::Screen::Dashboard),
            _ => PinAction::None,
        },
        PinScreen::PinInputActive => {
            use crate::tui::widgets::pin_input::PinInputAction;
            let action = if let Some(pin_input) = state.pin_input.as_mut() {
                pin_input.handle_key(key.code)
            } else {
                PinInputAction::Cancel
            };
            match action {
                PinInputAction::Submit => PinAction::ExecutePinOperation,
                PinInputAction::Cancel => {
                    state.pin_input = None;
                    state.pending_operation = None;
                    state.screen = PinScreen::Main;
                    state.message = None;
                    PinAction::None
                }
                PinInputAction::Continue => PinAction::None,
            }
        }
        PinScreen::OperationResult => {
            state.screen = PinScreen::Main;
            state.pin_input = None;
            state.pending_operation = None;
            state.operation_running = false;
            state.operation_status = None;
            PinAction::None
        }
        PinScreen::UnblockWizardCheck => match key.code {
            KeyCode::Char('1') => {
                if let Some(yk) = yubikey_state {
                    if yk.pin_status.reset_code_retries > 0 {
                        state.screen = PinScreen::UnblockWizardWithReset;
                        state.unblock_path = Some(UnblockPath::ResetCode);
                    }
                }
                PinAction::None
            }
            KeyCode::Char('2') => {
                if let Some(yk) = yubikey_state {
                    if yk.pin_status.admin_pin_retries > 0 {
                        state.screen = PinScreen::UnblockWizardWithAdmin;
                        state.unblock_path = Some(UnblockPath::AdminPin);
                    }
                }
                PinAction::None
            }
            KeyCode::Char('3') => {
                if let Some(yk) = yubikey_state {
                    if yk.pin_status.admin_pin_retries == 0 {
                        state.screen = PinScreen::UnblockWizardFactoryReset;
                        state.unblock_path = Some(UnblockPath::FactoryReset);
                    }
                }
                PinAction::None
            }
            KeyCode::Esc => {
                state.screen = PinScreen::Main;
                state.message = None;
                state.unblock_path = None;
                PinAction::None
            }
            _ => PinAction::None,
        },
        PinScreen::UnblockWizardWithReset => match key.code {
            KeyCode::Enter => {
                state.pending_operation = Some(PinScreen::UnblockWizardWithReset);
                state.pin_input = Some(PinInputState::new(
                    "Unblock with Reset Code",
                    &["Reset Code", "New User PIN", "Confirm New PIN"],
                ));
                state.screen = PinScreen::PinInputActive;
                PinAction::None
            }
            KeyCode::Esc => {
                state.screen = PinScreen::UnblockWizardCheck;
                state.unblock_path = None;
                PinAction::None
            }
            _ => PinAction::None,
        },
        PinScreen::UnblockWizardWithAdmin => match key.code {
            KeyCode::Enter => {
                state.pending_operation = Some(PinScreen::UnblockWizardWithAdmin);
                state.pin_input = Some(PinInputState::new(
                    "Unblock with Admin PIN",
                    &["Admin PIN", "New User PIN", "Confirm New PIN"],
                ));
                state.screen = PinScreen::PinInputActive;
                PinAction::None
            }
            KeyCode::Esc => {
                state.screen = PinScreen::UnblockWizardCheck;
                state.unblock_path = None;
                PinAction::None
            }
            _ => PinAction::None,
        },
        PinScreen::UnblockWizardFactoryReset => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if state.confirm_factory_reset {
                    PinAction::ExecuteFactoryReset
                } else {
                    state.confirm_factory_reset = true;
                    PinAction::None
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if state.confirm_factory_reset {
                    state.confirm_factory_reset = false;
                }
                PinAction::None
            }
            KeyCode::Esc => {
                state.confirm_factory_reset = false;
                state.screen = PinScreen::UnblockWizardCheck;
                state.unblock_path = None;
                PinAction::None
            }
            _ => PinAction::None,
        },
        _ => {
            if key.code == KeyCode::Esc {
                state.screen = PinScreen::Main;
                state.message = None;
            }
            PinAction::None
        }
    }
}

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
    // factory reset is implemented natively via PC/SC — no external tool needed
    /// Active TUI PIN input form; Some when screen == PinInputActive.
    pub pin_input: Option<PinInputState>,
    /// True while the gpg subprocess is executing.
    pub operation_running: bool,
    /// Human-readable status message shown during OperationRunning.
    pub operation_status: Option<String>,
    /// Spinner tick counter; incremented each render frame.
    pub progress_tick: usize,
    /// Which PIN operation triggered the PinInputActive screen.
    /// Stored as the PinScreen variant that initiated the flow.
    pub pending_operation: Option<PinScreen>,
}

impl Default for PinState {
    fn default() -> Self {
        Self {
            screen: PinScreen::Main,
            message: None,
            unblock_path: None,
            confirm_factory_reset: false,
            pin_input: None,
            operation_running: false,
            operation_status: None,
            progress_tick: 0,
            pending_operation: None,
        }
    }
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    yubikey_state: &Option<YubiKeyState>,
    state: &PinState,
) {
    match state.screen {
        PinScreen::Main => render_main(frame, area, yubikey_state, state),
        PinScreen::ChangeUserPin => render_change_user_pin(frame, area, state),
        PinScreen::ChangeAdminPin => render_change_admin_pin(frame, area, state),
        PinScreen::SetResetCode => render_set_reset_code(frame, area, state),
        PinScreen::UnblockUserPin => render_unblock_user_pin(frame, area, state),
        PinScreen::UnblockWizardCheck => {
            render_unblock_wizard_check(frame, area, yubikey_state, state)
        }
        PinScreen::UnblockWizardWithReset => {
            render_unblock_wizard_with_reset(frame, area, yubikey_state, state)
        }
        PinScreen::UnblockWizardWithAdmin => {
            render_unblock_wizard_with_admin(frame, area, yubikey_state, state)
        }
        PinScreen::UnblockWizardFactoryReset => {
            render_unblock_wizard_factory_reset(frame, area, state)
        }
        PinScreen::PinInputActive => {
            if let Some(pin_input) = &state.pin_input {
                render_pin_input(frame, area, pin_input);
            }
        }
        PinScreen::OperationRunning => {
            render_main(frame, area, yubikey_state, state);
            render_progress_popup(
                frame,
                area,
                "PIN Operation",
                state
                    .operation_status
                    .as_deref()
                    .unwrap_or("Working..."),
                state.progress_tick,
            );
        }
        PinScreen::OperationResult => {
            render_main(frame, area, yubikey_state, state);
            if let Some(msg) = &state.message {
                popup::render_popup(frame, area, "Result", msg, 60, 10);
            }
        }
    }
}

fn render_main(
    frame: &mut Frame,
    area: Rect,
    yubikey_state: &Option<YubiKeyState>,
    state: &PinState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(10),
        ])
        .split(area);

    let title = Paragraph::new("PIN Management")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let content = if let Some(yk) = yubikey_state {
        let pin = &yk.pin_status;

        let user_status = if pin.user_pin_blocked {
            ("BLOCKED", Color::Red)
        } else if pin.user_pin_retries <= 1 {
            ("DANGER", Color::Yellow)
        } else {
            ("OK", Color::Green)
        };

        let admin_status = if pin.admin_pin_blocked {
            ("BLOCKED", Color::Red)
        } else if pin.admin_pin_retries <= 1 {
            ("DANGER", Color::Yellow)
        } else {
            ("OK", Color::Green)
        };

        vec![
            Line::from(vec![
                Span::raw("User PIN: "),
                Span::styled(
                    format!("{}/3 retries", pin.user_pin_retries),
                    Style::default().fg(user_status.1),
                ),
                Span::raw(" "),
                Span::styled(user_status.0, Style::default().fg(user_status.1)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Admin PIN: "),
                Span::styled(
                    format!("{}/3 retries", pin.admin_pin_retries),
                    Style::default().fg(admin_status.1),
                ),
                Span::raw(" "),
                Span::styled(admin_status.0, Style::default().fg(admin_status.1)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Reset Code: "),
                Span::raw(if pin.reset_code_retries > 0 {
                    "Set"
                } else {
                    "Not set"
                }),
            ]),
        ]
    } else {
        vec![Line::from("No YubiKey detected. Press 'R' to refresh.")]
    };

    if let Some(ref msg) = state.message {
        let mut lines = content;
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Yellow)),
            Span::raw(msg),
        ]));
        let paragraph =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(paragraph, chunks[1]);
    } else {
        let paragraph =
            Paragraph::new(content).block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(paragraph, chunks[1]);
    }

    let actions = vec![
        ListItem::new("[C] Change User PIN").style(Style::default().fg(Color::Green)),
        ListItem::new("[A] Change Admin PIN").style(Style::default().fg(Color::Yellow)),
        ListItem::new("[R] Set Reset Code").style(Style::default().fg(Color::Cyan)),
        ListItem::new("[U] Unblock User PIN (Wizard)").style(Style::default().fg(Color::Magenta)),
        ListItem::new(""),
        ListItem::new("[ESC] Back to Dashboard"),
    ];

    let action_list =
        List::new(actions).block(Block::default().title("Actions").borders(Borders::ALL));
    frame.render_widget(action_list, chunks[2]);
}

fn render_change_user_pin(frame: &mut Frame, area: Rect, state: &PinState) {
    render_operation_screen(
        frame,
        area,
        "Change User PIN",
        "Launching GPG to change User PIN...\n\n\
         You will be prompted to:\n\
         1. Enter current User PIN\n\
         2. Enter new User PIN\n\
         3. Confirm new User PIN\n\n\
         Press ENTER to continue or ESC to cancel.",
        state,
    );
}

fn render_change_admin_pin(frame: &mut Frame, area: Rect, state: &PinState) {
    render_operation_screen(
        frame,
        area,
        "Change Admin PIN",
        "Launching GPG to change Admin PIN...\n\n\
         You will be prompted to:\n\
         1. Enter current Admin PIN\n\
         2. Enter new Admin PIN\n\
         3. Confirm new Admin PIN\n\n\
         Press ENTER to continue or ESC to cancel.",
        state,
    );
}

fn render_set_reset_code(frame: &mut Frame, area: Rect, state: &PinState) {
    render_operation_screen(
        frame,
        area,
        "Set Reset Code",
        "Launching GPG to set Reset Code...\n\n\
         The Reset Code allows you to unblock the User PIN\n\
         without using the Admin PIN.\n\n\
         Press ENTER to continue or ESC to cancel.",
        state,
    );
}

fn render_unblock_user_pin(frame: &mut Frame, area: Rect, state: &PinState) {
    render_operation_screen(
        frame,
        area,
        "Unblock User PIN",
        "Launching GPG to unblock User PIN...\n\n\
         You will need either:\n\
         - Reset Code (if set), OR\n\
         - Admin PIN\n\n\
         Press ENTER to continue or ESC to cancel.",
        state,
    );
}

fn render_unblock_wizard_check(
    frame: &mut Frame,
    area: Rect,
    yubikey_state: &Option<YubiKeyState>,
    _state: &PinState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(8),
        ])
        .split(area);

    // Title
    let title = Paragraph::new("PIN Unblock Wizard")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Status section
    let status_lines = if let Some(yk) = yubikey_state {
        let pin = &yk.pin_status;

        let user_color = if pin.user_pin_retries > 1 {
            Color::Green
        } else if pin.user_pin_retries == 1 {
            Color::Yellow
        } else {
            Color::Red
        };
        let admin_color = if pin.admin_pin_retries > 1 {
            Color::Green
        } else if pin.admin_pin_retries == 1 {
            Color::Yellow
        } else {
            Color::Red
        };
        let reset_color = if pin.reset_code_retries > 1 {
            Color::Green
        } else if pin.reset_code_retries == 1 {
            Color::Yellow
        } else {
            Color::Red
        };

        vec![
            Line::from("Current PIN status:"),
            Line::from(""),
            Line::from(vec![
                Span::raw("  User PIN retries:   "),
                Span::styled(
                    format!("{}/3", pin.user_pin_retries),
                    Style::default().fg(user_color),
                ),
            ]),
            Line::from(vec![
                Span::raw("  Admin PIN retries:  "),
                Span::styled(
                    format!("{}/3", pin.admin_pin_retries),
                    Style::default().fg(admin_color),
                ),
            ]),
            Line::from(vec![
                Span::raw("  Reset Code retries: "),
                Span::styled(
                    format!("{}/3", pin.reset_code_retries),
                    Style::default().fg(reset_color),
                ),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled(
            "No YubiKey detected.",
            Style::default().fg(Color::Red),
        ))]
    };

    let status_paragraph =
        Paragraph::new(status_lines).block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(status_paragraph, chunks[1]);

    // Actions section
    let action_lines = if let Some(yk) = yubikey_state {
        let pin = &yk.pin_status;
        let mut lines: Vec<Line> = Vec::new();

        if pin.reset_code_retries > 0 {
            lines.push(Line::from(Span::styled(
                "[1] Unblock with Reset Code (recommended)",
                Style::default().fg(Color::Green),
            )));
        }
        if pin.admin_pin_retries > 0 {
            lines.push(Line::from(Span::styled(
                "[2] Unblock with Admin PIN",
                Style::default().fg(Color::Yellow),
            )));
        }
        // Factory reset is the only way to recover a blocked Admin PIN.
        // Offer it whenever admin is blocked, even if the reset code is still available.
        if pin.admin_pin_retries == 0 {
            if pin.reset_code_retries == 0 {
                lines.push(Line::from(Span::styled(
                    "No recovery paths available — only factory reset remains.",
                    Style::default().fg(Color::Red),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "Admin PIN is blocked — cannot be unblocked without factory reset.",
                    Style::default().fg(Color::Red),
                )));
            }
            lines.push(Line::from(Span::styled(
                "[3] Factory Reset (DESTROYS ALL KEYS)",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from("[ESC] Cancel"));
        lines
    } else {
        vec![Line::from("[ESC] Cancel")]
    };

    let actions_paragraph = Paragraph::new(action_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Recovery Options"),
    );
    frame.render_widget(actions_paragraph, chunks[2]);
}

fn render_unblock_wizard_with_reset(
    frame: &mut Frame,
    area: Rect,
    yubikey_state: &Option<YubiKeyState>,
    state: &PinState,
) {
    let retries = yubikey_state
        .as_ref()
        .map(|yk| yk.pin_status.reset_code_retries)
        .unwrap_or(0);

    let body = format!(
        "Your Reset Code has {retries} retries remaining.\n\n\
         Press ENTER to enter PINs in-TUI, or ESC to go back."
    );

    render_operation_screen(frame, area, "Unblock with Reset Code", &body, state);
}

fn render_unblock_wizard_with_admin(
    frame: &mut Frame,
    area: Rect,
    yubikey_state: &Option<YubiKeyState>,
    state: &PinState,
) {
    let retries = yubikey_state
        .as_ref()
        .map(|yk| yk.pin_status.admin_pin_retries)
        .unwrap_or(0);

    let body = format!(
        "Your Admin PIN has {retries} retries remaining.\n\n\
         Press ENTER to enter PINs in-TUI, or ESC to go back."
    );

    render_operation_screen(frame, area, "Unblock with Admin PIN", &body, state);
}

fn render_unblock_wizard_factory_reset(frame: &mut Frame, area: Rect, state: &PinState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Title in RED BOLD
    let title = Paragraph::new("Factory Reset")
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let warning_lines = vec![
        Line::from(Span::styled(
            "[WARNING] DESTRUCTIVE OPERATION",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Both your Admin PIN and Reset Code are exhausted."),
        Line::from("The only way to recover this YubiKey is a full factory reset."),
        Line::from(""),
        Line::from(Span::styled(
            "THIS WILL PERMANENTLY DELETE:",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  - All GPG keys stored on the card",
            Style::default().fg(Color::Red),
        )),
        Line::from(Span::styled(
            "  - All certificates",
            Style::default().fg(Color::Red),
        )),
        Line::from(Span::styled(
            "  - All cardholder data",
            Style::default().fg(Color::Red),
        )),
        Line::from(""),
        Line::from("After reset, default PINs will be restored:"),
        Line::from("  - User PIN:  123456"),
        Line::from("  - Admin PIN: 12345678"),
        Line::from(""),
        Line::from("Press [Y] to confirm factory reset"),
        Line::from("Press [ESC] to cancel"),
    ];

    let content_paragraph = Paragraph::new(warning_lines)
        .block(Block::default().borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(content_paragraph, chunks[1]);

    // Overlay confirm dialog if needed
    if state.confirm_factory_reset {
        popup::render_confirm_dialog(
            frame,
            area,
            "Confirm Factory Reset",
            "Are you ABSOLUTELY sure? This cannot be undone.",
            true,
        );
    }
}

fn render_operation_screen(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    content: &str,
    state: &PinState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let title_widget = Paragraph::new(title)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title_widget, chunks[0]);

    let mut text = content.to_string();
    if let Some(ref msg) = state.message {
        text.push_str("\n\n");
        text.push_str(msg);
    }

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(paragraph, chunks[1]);
}
