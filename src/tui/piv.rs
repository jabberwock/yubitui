use std::cell::{Cell, RefCell};

use textual_rs::{Widget, Footer, Header, Label, Button, DataTable, ColumnDef, WidgetId};
use textual_rs::widget::context::AppContext;
use textual_rs::widget::EventPropagation;
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::reactive::Reactive;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::YubiKeyState;
use crate::model::piv_delete::{PivSlot, PIV_DEFAULT_MGMT_KEY_3DES};
use crate::tui::widgets::popup::{ConfirmScreen, PopupScreen};

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

/// Ordered list of slot IDs shown on the PIV screen.
static PIV_SLOT_IDS: &[&str] = &["9a", "9c", "9d", "9e"];

// ============================================================================
// TUI State
// ============================================================================

#[derive(Default, Clone, PartialEq)]
pub struct PivTuiState {
    pub scroll_offset: usize,
    /// Index into PIV_SLOT_IDS for the currently selected slot.
    pub selected_slot: usize,
}

// ============================================================================
// Key Bindings
// ============================================================================

static PIV_BINDINGS: &[KeyBinding] = &[
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
        key: KeyCode::Up,
        modifiers: KeyModifiers::NONE,
        action: "up",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Down,
        modifiers: KeyModifiers::NONE,
        action: "down",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('k'),
        modifiers: KeyModifiers::NONE,
        action: "up",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('j'),
        modifiers: KeyModifiers::NONE,
        action: "down",
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
        key: KeyCode::Char('d'),
        modifiers: KeyModifiers::NONE,
        action: "delete_slot",
        description: "D Delete",
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
];

// ============================================================================
// PivScreen Widget
// ============================================================================

/// PIV Certificates screen — shows each standard PIV slot and occupancy status.
///
/// Follows the textual-rs Widget pattern (D-01, D-07, D-15):
/// - Header("PIV Certificates")
/// - Sidebar (slot list as Labels) + hint to use V to view slot detail
/// - Footer with keybindings: Esc=back, D=delete, V=view_slot, R=refresh
/// - No hardcoded Color:: values
///
/// Per UI-SPEC layout contract: sidebar (33%) = slot list, main (67%) = slot detail.
/// Since textual-rs sidebar layout is applied by the component model, we use Labels
/// to represent slot status and let the framework handle the two-column arrangement.
pub struct PivScreen {
    pub yubikey_state: Option<YubiKeyState>,
    pub state: Reactive<PivTuiState>,
    own_id: Cell<Option<textual_rs::WidgetId>>,
}

impl PivScreen {
    pub fn new(yubikey_state: Option<YubiKeyState>) -> Self {
        PivScreen {
            yubikey_state,
            state: Reactive::new(PivTuiState::default()),
            own_id: Cell::new(None),
        }
    }
}

impl Widget for PivScreen {
    fn widget_type_name(&self) -> &'static str {
        "PivScreen"
    }

    fn on_mount(&self, id: textual_rs::WidgetId) {
        self.own_id.set(Some(id));
    }

    fn on_unmount(&self, _id: textual_rs::WidgetId) {
        self.own_id.set(None);
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let slot_defs: &[(&str, &str)] = &[
            ("9a", "Authentication (9a)"),
            ("9c", "Digital Signature (9c)"),
            ("9d", "Key Management (9d)"),
            ("9e", "Card Authentication (9e)"),
        ];

        let selected = self.state.get_untracked().selected_slot;

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("PIV Certificates")),
        ];

        match &self.yubikey_state {
            Some(yk) => {
                match &yk.piv {
                    Some(piv_state) => {
                        // PIV slot list as DataTable
                        let columns = vec![
                            ColumnDef::new("").with_width(2),
                            ColumnDef::new("Status").with_width(7),
                            ColumnDef::new("Slot").with_width(30),
                            ColumnDef::new("Occupancy").with_width(9),
                        ];
                        let mut table = DataTable::new(columns);

                        for (idx, (slot_id, slot_label)) in slot_defs.iter().enumerate() {
                            let occupied = piv_state.slots.iter().any(|s| s.slot == *slot_id);
                            let cursor = if idx == selected { ">" } else { " " };
                            let status = if occupied { "[OK]" } else { "[EMPTY]" };
                            let occupancy = if occupied { "Occupied" } else { "Empty" };
                            table.add_row(vec![
                                cursor.to_string(),
                                status.to_string(),
                                slot_label.to_string(),
                                occupancy.to_string(),
                            ]);
                        }

                        widgets.push(Box::new(table));
                        widgets.push(Box::new(Label::new("")));
                        widgets.push(Box::new(Button::new("[V] View Slot")));
                        widgets.push(Box::new(Button::new("[D] Delete Slot")));
                        widgets.push(Box::new(Button::new("[R] Refresh")));
                    }
                    None => {
                        widgets.push(Box::new(Label::new(
                            "PIV data unavailable for this YubiKey.",
                        )));
                    }
                }
            }
            None => {
                widgets.push(Box::new(Label::new("No YubiKey detected.")));
                widgets.push(Box::new(Label::new(
                    "Insert your YubiKey and press R to refresh.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Button::new("[R] Refresh")));
            }
        }

        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        PIV_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            for binding in PIV_BINDINGS {
                if binding.matches(key.code, key.modifiers) {
                    self.on_action(binding.action, ctx);
                    return EventPropagation::Stop;
                }
            }
        }
        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "back" => ctx.pop_screen_deferred(),

            "up" => {
                let current = self.state.get_untracked().selected_slot;
                if current > 0 {
                    self.state.update(|s| s.selected_slot = current - 1);
                    if let Some(id) = self.own_id.get() {
                        ctx.request_recompose(id);
                    }
                }
            }

            "down" => {
                let current = self.state.get_untracked().selected_slot;
                if current + 1 < PIV_SLOT_IDS.len() {
                    self.state.update(|s| s.selected_slot = current + 1);
                    if let Some(id) = self.own_id.get() {
                        ctx.request_recompose(id);
                    }
                }
            }

            "delete_slot" => {
                let selected = self.state.get_untracked().selected_slot;
                let slot_str = PIV_SLOT_IDS[selected];

                // Check if slot is occupied
                let occupied = self
                    .yubikey_state
                    .as_ref()
                    .and_then(|yk| yk.piv.as_ref())
                    .map(|piv| piv.slots.iter().any(|s| s.slot == slot_str))
                    .unwrap_or(false);

                if !occupied {
                    ctx.push_screen_deferred(Box::new(PopupScreen::new(
                        "Empty Slot",
                        "No certificate or key to delete in this slot.",
                    )));
                    return;
                }

                let piv_slot = match PivSlot::from_slot_str(slot_str) {
                    Some(s) => s,
                    None => return,
                };

                let firmware = self
                    .yubikey_state
                    .as_ref()
                    .map(|yk| yk.info.version.clone())
                    .unwrap_or(crate::model::Version { major: 5, minor: 0, patch: 0 });

                ctx.push_screen_deferred(Box::new(MgmtKeyThenDeleteScreen::new(piv_slot, firmware)));
            }

            "help" => {
                ctx.push_screen_deferred(Box::new(PopupScreen::new("PIV Help", PIV_HELP_TEXT)));
            }

            "view_slot" => {
                // View slot detail — full implementation in subsequent plans.
            }

            "refresh" => {
                // Re-detect YubiKey state from hardware and push fresh PivScreen
                let fresh_yk = crate::model::YubiKeyState::detect_all()
                    .ok()
                    .and_then(|mut v| if v.is_empty() { None } else { Some(v.remove(0)) });
                ctx.pop_screen_deferred();
                ctx.push_screen_deferred(Box::new(PivScreen::new(fresh_yk)));
            }

            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ============================================================================
// MgmtKeyThenDeleteScreen — collect management key, then proceed to confirm
// ============================================================================

/// Input screen for entering the PIV management key (hex), then pushing the
/// DeletePivConfirmScreen.
///
/// - Shows a hex input prompt with cursor.
/// - Pre-fills nothing (empty input = use default key on Enter).
/// - On Enter with empty input: uses PIV_DEFAULT_MGMT_KEY_3DES.
/// - On Enter with 48 hex chars: parses and proceeds.
/// - Validates hex and length before proceeding.
pub struct MgmtKeyThenDeleteScreen {
    slot: PivSlot,
    firmware: crate::model::Version,
    input: RefCell<String>,
    error: RefCell<Option<String>>,
    own_id: Cell<Option<WidgetId>>,
}

impl MgmtKeyThenDeleteScreen {
    pub fn new(slot: PivSlot, firmware: crate::model::Version) -> Self {
        Self {
            slot,
            firmware,
            input: RefCell::new(String::new()),
            error: RefCell::new(None),
            own_id: Cell::new(None),
        }
    }
}

static MGMT_KEY_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "cancel",
        description: "Esc Cancel",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "next",
        description: "Enter Next",
        show: true,
    },
];

impl Widget for MgmtKeyThenDeleteScreen {
    fn widget_type_name(&self) -> &'static str {
        "MgmtKeyThenDeleteScreen"
    }

    fn on_mount(&self, id: WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _id: WidgetId) { self.own_id.set(None); }

    fn can_focus(&self) -> bool {
        true
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let input = self.input.borrow().clone();
        let error = self.error.borrow().clone();

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("PIV Management Key")),
            Box::new(Label::new("")),
            Box::new(Label::new(format!("Delete slot: {}", self.slot.display_name()))),
            Box::new(Label::new("")),
            Box::new(Label::new("Enter PIV Management Key (48 hex chars = 24 bytes):")),
            Box::new(Label::new("Press Enter with empty input to use the default key.")),
            Box::new(Label::new("")),
            Box::new(Label::new(format!("> {}_", input))),
        ];

        if let Some(err) = error {
            widgets.push(Box::new(Label::new("")));
            widgets.push(Box::new(Label::new(format!("Error: {}", err))));
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        MGMT_KEY_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            match key.code {
                KeyCode::Esc => {
                    ctx.pop_screen_deferred();
                    return EventPropagation::Stop;
                }
                KeyCode::Enter => {
                    self.on_action("next", ctx);
                    return EventPropagation::Stop;
                }
                KeyCode::Backspace => {
                    self.input.borrow_mut().pop();
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c) => {
                    // Only accept hex characters (0-9, a-f, A-F)
                    if c.is_ascii_hexdigit() {
                        let len = self.input.borrow().len();
                        if len < 48 {
                            self.input.borrow_mut().push(c);
                        }
                    }
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return EventPropagation::Stop;
                }
                _ => {}
            }
        }
        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "cancel" => ctx.pop_screen_deferred(),
            "next" => {
                let input = self.input.borrow().clone();
                let key: [u8; 24] = if input.is_empty() {
                    *PIV_DEFAULT_MGMT_KEY_3DES
                } else if input.len() != 48 {
                    *self.error.borrow_mut() = Some(format!(
                        "Management key must be 48 hex characters (24 bytes). Got {} chars.",
                        input.len()
                    ));
                    return;
                } else {
                    // Parse 48 hex chars -> 24 bytes
                    let mut bytes = [0u8; 24];
                    for (i, chunk) in input.as_bytes().chunks(2).enumerate() {
                        let hi = (chunk[0] as char).to_digit(16).unwrap_or(0) as u8;
                        let lo = (chunk[1] as char).to_digit(16).unwrap_or(0) as u8;
                        bytes[i] = (hi << 4) | lo;
                    }
                    bytes
                };

                *self.error.borrow_mut() = None;

                ctx.push_screen_deferred(Box::new(DeletePivConfirmScreen::new(self.slot.clone(), key, self.firmware.clone())));
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ============================================================================
// DeletePivConfirmScreen — confirm deletion with firmware-gated messaging
// ============================================================================

/// Confirmation screen for deleting PIV slot contents.
///
/// Shows firmware-gated messaging:
/// - Firmware >= 5.7.0: "Both certificate and key will be deleted."
/// - Firmware < 5.7.0: "Certificate will be deleted. Key cannot be removed on firmware X.Y.Z."
///
/// On confirm: calls delete_piv_slot, pops screens, pushes fresh PivScreen + success popup.
pub struct DeletePivConfirmScreen {
    slot: PivSlot,
    key: [u8; 24],
    firmware: crate::model::Version,
    inner: ConfirmScreen,
}

impl DeletePivConfirmScreen {
    pub fn new(
        slot: PivSlot,
        key: [u8; 24],
        firmware: crate::model::Version,
    ) -> Self {
        let firmware_message = if firmware.major > 5
            || (firmware.major == 5 && firmware.minor >= 7)
        {
            "Both certificate and key will be deleted.".to_string()
        } else {
            format!(
                "Certificate will be deleted.\nKey material cannot be removed on firmware {}.{}.{} (requires 5.7+).",
                firmware.major, firmware.minor, firmware.patch
            )
        };

        let body = format!(
            "Permanently delete contents of PIV slot {}?\n\n{}\n\nThis cannot be undone.",
            slot.display_name(),
            firmware_message
        );

        let inner = ConfirmScreen::new("Delete PIV Slot", body, true);

        Self {
            slot,
            key,
            firmware,
            inner,
        }
    }
}

impl Widget for DeletePivConfirmScreen {
    fn widget_type_name(&self) -> &'static str {
        "DeletePivConfirmScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        self.inner.compose()
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        self.inner.key_bindings()
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "confirm" => {
                match crate::model::piv_delete::delete_piv_slot(
                    &self.slot,
                    &self.key,
                    &self.firmware,
                ) {
                    Ok(msg) => {
                        // Pop DeletePivConfirmScreen (self), MgmtKeyThenDeleteScreen, and
                        // parent PivScreen from the stack to return to the previous screen.
                        ctx.pop_screen_deferred();
                        ctx.pop_screen_deferred();
                        ctx.pop_screen_deferred();

                        // Re-detect full YubiKey state so PivScreen shows updated slot data
                        let fresh_yk = crate::model::YubiKeyState::detect_all()
                            .ok()
                            .and_then(|mut v| if v.is_empty() { None } else { Some(v.remove(0)) });
                        ctx.push_screen_deferred(Box::new(PivScreen::new(fresh_yk)));
                        ctx.push_screen_deferred(Box::new(PopupScreen::new("Success", msg)));
                    }
                    Err(e) => {
                        ctx.pop_screen_deferred();
                        ctx.push_screen_deferred(Box::new(PopupScreen::new("Error", format!("Delete failed: {}", e))));
                    }
                }
            }
            "cancel" => ctx.pop_screen_deferred(),
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ============================================================================
// Tests
// ============================================================================

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
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn piv_no_yubikey() {
        let mut app = TestApp::new_styled(80, 24, "", || {
            Box::new(PivScreen::new(None))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }
}
