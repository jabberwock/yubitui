use textual_rs::{Widget, Footer, Header, Label, Button, DataTable, ColumnDef};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::otp::{OtpSlotStatus, OtpState};

const OTP_HELP_TEXT: &str = "\
OTP Slots\n\
\n\
Your YubiKey has two OTP (One-Time Password) slots:\n\
- Slot 1 activates on short touch (2-3 seconds)\n\
- Slot 2 activates on long touch (3-5 seconds)\n\
\n\
Each slot can hold one of: Yubico OTP (cloud-validated 44-char string),\n\
HMAC-SHA1 (challenge-response), static password, or HOTP.\n\
\n\
Note: The configured credential type cannot be read back from hardware.\n\
Only occupied/empty status is detectable via the OTP status APDU.";

/// OTP Slots screen — shows slot 1 and slot 2 occupancy status.
///
/// NOTE: The credential type (Yubico OTP, HMAC-SHA1, static password, HOTP)
/// is write-only at configuration time and cannot be read back from hardware.
/// Only occupied vs empty is detectable, consistent with the Yubico SDK.
pub struct OtpScreen {
    pub otp_state: Option<OtpState>,
    pub key_present: bool,
}

impl OtpScreen {
    pub fn new(otp_state: Option<OtpState>) -> Self {
        OtpScreen { otp_state, key_present: false }
    }

    pub fn new_with_key(otp_state: Option<OtpState>) -> Self {
        OtpScreen { otp_state, key_present: true }
    }
}

impl Widget for OtpScreen {
    fn widget_type_name(&self) -> &'static str {
        "OtpScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("OTP Slots")),
        ];

        match &self.otp_state {
            Some(state) => {
                widgets.push(Box::new(Label::new("OTP Slot Configuration")));
                widgets.push(Box::new(Label::new("")));

                let slot1_status = match &state.slot1 {
                    OtpSlotStatus::Occupied => "[OK]",
                    OtpSlotStatus::Empty => "[EMPTY]",
                };
                let slot1_config = match &state.slot1 {
                    OtpSlotStatus::Occupied => {
                        if state.slot1_touch {
                            "Configured (touch required)"
                        } else {
                            "Configured"
                        }
                    }
                    OtpSlotStatus::Empty => "Empty",
                };

                let slot2_status = match &state.slot2 {
                    OtpSlotStatus::Occupied => "[OK]",
                    OtpSlotStatus::Empty => "[EMPTY]",
                };
                let slot2_config = match &state.slot2 {
                    OtpSlotStatus::Occupied => {
                        if state.slot2_touch {
                            "Configured (touch required)"
                        } else {
                            "Configured"
                        }
                    }
                    OtpSlotStatus::Empty => "Empty",
                };

                let mut table = DataTable::new(vec![
                    ColumnDef::new("Status").with_width(10),
                    ColumnDef::new("Slot").with_width(25),
                    ColumnDef::new("Configuration").with_width(25),
                ]);
                table.add_row(vec![
                    slot1_status.to_string(),
                    "Slot 1 (short touch)".to_string(),
                    slot1_config.to_string(),
                ]);
                table.add_row(vec![
                    slot2_status.to_string(),
                    "Slot 2 (long touch)".to_string(),
                    slot2_config.to_string(),
                ]);
                widgets.push(Box::new(table));

                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Note: Credential type (Yubico OTP, HMAC-SHA1, static password)",
                )));
                widgets.push(Box::new(Label::new(
                    "cannot be read back from hardware — only occupied/empty is detectable.",
                )));

                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Button::new("Refresh (R)")));
            }
            None => {
                if self.key_present {
                    widgets.push(Box::new(Label::new("OTP status unavailable")));
                    widgets.push(Box::new(Label::new(
                        "Could not read OTP slots via PC/SC on this hardware.",
                    )));
                } else {
                    widgets.push(Box::new(Label::new("No YubiKey Detected")));
                    widgets.push(Box::new(Label::new(
                        "Insert your YubiKey and press R to refresh.",
                    )));
                }
                widgets.push(Box::new(Button::new("Refresh (R)")));
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
            "back" | "refresh" => ctx.pop_screen_deferred(),
            "help" => {
                ctx.push_screen_deferred(Box::new(
                    crate::tui::widgets::popup::PopupScreen::new("OTP Slots Help", OTP_HELP_TEXT),
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
    use crate::model::otp::{OtpState, OtpSlotStatus};

    #[tokio::test]
    async fn otp_default_state() {
        let otp_state = Some(OtpState {
            slot1: OtpSlotStatus::Occupied,
            slot2: OtpSlotStatus::Empty,
            slot1_touch: false,
            slot2_touch: false,
        });
        let mut app = TestApp::new_styled(80, 24, "", move || {
            Box::new(OtpScreen::new(otp_state.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn otp_no_yubikey() {
        let mut app = TestApp::new_styled(80, 24, "", || {
            Box::new(OtpScreen::new(None))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }
}
