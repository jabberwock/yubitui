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
    KeyAttributes,   // read-only key algorithm display per slot
    SshPubkeyPopup,  // in-TUI SSH public key viewer
}

pub struct KeyState {
    pub screen: KeyScreen,
    pub message: Option<String>,
    pub available_keys: Vec<String>,
    pub selected_key_index: usize,
    pub key_attributes: Option<crate::yubikey::key_operations::KeyAttributes>,
    pub ssh_pubkey: Option<String>,
    // Reserved for future context menu integration (Plan 02-04)
    #[allow(dead_code)]
    pub show_context_menu: bool,
    #[allow(dead_code)]
    pub menu_selected_index: usize,
}

impl Default for KeyState {
    fn default() -> Self {
        Self {
            screen: KeyScreen::Main,
            message: None,
            available_keys: Vec::new(),
            selected_key_index: 0,
            key_attributes: None,
            ssh_pubkey: None,
            show_context_menu: false,
            menu_selected_index: 0,
        }
    }
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    yubikey_state: &Option<YubiKeyState>,
    state: &KeyState,
) {
    match state.screen {
        KeyScreen::Main => render_main(frame, area, yubikey_state, state),
        KeyScreen::ViewStatus => render_view_status(frame, area, yubikey_state, state),
        KeyScreen::ImportKey => render_import_key(frame, area, state),
        KeyScreen::GenerateKey => render_generate_key(frame, area, state),
        KeyScreen::ExportSSH => render_export_ssh(frame, area, state),
        KeyScreen::KeyAttributes => render_key_attributes(frame, area, state),
        KeyScreen::SshPubkeyPopup => render_ssh_pubkey_popup(frame, area, yubikey_state, state),
    }
}

fn render_main(
    frame: &mut Frame,
    area: Rect,
    yubikey_state: &Option<YubiKeyState>,
    state: &KeyState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(14),
        ])
        .split(area);

    let title = Paragraph::new("🔑 Key Management")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let content = if let Some(yk) = yubikey_state {
        let mut lines = vec![];

        if let Some(ref openpgp) = yk.openpgp {
            if let Some(ref sig) = openpgp.signature_key {
                lines.push(Line::from(vec![
                    Span::styled("✅ Signature: ", Style::default().fg(Color::Green)),
                    Span::raw(sig.fingerprint.get(..16).unwrap_or(&sig.fingerprint).to_string()),
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
                    Span::raw(enc.fingerprint.get(..16).unwrap_or(&enc.fingerprint).to_string()),
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
                    Span::raw(auth.fingerprint.get(..16).unwrap_or(&auth.fingerprint).to_string()),
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

    let paragraph = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title("📊 Keys on Card"),
    );
    frame.render_widget(paragraph, chunks[1]);

    let actions = vec![
        ListItem::new("[V] View full key details").style(Style::default().fg(Color::Cyan)),
        ListItem::new("[I] Import existing key to card").style(Style::default().fg(Color::Green)),
        ListItem::new("[G] Generate new key on card").style(Style::default().fg(Color::Yellow)),
        ListItem::new("[E] Export SSH public key").style(Style::default().fg(Color::Magenta)),
        ListItem::new("[A] View key attributes").style(Style::default().fg(Color::Blue)),
        ListItem::new("[S] Show SSH public key").style(Style::default().fg(Color::White)),
        ListItem::new(""),
        ListItem::new("[ESC] Back to Dashboard"),
    ];

    let action_list =
        List::new(actions).block(Block::default().title("⌨️  Actions").borders(Borders::ALL));
    frame.render_widget(action_list, chunks[2]);
}

fn render_view_status(
    frame: &mut Frame,
    area: Rect,
    _yubikey_state: &Option<YubiKeyState>,
    state: &KeyState,
) {
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(6),
            Constraint::Min(4),
            Constraint::Length(3),
        ])
        .split(area);

    let title_widget = Paragraph::new("Import Key to YubiKey")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title_widget, chunks[0]);

    let intro = Paragraph::new(
        "This will launch GPG to import an existing key to your YubiKey.\n\
         Prerequisites:\n\
         - You must have a GPG key already generated\n\
         - The key must be in your GPG keyring",
    )
    .block(Block::default().borders(Borders::ALL))
    .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(intro, chunks[1]);

    let key_list_block = Block::default()
        .borders(Borders::ALL)
        .title("Available Keys");

    if state.available_keys.is_empty() {
        let empty_msg = Paragraph::new(
            "  No GPG keys found in keyring.\n\
               Generate a key first, or import one with: gpg --import <file>",
        )
        .style(Style::default().fg(Color::Red))
        .block(key_list_block)
        .wrap(ratatui::widgets::Wrap { trim: true });
        frame.render_widget(empty_msg, chunks[2]);
    } else {
        let items: Vec<ListItem> = state
            .available_keys
            .iter()
            .enumerate()
            .map(|(i, key)| {
                if i == state.selected_key_index {
                    ListItem::new(format!("> [{}] {}", i + 1, key)).style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ListItem::new(format!("  [{}] {}", i + 1, key))
                        .style(Style::default().fg(Color::White))
                }
            })
            .collect();

        let key_list = List::new(items).block(key_list_block);
        frame.render_widget(key_list, chunks[2]);
    }

    let mut hint_text = "Use Up/Down to select, Enter to import, Esc to cancel".to_string();
    if let Some(ref msg) = state.message {
        hint_text.push('\n');
        hint_text.push_str(msg);
    }
    let hint = Paragraph::new(hint_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(hint, chunks[3]);
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

fn render_key_attributes(frame: &mut Frame, area: Rect, state: &KeyState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let title_widget = Paragraph::new("Key Attributes")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title_widget, chunks[0]);

    let mut lines: Vec<Line> = Vec::new();

    if let Some(ref attrs) = state.key_attributes {
        // Signature slot
        if let Some(ref slot) = attrs.signature {
            lines.push(Line::from(vec![
                Span::styled("Signature:      ", Style::default().fg(Color::Green)),
                Span::raw(format!(
                    "{} (Fingerprint: {})",
                    slot.algorithm, slot.fingerprint
                )),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Signature:      ", Style::default().fg(Color::DarkGray)),
                Span::styled("[empty]", Style::default().fg(Color::DarkGray)),
            ]));
        }

        // Encryption slot
        if let Some(ref slot) = attrs.encryption {
            lines.push(Line::from(vec![
                Span::styled("Encryption:     ", Style::default().fg(Color::Green)),
                Span::raw(format!(
                    "{} (Fingerprint: {})",
                    slot.algorithm, slot.fingerprint
                )),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Encryption:     ", Style::default().fg(Color::DarkGray)),
                Span::styled("[empty]", Style::default().fg(Color::DarkGray)),
            ]));
        }

        // Authentication slot
        if let Some(ref slot) = attrs.authentication {
            lines.push(Line::from(vec![
                Span::styled("Authentication: ", Style::default().fg(Color::Green)),
                Span::raw(format!(
                    "{} (Fingerprint: {})",
                    slot.algorithm, slot.fingerprint
                )),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Authentication: ", Style::default().fg(Color::DarkGray)),
                Span::styled("[empty]", Style::default().fg(Color::DarkGray)),
            ]));
        }
    } else {
        lines.push(Line::from(vec![Span::styled(
            "Key attributes unavailable. ykman required.",
            Style::default().fg(Color::Yellow),
        )]));
    }

    if let Some(ref msg) = state.message {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            msg.as_str(),
            Style::default().fg(Color::Red),
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "[ESC] Back",
        Style::default().fg(Color::DarkGray),
    )]));

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(paragraph, chunks[1]);
}

fn render_ssh_pubkey_popup(
    frame: &mut Frame,
    area: Rect,
    yubikey_state: &Option<YubiKeyState>,
    state: &KeyState,
) {
    // Render the main screen as background
    render_main(frame, area, yubikey_state, state);

    // Overlay the SSH pubkey popup
    if let Some(ref key) = state.ssh_pubkey {
        let body = format!(
            "{}\n\nAdd this key to:\n  - ~/.ssh/authorized_keys on remote servers\n  - GitHub > Settings > SSH Keys\n  - GitLab > Preferences > SSH Keys\n\nTip: Select and copy with your terminal's copy shortcut.\n\nPress ESC to close.",
            key
        );
        crate::ui::widgets::popup::render_popup(frame, area, "SSH Public Key", &body, 80, 16);
    } else {
        let body = "No authentication key found on card.\nImport or generate a key first.";
        crate::ui::widgets::popup::render_popup(frame, area, "SSH Public Key", body, 60, 8);
    }
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
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
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
