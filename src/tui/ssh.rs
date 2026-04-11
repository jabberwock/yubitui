use textual_rs::{Widget, Footer, Header, Label, Button, Vertical, Horizontal};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::reactive::Reactive;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use crate::tui::widgets::popup::PopupScreen;

const SSH_HELP_TEXT: &str = "\
SSH Setup Wizard\n\
\n\
Configure your YubiKey for SSH authentication. Your OpenPGP\n\
authentication subkey can serve as an SSH key via gpg-agent.\n\
\n\
The wizard walks through:\n\
1. Checking gpg-agent configuration\n\
2. Exporting your SSH public key\n\
3. Testing the SSH connection\n\
\n\
This replaces traditional SSH key files with hardware-bound keys\n\
that cannot be copied or extracted.";


/// SSH wizard sub-screen variants — retained as internal reactive state (D-04).
/// Sub-screens are represented as pushed screens via push_screen_deferred or
/// internal state transitions within SshScreen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SshScreen {
    Main,
    EnableSSH,
    ConfigureShell,
    RestartAgent,
    ExportKey,
    TestConnection,
}

/// SSH State — all fields retained from original (D-04: existing state preserved).
#[derive(Clone, PartialEq)]
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
    /// Scroll offset for scrollable list content
    pub scroll_offset: usize,
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
            scroll_offset: 0,
        }
    }
}

/// SSH Setup Wizard screen — migrated to textual-rs Widget pattern.
///
/// Per UI-SPEC:
/// - Header("SSH Setup Wizard")
/// - Sidebar (agent status summary): SSH enabled, shell configured, agent running
/// - Main (action area): setup steps as Labels + "Add to Agent" action
/// - Footer with Esc=back, A=add_to_agent, R=refresh
/// - No hardcoded Color:: values
///
/// Sub-screens (EnableSSH, ConfigureShell, etc.) are tracked via Reactive<SshState>.
/// In the textual-rs model they can be pushed as separate screens; here they are
/// rendered inline based on state.screen, preserving all original sub-screen content.
pub struct SshWizardScreen {
    pub state: Reactive<SshState>,
    own_id: std::cell::Cell<Option<textual_rs::WidgetId>>,
}

impl SshWizardScreen {
    pub fn new(initial_state: SshState) -> Self {
        SshWizardScreen {
            state: Reactive::new(initial_state),
            own_id: std::cell::Cell::new(None),
        }
    }
}

impl Widget for SshWizardScreen {
    fn widget_type_name(&self) -> &'static str {
        "SshWizardScreen"
    }

    fn on_mount(&self, id: textual_rs::WidgetId) {
        self.own_id.set(Some(id));
    }

    fn on_unmount(&self, _id: textual_rs::WidgetId) {
        self.own_id.set(None);
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let state = self.state.get_untracked();

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("SSH Setup Wizard")),
        ];

        match state.screen {
            SshScreen::Main => {
                // Setup progress card
                let ssh_status = if state.ssh_enabled {
                    "✓ SSH support enabled in gpg-agent.conf"
                } else {
                    "○ SSH support in gpg-agent.conf"
                };
                let shell_status = if state.shell_configured {
                    "✓ SSH_AUTH_SOCK configured in shell"
                } else {
                    "○ SSH_AUTH_SOCK configured in shell"
                };
                let agent_status = if state.agent_running {
                    "✓ GPG agent active"
                } else {
                    "⚠ GPG agent not running"
                };

                widgets.push(Box::new(Vertical::with_children(vec![
                    Box::new(Label::new("Setup Progress").with_class("section-title")),
                    Box::new(Label::new(ssh_status)),
                    Box::new(Label::new(shell_status)),
                    Box::new(Label::new(agent_status)),
                ]).with_class("status-card")));

                if let Some(ref msg) = state.message {
                    widgets.push(Box::new(Label::new(format!("Status: {}", msg))));
                }

                widgets.push(Box::new(Label::new("")));

                // Action buttons in two rows
                widgets.push(Box::new(Horizontal::with_children(vec![
                    Box::new(Button::new("Enable SSH Support").with_action("step_1")),
                    Box::new(Button::new("Configure Shell").with_action("step_2")),
                    Box::new(Button::new("Restart Agent").with_action("step_3")),
                ]).with_class("button-bar")));
                widgets.push(Box::new(Horizontal::with_children(vec![
                    Box::new(Button::new("Export SSH Key").with_action("step_4")),
                    Box::new(Button::new("Test Connection").with_action("step_5")),
                ]).with_class("button-bar")));
            }

            SshScreen::EnableSSH => {
                widgets.push(Box::new(Label::new("Enable SSH Support").with_class("section-title")));
                widgets.push(Box::new(Vertical::with_children(vec![
                    Box::new(Label::new("Add 'enable-ssh-support' to ~/.gnupg/gpg-agent.conf")),
                    Box::new(Label::new("")),
                    Box::new(Label::new("This tells GPG agent to handle SSH authentication.")),
                    Box::new(Label::new("Press Enter to enable or Esc to cancel.")),
                ]).with_class("status-card")));
                if let Some(ref msg) = state.message {
                    widgets.push(Box::new(Label::new(msg.clone())));
                }
            }

            SshScreen::ConfigureShell => {
                widgets.push(Box::new(Label::new("Configure Shell").with_class("section-title")));
                widgets.push(Box::new(Vertical::with_children(vec![
                    Box::new(Label::new("Add SSH_AUTH_SOCK export to your shell configuration.")),
                    Box::new(Label::new("")),
                    Box::new(Label::new("This will:")),
                    Box::new(Label::new("  1. Detect your shell (bash/zsh)")),
                    Box::new(Label::new("  2. Add export to ~/.bashrc or ~/.zshrc")),
                    Box::new(Label::new("  3. Configure SSH to use GPG agent")),
                    Box::new(Label::new("")),
                    Box::new(Label::new("After this, restart your shell or source the config.")),
                    Box::new(Label::new("Press Enter to configure or Esc to cancel.")),
                ]).with_class("status-card")));
                if let Some(ref msg) = state.message {
                    widgets.push(Box::new(Label::new(msg.clone())));
                }
            }

            SshScreen::RestartAgent => {
                widgets.push(Box::new(Label::new("Restart GPG Agent").with_class("section-title")));
                widgets.push(Box::new(Vertical::with_children(vec![
                    Box::new(Label::new("Restart GPG agent to apply configuration changes.")),
                    Box::new(Label::new("")),
                    Box::new(Label::new("This will kill the current agent and start a new one.")),
                    Box::new(Label::new("Press Enter to restart or Esc to cancel.")),
                ]).with_class("status-card")));
                if let Some(ref msg) = state.message {
                    widgets.push(Box::new(Label::new(msg.clone())));
                }
            }

            SshScreen::ExportKey => {
                widgets.push(Box::new(Label::new("Export SSH Public Key").with_class("section-title")));
                widgets.push(Box::new(Vertical::with_children(vec![
                    Box::new(Label::new("Export your YubiKey's authentication key as SSH public key.")),
                    Box::new(Label::new("")),
                    Box::new(Label::new("The key will be displayed on screen. Copy it to:")),
                    Box::new(Label::new("  - Remote servers (~/.ssh/authorized_keys)")),
                    Box::new(Label::new("  - GitHub/GitLab SSH keys")),
                    Box::new(Label::new("")),
                    Box::new(Label::new("Press Enter to export or Esc to cancel.")),
                ]).with_class("status-card")));
                if let Some(ref msg) = state.message {
                    widgets.push(Box::new(Label::new(msg.clone())));
                }
            }

            SshScreen::TestConnection => {
                widgets.push(Box::new(Label::new("Test SSH Connection").with_class("section-title")));
                widgets.push(Box::new(Vertical::with_children(vec![
                    Box::new(Label::new(format!("Username: {}", state.test_conn_user))),
                    Box::new(Label::new(format!("Hostname: {}", state.test_conn_host))),
                    Box::new(Label::new("")),
                    Box::new(Label::new("Type username and hostname, then press Enter to test.")),
                    Box::new(Label::new("Tab switches between fields. Esc cancels.")),
                ]).with_class("status-card")));
                if let Some(ref msg) = state.message {
                    widgets.push(Box::new(Label::new(format!("Result: {}", msg))));
                }
            }
        }

        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        &[
            KeyBinding {
                key: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                action: "back",
                description: "Esc Back",
                show: true,
            },
            KeyBinding {
                key: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                action: "back",
                description: "",
                show: false,
            },
            KeyBinding {
                key: KeyCode::Char('a'),
                modifiers: KeyModifiers::NONE,
                action: "add_to_agent",
                description: "A Add to agent",
                show: true,
            },
            KeyBinding {
                key: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
                action: "refresh",
                description: "R Refresh",
                show: true,
            },
            KeyBinding {
                key: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                action: "execute",
                description: "Enter Execute",
                show: false,
            },
            KeyBinding {
                key: KeyCode::Char('1'),
                modifiers: KeyModifiers::NONE,
                action: "step_1",
                description: "1 Enable SSH",
                show: false,
            },
            KeyBinding {
                key: KeyCode::Char('2'),
                modifiers: KeyModifiers::NONE,
                action: "step_2",
                description: "2 Configure Shell",
                show: false,
            },
            KeyBinding {
                key: KeyCode::Char('3'),
                modifiers: KeyModifiers::NONE,
                action: "step_3",
                description: "3 Restart Agent",
                show: false,
            },
            KeyBinding {
                key: KeyCode::Char('4'),
                modifiers: KeyModifiers::NONE,
                action: "step_4",
                description: "4 Export Key",
                show: false,
            },
            KeyBinding {
                key: KeyCode::Char('5'),
                modifiers: KeyModifiers::NONE,
                action: "step_5",
                description: "5 Test Connection",
                show: false,
            },
            KeyBinding {
                key: KeyCode::Char('?'),
                modifiers: KeyModifiers::NONE,
                action: "help",
                description: "? Help",
                show: true,
            },
        ]
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "back" => {
                let current = self.state.get_untracked().screen;
                if current == SshScreen::Main {
                    ctx.pop_screen_deferred();
                } else {
                    self.state.update(|s| {
                        s.screen = SshScreen::Main;
                        s.message = None;
                    });
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                }
            }
            "step_1" => {
                self.state.update(|s| { s.screen = SshScreen::EnableSSH; });
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
            "step_2" => {
                self.state.update(|s| { s.screen = SshScreen::ConfigureShell; });
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
            "step_3" => {
                self.state.update(|s| { s.screen = SshScreen::RestartAgent; });
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
            "step_4" => {
                self.state.update(|s| { s.screen = SshScreen::ExportKey; });
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
            "step_5" => {
                self.state.update(|s| { s.screen = SshScreen::TestConnection; });
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
            "add_to_agent" | "execute" => {
                let current = self.state.get_untracked().screen;
                match current {
                    SshScreen::EnableSSH => {
                        match crate::model::ssh_operations::enable_ssh_support() {
                            Ok(msg) => self.state.update(|s| { s.ssh_enabled = true; s.message = Some(msg); s.screen = SshScreen::Main; }),
                            Err(e) => self.state.update(|s| { s.message = Some(format!("Error: {}", e)); s.screen = SshScreen::Main; }),
                        }
                    }
                    SshScreen::ConfigureShell => {
                        match crate::model::ssh_operations::configure_shell_ssh() {
                            Ok(msg) => self.state.update(|s| { s.shell_configured = true; s.message = Some(msg); s.screen = SshScreen::Main; }),
                            Err(e) => self.state.update(|s| { s.message = Some(format!("Error: {}", e)); s.screen = SshScreen::Main; }),
                        }
                    }
                    SshScreen::RestartAgent => {
                        match crate::model::ssh_operations::restart_gpg_agent() {
                            Ok(msg) => self.state.update(|s| { s.agent_running = true; s.message = Some(msg); s.screen = SshScreen::Main; }),
                            Err(e) => self.state.update(|s| { s.message = Some(format!("Error: {}", e)); s.screen = SshScreen::Main; }),
                        }
                    }
                    SshScreen::ExportKey => {
                        let result = std::process::Command::new("ssh-add")
                            .arg("-L")
                            .stdin(std::process::Stdio::null())
                            .stderr(std::process::Stdio::piped())
                            .output();
                        match result {
                            Ok(o) if o.status.success() => {
                                let keys = String::from_utf8_lossy(&o.stdout).trim().to_string();
                                if keys.is_empty() {
                                    self.state.update(|s| { s.message = Some("ssh-add -L returned no keys. Insert YubiKey and try again.".to_string()); s.screen = SshScreen::Main; });
                                } else {
                                    self.state.update(|s| { s.message = Some(keys); s.screen = SshScreen::Main; });
                                }
                            }
                            Ok(o) => {
                                let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
                                self.state.update(|s| { s.message = Some(format!("ssh-add -L failed (exit {}): {}", o.status.code().unwrap_or(-1), stderr)); s.screen = SshScreen::Main; });
                            }
                            Err(e) => {
                                self.state.update(|s| { s.message = Some(format!("Failed to run ssh-add: {}", e)); s.screen = SshScreen::Main; });
                            }
                        }
                    }
                    SshScreen::TestConnection => {
                        let user = self.state.get_untracked().test_conn_user.clone();
                        let host = self.state.get_untracked().test_conn_host.clone();
                        if user.is_empty() || host.is_empty() {
                            self.state.update(|s| { s.message = Some("Enter both username and hostname".to_string()); });
                        } else {
                            match crate::model::ssh_operations::test_ssh_connection(&user, &host) {
                                Ok(msg) => self.state.update(|s| { s.message = Some(msg); s.screen = SshScreen::Main; }),
                                Err(e) => self.state.update(|s| { s.message = Some(format!("Error: {}", e)); s.screen = SshScreen::Main; }),
                            }
                        }
                    }
                    _ => {}
                }
                if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
            }
            "refresh" => {
                ctx.pop_screen_deferred();
            }
            "help" => {
                ctx.push_screen_deferred(Box::new(
                    PopupScreen::new("SSH Wizard Help", SSH_HELP_TEXT)
                ));
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textual_rs::TestApp;
    use crossterm::event::KeyCode;

    #[tokio::test]
    async fn ssh_main_screen() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || {
            Box::new(SshWizardScreen::new(SshState::default()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn ssh_enable_screen() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || {
            Box::new(SshWizardScreen::new(SshState::default()))
        });
        let mut pilot = app.pilot();
        pilot.press(KeyCode::Char('a')).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_snapshot!(app.backend());
    }
}
