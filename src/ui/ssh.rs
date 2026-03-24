use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::App;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SshScreen {
    Main,
    EnableSSH,
    ConfigureShell,
    RestartAgent,
    ExportKey,
    TestConnection,
}

pub struct SshState {
    pub screen: SshScreen,
    pub message: Option<String>,
    pub ssh_enabled: bool,
    pub shell_configured: bool,
    pub agent_running: bool,
}

impl Default for SshState {
    fn default() -> Self {
        Self {
            screen: SshScreen::Main,
            message: None,
            ssh_enabled: false,
            shell_configured: false,
            agent_running: false,
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &App, state: &SshState) {
    match state.screen {
        SshScreen::Main => render_main(frame, area, app, state),
        SshScreen::EnableSSH => render_enable_ssh(frame, area, state),
        SshScreen::ConfigureShell => render_configure_shell(frame, area, state),
        SshScreen::RestartAgent => render_restart_agent(frame, area, state),
        SshScreen::ExportKey => render_export_key(frame, area, state),
        SshScreen::TestConnection => render_test_connection(frame, area, state),
    }
}

fn render_main(frame: &mut Frame, area: Rect, _app: &App, state: &SshState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(14),
        ])
        .split(area);

    let title = Paragraph::new("🔧 SSH Setup Wizard")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let mut status_lines = vec![Line::from("Setup Progress:"), Line::from("")];

    status_lines.push(Line::from(vec![
        if state.ssh_enabled {
            Span::styled("✅ ", Style::default().fg(Color::Green))
        } else {
            Span::styled("❌ ", Style::default().fg(Color::Red))
        },
        Span::raw("SSH support enabled in gpg-agent.conf"),
    ]));

    status_lines.push(Line::from(vec![
        if state.shell_configured {
            Span::styled("✅ ", Style::default().fg(Color::Green))
        } else {
            Span::styled("❌ ", Style::default().fg(Color::Red))
        },
        Span::raw("SSH_AUTH_SOCK configured in shell"),
    ]));

    status_lines.push(Line::from(vec![
        if state.agent_running {
            Span::styled("✅ ", Style::default().fg(Color::Green))
        } else {
            Span::styled("⚠️  ", Style::default().fg(Color::Yellow))
        },
        Span::raw("GPG agent running"),
    ]));

    if let Some(ref msg) = state.message {
        status_lines.push(Line::from(""));
        status_lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Yellow)),
            Span::raw(msg),
        ]));
    }

    let paragraph = Paragraph::new(status_lines)
        .block(Block::default().borders(Borders::ALL).title("📊 Status"));
    frame.render_widget(paragraph, chunks[1]);

    let actions = vec![
        ListItem::new("[1] Enable SSH support in gpg-agent.conf")
            .style(Style::default().fg(Color::Green)),
        ListItem::new("[2] Configure SSH_AUTH_SOCK in shell")
            .style(Style::default().fg(Color::Yellow)),
        ListItem::new("[3] Restart GPG agent").style(Style::default().fg(Color::Cyan)),
        ListItem::new("[4] Export SSH public key").style(Style::default().fg(Color::Magenta)),
        ListItem::new("[5] Test SSH connection").style(Style::default().fg(Color::Blue)),
        ListItem::new(""),
        ListItem::new("[R] Refresh status").style(Style::default().fg(Color::White)),
        ListItem::new("[ESC] Back to Dashboard"),
    ];

    let action_list =
        List::new(actions).block(Block::default().title("⌨️  Actions").borders(Borders::ALL));
    frame.render_widget(action_list, chunks[2]);
}

fn render_enable_ssh(frame: &mut Frame, area: Rect, state: &SshState) {
    render_operation_screen(
        frame,
        area,
        "Enable SSH Support",
        "Add 'enable-ssh-support' to ~/.gnupg/gpg-agent.conf\n\n\
         This tells GPG agent to handle SSH authentication.\n\n\
         Press ENTER to enable or ESC to cancel.",
        state,
    );
}

fn render_configure_shell(frame: &mut Frame, area: Rect, state: &SshState) {
    render_operation_screen(
        frame,
        area,
        "Configure Shell",
        "Add SSH_AUTH_SOCK export to your shell configuration.\n\n\
         This will:\n\
         1. Detect your shell (bash/zsh)\n\
         2. Add export to ~/.bashrc or ~/.zshrc\n\
         3. Configure SSH to use GPG agent\n\n\
         After this, restart your shell or source the config.\n\n\
         Press ENTER to configure or ESC to cancel.",
        state,
    );
}

fn render_restart_agent(frame: &mut Frame, area: Rect, state: &SshState) {
    render_operation_screen(
        frame,
        area,
        "Restart GPG Agent",
        "Restart GPG agent to apply configuration changes.\n\n\
         This will kill the current agent and start a new one.\n\n\
         Press ENTER to restart or ESC to cancel.",
        state,
    );
}

fn render_export_key(frame: &mut Frame, area: Rect, state: &SshState) {
    render_operation_screen(
        frame,
        area,
        "Export SSH Public Key",
        "Export your YubiKey's authentication key as SSH public key.\n\n\
         The key will be displayed on screen.\n\
         You can copy it to:\n\
         - Remote servers (~/.ssh/authorized_keys)\n\
         - GitHub/GitLab SSH keys\n\n\
         Press ENTER to export or ESC to cancel.",
        state,
    );
}

fn render_test_connection(frame: &mut Frame, area: Rect, state: &SshState) {
    render_operation_screen(
        frame,
        area,
        "Test SSH Connection",
        "Test SSH connection to a remote server.\n\n\
         You will be prompted for:\n\
         - Username\n\
         - Hostname\n\n\
         The connection will use your YubiKey for authentication.\n\n\
         Press ENTER to test or ESC to cancel.",
        state,
    );
}

fn render_operation_screen(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    content: &str,
    state: &SshState,
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
