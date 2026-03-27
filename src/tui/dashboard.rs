use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
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

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum DashboardAction {
    None,
    Quit,
    NavigateTo(crate::model::Screen),
    OpenContextMenu,
    SwitchYubiKey,
    Refresh,
    SelectMenuItem(usize),
    CloseContextMenu,
    MenuUp,
    MenuDown,
}

/// Handle key events for the Dashboard screen.
/// Returns an action for app.rs to interpret.
pub fn handle_key(
    state: &mut DashboardState,
    key: KeyEvent,
    yubikey_count: usize,
) -> DashboardAction {
    if state.show_context_menu {
        match key.code {
            KeyCode::Up => {
                if state.menu_selected_index > 0 {
                    state.menu_selected_index -= 1;
                }
                DashboardAction::None
            }
            KeyCode::Down => {
                if state.menu_selected_index < 5 {
                    state.menu_selected_index += 1;
                }
                DashboardAction::None
            }
            KeyCode::Enter => {
                let idx = state.menu_selected_index;
                state.show_context_menu = false;
                state.menu_selected_index = 0;
                DashboardAction::SelectMenuItem(idx)
            }
            KeyCode::Esc => {
                state.show_context_menu = false;
                state.menu_selected_index = 0;
                DashboardAction::None
            }
            _ => DashboardAction::None,
        }
    } else {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => DashboardAction::Quit,
            KeyCode::Tab => {
                if yubikey_count > 0 {
                    DashboardAction::SwitchYubiKey
                } else {
                    DashboardAction::None
                }
            }
            KeyCode::Char('1') => DashboardAction::NavigateTo(crate::model::Screen::Dashboard),
            KeyCode::Char('2') => {
                DashboardAction::NavigateTo(crate::model::Screen::Diagnostics)
            }
            KeyCode::Char('3') => DashboardAction::NavigateTo(crate::model::Screen::Keys),
            KeyCode::Char('4') => {
                DashboardAction::NavigateTo(crate::model::Screen::PinManagement)
            }
            KeyCode::Char('5') => DashboardAction::NavigateTo(crate::model::Screen::SshWizard),
            KeyCode::Char('6') => DashboardAction::NavigateTo(crate::model::Screen::Piv),
            KeyCode::Char('r') => DashboardAction::Refresh,
            KeyCode::Enter | KeyCode::Char('m') => {
                state.show_context_menu = true;
                state.menu_selected_index = 0;
                DashboardAction::None
            }
            _ => DashboardAction::None,
        }
    }
}

/// Handle mouse events for the Dashboard screen.
pub fn handle_mouse(state: &mut DashboardState, mouse: MouseEvent) -> DashboardAction {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if state.show_context_menu {
                state.show_context_menu = false;
            }
            DashboardAction::None
        }
        MouseEventKind::ScrollUp => {
            if state.show_context_menu && state.menu_selected_index > 0 {
                state.menu_selected_index -= 1;
            }
            DashboardAction::None
        }
        MouseEventKind::ScrollDown => {
            if state.show_context_menu && state.menu_selected_index < 5 {
                state.menu_selected_index += 1;
            }
            DashboardAction::None
        }
        _ => DashboardAction::None,
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &App, state: &DashboardState, click_regions: &mut Vec<crate::model::click_region::ClickRegion>) {
    click_regions.clear();
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
        ListItem::new("  [6] PIV Certificates  View PIV slot occupancy"),
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

    // Register click regions for nav menu items (background elements — pushed first)
    // Nav menu items are rendered in chunks[2] starting at y+1 (skip border)
    let nav_y = chunks[2].y + 1;
    let nav_x = chunks[2].x + 1;
    let nav_w = chunks[2].width.saturating_sub(2);
    let nav_screens = [
        crate::model::Screen::Dashboard,
        crate::model::Screen::Diagnostics,
        crate::model::Screen::Keys,
        crate::model::Screen::PinManagement,
        crate::model::Screen::SshWizard,
        crate::model::Screen::Piv,
    ];
    for (i, screen) in nav_screens.iter().enumerate() {
        let row = nav_y + i as u16;
        if row < chunks[2].y + chunks[2].height {
            click_regions.push(crate::model::click_region::ClickRegion {
                region: crate::model::click_region::Region { x: nav_x, y: row, w: nav_w, h: 1 },
                action: crate::model::click_region::ClickAction::Dashboard(
                    DashboardAction::NavigateTo(*screen),
                ),
            });
        }
    }

    // Refresh/Menu items row (index 7 = "[R] Refresh ...")
    let refresh_row = nav_y + 7;
    if refresh_row < chunks[2].y + chunks[2].height {
        click_regions.push(crate::model::click_region::ClickRegion {
            region: crate::model::click_region::Region { x: nav_x, y: refresh_row, w: nav_w / 3, h: 1 },
            action: crate::model::click_region::ClickAction::Dashboard(DashboardAction::Refresh),
        });
        click_regions.push(crate::model::click_region::ClickRegion {
            region: crate::model::click_region::Region { x: nav_x + nav_w / 3, y: refresh_row, w: nav_w / 3, h: 1 },
            action: crate::model::click_region::ClickAction::Dashboard(DashboardAction::OpenContextMenu),
        });
    }

    // Context menu overlay — rendered last so it appears on top
    // Its click regions are pushed AFTER nav regions so .iter().rev() checks them first.
    if state.show_context_menu {
        let context_items = &[
            "Diagnostics",
            "Key Management",
            "PIN Management",
            "SSH Setup Wizard",
            "PIV Certificates",
            "Help",
        ];
        let popup_area = crate::tui::widgets::popup::render_context_menu(
            frame,
            area,
            "Navigate",
            context_items,
            state.menu_selected_index,
        );

        // Register each context menu item row as a click region
        // These are pushed AFTER background nav regions — .iter().rev() checks them first
        let menu_items_y = popup_area.y + 1; // skip top border
        let menu_items_x = popup_area.x + 1;
        let menu_items_w = popup_area.width.saturating_sub(2);
        for i in 0..context_items.len() {
            let row = menu_items_y + i as u16;
            if row < popup_area.y + popup_area.height {
                click_regions.push(crate::model::click_region::ClickRegion {
                    region: crate::model::click_region::Region { x: menu_items_x, y: row, w: menu_items_w, h: 1 },
                    action: crate::model::click_region::ClickAction::Dashboard(
                        DashboardAction::SelectMenuItem(i),
                    ),
                });
            }
        }
    }
}
