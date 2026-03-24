use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::yubikey::YubiKeyState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PinScreen {
    Main,
    ChangeUserPin,
    ChangeAdminPin,
    SetResetCode,
    UnblockUserPin,
}

pub struct PinState {
    pub screen: PinScreen,
    pub message: Option<String>,
}

impl Default for PinState {
    fn default() -> Self {
        Self {
            screen: PinScreen::Main,
            message: None,
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

    let title = Paragraph::new("🔐 PIN Management")
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
            ("🔒 BLOCKED", Color::Red)
        } else if pin.user_pin_retries <= 1 {
            ("⚠️  DANGER", Color::Yellow)
        } else {
            ("✅ OK", Color::Green)
        };

        let admin_status = if pin.admin_pin_blocked {
            ("🔒 BLOCKED", Color::Red)
        } else if pin.admin_pin_retries <= 1 {
            ("⚠️  DANGER", Color::Yellow)
        } else {
            ("✅ OK", Color::Green)
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
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("📊 Status"));
        frame.render_widget(paragraph, chunks[1]);
    } else {
        let paragraph = Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL).title("📊 Status"));
        frame.render_widget(paragraph, chunks[1]);
    }

    let actions = vec![
        ListItem::new("[C] Change User PIN").style(Style::default().fg(Color::Green)),
        ListItem::new("[A] Change Admin PIN").style(Style::default().fg(Color::Yellow)),
        ListItem::new("[R] Set Reset Code").style(Style::default().fg(Color::Cyan)),
        ListItem::new("[U] Unblock User PIN").style(Style::default().fg(Color::Magenta)),
        ListItem::new(""),
        ListItem::new("[ESC] Back to Dashboard"),
    ];

    let action_list =
        List::new(actions).block(Block::default().title("⌨️  Actions").borders(Borders::ALL));
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
