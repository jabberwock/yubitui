use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::diagnostics::Diagnostics;

#[derive(Default)]
pub struct DiagnosticsTuiState {
    pub scroll_offset: usize,
}

#[derive(Clone, Debug)]
pub enum DiagnosticsAction {
    None,
    NavigateTo(crate::model::Screen),
}

pub fn handle_key(key: KeyEvent) -> DiagnosticsAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            DiagnosticsAction::NavigateTo(crate::model::Screen::Dashboard)
        }
        _ => DiagnosticsAction::None,
    }
}

pub fn render(frame: &mut Frame, area: Rect, diagnostics: &Diagnostics) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Title
    let title = Paragraph::new("System Diagnostics")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Diagnostics list
    let mut items = vec![];

    // PC/SC Daemon
    items.push(ListItem::new(format!(
        "{} PC/SC Daemon (pcscd): {}",
        if diagnostics.pcscd.running {
            "✅"
        } else {
            "❌"
        },
        if diagnostics.pcscd.running {
            "Running"
        } else if cfg!(target_os = "macos") {
            "Not running - Start with: brew services start pcsc-lite"
        } else if cfg!(target_os = "linux") {
            "Not running - Start with: sudo systemctl start pcscd"
        } else if cfg!(windows) {
            "Not running - Start with: Start-Service SCardSvr (as admin)"
        } else {
            "Not running"
        }
    )));

    if let Some(ref version) = diagnostics.pcscd.version {
        items.push(ListItem::new(format!("   Version: {}", version)));
    }

    items.push(ListItem::new(""));

    // GPG Agent
    items.push(ListItem::new(format!(
        "{} GPG Agent: {}",
        if diagnostics.gpg_agent.running {
            "✅"
        } else {
            "❌"
        },
        if diagnostics.gpg_agent.running {
            "Running"
        } else {
            "Not running - Start with: gpgconf --launch gpg-agent"
        }
    )));

    if let Some(ref version) = diagnostics.gpg_agent.version {
        items.push(ListItem::new(format!("   Version: {}", version)));
    }

    if let Some(ref socket) = diagnostics.gpg_agent.socket_path {
        items.push(ListItem::new(format!("   Socket: {}", socket)));
    }

    items.push(ListItem::new(""));

    // Scdaemon
    items.push(ListItem::new(format!(
        "{} Scdaemon: {}",
        if diagnostics.scdaemon.configured {
            "✅"
        } else {
            "⚠️"
        },
        if diagnostics.scdaemon.configured {
            "Configured"
        } else {
            "Not configured - Create ~/.gnupg/scdaemon.conf"
        }
    )));

    if let Some(ref issues) = diagnostics.scdaemon.issues {
        items.push(ListItem::new(format!("   Issues: {}", issues)));
    }

    items.push(ListItem::new(""));

    // SSH Agent
    items.push(ListItem::new(format!(
        "{} SSH Agent Integration: {}",
        if diagnostics.ssh_agent.configured {
            "✅"
        } else {
            "⚠️"
        },
        if diagnostics.ssh_agent.configured {
            "Configured for GPG"
        } else {
            "Not configured - Add enable-ssh-support to gpg-agent.conf"
        }
    )));

    if let Some(ref sock) = diagnostics.ssh_agent.auth_sock {
        items.push(ListItem::new(format!("   SSH_AUTH_SOCK: {}", sock)));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    frame.render_widget(list, chunks[1]);

}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};
    use crate::diagnostics::Diagnostics;

    #[test]
    fn diagnostics_default() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let diag = Diagnostics::default();
        terminal.draw(|frame| {
            render(frame, frame.area(), &diag);
        }).unwrap();
        assert_snapshot!(terminal.backend());
    }
}
