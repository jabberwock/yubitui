use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(8),
        ])
        .split(area);

    // Title
    let title = Paragraph::new("🔐 YubiTUI - YubiKey Management")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Main menu
    let menu_items = vec![
        ListItem::new("1. Dashboard (Current)"),
        ListItem::new("2. System Diagnostics"),
        ListItem::new("3. Key Management"),
        ListItem::new("4. PIN Management"),
        ListItem::new("5. SSH Setup Wizard"),
    ];

    let menu = List::new(menu_items)
        .block(
            Block::default()
                .title("Main Menu")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(menu, chunks[1]);

    // Quick status
    let status_text = if let Some(yk) = app.yubikey_state() {
        let pin_status = &yk.pin_status;
        let pin_emoji = if pin_status.is_healthy() {
            "✅"
        } else if pin_status.needs_attention() {
            "⚠️"
        } else {
            "❌"
        };

        format!(
            "YubiKey: {} {}\n\
             Firmware: {}\n\
             Serial: {}\n\
             \n\
             {} PIN Status: {} retries remaining\n\
             Admin PIN: {} retries remaining",
            yk.info.model,
            yk.info.form_factor,
            yk.info.version,
            yk.info.serial,
            pin_emoji,
            pin_status.user_pin_retries,
            pin_status.admin_pin_retries
        )
    } else {
        "No YubiKey detected.\n\n\
         Please insert a YubiKey and press 'R' to refresh.\n\n\
         Troubleshooting:\n\
         • Ensure pcscd is running\n\
         • Check USB connection\n\
         • Try: pcsc_scan"
            .to_string()
    };

    let status = Paragraph::new(status_text)
        .block(
            Block::default()
                .title("Quick Status")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(status, chunks[2]);
}
