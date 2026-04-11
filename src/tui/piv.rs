use std::cell::{Cell, RefCell};

use textual_rs::{Widget, Footer, Header, Label, Button, ButtonVariant, DataTable, ColumnDef, WidgetId, Markdown};
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
        key: KeyCode::Char('m'),
        modifiers: KeyModifiers::NONE,
        action: "change_mgmt_key",
        description: "M Mgmt Key",
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
                        // PIV slot list as DataTable — show algorithm when available
                        let columns = vec![
                            ColumnDef::new("").with_width(2),
                            ColumnDef::new("Status").with_width(7),
                            ColumnDef::new("Slot").with_width(26),
                            ColumnDef::new("Algorithm").with_width(12),
                            ColumnDef::new("Subject").with_width(20),
                        ];
                        let mut table = DataTable::new(columns);

                        for (idx, (slot_id, slot_label)) in slot_defs.iter().enumerate() {
                            let slot_info = piv_state.slots.iter().find(|s| s.slot == *slot_id);
                            let occupied = slot_info.is_some();
                            let cursor = if idx == selected { ">" } else { " " };
                            let status = if occupied { "✓ Set" } else { "○ Empty" };
                            let algorithm = slot_info
                                .and_then(|s| s.algorithm.as_deref())
                                .unwrap_or("-");
                            let subject = slot_info
                                .and_then(|s| s.subject.as_deref())
                                .unwrap_or("-");
                            table.add_row(vec![
                                cursor.to_string(),
                                status.to_string(),
                                slot_label.to_string(),
                                algorithm.to_string(),
                                subject.to_string(),
                            ]);
                        }

                        widgets.push(Box::new(table));

                        // PIV-04: warn when management key is still factory default
                        if piv_state.mgmt_key_is_default {
                            widgets.push(Box::new(Label::new("")));
                            widgets.push(Box::new(Label::new(
                                "⚠ Management key is factory default — change it to secure your PIV applet.",
                            )));
                        }

                        widgets.push(Box::new(Label::new("")));
                        widgets.push(Box::new(Button::new("View Certificate").with_action("view_slot")));
                        widgets.push(Box::new(Button::new("Delete Slot").with_variant(ButtonVariant::Warning).with_action("delete_slot")));
                        widgets.push(Box::new(Button::new("Change Management Key").with_action("change_mgmt_key")));
                        widgets.push(Box::new(Button::new("Refresh").with_action("refresh")));
                    }
                    None => {
                        widgets.push(Box::new(Markdown::new(
                            "## PIV Data Unavailable\n\nCould not read PIV data from this YubiKey.\n\nTry pressing **R** to refresh.",
                        )));
                    }
                }
            }
            None => {
                widgets.push(Box::new(Markdown::new(
                    "## No YubiKey Detected\n\nInsert your YubiKey and press **R** to refresh.",
                )));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Button::new("Refresh").with_action("refresh")));
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
                let selected = self.state.get_untracked().selected_slot;
                let slot_str = PIV_SLOT_IDS[selected];

                let slot_info = self
                    .yubikey_state
                    .as_ref()
                    .and_then(|yk| yk.piv.as_ref())
                    .and_then(|piv| piv.slots.iter().find(|s| s.slot == slot_str))
                    .cloned();

                let body = match slot_info {
                    None => format!("Slot {} is empty — no certificate present.", slot_str),
                    Some(info) => {
                        let mut lines = Vec::new();
                        lines.push(format!("Slot:      {}", slot_str.to_uppercase()));
                        if let Some(subj) = &info.subject {
                            lines.push(format!("Subject:   {}", subj));
                        }
                        if let Some(issuer) = &info.issuer {
                            lines.push(format!("Issuer:    {}", issuer));
                        }
                        if let Some(alg) = &info.algorithm {
                            lines.push(format!("Algorithm: {}", alg));
                        }
                        if let Some(val) = &info.validity {
                            lines.push(format!("Valid:     {}", val));
                        }
                        if lines.len() == 1 {
                            // Only slot header — cert was present but no parsed fields
                            lines.push("Certificate present (no parsed details available).".to_string());
                        }
                        lines.join("\n")
                    }
                };

                let title = format!("PIV Slot {} Certificate", slot_str.to_uppercase());
                ctx.push_screen_deferred(Box::new(PopupScreen::new(title, body)));
            }

            "change_mgmt_key" => {
                let is_default = self
                    .yubikey_state
                    .as_ref()
                    .and_then(|yk| yk.piv.as_ref())
                    .map(|piv| piv.mgmt_key_is_default)
                    .unwrap_or(false);
                ctx.push_screen_deferred(Box::new(ChangeMgmtKeyScreen::new(is_default)));
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
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
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
// ChangeMgmtKeyScreen — enter current management key
// ============================================================================

/// Step 1 of the management key change flow.
///
/// Prompts the user for the current management key (48 hex chars).
/// Empty input uses the factory default (`01020304...`).
///
/// If `is_default` is true we display a hint that the current key is still
/// factory default and the user can just press Enter.
pub struct ChangeMgmtKeyScreen {
    is_default: bool,
    input: RefCell<String>,
    error: RefCell<Option<String>>,
    own_id: Cell<Option<WidgetId>>,
}

impl ChangeMgmtKeyScreen {
    pub fn new(is_default: bool) -> Self {
        Self {
            is_default,
            input: RefCell::new(String::new()),
            error: RefCell::new(None),
            own_id: Cell::new(None),
        }
    }
}

static CHANGE_KEY_BINDINGS: &[KeyBinding] = &[
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

impl Widget for ChangeMgmtKeyScreen {
    fn widget_type_name(&self) -> &'static str {
        "ChangeMgmtKeyScreen"
    }

    fn on_mount(&self, id: WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _id: WidgetId) { self.own_id.set(None); }

    fn can_focus(&self) -> bool { true }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let input = self.input.borrow().clone();
        let error = self.error.borrow().clone();

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("Change PIV Management Key (1/2)")),
            Box::new(Label::new("")),
            Box::new(Label::new("Step 1: Enter your current management key.")),
            Box::new(Label::new("")),
        ];

        if self.is_default {
            widgets.push(Box::new(Label::new(
                "Your management key is currently the factory default.",
            )));
            widgets.push(Box::new(Label::new(
                "Press Enter with empty input to use the default key.",
            )));
        } else {
            widgets.push(Box::new(Label::new(
                "Enter current management key (48 hex chars = 24 bytes):",
            )));
            widgets.push(Box::new(Label::new(
                "Press Enter with empty input to try the default key.",
            )));
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Label::new(format!("> {}_", input))));

        if let Some(err) = error {
            widgets.push(Box::new(Label::new("")));
            widgets.push(Box::new(Label::new(format!("Error: {}", err))));
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        CHANGE_KEY_BINDINGS
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
                    if c.is_ascii_hexdigit() && self.input.borrow().len() < 48 {
                        self.input.borrow_mut().push(c);
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
                let current_key: [u8; 24] = if input.is_empty() {
                    *PIV_DEFAULT_MGMT_KEY_3DES
                } else if input.len() != 48 {
                    *self.error.borrow_mut() = Some(format!(
                        "Key must be 48 hex characters (24 bytes). Got {}.",
                        input.len()
                    ));
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return;
                } else {
                    let mut bytes = [0u8; 24];
                    for (i, chunk) in input.as_bytes().chunks(2).enumerate() {
                        let hi = (chunk[0] as char).to_digit(16).unwrap_or(0) as u8;
                        let lo = (chunk[1] as char).to_digit(16).unwrap_or(0) as u8;
                        bytes[i] = (hi << 4) | lo;
                    }
                    bytes
                };

                *self.error.borrow_mut() = None;
                ctx.push_screen_deferred(Box::new(NewMgmtKeyScreen::new(current_key)));
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ============================================================================
// NewMgmtKeyScreen — enter and confirm new management key
// ============================================================================

/// Step 2 of the management key change flow.
///
/// Prompts for the new management key (48 hex chars).
/// On Enter, executes `change_piv_management_key()` immediately and reports
/// success or error via a `PopupScreen`.
pub struct NewMgmtKeyScreen {
    current_key: [u8; 24],
    input: RefCell<String>,
    error: RefCell<Option<String>>,
    own_id: Cell<Option<WidgetId>>,
}

impl NewMgmtKeyScreen {
    pub fn new(current_key: [u8; 24]) -> Self {
        Self {
            current_key,
            input: RefCell::new(String::new()),
            error: RefCell::new(None),
            own_id: Cell::new(None),
        }
    }
}

impl Widget for NewMgmtKeyScreen {
    fn widget_type_name(&self) -> &'static str {
        "NewMgmtKeyScreen"
    }

    fn on_mount(&self, id: WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _id: WidgetId) { self.own_id.set(None); }

    fn can_focus(&self) -> bool { true }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let input = self.input.borrow().clone();
        let error = self.error.borrow().clone();

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("Change PIV Management Key (2/2)")),
            Box::new(Label::new("")),
            Box::new(Label::new("Step 2: Enter your new management key (48 hex chars = 24 bytes).")),
            Box::new(Label::new("")),
            Box::new(Label::new("Use a random value — do not reuse the default key.")),
            Box::new(Label::new("Store it in a password manager before confirming.")),
            Box::new(Label::new("")),
            Box::new(Label::new(format!("> {}_", input))),
            Box::new(Label::new(format!("  ({}/48 chars entered)", input.len()))),
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
        CHANGE_KEY_BINDINGS
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
                    if c.is_ascii_hexdigit() && self.input.borrow().len() < 48 {
                        self.input.borrow_mut().push(c);
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
                if input.len() != 48 {
                    *self.error.borrow_mut() = Some(format!(
                        "New key must be exactly 48 hex characters (24 bytes). Got {}.",
                        input.len()
                    ));
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return;
                }

                let mut new_key = [0u8; 24];
                for (i, chunk) in input.as_bytes().chunks(2).enumerate() {
                    let hi = (chunk[0] as char).to_digit(16).unwrap_or(0) as u8;
                    let lo = (chunk[1] as char).to_digit(16).unwrap_or(0) as u8;
                    new_key[i] = (hi << 4) | lo;
                }

                match crate::model::piv_delete::change_piv_management_key(&self.current_key, &new_key) {
                    Ok(()) => {
                        // Pop NewMgmtKeyScreen and ChangeMgmtKeyScreen
                        ctx.pop_screen_deferred();
                        ctx.pop_screen_deferred();
                        ctx.push_screen_deferred(Box::new(PopupScreen::new(
                            "Success",
                            "PIV management key changed successfully.\nStore the new key in a safe place.",
                        )));
                    }
                    Err(e) => {
                        ctx.pop_screen_deferred();
                        ctx.push_screen_deferred(Box::new(PopupScreen::new(
                            "Error",
                            format!("Failed to change management key: {}", e),
                        )));
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
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(PivScreen::new(yk.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn piv_no_yubikey() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || {
            Box::new(PivScreen::new(None))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn piv_change_mgmt_key_screen() {
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, || {
            Box::new(ChangeMgmtKeyScreen::new(true))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn piv_new_mgmt_key_screen() {
        let current = *crate::model::piv_delete::PIV_DEFAULT_MGMT_KEY_3DES;
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(NewMgmtKeyScreen::new(current))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn piv_view_cert_popup() {
        let yk = mock_yubikey_states().into_iter().next();
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(PivScreen::new(yk.clone()))
        });
        let mut pilot = app.pilot();
        // Press 'v' to open the cert detail popup for slot 9a (pre-selected)
        pilot.press(crossterm::event::KeyCode::Char('v')).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_snapshot!(app.backend());
    }
}
