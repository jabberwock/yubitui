use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::yubikey::YubiKeyState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyScreen {
    Main,
    ViewStatus,
    ImportKey,
    GenerateKey,
    ExportSSH,
}

pub struct KeyState {
    pub screen: KeyScreen,
    pub message: Option<String>,
    pub available_keys: Vec<String>,
}

impl Default for KeyState {
    fn default() -> Self {
        Self {
            screen: KeyScreen::Main,
            message: None,
            available_keys: Vec::new(),
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, yubikey_state: &Option<YubiKeyState>, state: &KeyState) {
    match state.screen {
        KeyScreen::Main => render_main(frame, area, yubikey_state, state),
        KeyScreen::ViewStatus => render_view_status(frame, area, yubikey_state, state),
        KeyScreen::ImportKey => render_import_key(frame, area, state),
        KeyScreen::GenerateKey => render_generate_key(frame, area, state),
        KeyScreen::ExportSSH => render_export_ssh(frame, area, state),
    }
}

fn render_main(frame: &mut Frame, area: Rect, yubikey_state: &Option<YubiKeyState>, state: &KeyState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(12),
        ])
        .split(area);

    let title = Paragraph::new("🔑 Key Management")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let content = if let Some(yk) = yubikey_state {
        let mut lines = vec![];
        
        if let Some(ref openpgp) = yk.openpgp {
            if let Some(ref sig) = openpgp.signature_key {
                lines.push(Line::from(vec![
                    Span::styled("✅ Signature: ", Style::default().fg(Color::Green)),
                    Span::raw(&sig.fingerprint[..16]),
                    Span::raw("..."),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("❌ Signature: ", Style::default().fg(Color::Red)),
                    Span::raw("Not set"),
                ]));
            }
            
            if let Some(ref enc) = openpgp.encryption_key {
                lines.push(Line::from(vec![
                    Span::styled("✅ Encryption: ", Style::default().fg(Color::Green)),
                    Span::raw(&enc.fingerprint[..16]),
                    Span::raw("..."),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("❌ Encryption: ", Style::default().fg(Color::Red)),
                    Span::raw("Not set"),
                ]));
            }
            
            if let Some(ref auth) = openpgp.authentication_key {
                lines.push(Line::from(vec![
                    Span::styled("✅ Authentication: ", Style::default().fg(Color::Green)),
                    Span::raw(&auth.fingerprint[..16]),
                    Span::raw("..."),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("❌ Authentication: ", Style::default().fg(Color::Red)),
                    Span::raw("Not set (required for SSH)"),
                ]));
            }
        }
        
        if let Some(ref msg) = state.message {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Yellow)),
                Span::raw(msg),
            ]));
        }
        
        lines
    } else {
        vec![Line::from("No YubiKey detected. Press 'R' to refresh.")]
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("📊 Keys on Card"));
    frame.render_widget(paragraph, chunks[1]);

    let actions = vec![
        ListItem::new("[V] View full key details").style(Style::default().fg(Color::Cyan)),
        ListItem::new("[I] Import existing key to card").style(Style::default().fg(Color::Green)),
        ListItem::new("[G] Generate new key on card").style(Style::default().fg(Color::Yellow)),
        ListItem::new("[E] Export SSH public key").style(Style::default().fg(Color::Magenta)),
        ListItem::new(""),
        ListItem::new("[ESC] Back to Dashboard"),
    ];

    let action_list = List::new(actions)
        .block(Block::default().title("⌨️  Actions").borders(Borders::ALL));
    frame.render_widget(action_list, chunks[2]);
}

fn render_view_status(frame: &mut Frame, area: Rect, _yubikey_state: &Option<YubiKeyState>, state: &KeyState) {
    render_operation_screen(
        frame,
        area,
        "View Card Status",
        "Launching GPG to show full card status...\n\n\
         This will display all card details including:\n\
         - Key fingerprints\n\
         - Key attributes\n\
         - Cardholder name\n\
         - PIN retry counters\n\n\
         Press ENTER to continue or ESC to cancel.",
        state,
    );
}

fn render_import_key(frame: &mut Frame, area: Rect, state: &KeyState) {
    let mut text = "Import Key to YubiKey\n\n\
         This will launch GPG to import an existing key.\n\n\
         Prerequisites:\n\
         - You must have a GPG key already generated\n\
         - The key must be in your GPG keyring\n\n\
         Available keys:\n".to_string();
    
    if state.available_keys.is_empty() {
        text.push_str("  (Loading keys...)\n");
    } else {
        for key in &state.available_keys {
            text.push_str(&format!("  • {}\n", key));
        }
    }
    
    text.push_str("\nPress ENTER to continue or ESC to cancel.");
    
    render_operation_screen(
        frame,
        area,
        "Import Key",
        &text,
        state,
    );
}

fn render_generate_key(frame: &mut Frame, area: Rect, state: &KeyState) {
    render_operation_screen(
        frame,
        area,
        "Generate Key on Card",
        "Generate a new GPG key directly on your YubiKey.\n\n\
         This will:\n\
         1. Generate a master key and subkeys on the card\n\
         2. Set up signature, encryption, and authentication\n\
         3. The private keys NEVER leave the YubiKey\n\n\
         You will be prompted for:\n\
         - Key type and size\n\
         - Expiration date\n\
         - User ID (name, email)\n\
         - Passphrase\n\n\
         ⚠️  This operation is irreversible.\n\
         Press ENTER to continue or ESC to cancel.",
        state,
    );
}

fn render_export_ssh(frame: &mut Frame, area: Rect, state: &KeyState) {
    render_operation_screen(
        frame,
        area,
        "Export SSH Public Key",
        "Export the authentication key as SSH public key.\n\n\
         This will:\n\
         1. Read the authentication key from your YubiKey\n\
         2. Export it in SSH format\n\
         3. Display it for copying\n\n\
         You can then add this key to:\n\
         - ~/.ssh/authorized_keys on remote servers\n\
         - GitHub/GitLab SSH keys\n\
         - Any SSH server\n\n\
         Press ENTER to continue or ESC to cancel.",
        state,
    );
}

fn render_operation_screen(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    content: &str,
    state: &KeyState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let title_widget = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
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
