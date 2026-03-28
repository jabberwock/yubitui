use textual_rs::{Widget, Footer, Header, Label};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::otp::{OtpSlotStatus, OtpState};

/// OTP Slots screen — shows slot 1 and slot 2 occupancy status.
///
/// NOTE: The credential type (Yubico OTP, HMAC-SHA1, static password, HOTP)
/// is write-only at configuration time and cannot be read back from hardware.
/// Only occupied vs empty is detectable, consistent with the Yubico SDK.
pub struct OtpScreen {
    pub otp_state: Option<OtpState>,
}

impl OtpScreen {
    pub fn new(otp_state: Option<OtpState>) -> Self {
        OtpScreen { otp_state }
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

                // Slot 1 (short touch)
                let slot1_label = match &state.slot1 {
                    OtpSlotStatus::Occupied => {
                        if state.slot1_touch {
                            "  Slot 1 (short touch): Configured (touch required)".to_string()
                        } else {
                            "  Slot 1 (short touch): Configured".to_string()
                        }
                    }
                    OtpSlotStatus::Empty => "  Slot 1 (short touch): Empty".to_string(),
                };
                widgets.push(Box::new(Label::new(slot1_label)));

                // Slot 2 (long touch)
                let slot2_label = match &state.slot2 {
                    OtpSlotStatus::Occupied => {
                        if state.slot2_touch {
                            "  Slot 2 (long touch): Configured (touch required)".to_string()
                        } else {
                            "  Slot 2 (long touch): Configured".to_string()
                        }
                    }
                    OtpSlotStatus::Empty => "  Slot 2 (long touch): Empty".to_string(),
                };
                widgets.push(Box::new(Label::new(slot2_label)));

                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new(
                    "Note: Credential type (Yubico OTP, HMAC-SHA1, static password)",
                )));
                widgets.push(Box::new(Label::new(
                    "cannot be read back from hardware — only occupied/empty is detectable.",
                )));
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
                key: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
                action: "refresh",
                description: "R Refresh",
                show: true,
            },
            KeyBinding {
                key: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                action: "back",
                description: "Q Back",
                show: false,
            },
        ]
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "back" | "refresh" => ctx.pop_screen_deferred(),
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
    use crate::model::otp::{OtpState, OtpSlotStatus};

    #[tokio::test]
    async fn otp_default_state() {
        let otp_state = Some(OtpState {
            slot1: OtpSlotStatus::Occupied,
            slot2: OtpSlotStatus::Empty,
            slot1_touch: false,
            slot2_touch: false,
        });
        let mut app = TestApp::new(80, 24, move || {
            Box::new(OtpScreen::new(otp_state.clone()))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn otp_no_yubikey() {
        let mut app = TestApp::new(80, 24, || {
            Box::new(OtpScreen::new(None))
        });
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }
}
