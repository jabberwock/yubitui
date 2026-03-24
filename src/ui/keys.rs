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
    let title = Paragraph::new("Key Management")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Content
    let content = if let Some(yk) = yubikey_state {
        let mut text = String::new();
        
        text.push_str("OpenPGP Keys\n");
        text.push_str("============\n\n");
        
        if let Some(ref openpgp) = yk.openpgp {
            if let Some(ref sig) = openpgp.signature_key {
                text.push_str(&format!("Signature Key: {}\n", sig.fingerprint));
            } else {
                text.push_str("Signature Key: [not set]\n");
            }
            
            if let Some(ref enc) = openpgp.encryption_key {
                text.push_str(&format!("Encryption Key: {}\n", enc.fingerprint));
            } else {
                text.push_str("Encryption Key: [not set]\n");
            }
            
            if let Some(ref auth) = openpgp.authentication_key {
                text.push_str(&format!("Authentication Key: {}\n", auth.fingerprint));
            } else {
                text.push_str("Authentication Key: [not set]\n");
            }
        } else {
            text.push_str("No OpenPGP card data available.\n");
        }
        
        text.push_str("\n\nPIV Keys\n");
        text.push_str("========\n\n");
        
        if let Some(ref piv) = yk.piv {
            if piv.slots.is_empty() {
                text.push_str("No PIV keys found.\n");
            } else {
                for slot in &piv.slots {
                    text.push_str(&format!("Slot {}: ", slot.slot));
                    if let Some(ref algo) = slot.algorithm {
                        text.push_str(&format!("{} ", algo));
                    }
                    if let Some(ref subj) = slot.subject {
                        text.push_str(&format!("({})", subj));
                    }
                    text.push('\n');
                }
            }
        } else {
            text.push_str("No PIV data available.\n");
        }
        
        text.push_str("\n\nActions:\n");
        text.push_str("• I: Import key\n");
        text.push_str("• G: Generate key on device\n");
        text.push_str("• E: Export public key\n");
        text.push_str("• D: Delete key (requires admin PIN)\n");
        
        text
    } else {
        "No YubiKey detected. Please insert a YubiKey and press 'R' to refresh.".to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, chunks[1]);
}
