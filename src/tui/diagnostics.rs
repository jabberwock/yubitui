use textual_rs::{Widget, Footer, Header, Label, Button, Horizontal, Vertical};
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
    #[allow(dead_code)]
    own_id: std::cell::Cell<Option<textual_rs::WidgetId>>,
}

impl DiagnosticsScreen {
    pub fn new(diagnostics: Diagnostics) -> Self {
        DiagnosticsScreen {
            diagnostics,
            state: Reactive::new(DiagnosticsTuiState::default()),
            own_id: std::cell::Cell::new(None),
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
            Box::new(Label::new("")),
        ];

        // ── Row 1: PC/SC + GPG Agent ────────────────────────────────
        let pcscd_class = if d.pcscd.running { "diag-card-ok" } else { "diag-card-error" };
        let pcscd_status = if d.pcscd.running { "✓ Running" } else { "✗ Down" };
        let pcscd_detail = if d.pcscd.running {
            d.pcscd.version.as_deref().unwrap_or("").to_string()
        } else if cfg!(target_os = "macos") {
            "brew services start pcsc-lite".to_string()
        } else if cfg!(target_os = "linux") {
            "sudo systemctl start pcscd".to_string()
        } else {
            "Not running".to_string()
        };

        let gpg_class = if d.gpg_agent.running { "diag-card-ok" } else { "diag-card-error" };
        let gpg_status = if d.gpg_agent.running { "✓ Running" } else { "✗ Down" };
        let gpg_detail = if d.gpg_agent.running {
            d.gpg_agent.version.as_deref().unwrap_or("").to_string()
        } else {
            "gpgconf --launch gpg-agent".to_string()
        };

        widgets.push(Box::new(Horizontal::with_children(vec![
            Box::new(Vertical::with_children(vec![
                Box::new(Label::new("PC/SC Daemon").with_class("section-title")),
                Box::new(Label::new(pcscd_status)),
                Box::new(Label::new(pcscd_detail)),
            ]).with_class("diag-card").with_class(pcscd_class)),
            Box::new(Vertical::with_children(vec![
                Box::new(Label::new("GPG Agent").with_class("section-title")),
                Box::new(Label::new(gpg_status)),
                Box::new(Label::new(gpg_detail)),
            ]).with_class("diag-card").with_class(gpg_class)),
        ]).with_class("status-row")));

        // ── Row 2: Scdaemon + SSH Agent ─────────────────────────────
        let scd_class = if d.scdaemon.configured { "diag-card-ok" } else { "diag-card-warn" };
        let scd_status = if d.scdaemon.configured { "✓ Configured" } else { "○ Not configured" };
        let scd_detail = if d.scdaemon.configured {
            String::new()
        } else {
            "Create ~/.gnupg/scdaemon.conf".to_string()
        };

        let ssh_class = if d.ssh_agent.configured { "diag-card-ok" } else { "diag-card-warn" };
        let ssh_status = if d.ssh_agent.configured { "✓ GPG-enabled" } else { "○ Not configured" };
        let ssh_detail = if d.ssh_agent.configured {
            d.ssh_agent.auth_sock.as_deref().unwrap_or("").to_string()
        } else {
            "Add enable-ssh-support to gpg-agent.conf".to_string()
        };

        widgets.push(Box::new(Horizontal::with_children(vec![
            Box::new(Vertical::with_children(vec![
                Box::new(Label::new("Scdaemon").with_class("section-title")),
                Box::new(Label::new(scd_status)),
                Box::new(Label::new(scd_detail)),
            ]).with_class("diag-card").with_class(scd_class)),
            Box::new(Vertical::with_children(vec![
                Box::new(Label::new("SSH Agent").with_class("section-title")),
                Box::new(Label::new(ssh_status)),
                Box::new(Label::new(ssh_detail)),
            ]).with_class("diag-card").with_class(ssh_class)),
        ]).with_class("status-row")));

        // Supplemental details
        if let Some(ref socket) = d.gpg_agent.socket_path {
            widgets.push(Box::new(Label::new(format!("GPG socket: {}", socket))));
        }
        if let Some(ref issues) = d.scdaemon.issues {
            widgets.push(Box::new(Label::new(format!("Scdaemon issues: {}", issues))));
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Button::new("Run Diagnostics (R)").with_action("run_diagnostics")));

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
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || {
            Box::new(DiagnosticsScreen::new(Diagnostics::default()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }
}
