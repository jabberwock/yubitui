use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::yubikey::YubiKeyState;

pub fn render(frame: &mut Frame, area: Rect, yubikey_state: &Option<YubiKeyState>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Title
    let title = Paragraph::new("PIN Management")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Content
    let content = if let Some(yk) = yubikey_state {
        let pin = &yk.pin_status;
        
        format!(
            "PIN Status\n\
             ==========\n\n\
             User PIN:\n\
             • Retries remaining: {} {}\n\
             • Status: {}\n\
             • Default PIN: 123456\n\n\
             Admin PIN:\n\
             • Retries remaining: {} {}\n\
             • Status: {}\n\
             • Default Admin PIN: 12345678\n\n\
             Reset Code:\n\
             • Retries remaining: {}\n\
             • Used to unblock User PIN\n\n\
             ⚠️  Warning: 3 failed attempts will lock the PIN!\n\
             ⚠️  A locked Admin PIN can only be recovered by factory reset!\n\n\
             Actions:\n\
             • C: Change User PIN\n\
             • A: Change Admin PIN\n\
             • R: Set Reset Code\n\
             • U: Unblock User PIN (requires Reset Code or Admin PIN)\n\
             • F: Factory Reset (DESTROYS ALL KEYS)",
            pin.user_pin_retries,
            if pin.user_pin_blocked { "🔒" } else { "" },
            if pin.user_pin_blocked {
                "BLOCKED"
            } else if pin.user_pin_retries <= 1 {
                "⚠️  WARNING - Only 1 retry left!"
            } else {
                "OK"
            },
            pin.admin_pin_retries,
            if pin.admin_pin_blocked { "🔒" } else { "" },
            if pin.admin_pin_blocked {
                "BLOCKED - FACTORY RESET REQUIRED"
            } else if pin.admin_pin_retries <= 1 {
                "⚠️  WARNING - Only 1 retry left!"
            } else {
                "OK"
            },
            pin.reset_code_retries
        )
    } else {
        "No YubiKey detected. Please insert a YubiKey and press 'R' to refresh.".to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, chunks[1]);
}
