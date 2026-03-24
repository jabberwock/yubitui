use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::App;

#[derive(Default)]
pub struct DashboardState {
    pub show_context_menu: bool,
    pub menu_selected_index: usize,
}

pub fn render(frame: &mut Frame, area: Rect, app: &App, state: &DashboardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(10),
        ])
        .split(area);

    // Title
    let title = Paragraph::new("🔐 YubiTUI - YubiKey Management Dashboard")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Multi-key indicator
    let multi_key_line = if app.yubikey_count() > 1 {
        format!(
            "Key {}/{} (Tab to switch)\n",
            app.selected_yubikey_idx() + 1,
            app.yubikey_count()
        )
    } else {
        String::new()
    };

    // Quick status
    let status_text = if let Some(yk) = app.yubikey_state() {
        let pin_status = &yk.pin_status;
        let pin_emoji = if pin_status.is_healthy() {
            "✅"
        } else if pin_status.needs_attention() {
            "⚠️"
        } else {
            "❌"
        };

        let keys_info = if let Some(ref openpgp) = yk.openpgp {
            let sig = if openpgp.signature_key.is_some() {
                "✅"
            } else {
                "❌"
            };
            let enc = if openpgp.encryption_key.is_some() {
                "✅"
            } else {
                "❌"
            };
            let auth = if openpgp.authentication_key.is_some() {
                "✅"
            } else {
                "❌"
            };
            format!("Keys: {} Sign  {} Encrypt  {} Auth", sig, enc, auth)
        } else {
            "Keys: Not detected".to_string()
        };

        format!(
            "{}Device: {} {} | FW: {} | SN: {}\n\
             {} PIN: {}/3 retries | Admin: {}/3 retries\n\
             {}\n\
             \n\
             All systems operational - Your YubiKey is ready to use!",
            multi_key_line,
            yk.info.model,
            yk.info.form_factor,
            yk.info.version,
            yk.info.serial,
            pin_emoji,
            pin_status.user_pin_retries,
            pin_status.admin_pin_retries,
            keys_info
        )
    } else {
        "❌ No YubiKey Detected\n\
         \n\
         Please insert your YubiKey and press 'R' to refresh.\n\
         \n\
         Troubleshooting:\n\
         • Check USB connection\n\
         • Run diagnostics with '2' key"
            .to_string()
    };

    let status = Paragraph::new(status_text)
        .block(
            Block::default()
                .title("📊 Status")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(status, chunks[1]);

    // Navigation menu - make it clear and actionable
    let menu_items = vec![
        ListItem::new("  [1] Dashboard         You are here →").style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        ListItem::new("  [2] System Check      Diagnose PC/SC, GPG, SSH configuration"),
        ListItem::new("  [3] Key Management    View and manage OpenPGP/PIV keys"),
        ListItem::new("  [4] PIN Management    Change PINs, view retry counters"),
        ListItem::new("  [5] SSH Setup         Configure SSH authentication"),
        ListItem::new(""),
        ListItem::new("  [R] Refresh          [Q] Quit          [?] Help          [m] Menu"),
    ];

    let menu = List::new(menu_items).block(
        Block::default()
            .title("⌨️  Navigation - Press number keys to switch screens")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(menu, chunks[2]);

    // Context menu overlay — rendered last so it appears on top
    if state.show_context_menu {
        let context_items = &[
            "Diagnostics",
            "Key Management",
            "PIN Management",
            "SSH Setup Wizard",
            "Help",
        ];
        crate::ui::widgets::popup::render_context_menu(
            frame,
            area,
            "Navigate",
            context_items,
            state.menu_selected_index,
        );
    }
}
