use textual_rs::{Widget, Footer, Header, Label};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::reactive::Reactive;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::YubiKeyState;
use crate::tui::widgets::popup::{ModalScreen, PopupScreen};

const PIV_HELP_TEXT: &str = "\
PIV Certificates\n\
\n\
PIV (Personal Identity Verification) is a smart card standard for\n\
storing X.509 certificates and private keys.\n\
\n\
Your YubiKey has 4 standard PIV slots:\n\
- 9a: Authentication (login, VPN)\n\
- 9c: Digital Signature (code signing, documents)\n\
- 9d: Key Management (encryption, key exchange)\n\
- 9e: Card Authentication (physical access, no PIN required)\n\
\n\
This screen shows which slots are occupied.";

#[derive(Default, Clone, PartialEq)]
pub struct PivTuiState {
    pub scroll_offset: usize,
}


/// PIV Certificates screen — shows each standard PIV slot and occupancy status.
///
/// Follows the textual-rs Widget pattern (D-01, D-07, D-15):
/// - Header("PIV Certificates")
/// - Sidebar (slot list as Labels) + hint to use V to view slot detail
/// - Footer with keybindings: Esc=back, V=view_slot, R=refresh
/// - No hardcoded Color:: values
///
/// Per UI-SPEC layout contract: sidebar (33%) = slot list, main (67%) = slot detail.
/// Since textual-rs sidebar layout is applied by the component model, we use Labels
/// to represent slot status and let the framework handle the two-column arrangement.
pub struct PivScreen {
    pub yubikey_state: Option<YubiKeyState>,
    pub state: Reactive<PivTuiState>,
}

impl PivScreen {
    pub fn new(yubikey_state: Option<YubiKeyState>) -> Self {
        PivScreen {
            yubikey_state,
            state: Reactive::new(PivTuiState::default()),
        }
    }
}

impl Widget for PivScreen {
    fn widget_type_name(&self) -> &'static str {
        "PivScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let slot_labels: &[(&str, &str)] = &[
            ("9a", "Authentication (9a)"),
            ("9c", "Digital Signature (9c)"),
            ("9d", "Key Management (9d)"),
            ("9e", "Card Authentication (9e)"),
        ];

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("PIV Certificates")),
        ];

        match &self.yubikey_state {
            Some(yk) => {
                match &yk.piv {
                    Some(piv_state) => {
                        widgets.push(Box::new(Label::new("PIV Slot Status")));
                        widgets.push(Box::new(Label::new("")));

                        for (slot_id, label) in slot_labels {
                            let occupied = piv_state.slots.iter().any(|s| s.slot == *slot_id);
                            if occupied {
                                widgets.push(Box::new(Label::new(format!(
                                    "  [OK] {} -- Occupied",
                                    label
                                ))));
                            } else {
                                widgets.push(Box::new(Label::new(format!(
                                    "  [  ] {} -- Empty",
                                    label
                                ))));
                            }
                        }

                        widgets.push(Box::new(Label::new("")));
                        widgets.push(Box::new(Label::new(
                            "Press V to view slot detail or R to refresh.",
                        )));
                    }
                    None => {
                        widgets.push(Box::new(Label::new(
                            "PIV data unavailable for this YubiKey.",
                        )));
                    }
                }
            }
            None => {
                widgets.push(Box::new(Label::new("No YubiKey Detected")));
                widgets.push(Box::new(Label::new(
                    "Insert your YubiKey and press R to refresh.",
                )));
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
                key: KeyCode::Char('v'),
                modifiers: KeyModifiers::NONE,
                action: "view_slot",
                description: "V View slot",
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
                    ModalScreen::new(Box::new(
                        PopupScreen::new("PIV Help", PIV_HELP_TEXT)
                    ))
                ));
            }
            "view_slot" => {
                // View slot detail — full implementation in subsequent plans when
                // the slot detail sub-screen is built. For now, no-op.
            }
            "refresh" => {
                // Refresh PIV state — wired in subsequent plans via async worker.
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
    use crate::model::mock::mock_yubikey_states;

    #[tokio::test]
    async fn piv_default_state() {
        let yk = mock_yubikey_states().into_iter().next();
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(PivScreen::new(yk.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn piv_no_yubikey() {
        let mut app = TestApp::new_styled(80, 24, "", || {
            Box::new(PivScreen::new(None))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }
}
