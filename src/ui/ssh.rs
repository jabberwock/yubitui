use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Title
    let title = Paragraph::new("SSH Setup Wizard")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Content
    let content = if let Some(_yk) = app.yubikey_state() {
        "SSH Configuration Wizard\n\
         ========================\n\n\
         This wizard will guide you through configuring SSH authentication with your YubiKey.\n\n\
         Steps:\n\
         1. ✅ Detect YubiKey\n\
         2. ⏳ Check for authentication key\n\
         3. ⏳ Configure gpg-agent for SSH support\n\
         4. ⏳ Export SSH public key\n\
         5. ⏳ Add to authorized_keys\n\n\
         Recommended Configuration:\n\
         • Algorithm: Ed25519 (fastest, most secure)\n\
         • Touch policy: Required (for security)\n\
         • PIN caching: Enabled (for convenience)\n\n\
         Press 'N' to continue to next step...\n\n\
         📖 Need help? See: https://github.com/drduh/YubiKey-Guide"
            .to_string()
    } else {
        "No YubiKey detected. Please insert a YubiKey and press 'R' to refresh.".to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, chunks[1]);
}
