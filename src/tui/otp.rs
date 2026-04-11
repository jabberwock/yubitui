use textual_rs::{Widget, Footer, Header, Label, Button, ButtonVariant, DataTable, ColumnDef, Vertical, Horizontal};
use textual_rs::widget::context::AppContext;
use textual_rs::widget::EventPropagation;
use textual_rs::event::keybinding::KeyBinding;
use crossterm::event::{KeyCode, KeyModifiers, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::cell::Cell;

use crate::model::otp::{OtpSlotStatus, OtpState, OtpCredentialType};

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
                    OtpSlotStatus::Occupied => "✓ Set",
                    OtpSlotStatus::Empty => "○ Empty",
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
                    OtpSlotStatus::Occupied => "✓ Set",
                    OtpSlotStatus::Empty => "○ Empty",
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

                // Action buttons
                widgets.push(Box::new(Horizontal::with_children(vec![
                    Box::new(Button::new("Configure Slot 1").with_action("configure_slot1")),
                    Box::new(Button::new("Configure Slot 2").with_action("configure_slot2")),
                ]).with_class("button-bar")));
                widgets.push(Box::new(Horizontal::with_children(vec![
                    Box::new(Button::new("Delete Slot 1").with_variant(ButtonVariant::Warning).with_action("delete_slot1")),
                    Box::new(Button::new("Delete Slot 2").with_variant(ButtonVariant::Warning).with_action("delete_slot2")),
                ]).with_class("button-bar")));
                widgets.push(Box::new(Button::new("Refresh (R)").with_action("refresh")));
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
                widgets.push(Box::new(Button::new("Refresh (R)").with_action("refresh")));
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
                key: KeyCode::Char('1'),
                modifiers: KeyModifiers::NONE,
                action: "configure_slot1",
                description: "1 Config Slot 1",
                show: true,
            },
            KeyBinding {
                key: KeyCode::Char('2'),
                modifiers: KeyModifiers::NONE,
                action: "configure_slot2",
                description: "2 Config Slot 2",
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
            "back" | "refresh" => ctx.pop_screen_deferred(),
            "help" => {
                ctx.push_screen_deferred(Box::new(
                    crate::tui::widgets::popup::PopupScreen::new("OTP Slots Help", OTP_HELP_TEXT),
                ));
            }
            "configure_slot1" => {
                ctx.push_screen_deferred(Box::new(OtpConfigScreen::new(1)));
            }
            "configure_slot2" => {
                ctx.push_screen_deferred(Box::new(OtpConfigScreen::new(2)));
            }
            "delete_slot1" => {
                ctx.push_screen_deferred(Box::new(
                    crate::tui::widgets::popup::ConfirmScreen::new(
                        "Delete OTP Slot 1",
                        "This will erase the credential in Slot 1 (short touch).\nThe slot will become empty.",
                        true,
                    ),
                ));
            }
            "delete_slot2" => {
                ctx.push_screen_deferred(Box::new(
                    crate::tui::widgets::popup::ConfirmScreen::new(
                        "Delete OTP Slot 2",
                        "This will erase the credential in Slot 2 (long touch).\nThe slot will become empty.",
                        true,
                    ),
                ));
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ---------------------------------------------------------------------------
// OTP Slot Configuration Screen
// ---------------------------------------------------------------------------

/// Screen for configuring an OTP slot with a specific credential type.
pub struct OtpConfigScreen {
    slot: u8,
    own_id: Cell<Option<textual_rs::WidgetId>>,
}

impl OtpConfigScreen {
    pub fn new(slot: u8) -> Self {
        Self {
            slot,
            own_id: Cell::new(None),
        }
    }

    fn program_and_show_result(&self, config: &crate::model::otp::OtpConfig, cred_type: OtpCredentialType, ctx: &AppContext) {
        match crate::model::otp::program_otp_slot(config) {
            Ok(()) => {
                ctx.pop_screen_deferred();
                ctx.push_screen_deferred(Box::new(
                    crate::tui::widgets::popup::PopupScreen::new(
                        "Slot Configured",
                        format!("OTP Slot {} programmed with {}.", self.slot, cred_type),
                    ),
                ));
            }
            Err(e) => {
                ctx.pop_screen_deferred();
                ctx.push_screen_deferred(Box::new(
                    crate::tui::widgets::popup::PopupScreen::new(
                        "Configuration Failed",
                        format!("Failed to program slot {}: {}", self.slot, e),
                    ),
                ));
            }
        }
    }
}


impl Widget for OtpConfigScreen {
    fn widget_type_name(&self) -> &'static str { "OtpConfigScreen" }

    fn on_mount(&self, id: textual_rs::WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _id: textual_rs::WidgetId) { self.own_id.set(None); }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new(if self.slot == 1 { "Configure OTP Slot 1" } else { "Configure OTP Slot 2" })),
            Box::new(Label::new("")),
            Box::new(Label::new("Select credential type:").with_class("section-title")),
            Box::new(Label::new("")),
            Box::new(Button::new("HMAC-SHA1 Challenge-Response").with_action("hmac")),
            Box::new(Label::new("  For KeePassXC, offline 2FA")),
            Box::new(Label::new("")),
            Box::new(Button::new("Yubico OTP").with_action("yubico_otp")),
            Box::new(Label::new("  Cloud-validated one-time passwords")),
            Box::new(Label::new("")),
            Box::new(Button::new("Static Password").with_action("static_pw")),
            Box::new(Label::new("  Types a fixed string on touch")),
            Box::new(Label::new("")),
            Box::new(Vertical::with_children(vec![
                Box::new(Label::new("WARNING: This overwrites any existing slot configuration.")),
            ]).with_class("status-card-warn")),
            Box::new(Footer),
        ];
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        &[
            KeyBinding { key: KeyCode::Esc, modifiers: KeyModifiers::NONE, action: "back", description: "Esc Cancel", show: true },
            KeyBinding { key: KeyCode::Char('1'), modifiers: KeyModifiers::NONE, action: "hmac", description: "1 HMAC", show: true },
            KeyBinding { key: KeyCode::Char('2'), modifiers: KeyModifiers::NONE, action: "yubico_otp", description: "2 Yubico", show: true },
            KeyBinding { key: KeyCode::Char('3'), modifiers: KeyModifiers::NONE, action: "static_pw", description: "3 Static", show: true },
        ]
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            match key.code {
                KeyCode::Esc => { ctx.pop_screen_deferred(); return EventPropagation::Stop; }
                KeyCode::Char('1') => { self.on_action("hmac", ctx); return EventPropagation::Stop; }
                KeyCode::Char('2') => { self.on_action("yubico_otp", ctx); return EventPropagation::Stop; }
                KeyCode::Char('3') => { self.on_action("static_pw", ctx); return EventPropagation::Stop; }
                _ => {}
            }
        }
        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "back" => ctx.pop_screen_deferred(),
            "hmac" => {
                let config = crate::model::otp::OtpConfig::new(self.slot, OtpCredentialType::ChallengeResponse);
                self.program_and_show_result(&config, OtpCredentialType::ChallengeResponse, ctx);
            }
            "yubico_otp" => {
                let config = crate::model::otp::OtpConfig::new(self.slot, OtpCredentialType::YubicoOtp);
                self.program_and_show_result(&config, OtpCredentialType::YubicoOtp, ctx);
            }
            "static_pw" => {
                // Push a password input screen, then program
                ctx.push_screen_deferred(Box::new(OtpStaticPwScreen::new(self.slot)));
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ---------------------------------------------------------------------------
// Static Password Input Screen
// ---------------------------------------------------------------------------

pub struct OtpStaticPwScreen {
    slot: u8,
    password: std::cell::RefCell<String>,
    error: std::cell::RefCell<Option<String>>,
    own_id: Cell<Option<textual_rs::WidgetId>>,
}

impl OtpStaticPwScreen {
    pub fn new(slot: u8) -> Self {
        Self {
            slot,
            password: std::cell::RefCell::new(String::new()),
            error: std::cell::RefCell::new(None),
            own_id: Cell::new(None),
        }
    }
    fn recompose(&self, ctx: &AppContext) {
        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
    }
}

impl Widget for OtpStaticPwScreen {
    fn widget_type_name(&self) -> &'static str { "OtpStaticPwScreen" }
    fn on_mount(&self, id: textual_rs::WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _id: textual_rs::WidgetId) { self.own_id.set(None); }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let pw = self.password.borrow();
        let err = self.error.borrow().clone();
        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new(if self.slot == 1 { "Static Password — Slot 1" } else { "Static Password — Slot 2" })),
            Box::new(Label::new("")),
            Box::new(Vertical::with_children(vec![
                Box::new(Label::new("Enter the static password to program (max 38 chars):")),
                Box::new(Label::new(format!("> {}_", *pw))),
                Box::new(Label::new(format!("  ({}/38 chars)", pw.len()))),
            ]).with_class("status-card")),
        ];
        if let Some(e) = err {
            widgets.push(Box::new(Label::new(format!("Error: {}", e))));
        }
        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Button::new("Program Static Password").with_action("submit")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        &[
            KeyBinding { key: KeyCode::Esc, modifiers: KeyModifiers::NONE, action: "back", description: "Esc Cancel", show: true },
            KeyBinding { key: KeyCode::Enter, modifiers: KeyModifiers::NONE, action: "submit", description: "Enter Program", show: true },
        ]
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            match key.code {
                KeyCode::Esc => { ctx.pop_screen_deferred(); return EventPropagation::Stop; }
                KeyCode::Backspace => {
                    self.password.borrow_mut().pop();
                    self.recompose(ctx);
                    return EventPropagation::Stop;
                }
                KeyCode::Enter => { self.on_action("submit", ctx); return EventPropagation::Stop; }
                KeyCode::Char(c) => {
                    if self.password.borrow().len() < 38 {
                        self.password.borrow_mut().push(c);
                        self.recompose(ctx);
                    }
                    return EventPropagation::Stop;
                }
                _ => {}
            }
        }
        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "back" => ctx.pop_screen_deferred(),
            "submit" => {
                let pw = self.password.borrow().clone();
                if pw.is_empty() {
                    *self.error.borrow_mut() = Some("Password cannot be empty.".to_string());
                    self.recompose(ctx);
                    return;
                }
                let mut config = crate::model::otp::OtpConfig::new(self.slot, OtpCredentialType::StaticPassword);
                config.static_password = Some(pw);
                match crate::model::otp::program_otp_slot(&config) {
                    Ok(()) => {
                        ctx.pop_screen_deferred(); // pop OtpStaticPwScreen
                        ctx.pop_screen_deferred(); // pop OtpConfigScreen
                        ctx.push_screen_deferred(Box::new(
                            crate::tui::widgets::popup::PopupScreen::new(
                                "Slot Configured",
                                format!("OTP Slot {} programmed with Static Password.", self.slot),
                            ),
                        ));
                    }
                    Err(e) => {
                        *self.error.borrow_mut() = Some(e.to_string());
                        self.password.borrow_mut().clear();
                        self.recompose(ctx);
                    }
                }
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
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(OtpScreen::new(otp_state.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn otp_no_yubikey() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || {
            Box::new(OtpScreen::new(None))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }
}
