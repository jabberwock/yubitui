use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::yubikey::YubiKeyState;

pub fn render(frame: &mut Frame, area: Rect, yubikey_state: &Option<YubiKeyState>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), 
            Constraint::Min(10),
            Constraint::Length(14),
        ])
        .split(area);

    // Title
    let title = Paragraph::new("🔑 Key Management")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Content
    let content = if let Some(yk) = yubikey_state {
        let mut lines = vec![];
        
        lines.push("OpenPGP Keys".to_string());
        lines.push("============".to_string());
        lines.push("".to_string());
        
        if let Some(ref openpgp) = yk.openpgp {
            // Signature key
            if let Some(ref sig) = openpgp.signature_key {
                lines.push(format!("✅ Signature Key:"));
                lines.push(format!("   {}", sig.fingerprint));
            } else {
                lines.push(format!("❌ Signature Key: Not set"));
            }
            lines.push("".to_string());
            
            // Encryption key
            if let Some(ref enc) = openpgp.encryption_key {
                lines.push(format!("✅ Encryption Key:"));
                lines.push(format!("   {}", enc.fingerprint));
            } else {
                lines.push(format!("❌ Encryption Key: Not set"));
            }
            lines.push("".to_string());
            
            // Authentication key  
            if let Some(ref auth) = openpgp.authentication_key {
                lines.push(format!("✅ Authentication Key (for SSH):"));
                lines.push(format!("   {}", auth.fingerprint));
            } else {
                lines.push(format!("❌ Authentication Key: Not set"));
                lines.push(format!("   (Required for SSH authentication)"));
            }
            lines.push("".to_string());
        } else {
            lines.push("No OpenPGP data available.".to_string());
            lines.push("".to_string());
        }
        
        lines.push("".to_string());
        lines.push("PIV Keys (Smart Card)".to_string());
        lines.push("=====================".to_string());
        lines.push("".to_string());
        
        if let Some(ref piv) = yk.piv {
            if piv.slots.is_empty() {
                lines.push("No PIV keys found.".to_string());
                lines.push("(PIV requires ykman tool)".to_string());
            } else {
                for slot in &piv.slots {
                    lines.push(format!("Slot {}: {:?}", slot.slot, slot.algorithm));
                }
            }
        } else {
            lines.push("PIV data not available.".to_string());
        }
        
        lines.join("\n")
    } else {
        "No YubiKey detected. Press 'R' to refresh.".to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("📊 Current Keys"))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, chunks[1]);
    
    // Action instructions
    let actions = vec![
        ListItem::new("To manage keys, use GPG command line:").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ListItem::new(""),
        ListItem::new("View keys on card:"),
        ListItem::new("  $ gpg --card-status"),
        ListItem::new(""),
        ListItem::new("Import existing key to YubiKey:"),
        ListItem::new("  $ gpg --edit-key YOUR_KEY_ID"),
        ListItem::new("  > keytocard"),
        ListItem::new(""),
        ListItem::new("Generate new key on card:"),
        ListItem::new("  $ gpg --card-edit"),
        ListItem::new("  > admin"),
        ListItem::new("  > generate"),
    ];

    let action_list = List::new(actions)
        .block(
            Block::default()
                .title("💡 Key Management Commands")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );

    frame.render_widget(action_list, chunks[2]);
}
