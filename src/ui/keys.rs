use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::yubikey::YubiKeyState;

/// Steps in the key generation wizard (per D-09).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyGenStep {
    Algorithm, // (1) algorithm selection
    Expiry,    // (2) expiry selection
    Identity,  // (3) name + email fields
    Backup,    // (4) backup yes/no + path
    Confirm,   // (5) summary + admin PIN entry
    Running,   // (6) operation in progress
    Result,    // (7) result display
}

/// Wizard state for key generation (per D-09).
pub struct KeyGenWizard {
    pub step: KeyGenStep,
    pub algorithm_index: usize, // 0=Ed25519, 1=RSA2048, 2=RSA4096
    pub expiry_index: usize,    // 0=None, 1=1yr, 2=2yr, 3=Custom
    pub custom_expiry: String,  // for custom date input
    pub name: String,
    pub email: String,
    pub backup: bool,
    pub backup_path: String,
    pub active_field: usize,         // for identity step (0=name, 1=email)
    pub editing_path: bool,          // true when editing backup path
    pub editing_custom_expiry: bool, // true when editing custom expiry date
}

impl KeyGenWizard {
    pub fn new(default_backup_date: &str) -> Self {
        Self {
            step: KeyGenStep::Algorithm,
            algorithm_index: 0,
            expiry_index: 0,
            custom_expiry: String::new(),
            name: String::new(),
            email: String::new(),
            backup: false,
            backup_path: format!("~/yubikey-backup-{}.gpg", default_backup_date),
            active_field: 0,
            editing_path: false,
            editing_custom_expiry: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyScreen {
    Main,
    ViewStatus,
    ImportKey, // legacy: shows key list for selection (pre-wizard)
    ExportSSH,
    KeyAttributes,          // read-only key algorithm display per slot
    SshPubkeyPopup,         // in-TUI SSH public key viewer
    SetTouchPolicy,         // slot selection
    SetTouchPolicySelect,   // policy selection
    SetTouchPolicyConfirm,  // irreversibility confirmation
    SetTouchPolicyPinInput, // collecting admin PIN for touch policy set
    KeyGenWizardActive,     // wizard is driving the UI
    KeyImportRunning,       // import operation in progress
    KeyImportPinInput,      // collecting admin PIN for import
    KeyOperationResult,     // showing result after keygen or import
}

pub struct KeyState {
    pub screen: KeyScreen,
    pub message: Option<String>,
    pub available_keys: Vec<String>,
    pub selected_key_index: usize,
    pub key_attributes: Option<crate::yubikey::key_operations::KeyAttributes>,
    pub ssh_pubkey: Option<String>,
    pub touch_slot_index: usize,           // 0=sig, 1=enc, 2=aut, 3=att
    pub touch_policy_index: usize,         // 0=Off, 1=On, 2=Fixed, 3=Cached, 4=CachedFixed
    pub attestation_popup: Option<String>, // PEM content for popup display
    // Key generation wizard state
    pub keygen_wizard: Option<KeyGenWizard>,
    pub pin_input: Option<crate::ui::widgets::pin_input::PinInputState>,
    pub operation_status: Option<String>,
    pub progress_tick: usize,
    pub import_result: Option<String>, // formatted SIG/ENC/AUT result
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
            touch_slot_index: 0,
            touch_policy_index: 0,
            attestation_popup: None,
            keygen_wizard: None,
            pin_input: None,
            operation_status: None,
            progress_tick: 0,
            import_result: None,
        }
    }
}

pub fn touch_slot_name(index: usize) -> &'static str {
    match index {
        0 => "sig",
        1 => "enc",
        2 => "aut",
        3 => "att",
        _ => "sig",
    }
}

pub fn touch_slot_display(index: usize) -> &'static str {
    match index {
        0 => "Signature",
        1 => "Encryption",
        2 => "Authentication",
        3 => "Attestation",
        _ => "Signature",
    }
}

pub fn touch_policy_from_index(index: usize) -> crate::yubikey::touch_policy::TouchPolicy {
    use crate::yubikey::touch_policy::TouchPolicy;
    match index {
        0 => TouchPolicy::Off,
        1 => TouchPolicy::On,
        2 => TouchPolicy::Fixed,
        3 => TouchPolicy::Cached,
        4 => TouchPolicy::CachedFixed,
        _ => TouchPolicy::Off,
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
        KeyScreen::ExportSSH => render_export_ssh(frame, area, state),
        KeyScreen::KeyAttributes => render_key_attributes(frame, area, yubikey_state, state),
        KeyScreen::SshPubkeyPopup => render_ssh_pubkey_popup(frame, area, yubikey_state, state),
        KeyScreen::SetTouchPolicy => render_set_touch_policy(frame, area, yubikey_state, state),
        KeyScreen::SetTouchPolicySelect => render_set_touch_policy_select(frame, area, state),
        KeyScreen::SetTouchPolicyConfirm => render_set_touch_policy_confirm(frame, area, state),
        KeyScreen::SetTouchPolicyPinInput => render_key_import_pin_input(frame, area, state),
        KeyScreen::KeyGenWizardActive => render_keygen_wizard(frame, area, state),
        KeyScreen::KeyImportPinInput => render_key_import_pin_input(frame, area, state),
        KeyScreen::KeyImportRunning => {
            render_key_operation_running(frame, area, "Importing key...", state)
        }
        KeyScreen::KeyOperationResult => render_key_operation_result(frame, area, state),
    }

    // Attestation popup overlays any other screen
    if state.attestation_popup.is_some() {
        render_attestation_popup(frame, area, state);
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

    let title = Paragraph::new("Key Management")
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
                    Span::styled("Signature:      ", Style::default().fg(Color::Green)),
                    Span::raw(
                        sig.fingerprint
                            .get(..16)
                            .unwrap_or(&sig.fingerprint)
                            .to_string(),
                    ),
                    Span::raw("..."),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("Signature:      ", Style::default().fg(Color::Red)),
                    Span::raw("Not set"),
                ]));
            }

            if let Some(ref enc) = openpgp.encryption_key {
                lines.push(Line::from(vec![
                    Span::styled("Encryption:     ", Style::default().fg(Color::Green)),
                    Span::raw(
                        enc.fingerprint
                            .get(..16)
                            .unwrap_or(&enc.fingerprint)
                            .to_string(),
                    ),
                    Span::raw("..."),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("Encryption:     ", Style::default().fg(Color::Red)),
                    Span::raw("Not set"),
                ]));
            }

            if let Some(ref auth) = openpgp.authentication_key {
                lines.push(Line::from(vec![
                    Span::styled("Authentication: ", Style::default().fg(Color::Green)),
                    Span::raw(
                        auth.fingerprint
                            .get(..16)
                            .unwrap_or(&auth.fingerprint)
                            .to_string(),
                    ),
                    Span::raw("..."),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("Authentication: ", Style::default().fg(Color::Red)),
                    Span::raw("Not set (required for SSH)"),
                ]));
            }
        }

        // Touch policy display
        if let Some(ref tp) = yk.touch_policies {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Touch Policies:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(vec![
                Span::styled("  Signature:      ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", tp.signature)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Encryption:     ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", tp.encryption)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Authentication: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", tp.authentication)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Attestation:    ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", tp.attestation)),
            ]));
        }

        lines
    } else {
        let mut lines = vec![Line::from("No YubiKey detected. Press 'R' to refresh.")];
        if let Some(ref msg) = state.message {
            lines.push(Line::from(""));
            // Split multi-line messages into separate Lines so ratatui renders
            // each line on its own row (Span::raw does not break on \n).
            let mut first = true;
            for text_line in msg.lines() {
                if first {
                    lines.push(Line::from(vec![
                        Span::styled("Status: ", Style::default().fg(Color::Yellow)),
                        Span::raw(text_line.to_string()),
                    ]));
                    first = false;
                } else {
                    lines.push(Line::from(vec![Span::raw(format!(
                        "        {}",
                        text_line
                    ))]));
                }
            }
        }
        lines
    };

    // Always show message below card info, even when yubikey present
    let mut content = content;
    if yubikey_state.is_some() {
        if let Some(ref msg) = state.message {
            content.push(Line::from(""));
            // Split multi-line messages (e.g. card status output) into separate
            // Lines — ratatui does not break Span::raw on embedded \n characters.
            for text_line in msg.lines() {
                content.push(Line::from(vec![Span::raw(text_line.to_string())]));
            }
        }
    }

    let paragraph =
        Paragraph::new(content).block(Block::default().borders(Borders::ALL).title("Keys on Card"));
    frame.render_widget(paragraph, chunks[1]);

    let actions = vec![
        ListItem::new("[V] View full key details").style(Style::default().fg(Color::Cyan)),
        ListItem::new("[I] Import existing key to card").style(Style::default().fg(Color::Green)),
        ListItem::new("[G] Generate new key on card").style(Style::default().fg(Color::Yellow)),
        ListItem::new("[E] Export SSH public key").style(Style::default().fg(Color::Magenta)),
        ListItem::new("[K] Key attributes  [S] SSH pubkey").style(Style::default().fg(Color::Blue)),
        ListItem::new("[T] Touch policy  [A] Attestation").style(Style::default().fg(Color::White)),
        ListItem::new(""),
        ListItem::new("[ESC] Back to Dashboard"),
    ];

    let action_list =
        List::new(actions).block(Block::default().title("Actions").borders(Borders::ALL));
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
        "Read card status via native PC/SC.\n\n\
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

#[allow(dead_code)]
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

fn render_key_attributes(
    frame: &mut Frame,
    area: Rect,
    yubikey_state: &Option<crate::yubikey::YubiKeyState>,
    state: &KeyState,
) {
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
            "Key attributes unavailable.",
            Style::default().fg(Color::Yellow),
        )]));
    }

    // Touch policies from YubiKeyState
    if let Some(ref yk) = yubikey_state {
        if let Some(ref tp) = yk.touch_policies {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Touch Policies:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(vec![
                Span::styled("  Signature:      ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", tp.signature)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Encryption:     ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", tp.encryption)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Authentication: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", tp.authentication)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Attestation:    ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", tp.attestation)),
            ]));
        }
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

fn render_set_touch_policy(frame: &mut Frame, area: Rect, yubikey_state: &Option<YubiKeyState>, state: &KeyState) {
    let openpgp = yubikey_state.as_ref().and_then(|yk| yk.openpgp.as_ref());
    let slot_has_key = [
        openpgp.map_or(false, |o| o.signature_key.is_some()),
        openpgp.map_or(false, |o| o.encryption_key.is_some()),
        openpgp.map_or(false, |o| o.authentication_key.is_some()),
        true, // attestation slot is factory-programmed, always present
    ];
    let slots = [
        "Signature (sig)",
        "Encryption (enc)",
        "Authentication (aut)",
        "Attestation (att)",
    ];
    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            "Select slot for touch policy:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];
    for (i, slot) in slots.iter().enumerate() {
        let has_key = slot_has_key[i];
        let indicator = if has_key { "✓ " } else { "✗ " };
        let indicator_style = if has_key {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        if i == state.touch_slot_index {
            lines.push(Line::from(vec![
                Span::styled(
                    "> ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(indicator, indicator_style),
                Span::styled(
                    *slot,
                    if has_key {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                if !has_key {
                    Span::styled(
                        "  [no key loaded]",
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    Span::raw("")
                },
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(indicator, indicator_style),
                if has_key {
                    Span::raw(*slot)
                } else {
                    Span::styled(*slot, Style::default().fg(Color::DarkGray))
                },
            ]));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "[Up/Down] Select  [Enter] Confirm  [Esc] Cancel",
        Style::default().fg(Color::DarkGray),
    )]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Set Touch Policy"),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_set_touch_policy_select(frame: &mut Frame, area: Rect, state: &KeyState) {
    let slot_display = touch_slot_display(state.touch_slot_index);
    let policies = [
        "Off",
        "On",
        "Fixed (IRREVERSIBLE)",
        "Cached",
        "Cached-Fixed (IRREVERSIBLE)",
    ];
    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            format!("Select touch policy for {}:", slot_display),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];
    for (i, policy) in policies.iter().enumerate() {
        if i == state.touch_policy_index {
            lines.push(Line::from(vec![
                Span::styled(
                    "> ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    *policy,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![Span::raw("  "), Span::raw(*policy)]));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "[Up/Down] Select  [Enter] Confirm  [Esc] Back to slot selection",
        Style::default().fg(Color::DarkGray),
    )]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Set Touch Policy"),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_set_touch_policy_confirm(frame: &mut Frame, area: Rect, state: &KeyState) {
    let slot_display = touch_slot_display(state.touch_slot_index);
    let policy = touch_policy_from_index(state.touch_policy_index);
    let text = format!(
        "WARNING: IRREVERSIBLE OPERATION\n\n\
         Setting {} touch policy on {} is IRREVERSIBLE.\n\
         The policy cannot be changed without deleting the private key.\n\n\
         Press 'y' to confirm or any other key to cancel.",
        policy, slot_display
    );
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::Red))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Confirm IRREVERSIBLE Change"),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn render_attestation_popup(frame: &mut Frame, area: Rect, state: &KeyState) {
    if let Some(ref pem) = state.attestation_popup {
        let body = format!("{}\n\nPress ESC to close.", pem);
        crate::ui::widgets::popup::render_popup(
            frame,
            area,
            "Attestation Certificate (SIG)",
            &body,
            80,
            20,
        );
    }
}

// ── Key generation wizard render functions (Plan 04-03) ──────────────────────

/// Render the appropriate wizard step based on wizard.step.
fn render_keygen_wizard(frame: &mut Frame, area: Rect, state: &KeyState) {
    let Some(ref wizard) = state.keygen_wizard else {
        // Fallback: no wizard state — return to main
        render_main_placeholder(frame, area);
        return;
    };

    match wizard.step {
        KeyGenStep::Algorithm => render_keygen_algorithm(frame, area, wizard),
        KeyGenStep::Expiry => render_keygen_expiry(frame, area, wizard),
        KeyGenStep::Identity => render_keygen_identity(frame, area, wizard),
        KeyGenStep::Backup => render_keygen_backup(frame, area, wizard),
        KeyGenStep::Confirm => render_keygen_confirm(frame, area, wizard),
        KeyGenStep::Running => {
            render_key_operation_running(frame, area, "Generating key...", state)
        }
        KeyGenStep::Result => render_key_operation_result(frame, area, state),
    }

    // Overlay PIN input if active
    if state.pin_input.is_some() {
        if let Some(ref pin_state) = state.pin_input {
            crate::ui::widgets::pin_input::render_pin_input(frame, area, pin_state);
        }
    }
}

fn render_main_placeholder(frame: &mut Frame, area: Rect) {
    let p = Paragraph::new("Loading...").block(Block::default().borders(Borders::ALL));
    frame.render_widget(p, area);
}

/// Step 1: Algorithm selection.
pub fn render_keygen_algorithm(frame: &mut Frame, area: Rect, wizard: &KeyGenWizard) {
    let algorithms = [
        "> Ed25519/Cv25519 (recommended)",
        "  RSA 2048",
        "  RSA 4096",
    ];
    let descriptions = [
        "Modern elliptic curve — small keys, fast, recommended for new keys",
        "Classic RSA — widely compatible, larger key size",
        "Classic RSA — widest compatibility, slowest, largest key size",
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            "Step 1/5: Select Key Algorithm",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    for (i, algo) in algorithms.iter().enumerate() {
        let is_selected = i == wizard.algorithm_index;
        let prefix = if is_selected { "> " } else { "  " };
        let display = format!(
            "{}{}",
            prefix,
            algo.trim_start_matches("> ").trim_start_matches("  ")
        );
        if is_selected {
            lines.push(Line::from(vec![Span::styled(
                display,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(vec![Span::styled(
                format!("    {}", descriptions[i]),
                Style::default().fg(Color::DarkGray),
            )]));
        } else {
            lines.push(Line::from(vec![Span::raw(display)]));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![Span::styled(
        "[Up/Down] Select  [Enter] Confirm  [Esc] Cancel",
        Style::default().fg(Color::DarkGray),
    )]));

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Generate Key — Algorithm"),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(p, area);
}

/// Step 2: Expiry selection.
fn render_keygen_expiry(frame: &mut Frame, area: Rect, wizard: &KeyGenWizard) {
    let options = ["No expiry", "1 year", "2 years", "Custom date"];

    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            "Step 2/5: Select Key Expiry",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    for (i, opt) in options.iter().enumerate() {
        let is_selected = i == wizard.expiry_index;
        let prefix = if is_selected { "> " } else { "  " };
        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![Span::styled(
            format!("{}{}", prefix, opt),
            style,
        )]));
    }

    // Show custom date input if Custom selected
    if wizard.expiry_index == 3 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Enter date (YYYY-MM-DD):",
            Style::default().fg(Color::Cyan),
        )]));
        let display = if wizard.editing_custom_expiry {
            format!("{}\u{2588}", wizard.custom_expiry) // cursor block
        } else {
            wizard.custom_expiry.clone()
        };
        lines.push(Line::from(vec![Span::styled(
            display,
            Style::default().fg(Color::Yellow),
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "[Up/Down] Select  [Enter] Confirm  [Esc] Back",
        Style::default().fg(Color::DarkGray),
    )]));

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Generate Key — Expiry"),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(p, area);
}

/// Step 3: Identity (name + email).
fn render_keygen_identity(frame: &mut Frame, area: Rect, wizard: &KeyGenWizard) {
    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            "Step 3/5: Enter Identity",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Name:",
            Style::default().fg(Color::White),
        )]),
    ];

    // Name field
    let name_style = if wizard.active_field == 0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let name_display = if wizard.active_field == 0 {
        format!("{}\u{2588}", wizard.name)
    } else {
        wizard.name.clone()
    };
    lines.push(Line::from(vec![Span::styled(name_display, name_style)]));
    lines.push(Line::from(""));

    // Email field
    lines.push(Line::from(vec![Span::styled(
        "Email:",
        Style::default().fg(Color::White),
    )]));
    let email_style = if wizard.active_field == 1 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let email_display = if wizard.active_field == 1 {
        format!("{}\u{2588}", wizard.email)
    } else {
        wizard.email.clone()
    };
    lines.push(Line::from(vec![Span::styled(email_display, email_style)]));
    lines.push(Line::from(""));

    let ready = !wizard.name.is_empty() && !wizard.email.is_empty();
    let hint = if ready {
        "[Tab] Switch field  [Enter] Continue  [Esc] Back"
    } else {
        "[Tab] Switch field  [Enter] Next field  [Esc] Back"
    };
    lines.push(Line::from(vec![Span::styled(
        hint,
        Style::default().fg(Color::DarkGray),
    )]));

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Generate Key — Identity"),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(p, area);
}

/// Step 4: Backup.
fn render_keygen_backup(frame: &mut Frame, area: Rect, wizard: &KeyGenWizard) {
    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            "Step 4/5: Create Backup Copy?",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("A backup exports the secret key to a file before moving it to the"),
        Line::from("card. Store in a secure location (e.g. encrypted drive)."),
        Line::from(""),
    ];

    let yes_style = if wizard.backup {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let no_style = if !wizard.backup {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    lines.push(Line::from(vec![Span::styled(
        if wizard.backup {
            "> [Y] Create backup"
        } else {
            "  [Y] Create backup"
        },
        yes_style,
    )]));
    lines.push(Line::from(vec![Span::styled(
        if !wizard.backup {
            "> [N] Skip backup"
        } else {
            "  [N] Skip backup"
        },
        no_style,
    )]));

    if wizard.backup {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Backup path:",
            Style::default().fg(Color::White),
        )]));
        let path_style = if wizard.editing_path {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Cyan)
        };
        let path_display = if wizard.editing_path {
            format!("{}\u{2588}", wizard.backup_path)
        } else {
            format!("{} [Enter to edit]", wizard.backup_path)
        };
        lines.push(Line::from(vec![Span::styled(path_display, path_style)]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "[Y/N] Toggle  [Enter] Continue  [Esc] Back",
        Style::default().fg(Color::DarkGray),
    )]));

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Generate Key — Backup"),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(p, area);
}

/// Step 5: Confirmation summary before generating.
fn render_keygen_confirm(frame: &mut Frame, area: Rect, wizard: &KeyGenWizard) {
    use crate::yubikey::key_operations::KeyAlgorithm;

    let algo_names = ["Ed25519/Cv25519", "RSA 2048", "RSA 4096"];
    let algo_display = match wizard.algorithm_index {
        0 => KeyAlgorithm::Ed25519.to_string(),
        1 => KeyAlgorithm::Rsa2048.to_string(),
        _ => KeyAlgorithm::Rsa4096.to_string(),
    };
    let expiry_opts = ["No expiry", "1 year", "2 years"];
    let expiry_display = if wizard.expiry_index < 3 {
        expiry_opts[wizard.expiry_index].to_string()
    } else {
        format!("Custom: {}", wizard.custom_expiry)
    };
    let backup_display = if wizard.backup {
        format!("Yes ({})", wizard.backup_path)
    } else {
        "No".to_string()
    };

    let _ = algo_names; // suppress unused warning

    let lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            "Step 5/5: Confirm Key Generation",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Algorithm:  ", Style::default().fg(Color::White)),
            Span::raw(&algo_display),
        ]),
        Line::from(vec![
            Span::styled("Expiry:     ", Style::default().fg(Color::White)),
            Span::raw(&expiry_display),
        ]),
        Line::from(vec![
            Span::styled("Name:       ", Style::default().fg(Color::White)),
            Span::raw(&wizard.name),
        ]),
        Line::from(vec![
            Span::styled("Email:      ", Style::default().fg(Color::White)),
            Span::raw(&wizard.email),
        ]),
        Line::from(vec![
            Span::styled("Backup:     ", Style::default().fg(Color::White)),
            Span::raw(&backup_display),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press Enter to generate key. You will be prompted for the Admin PIN.",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "[Enter] Generate  [Esc] Back",
            Style::default().fg(Color::DarkGray),
        )]),
    ];

    let _ = lines.iter(); // suppress unused warning
    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Generate Key — Confirm"),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(p, area);
}

/// Running state: progress popup overlay.
fn render_key_operation_running(frame: &mut Frame, area: Rect, msg: &str, state: &KeyState) {
    // Render main screen as background
    let background_msg = "Operation in progress...";
    let bg = Paragraph::new(background_msg)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(bg, area);

    // Overlay progress popup
    crate::ui::widgets::progress::render_progress_popup(
        frame,
        area,
        "Key Operation",
        state.operation_status.as_deref().unwrap_or(msg),
        state.progress_tick,
    );
}

/// Result screen after keygen or import completes.
fn render_key_operation_result(frame: &mut Frame, area: Rect, state: &KeyState) {
    let msg = state.message.as_deref().unwrap_or("Operation complete.");
    let import_result = state.import_result.as_deref().unwrap_or("");

    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            "Operation Complete",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    for line in msg.lines() {
        lines.push(Line::from(vec![Span::raw(line.to_string())]));
    }

    if !import_result.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Slots filled: ", Style::default().fg(Color::White)),
            Span::styled(
                import_result.to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Press any key to return.",
        Style::default().fg(Color::DarkGray),
    )]));

    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Result"))
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(p, area);
}

/// Render admin PIN input for key import.
fn render_key_import_pin_input(frame: &mut Frame, area: Rect, state: &KeyState) {
    // Background
    let available_keys = &state.available_keys;
    let selected = state.selected_key_index;
    let key_display = available_keys
        .get(selected)
        .map(|k| k.as_str())
        .unwrap_or("(none)");
    let bg_text = format!(
        "Import key to card\n\nSelected key: {}\n\nEnter Admin PIN to proceed.",
        key_display
    );
    let bg = Paragraph::new(bg_text)
        .block(Block::default().borders(Borders::ALL).title("Import Key"))
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(bg, area);

    // Overlay PIN input
    if let Some(ref pin_state) = state.pin_input {
        crate::ui::widgets::pin_input::render_pin_input(frame, area, pin_state);
    }
}
