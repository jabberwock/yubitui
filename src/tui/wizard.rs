//! Provisioning Wizards — guided multi-step flows for first-time setup.
//!
//! WIZARD-01: InitialSetupWizardScreen — walks through FIDO2 PIN, first OATH
//!            account, and PIV/SSH key configuration with device state visible.
//! WIZARD-02: SshTouchPolicyWizardScreen — selects touch policy before key gen.
//! WIZARD-03: Touch policy descriptions in plain language (surfaced in SSH wizard).
//! WIZARD-05: Each wizard step shows current device state.

use std::cell::Cell;

use textual_rs::{Widget, Header, Footer, Label, Button, WidgetId};
use textual_rs::widget::context::AppContext;
use textual_rs::widget::EventPropagation;
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::reactive::Reactive;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::YubiKeyState;

// ============================================================================
// WizardMenuScreen — choose which wizard to launch
// ============================================================================

/// Launch pad shown when the user presses 'W' from the dashboard.
pub struct WizardMenuScreen {
    yubikey_state: Option<YubiKeyState>,
    own_id: Cell<Option<WidgetId>>,
}

impl WizardMenuScreen {
    pub fn new(yubikey_state: Option<YubiKeyState>) -> Self {
        Self {
            yubikey_state,
            own_id: Cell::new(None),
        }
    }
}

static WIZARD_MENU_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Esc Back",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('1'),
        modifiers: KeyModifiers::NONE,
        action: "initial_setup",
        description: "1 Initial Setup",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('2'),
        modifiers: KeyModifiers::NONE,
        action: "ssh_wizard",
        description: "2 New SSH Key",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('3'),
        modifiers: KeyModifiers::NONE,
        action: "touch_policy",
        description: "3 Touch Policy",
        show: true,
    },
];

impl Widget for WizardMenuScreen {
    fn widget_type_name(&self) -> &'static str {
        "WizardMenuScreen"
    }

    fn on_mount(&self, id: WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _id: WidgetId) { self.own_id.set(None); }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        vec![
            Box::new(Header::new("Setup Wizards")),
            Box::new(Label::new("")),
            Box::new(Label::new("Choose a guided setup flow:").with_class("section-title")),
            Box::new(Label::new("")),
            Box::new(textual_rs::Vertical::with_children(vec![
                Box::new(Button::new("[1] Initial YubiKey Setup").with_action("initial_setup")),
                Box::new(Label::new("    Set FIDO2 PIN, add first OATH account, configure PIV/SSH.")),
            ]).with_class("status-card")),
            Box::new(textual_rs::Vertical::with_children(vec![
                Box::new(Button::new("[2] Generate New SSH Key").with_action("ssh_wizard")),
                Box::new(Label::new("    Generate a NEW key with touch policy — REPLACES existing key.")),
            ]).with_class("status-card")),
            Box::new(textual_rs::Vertical::with_children(vec![
                Box::new(Button::new("[3] Set Touch Policy (Existing Key)").with_action("touch_policy")),
                Box::new(Label::new("    Change touch policy on a key already on the card.")),
            ]).with_class("status-card")),
            Box::new(Label::new("")),
            Box::new(Footer),
        ]
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        WIZARD_MENU_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            for binding in WIZARD_MENU_BINDINGS {
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
            "initial_setup" => ctx.push_screen_deferred(Box::new(
                InitialSetupWizardScreen::new(self.yubikey_state.clone()),
            )),
            "ssh_wizard" => ctx.push_screen_deferred(Box::new(
                SshTouchPolicyWizardScreen::new(self.yubikey_state.clone()),
            )),
            "touch_policy" => ctx.push_screen_deferred(Box::new(
                crate::tui::keys::TouchPolicyScreen::new(self.yubikey_state.clone()),
            )),
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ============================================================================
// InitialSetupWizardScreen — WIZARD-01, WIZARD-05
// ============================================================================

/// Multi-step "Initial YubiKey Setup" wizard.
///
/// Steps:
///   0  Welcome      — device info overview
///   1  FIDO2 PIN    — current state + link to PIN setup
///   2  OATH Account — credential count + link to add account
///   3  PIV/SSH Key  — slot 9a occupancy + link to PIV/SSH setup
///   4  Done         — completion summary
///
/// Each step shows current device state (WIZARD-05). The user can skip any
/// step or go back.
pub struct InitialSetupWizardScreen {
    yubikey_state: Option<YubiKeyState>,
    step: Reactive<usize>,
    own_id: Cell<Option<WidgetId>>,
}

const INITIAL_SETUP_STEPS: usize = 5; // 0..=4

impl InitialSetupWizardScreen {
    pub fn new(yubikey_state: Option<YubiKeyState>) -> Self {
        Self {
            yubikey_state,
            step: Reactive::new(0),
            own_id: Cell::new(None),
        }
    }

    fn progress_bar(current: usize, total: usize) -> String {
        let filled = current.min(total);
        let bar: String = (0..total)
            .map(|i| if i < filled { '#' } else { '-' })
            .collect();
        format!("[{}] Step {}/{}", bar, current, total)
    }
}

static SETUP_WIZARD_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Esc Exit wizard",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "next",
        description: "Enter Next",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('s'),
        modifiers: KeyModifiers::NONE,
        action: "skip",
        description: "S Skip step",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('b'),
        modifiers: KeyModifiers::NONE,
        action: "prev",
        description: "B Back",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('o'),
        modifiers: KeyModifiers::NONE,
        action: "open",
        description: "O Open screen",
        show: true,
    },
];

impl Widget for InitialSetupWizardScreen {
    fn widget_type_name(&self) -> &'static str {
        "InitialSetupWizardScreen"
    }

    fn on_mount(&self, id: WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _id: WidgetId) { self.own_id.set(None); }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let step = self.step.get_untracked();
        let yk = self.yubikey_state.as_ref();

        let progress = Self::progress_bar(step, INITIAL_SETUP_STEPS - 1);

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("Initial YubiKey Setup")),
            Box::new(Label::new(progress)),
            Box::new(Label::new("")),
        ];

        match step {
            // Step 0: Welcome
            0 => {
                let mut info: Vec<Box<dyn Widget>> = vec![
                    Box::new(Label::new("Welcome to the Initial YubiKey Setup Wizard.")),
                ];
                if let Some(yk) = yk {
                    info.push(Box::new(Label::new(format!("Device: {} (SN {})", yk.info.model, yk.info.serial))));
                    info.push(Box::new(Label::new(format!("Firmware: {}.{}.{}", yk.info.version.major, yk.info.version.minor, yk.info.version.patch))));
                } else {
                    info.push(Box::new(Label::new("No YubiKey detected. Insert your key and restart.")));
                }
                widgets.push(Box::new(textual_rs::Vertical::with_children(info).with_class("status-card")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(textual_rs::Vertical::with_children(vec![
                    Box::new(Label::new("This wizard will guide you through:")),
                    Box::new(Label::new("  1. Setting a FIDO2 PIN")),
                    Box::new(Label::new("  2. Adding your first OATH account")),
                    Box::new(Label::new("  3. Configuring a PIV or SSH key")),
                ]).with_class("status-card")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Press Enter to begin, or Esc to exit.")));
            }

            // Step 1: FIDO2 PIN
            1 => {
                widgets.push(Box::new(Label::new("Step 1: FIDO2 PIN").with_class("section-title")));
                let pin_state = yk.and_then(|y| y.fido2.as_ref()).map(|f| f.pin_is_set);
                let mut card: Vec<Box<dyn Widget>> = Vec::new();
                match pin_state {
                    Some(true) => {
                        card.push(Box::new(Label::new("✓ FIDO2 PIN is SET.")));
                        card.push(Box::new(Label::new("Your key is protected.")));
                    }
                    Some(false) => {
                        card.push(Box::new(Label::new("○ FIDO2 PIN is NOT SET.")));
                        card.push(Box::new(Label::new("Anyone with physical access can use your key.")));
                        card.push(Box::new(Label::new("Press O to set a PIN.")));
                    }
                    None => {
                        card.push(Box::new(Label::new("? FIDO2 state unavailable.")));
                        card.push(Box::new(Label::new("Press O to open the FIDO2 screen.")));
                    }
                }
                let cls = if pin_state == Some(true) { "status-card-ok" } else { "status-card-warn" };
                widgets.push(Box::new(textual_rs::Vertical::with_children(card).with_class(cls)));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Enter/S to continue.")));
            }

            // Step 2: OATH Account
            2 => {
                widgets.push(Box::new(Label::new("Step 2: OATH Authenticator").with_class("section-title")));
                let cred_count = yk.and_then(|y| y.oath.as_ref()).map(|o| o.credentials.len()).unwrap_or(0);
                let pass_req = yk.and_then(|y| y.oath.as_ref()).map(|o| o.password_required).unwrap_or(false);
                let mut card: Vec<Box<dyn Widget>> = vec![
                    Box::new(Label::new(format!("{} OATH credential(s) on this YubiKey.", cred_count))),
                ];
                if pass_req { card.push(Box::new(Label::new("Note: OATH applet is password-protected."))); }
                if cred_count == 0 {
                    card.push(Box::new(Label::new("Press O to add your first TOTP account.")));
                } else {
                    card.push(Box::new(Label::new("OATH credentials already configured.")));
                }
                let cls = if cred_count > 0 { "status-card-ok" } else { "status-card" };
                widgets.push(Box::new(textual_rs::Vertical::with_children(card).with_class(cls)));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Enter/S to continue.")));
            }

            // Step 3: PIV / SSH Key
            3 => {
                widgets.push(Box::new(Label::new("Step 3: PIV / SSH Key").with_class("section-title")));
                let slot_9a_occupied = yk.and_then(|y| y.piv.as_ref()).map(|piv| piv.slots.iter().any(|s| s.slot == "9a")).unwrap_or(false);
                let mgmt_default = yk.and_then(|y| y.piv.as_ref()).map(|piv| piv.mgmt_key_is_default).unwrap_or(false);
                let mut card: Vec<Box<dyn Widget>> = vec![
                    Box::new(Label::new(format!("PIV slot 9a: {}", if slot_9a_occupied { "OCCUPIED" } else { "EMPTY" }))),
                ];
                if mgmt_default {
                    card.push(Box::new(Label::new("⚠ Management key is factory default.")));
                }
                if !slot_9a_occupied {
                    card.push(Box::new(Label::new("Press O to open the SSH Setup wizard.")));
                } else {
                    card.push(Box::new(Label::new("PIV/SSH already configured. Press O for PIV screen.")));
                }
                let cls = if slot_9a_occupied { "status-card-ok" } else { "status-card" };
                widgets.push(Box::new(textual_rs::Vertical::with_children(card).with_class(cls)));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Enter/S to continue.")));
            }

            // Step 4: Done
            _ => {
                widgets.push(Box::new(Label::new("Setup Complete!").with_class("section-title")));
                widgets.push(Box::new(Label::new("")));
                let mut summary: Vec<Box<dyn Widget>> = vec![
                    Box::new(Label::new("Your YubiKey is configured:")),
                ];
                if let Some(yk) = yk {
                    let pin_set = yk.fido2.as_ref().map(|f| f.pin_is_set).unwrap_or(false);
                    let cred_count = yk.oath.as_ref().map(|o| o.credentials.len()).unwrap_or(0);
                    let slot_9a = yk.piv.as_ref().map(|p| p.slots.iter().any(|s| s.slot == "9a")).unwrap_or(false);
                    summary.push(Box::new(Label::new(format!("  FIDO2 PIN:     {}", if pin_set { "Set" } else { "Not set" }))));
                    summary.push(Box::new(Label::new(format!("  OATH accounts: {}", cred_count))));
                    summary.push(Box::new(Label::new(format!("  PIV slot 9a:   {}", if slot_9a { "Configured" } else { "Empty" }))));
                }
                widgets.push(Box::new(textual_rs::Vertical::with_children(summary).with_class("status-card-ok")));
                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Press Esc to return to the dashboard.")));
            }
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        SETUP_WIZARD_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            for binding in SETUP_WIZARD_BINDINGS {
                if binding.matches(key.code, key.modifiers) {
                    self.on_action(binding.action, ctx);
                    return EventPropagation::Stop;
                }
            }
        }
        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        let step = self.step.get_untracked();
        let yk = self.yubikey_state.clone();

        match action {
            "back" | "cancel" => ctx.pop_screen_deferred(),

            "next" | "skip" => {
                if step + 1 >= INITIAL_SETUP_STEPS {
                    // Already on done step — exit
                    ctx.pop_screen_deferred();
                } else {
                    self.step.update(|s| *s += 1);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                }
            }

            "prev" => {
                if step > 0 {
                    self.step.update(|s| *s -= 1);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                }
            }

            "open" => {
                match step {
                    1 => {
                        let fido2 = yk.as_ref().and_then(|y| y.fido2.clone());
                        ctx.push_screen_deferred(Box::new(crate::tui::fido2::Fido2Screen::new(fido2)));
                    }
                    2 => {
                        let oath = yk.as_ref().and_then(|y| y.oath.clone());
                        ctx.push_screen_deferred(Box::new(crate::tui::oath::OathScreen::new(oath)));
                    }
                    3 => ctx.push_screen_deferred(Box::new(SshTouchPolicyWizardScreen::new(yk))),
                    _ => {}
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
// SshTouchPolicyWizardScreen — WIZARD-02, WIZARD-03, WIZARD-05
// ============================================================================

/// Touch policy options for SSH key generation (WIZARD-03).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchPolicy {
    Never,
    Always,
    Cached,
}

impl TouchPolicy {
    pub fn label(self) -> &'static str {
        match self {
            TouchPolicy::Never  => "No touch required",
            TouchPolicy::Always => "Touch required for every use",
            TouchPolicy::Cached => "Touch required (cached for 15 s)",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            TouchPolicy::Never =>
                "The key signs without any physical confirmation.\n\
                 Use this if you want seamless SSH logins but accept that\n\
                 any process running as you can use the key without warning.",
            TouchPolicy::Always =>
                "You must physically touch the YubiKey for every SSH operation.\n\
                 This prevents silent use — if you did not initiate a login,\n\
                 you will notice the LED blinking and can refuse.",
            TouchPolicy::Cached =>
                "Touch required once per 15-second window.\n\
                 Balances security (physical confirmation) with convenience\n\
                 (scripts/agents can reuse the key briefly without repeated touch).",
        }
    }
}

/// "SSH Key with Touch Policy" wizard.
///
/// Steps:
///   0  Touch policy selection — plain-language descriptions (WIZARD-03)
///   1  Slot overview — shows current 9a state (WIZARD-05)
///   2  Confirm & launch — pushes `KeyGenWizardScreen`
pub struct SshTouchPolicyWizardScreen {
    yubikey_state: Option<YubiKeyState>,
    step: Reactive<usize>,
    policy_idx: Reactive<usize>,
    own_id: Cell<Option<WidgetId>>,
}

const SSH_WIZARD_STEPS: usize = 3;
const TOUCH_POLICIES: &[TouchPolicy] = &[
    TouchPolicy::Never,
    TouchPolicy::Always,
    TouchPolicy::Cached,
];

impl SshTouchPolicyWizardScreen {
    pub fn new(yubikey_state: Option<YubiKeyState>) -> Self {
        Self {
            yubikey_state,
            step: Reactive::new(0),
            policy_idx: Reactive::new(1), // Default: Always (most secure)
            own_id: Cell::new(None),
        }
    }

    fn progress_bar(current: usize, total: usize) -> String {
        let filled = current.min(total);
        let bar: String = (0..total)
            .map(|i| if i < filled { '#' } else { '-' })
            .collect();
        format!("[{}] Step {}/{}", bar, current, total)
    }
}

static SSH_WIZARD_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Esc Back",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "next",
        description: "Enter Next",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Up,
        modifiers: KeyModifiers::NONE,
        action: "policy_up",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Down,
        modifiers: KeyModifiers::NONE,
        action: "policy_down",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('k'),
        modifiers: KeyModifiers::NONE,
        action: "policy_up",
        description: "K/J Select policy",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('j'),
        modifiers: KeyModifiers::NONE,
        action: "policy_down",
        description: "",
        show: false,
    },
];

impl Widget for SshTouchPolicyWizardScreen {
    fn widget_type_name(&self) -> &'static str {
        "SshTouchPolicyWizardScreen"
    }

    fn on_mount(&self, id: WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _id: WidgetId) { self.own_id.set(None); }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let step = self.step.get_untracked();
        let policy_idx = self.policy_idx.get_untracked();
        let yk = self.yubikey_state.as_ref();

        let progress = Self::progress_bar(step, SSH_WIZARD_STEPS);

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("SSH Key with Touch Policy")),
            Box::new(Label::new(progress)),
            Box::new(Label::new("")),
        ];

        match step {
            // Step 0: Touch policy selection (WIZARD-03)
            0 => {
                widgets.push(Box::new(Label::new("Step 1: Choose Touch Policy").with_class("section-title")));
                widgets.push(Box::new(Label::new("")));

                let mut policy_labels: Vec<Box<dyn Widget>> = Vec::new();
                for (i, &policy) in TOUCH_POLICIES.iter().enumerate() {
                    let cursor = if i == policy_idx { ">" } else { " " };
                    policy_labels.push(Box::new(Label::new(format!(
                        "{} [{}] {}", cursor, i + 1, policy.label()
                    ))));
                }
                widgets.push(Box::new(textual_rs::Vertical::with_children(policy_labels).with_class("status-card")));

                widgets.push(Box::new(Label::new("")));
                let selected = TOUCH_POLICIES[policy_idx];
                let mut desc_lines: Vec<Box<dyn Widget>> = vec![
                    Box::new(Label::new("About this choice:").with_class("section-title")),
                ];
                for line in selected.description().lines() {
                    desc_lines.push(Box::new(Label::new(format!("  {}", line))));
                }
                widgets.push(Box::new(textual_rs::Vertical::with_children(desc_lines).with_class("status-card")));

                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Press Enter to confirm and continue.")));
            }

            // Step 1: Slot overview (WIZARD-05)
            1 => {
                let policy = TOUCH_POLICIES[policy_idx];
                widgets.push(Box::new(Label::new("Step 2: Current Slot State").with_class("section-title")));
                widgets.push(Box::new(Label::new("")));

                let slot_9a_info = yk
                    .and_then(|y| y.piv.as_ref())
                    .and_then(|piv| piv.slots.iter().find(|s| s.slot == "9a"))
                    .cloned();

                let slot_occupied = slot_9a_info.is_some();
                let mut slot_lines: Vec<Box<dyn Widget>> = vec![
                    Box::new(Label::new(format!("Selected touch policy: {}", policy.label()))),
                    Box::new(Label::new("")),
                    Box::new(Label::new("PIV slot 9a (Authentication):")),
                ];

                match slot_9a_info {
                    None => {
                        slot_lines.push(Box::new(Label::new("  Status: EMPTY (no key present)")));
                        slot_lines.push(Box::new(Label::new("  A new key will be generated in this slot.")));
                    }
                    Some(info) => {
                        slot_lines.push(Box::new(Label::new("  Status: OCCUPIED")));
                        if let Some(subj) = &info.subject {
                            slot_lines.push(Box::new(Label::new(format!("  Subject:   {}", subj))));
                        }
                        if let Some(alg) = &info.algorithm {
                            slot_lines.push(Box::new(Label::new(format!("  Algorithm: {}", alg))));
                        }
                    }
                }
                widgets.push(Box::new(textual_rs::Vertical::with_children(slot_lines).with_class("status-card")));

                if slot_occupied {
                    widgets.push(Box::new(textual_rs::Vertical::with_children(vec![
                        Box::new(Label::new("WARNING: Generating a new key will REPLACE the existing key.")),
                        Box::new(Label::new("Make sure the existing key is backed up before continuing.")),
                    ]).with_class("status-card-warn")));
                }

                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(Label::new("Enter to continue, B to go back.")));
            }

            // Step 2: Launch key generation
            _ => {
                let policy = TOUCH_POLICIES[policy_idx];
                widgets.push(Box::new(Label::new("Step 3: Generate SSH Key").with_class("section-title")));
                widgets.push(Box::new(Label::new("")));

                widgets.push(Box::new(textual_rs::Vertical::with_children(vec![
                    Box::new(Label::new(format!("Touch policy: {}", policy.label()))),
                    Box::new(Label::new("Slot:         PIV 9a (Authentication)")),
                ]).with_class("status-card")));

                widgets.push(Box::new(Label::new("")));
                widgets.push(Box::new(textual_rs::Vertical::with_children(vec![
                    Box::new(Label::new("Press Enter to open the Key Generation wizard.")),
                    Box::new(Label::new("The touch policy will be applied to the generated key.")),
                    Box::new(Label::new("")),
                    Box::new(Label::new("After generation, export the SSH public key from Keys screen.")),
                    Box::new(Label::new("Add it to ~/.ssh/authorized_keys on your servers.")),
                ]).with_class("status-card")));
            }
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        SSH_WIZARD_BINDINGS
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> EventPropagation {
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            for binding in SSH_WIZARD_BINDINGS {
                if binding.matches(key.code, key.modifiers) {
                    self.on_action(binding.action, ctx);
                    return EventPropagation::Stop;
                }
            }
        }
        EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        let step = self.step.get_untracked();
        let policy_idx = self.policy_idx.get_untracked();

        match action {
            "back" => {
                if step == 0 {
                    ctx.pop_screen_deferred();
                } else {
                    self.step.update(|s| *s -= 1);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                }
            }

            "next" => {
                if step + 1 >= SSH_WIZARD_STEPS {
                    // Last step: launch keygen wizard
                    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                    let wizard = crate::tui::keys::KeyGenWizard::new(&today);
                    ctx.push_screen_deferred(Box::new(
                        crate::tui::keys::KeyGenWizardScreen::new(wizard),
                    ));
                } else {
                    self.step.update(|s| *s += 1);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                }
            }

            "policy_up" => {
                if policy_idx > 0 && step == 0 {
                    self.policy_idx.update(|i| *i -= 1);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                }
            }

            "policy_down" => {
                if policy_idx + 1 < TOUCH_POLICIES.len() && step == 0 {
                    self.policy_idx.update(|i| *i += 1);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
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
    async fn wizard_menu_screen() {
        let yk = mock_yubikey_states().into_iter().next();
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(WizardMenuScreen::new(yk.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn initial_setup_wizard_step_welcome() {
        let yk = mock_yubikey_states().into_iter().next();
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(InitialSetupWizardScreen::new(yk.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn initial_setup_wizard_step_fido2() {
        let yk = mock_yubikey_states().into_iter().next();
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(InitialSetupWizardScreen::new(yk.clone()))
        });
        let mut pilot = app.pilot();
        pilot.press(KeyCode::Enter).await; // advance to step 1
        pilot.settle().await;
        drop(pilot);
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn ssh_touch_policy_wizard_step_policy() {
        let yk = mock_yubikey_states().into_iter().next();
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(SshTouchPolicyWizardScreen::new(yk.clone()))
        });
        app.pilot().settle().await;
        insta::assert_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn ssh_touch_policy_wizard_step_slot() {
        let yk = mock_yubikey_states().into_iter().next();
        let mut app = TestApp::new_styled(80, 24, crate::app::SCREEN_CSS, move || {
            Box::new(SshTouchPolicyWizardScreen::new(yk.clone()))
        });
        let mut pilot = app.pilot();
        pilot.press(KeyCode::Enter).await; // advance to step 1
        pilot.settle().await;
        drop(pilot);
        insta::assert_snapshot!(app.backend());
    }
}
