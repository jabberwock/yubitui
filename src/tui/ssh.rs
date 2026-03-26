use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::App;

pub enum SshAction {
    None,
    NavigateTo(crate::model::Screen),
    ExecuteSshOperation,
    RefreshSshStatus,
}

/// Handle key events for the SSH Wizard screen.
/// Sub-screen navigation is handled internally by mutating state.
/// Only actions requiring App context are returned.
pub fn handle_key(state: &mut SshState, key: KeyEvent) -> SshAction {
    match state.screen {
        SshScreen::Main => match key.code {
            KeyCode::Char('1') => {
                state.screen = SshScreen::EnableSSH;
                SshAction::None
            }
            KeyCode::Char('2') => {
                state.screen = SshScreen::ConfigureShell;
                SshAction::None
            }
            KeyCode::Char('3') => {
                state.screen = SshScreen::RestartAgent;
                SshAction::None
            }
            KeyCode::Char('4') => {
                state.screen = SshScreen::ExportKey;
                SshAction::None
            }
            KeyCode::Char('5') => {
                state.screen = SshScreen::TestConnection;
                SshAction::None
            }
            KeyCode::Char('r') => SshAction::RefreshSshStatus,
            KeyCode::Esc => SshAction::NavigateTo(crate::model::Screen::Dashboard),
            _ => SshAction::None,
        },
        SshScreen::TestConnection => match key.code {
            KeyCode::Enter => SshAction::ExecuteSshOperation,
            KeyCode::Esc => {
                state.screen = SshScreen::Main;
                state.message = None;
                state.test_conn_user.clear();
                state.test_conn_host.clear();
                state.test_conn_focused = 0;
                SshAction::None
            }
            KeyCode::Tab => {
                state.test_conn_focused = 1 - state.test_conn_focused;
                SshAction::None
            }
            KeyCode::Backspace => {
                if state.test_conn_focused == 0 {
                    state.test_conn_user.pop();
                } else {
                    state.test_conn_host.pop();
                }
                SshAction::None
            }
            KeyCode::Char(c) => {
                if state.test_conn_focused == 0 {
                    state.test_conn_user.push(c);
                } else {
                    state.test_conn_host.push(c);
                }
                SshAction::None
            }
            _ => SshAction::None,
        },
        _ => match key.code {
            KeyCode::Enter => SshAction::ExecuteSshOperation,
            KeyCode::Esc => {
                state.screen = SshScreen::Main;
                state.message = None;
                SshAction::None
            }
            _ => SshAction::None,
        },
    }
}

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
    /// Username field for TestConnection screen
    pub test_conn_user: String,
    /// Hostname field for TestConnection screen
    pub test_conn_host: String,
    /// Which field is focused: 0 = username, 1 = hostname
    pub test_conn_focused: u8,
}

impl Default for SshState {
    fn default() -> Self {
        Self {
            screen: SshScreen::Main,
            message: None,
            ssh_enabled: false,
            shell_configured: false,
            agent_running: false,
            test_conn_user: String::new(),
            test_conn_host: String::new(),
            test_conn_focused: 0,
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let title_widget = Paragraph::new("Test SSH Connection")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title_widget, chunks[0]);

    let user_style = if state.test_conn_focused == 0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    let user_widget = Paragraph::new(state.test_conn_user.as_str())
        .style(user_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Username (Tab to switch, Enter to test)"),
        );
    frame.render_widget(user_widget, chunks[1]);

    let host_style = if state.test_conn_focused == 1 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };
    let host_widget = Paragraph::new(state.test_conn_host.as_str())
        .style(host_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Hostname (Enter to test, ESC to cancel)"),
        );
    frame.render_widget(host_widget, chunks[2]);

    let mut help_lines = vec![
        Line::from("Type username and hostname, then press Enter to test."),
        Line::from("Tab switches between fields. ESC cancels."),
        Line::from("Uses BatchMode=yes (no password prompts, YubiKey auth only)."),
    ];
    if let Some(ref msg) = state.message {
        help_lines.push(Line::from(""));
        help_lines.push(Line::from(vec![
            Span::styled("Result: ", Style::default().fg(Color::Yellow)),
            Span::raw(msg.as_str()),
        ]));
    }
    let help_widget = Paragraph::new(help_lines)
        .block(Block::default().borders(Borders::ALL).title("Info"))
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(help_widget, chunks[3]);
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
