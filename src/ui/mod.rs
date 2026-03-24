pub mod dashboard;
pub mod diagnostics;
pub mod keys;
pub mod pin;
pub mod ssh;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, Screen};

pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let status_text = match app.current_screen() {
        Screen::Dashboard => "Dashboard",
        Screen::Diagnostics => "Diagnostics",
        Screen::Keys => "Key Management",
        Screen::PinManagement => "PIN Management",
        Screen::SshWizard => "SSH Setup Wizard",
    };

    let help_text = match app.current_screen() {
        Screen::Dashboard => "1-5: Switch View | R: Refresh | Q: Quit",
        _ => "ESC: Back | R: Refresh | Q: Quit",
    };

    let yubikey_status = if let Some(yk) = app.yubikey_state() {
        format!("🔐 {} (SN: {})", yk.info.model, yk.info.serial)
    } else {
        "❌ No YubiKey detected".to_string()
    };

    let status_line = format!("{} | {} | {}", status_text, yubikey_status, help_text);

    let paragraph = Paragraph::new(status_line)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(paragraph, area);
}
