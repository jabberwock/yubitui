use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, _app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Title
    let title = Paragraph::new("🔧 SSH Setup Guide")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Instructional content - no "press N" nonsense
    let content = "SSH Authentication with YubiKey - Setup Guide\n\
         =============================================\n\n\
         📋 Prerequisites:\n\
         1. GPG agent must be running (check in System Diagnostics)\n\
         2. Authentication key must be on your YubiKey (check Key Management)\n\
         3. enable-ssh-support must be in ~/.gnupg/gpg-agent.conf\n\n\
         🔧 Configuration Steps:\n\n\
         Step 1: Enable SSH Support\n\
         ---------------------------\n\
         Add this line to ~/.gnupg/gpg-agent.conf:\n\
           enable-ssh-support\n\n\
         Then restart GPG agent:\n\
           $ gpgconf --kill gpg-agent\n\n\
         Step 2: Set SSH_AUTH_SOCK\n\
         --------------------------\n\
         Add to your ~/.zshrc or ~/.bashrc:\n\
           export SSH_AUTH_SOCK=$(gpgconf --list-dirs agent-ssh-socket)\n\n\
         Then reload:\n\
           $ source ~/.zshrc\n\n\
         Step 3: Export Public Key\n\
         --------------------------\n\
         Export your authentication key:\n\
           $ gpg --export-ssh-key YOUR_KEY_ID\n\n\
         Add the output to ~/.ssh/authorized_keys on remote servers.\n\n\
         Step 4: Test Connection\n\
         ------------------------\n\
         Test SSH with your YubiKey:\n\
           $ ssh -v user@host\n\n\
         You should be prompted for your YubiKey PIN.\n\n\
         ✅ Verify: Run System Diagnostics (press '2') to check your configuration.\n\n\
         📖 Full Guide: https://github.com/drduh/YubiKey-Guide#ssh"
            .to_string();

    let paragraph = Paragraph::new(content)
        .block(Block::default()
            .borders(Borders::ALL)
            .title("Copy these commands to your terminal"))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, chunks[1]);
}
