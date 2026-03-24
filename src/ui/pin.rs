use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::yubikey::YubiKeyState;

pub fn render(frame: &mut Frame, area: Rect, yubikey_state: &Option<YubiKeyState>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(12)])
        .split(area);

    // Title
    let title = Paragraph::new("🔐 PIN Management")
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
        
        let user_status = if pin.user_pin_blocked {
            ("🔒 BLOCKED", Color::Red)
        } else if pin.user_pin_retries <= 1 {
            ("⚠️  DANGER - 1 retry left!", Color::Yellow)
        } else if pin.user_pin_retries == 2 {
            ("⚠️  Warning - 2 retries left", Color::Yellow)
        } else {
            ("✅ OK", Color::Green)
        };
        
        let admin_status = if pin.admin_pin_blocked {
            ("🔒 BLOCKED - FACTORY RESET REQUIRED", Color::Red)
        } else if pin.admin_pin_retries <= 1 {
            ("⚠️  DANGER - 1 retry left!", Color::Yellow)
        } else if pin.admin_pin_retries == 2 {
            ("⚠️  Warning - 2 retries left", Color::Yellow)
        } else {
            ("✅ OK", Color::Green)
        };

        vec![
            format!("User PIN (for signing/decryption):"),
            format!("  Retries: {}/3", pin.user_pin_retries),
            format!("  Status: {}", user_status.0),
            format!("  Default: 123456 (change immediately!)"),
            format!(""),
            format!("Admin PIN (for card configuration):"),
            format!("  Retries: {}/3", pin.admin_pin_retries),
            format!("  Status: {}", admin_status.0),
            format!("  Default: 12345678 (change immediately!)"),
            format!(""),
            format!("Reset Code (to unblock User PIN):"),
            format!("  Set: {}", if pin.reset_code_retries > 0 { "Yes" } else { "No (recommended to set)" }),
            format!(""),
            format!("⚠️  Security Warning:"),
            format!("• 3 wrong PIN attempts = BLOCKED"),
            format!("• Blocked Admin PIN = Factory reset required (ALL KEYS LOST)"),
            format!("• Always change default PINs before storing keys"),
        ]
    } else {
        vec!["No YubiKey detected. Press 'R' to refresh.".to_string()]
    };

    let text = content.join("\n");
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("📊 Status"))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, chunks[1]);

    // Action menu
    let actions = vec![
        ListItem::new("To change PINs, use GPG command line:").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ListItem::new(""),
        ListItem::new("  $ gpg --card-edit"),
        ListItem::new("  > admin"),
        ListItem::new("  > passwd"),
        ListItem::new("  > 1    # Change User PIN"),
        ListItem::new("  > 3    # Change Admin PIN"),
        ListItem::new("  > q    # Quit"),
        ListItem::new(""),
        ListItem::new("Interactive PIN changes require terminal access."),
        ListItem::new("Copy the commands above and run them in your shell."),
    ];

    let action_list = List::new(actions)
        .block(
            Block::default()
                .title("💡 How to Change PINs")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );

    frame.render_widget(action_list, chunks[2]);
}
