use textual_rs::{Widget, Footer, Header, Label};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::reactive::Reactive;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

#[derive(Clone, Debug)]
pub enum SshAction {
    None,
    NavigateTo(crate::model::Screen),
    ExecuteSshOperation,
    RefreshSshStatus,
}

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
}

impl SshWizardScreen {
    pub fn new(initial_state: SshState) -> Self {
        SshWizardScreen {
            state: Reactive::new(initial_state),
        }
    }
}

impl Widget for SshWizardScreen {
    fn widget_type_name(&self) -> &'static str {
        "SshWizardScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let state = self.state.get_untracked();

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("SSH Setup Wizard")),
        ];

        match state.screen {
            SshScreen::Main => {
                // Status sidebar area (agent status summary)
                widgets.push(Box::new(Label::new("Setup Progress:")));
                widgets.push(Box::new(Label::new("")));

                let ssh_status = if state.ssh_enabled {
                    "[OK] SSH support enabled in gpg-agent.conf".to_string()
                } else {
                    "[  ] SSH support enabled in gpg-agent.conf".to_string()
                };
                widgets.push(Box::new(Label::new(ssh_status)));

                let shell_status = if state.shell_configured {
                    "[OK] SSH_AUTH_SOCK configured in shell".to_string()
                } else {
                    "[  ] SSH_AUTH_SOCK configured in shell".to_string()
                };
                widgets.push(Box::new(Label::new(shell_status)));

                let agent_status = if state.agent_running {
                    "[OK] GPG agent running".to_string()
                } else {
                    "[ !] GPG agent running".to_string()
                };
                widgets.push(Box::new(Label::new(agent_status)));

                if let Some(ref msg) = state.message {
                    widgets.push(Box::new(Label::new("")));
                    widgets.push(Box::new(Label::new(format!("Status: {}", msg))));
                }

                widgets.push(Box::new(Label::new("")));

                // Main action area
                widgets.push(Box::new(Label::new("Actions:")));
                widgets.push(Box::new(Label::new(
                    "[1] Enable SSH support in gpg-agent.conf",
                )));
                widgets.push(Box::new(Label::new(
                    "[2] Configure SSH_AUTH_SOCK in shell",
                )));
                widgets.push(Box::new(Label::new("[3] Restart GPG agent")));
                widgets.push(Box::new(Label::new("[4] Export SSH public key")));
                widgets.push(Box::new(Label::new("[5] Test SSH connection")));
            }

            SshScreen::EnableSSH => {
                widgets.push(Box::new(Label::new("Enable SSH Support")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Add 'enable-ssh-support' to ~/.gnupg/gpg-agent.conf",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "This tells GPG agent to handle SSH authentication.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Press Enter to enable or Esc to cancel.",
                )));
                if let Some(ref msg) = state.message {
                    widgets.push(Box::new(Label::new("")));
                    widgets.push(Box::new(Label::new(msg.clone())));
                }
            }

            SshScreen::ConfigureShell => {
                widgets.push(Box::new(Label::new("Configure Shell")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Add SSH_AUTH_SOCK export to your shell configuration.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("This will:")));
                widgets.push(Box::new(Label::new("  1. Detect your shell (bash/zsh)")));
                widgets.push(Box::new(Label::new(
                    "  2. Add export to ~/.bashrc or ~/.zshrc",
                )));
                widgets.push(Box::new(Label::new(
                    "  3. Configure SSH to use GPG agent",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "After this, restart your shell or source the config.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Press Enter to configure or Esc to cancel.",
                )));
                if let Some(ref msg) = state.message {
                    widgets.push(Box::new(Label::new("")));
                    widgets.push(Box::new(Label::new(msg.clone())));
                }
            }

            SshScreen::RestartAgent => {
                widgets.push(Box::new(Label::new("Restart GPG Agent")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Restart GPG agent to apply configuration changes.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "This will kill the current agent and start a new one.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Press Enter to restart or Esc to cancel.",
                )));
                if let Some(ref msg) = state.message {
                    widgets.push(Box::new(Label::new("")));
                    widgets.push(Box::new(Label::new(msg.clone())));
                }
            }

            SshScreen::ExportKey => {
                widgets.push(Box::new(Label::new("Export SSH Public Key")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Export your YubiKey's authentication key as SSH public key.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("The key will be displayed on screen.")));
                widgets.push(Box::new(Label::new("You can copy it to:")));
                widgets.push(Box::new(Label::new(
                    "  - Remote servers (~/.ssh/authorized_keys)",
                )));
                widgets.push(Box::new(Label::new("  - GitHub/GitLab SSH keys")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Press Enter to export or Esc to cancel.",
                )));
                if let Some(ref msg) = state.message {
                    widgets.push(Box::new(Label::new("")));
                    widgets.push(Box::new(Label::new(msg.clone())));
                }
            }

            SshScreen::TestConnection => {
                widgets.push(Box::new(Label::new("Test SSH Connection")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(format!(
                    "Username: {}",
                    state.test_conn_user
                ))));
                widgets.push(Box::new(Label::new(format!(
                    "Hostname: {}",
                    state.test_conn_host
                ))));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Type username and hostname, then press Enter to test.",
                )));
                widgets.push(Box::new(Label::new(
                    "Tab switches between fields. Esc cancels.",
                )));
                widgets.push(Box::new(Label::new(
                    "Uses BatchMode=yes (no password prompts, YubiKey auth only).",
                )));
                if let Some(ref msg) = state.message {
                    widgets.push(Box::new(Label::new("")));
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
                }
            }
            "step_1" => {
                self.state.update(|s| {
                    s.screen = SshScreen::EnableSSH;
                });
            }
            "step_2" => {
                self.state.update(|s| {
                    s.screen = SshScreen::ConfigureShell;
                });
            }
            "step_3" => {
                self.state.update(|s| {
                    s.screen = SshScreen::RestartAgent;
                });
            }
            "step_4" => {
                self.state.update(|s| {
                    s.screen = SshScreen::ExportKey;
                });
            }
            "step_5" => {
                self.state.update(|s| {
                    s.screen = SshScreen::TestConnection;
                });
            }
            "add_to_agent" | "execute" => {
                // SSH operations are executed by app.rs runner — defer via pop+action.
                // Full wiring happens in subsequent plans.
                ctx.pop_screen_deferred();
            }
            "refresh" => {
                // Refresh SSH status — wired in subsequent plans.
                ctx.pop_screen_deferred();
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {
        // Rendering handled by compose() — leaf rendering not needed for container screens.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textual_rs::TestApp;
    use crossterm::event::KeyCode;

    #[tokio::test]
    async fn ssh_main_screen() {
        let mut app = TestApp::new(80, 24, || {
            Box::new(SshWizardScreen::new(SshState::default()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn ssh_enable_screen() {
        let mut app = TestApp::new(80, 24, || {
            Box::new(SshWizardScreen::new(SshState::default()))
        });
        let mut pilot = app.pilot();
        pilot.press(KeyCode::Char('a')).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_display_snapshot!(app.backend());
    }
}
