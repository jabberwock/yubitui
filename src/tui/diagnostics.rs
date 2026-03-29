use textual_rs::{Widget, Footer, Header, Label, Button, DataTable, ColumnDef};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::reactive::Reactive;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::diagnostics::Diagnostics;
use crate::tui::widgets::popup::PopupScreen;

const DIAGNOSTICS_HELP_TEXT: &str = "\
Diagnostics\n\
\n\
System checks for YubiKey connectivity and tool availability.\n\
\n\
Verifies:\n\
- PC/SC smart card service is running\n\
- Card readers are detected\n\
- GPG is installed and accessible\n\
- gpg-agent is running\n\
\n\
Run diagnostics if your YubiKey is not detected or operations fail.";

#[derive(Default, Clone, PartialEq)]
pub struct DiagnosticsTuiState {
    pub scroll_offset: usize,
}


/// Diagnostics screen — displays PC/SC, GPG agent, Scdaemon, and SSH agent status.
///
/// Follows the textual-rs Widget pattern (D-01, D-07, D-15):
/// - Header with screen title
/// - Full-width content area (no sidebar — all items in one list, per plan guidance)
/// - Footer with visible keybindings
/// - No hardcoded Color:: values — theme variables used via Label content
pub struct DiagnosticsScreen {
    pub diagnostics: Diagnostics,
    #[allow(dead_code)]
    pub state: Reactive<DiagnosticsTuiState>,
}

impl DiagnosticsScreen {
    pub fn new(diagnostics: Diagnostics) -> Self {
        DiagnosticsScreen {
            diagnostics,
            state: Reactive::new(DiagnosticsTuiState::default()),
        }
    }
}

impl Widget for DiagnosticsScreen {
    fn widget_type_name(&self) -> &'static str {
        "DiagnosticsScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let d = &self.diagnostics;

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("System Diagnostics")),
        ];

        // Build DataTable with 4 diagnostic check rows
        let columns = vec![
            ColumnDef::new("Status").with_width(8),
            ColumnDef::new("Component").with_width(25),
            ColumnDef::new("Detail").with_width(40),
        ];
        let mut table = DataTable::new(columns);

        // Row 1: PC/SC Daemon
        let pcscd_badge = if d.pcscd.running { "[OK]" } else { "[!!]" };
        let pcscd_detail = if d.pcscd.running {
            "Running".to_string()
        } else if cfg!(target_os = "macos") {
            "brew services start pcsc-lite".to_string()
        } else if cfg!(target_os = "linux") {
            "sudo systemctl start pcscd".to_string()
        } else if cfg!(windows) {
            "Start-Service SCardSvr (admin)".to_string()
        } else {
            "Not running".to_string()
        };
        table.add_row(vec![pcscd_badge.to_string(), "PC/SC Daemon".to_string(), pcscd_detail]);

        // Row 2: GPG Agent
        let gpg_badge = if d.gpg_agent.running { "[OK]" } else { "[!!]" };
        let gpg_detail = if d.gpg_agent.running {
            "Running".to_string()
        } else {
            "gpgconf --launch gpg-agent".to_string()
        };
        table.add_row(vec![gpg_badge.to_string(), "GPG Agent".to_string(), gpg_detail]);

        // Row 3: Scdaemon
        let scd_badge = if d.scdaemon.configured { "[OK]" } else { "[  ]" };
        let scd_detail = if d.scdaemon.configured {
            "Configured".to_string()
        } else {
            "Create ~/.gnupg/scdaemon.conf".to_string()
        };
        table.add_row(vec![scd_badge.to_string(), "Scdaemon".to_string(), scd_detail]);

        // Row 4: SSH Agent
        let ssh_badge = if d.ssh_agent.configured { "[OK]" } else { "[  ]" };
        let ssh_detail = if d.ssh_agent.configured {
            "Configured for GPG".to_string()
        } else {
            "Add enable-ssh-support to gpg-agent.conf".to_string()
        };
        table.add_row(vec![ssh_badge.to_string(), "SSH Agent".to_string(), ssh_detail]);

        widgets.push(Box::new(table));

        // Supplemental detail lines (only shown when data exists)
        if let Some(ref version) = d.pcscd.version {
            widgets.push(Box::new(Label::new(format!("  PC/SC version: {}", version))));
        }
        if let Some(ref version) = d.gpg_agent.version {
            widgets.push(Box::new(Label::new(format!("  GPG Agent version: {}", version))));
        }
        if let Some(ref socket) = d.gpg_agent.socket_path {
            widgets.push(Box::new(Label::new(format!("  GPG socket: {}", socket))));
        }
        if let Some(ref issues) = d.scdaemon.issues {
            widgets.push(Box::new(Label::new(format!("  Scdaemon issues: {}", issues))));
        }
        if let Some(ref sock) = d.ssh_agent.auth_sock {
            widgets.push(Box::new(Label::new(format!("  SSH_AUTH_SOCK: {}", sock))));
        }

        // Spacer + action button
        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Button::new("Run Diagnostics (R)")));

        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        &[
            KeyBinding {
                key: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                action: "back",
                description: "Back",
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
                key: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
                action: "run_diagnostics",
                description: "R Run",
                show: true,
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
            "back" => ctx.pop_screen_deferred(),
            "help" => {
                ctx.push_screen_deferred(Box::new(
                    PopupScreen::new("Diagnostics Help", DIAGNOSTICS_HELP_TEXT)
                ));
            }
            "run_diagnostics" => {
                // Re-run diagnostics is handled by the parent runner (app.rs)
                // For now, pop back so user can re-enter (triggers fresh diagnostics).
                // Full async refresh will be wired in subsequent plans.
                ctx.pop_screen_deferred();
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
    use crate::diagnostics::Diagnostics;

    #[tokio::test]
    async fn diagnostics_default() {
        let mut app = TestApp::new_styled(80, 24, "", || {
            Box::new(DiagnosticsScreen::new(Diagnostics::default()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }
}
