pub mod dashboard;
pub mod diagnostics;
pub mod help;
pub mod keys;
pub mod pin;
pub mod piv;
pub mod ssh;
pub mod widgets;

#[allow(unused_imports)]
pub use keys::{KeyScreen, KeyState};
#[allow(unused_imports)]
pub use pin::{PinScreen, PinState};
#[allow(unused_imports)]
pub use ssh::{SshScreen, SshState};

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::model::Screen;

pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let is_mock = app.is_mock();

    let status_text = match app.current_screen() {
        Screen::Dashboard => "Dashboard",
        Screen::Diagnostics => "Diagnostics",
        Screen::Help => "Help",
        Screen::Keys => "Key Management",
        Screen::PinManagement => "PIN Management",
        Screen::SshWizard => "SSH Setup Wizard",
        Screen::Piv => "PIV Certificates",
    };

    let help_text = match app.current_screen() {
        Screen::Dashboard => "1-5: Switch View | R: Refresh | Q: Quit",
        Screen::Help => "?: Close Help | ESC: Close Help",
        _ => "ESC: Back | R: Refresh | Q: Quit",
    };

    let (status_line, bar_style) = if is_mock {
        let yubikey_label = if let Some(yk) = app.yubikey_state() {
            format!("Mock {} (SN: {})", yk.info.model, yk.info.serial)
        } else {
            "Mock YubiKey 5 NFC (SN: 12345678)".to_string()
        };
        let line = format!(
            "[MOCK] YubiTUI \u{2014} Hardware simulation active | {} | {} | {}",
            yubikey_label, status_text, help_text
        );
        let style = Style::default().bg(Color::Yellow).fg(Color::Black);
        (line, style)
    } else {
        let yubikey_status = if let Some(yk) = app.yubikey_state() {
            format!("\u{1f510} {} (SN: {})", yk.info.model, yk.info.serial)
        } else {
            "\u{274c} No YubiKey detected".to_string()
        };
        let line = format!("{} | {} | {}", status_text, yubikey_status, help_text);
        let style = Style::default().bg(Color::DarkGray).fg(Color::White);
        (line, style)
    };

    let paragraph = Paragraph::new(status_line)
        .style(bar_style)
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(paragraph, area);
}
