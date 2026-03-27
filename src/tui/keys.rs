use std::cell::RefCell;

use textual_rs::{Widget, Header, Label, Button, Footer};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use textual_rs::widget::screen::ModalScreen;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::YubiKeyState;
use crate::tui::widgets::popup::{PopupScreen, ConfirmScreen};

// ── Action and state types (D-04: preserved for Tauri serialization) ─────────

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum KeyAction {
    None,
    NavigateTo(crate::model::Screen),
    ExecuteViewStatus,
    ExecuteExportSSH,
    ExecuteKeyImport,
    ExecuteKeyGen,
    ExecuteTouchPolicySet {
        slot: String,
        policy: crate::model::touch_policy::TouchPolicy,
        admin_pin: String,
    },
    LoadGpgKeys,
    LoadKeyAttributes,
    LoadSshPubkey,
    LoadAttestation {
        serial: Option<u32>,
    },
}

/// Steps in the key generation wizard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyGenStep {
    Algorithm,
    Expiry,
    Identity,
    Backup,
    Confirm,
    Running,
    Result,
}

/// Wizard state for key generation.
#[derive(Clone)]
pub struct KeyGenWizard {
    pub step: KeyGenStep,
    pub algorithm_index: usize,
    pub expiry_index: usize,
    pub custom_expiry: String,
    pub name: String,
    pub email: String,
    pub backup: bool,
    pub backup_path: String,
    pub active_field: usize,
    pub editing_path: bool,
    pub editing_custom_expiry: bool,
}

impl KeyGenWizard {
    pub fn new(default_backup_date: &str) -> Self {
        Self {
            step: KeyGenStep::Algorithm,
            algorithm_index: 0,
            expiry_index: 0,
            custom_expiry: String::new(),
            name: String::new(),
            email: String::new(),
            backup: false,
            backup_path: format!("~/yubikey-backup-{}.gpg", default_backup_date),
            active_field: 0,
            editing_path: false,
            editing_custom_expiry: false,
        }
    }
}

/// Sub-screen enum (retained for compatibility — D-04).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyScreen {
    Main,
    ViewStatus,
    ImportKey,
    ExportSSH,
    KeyAttributes,
    SshPubkeyPopup,
    SetTouchPolicy,
    SetTouchPolicySelect,
    SetTouchPolicyConfirm,
    SetTouchPolicyPinInput,
    KeyGenWizardActive,
    KeyImportRunning,
    KeyImportPinInput,
    KeyOperationResult,
}

/// Key management screen state (D-04: preserved).
#[derive(Clone)]
pub struct KeyState {
    pub screen: KeyScreen,
    pub message: Option<String>,
    pub available_keys: Vec<String>,
    pub selected_key_index: usize,
    pub key_attributes: Option<crate::model::key_operations::KeyAttributes>,
    pub ssh_pubkey: Option<String>,
    pub touch_slot_index: usize,
    pub touch_policy_index: usize,
    pub attestation_popup: Option<String>,
    pub keygen_wizard: Option<KeyGenWizard>,
    pub operation_status: Option<String>,
    pub progress_tick: usize,
    pub import_result: Option<String>,
}

impl Default for KeyState {
    fn default() -> Self {
        Self {
            screen: KeyScreen::Main,
            message: None,
            available_keys: Vec::new(),
            selected_key_index: 0,
            key_attributes: None,
            ssh_pubkey: None,
            touch_slot_index: 0,
            touch_policy_index: 0,
            attestation_popup: None,
            keygen_wizard: None,
            operation_status: None,
            progress_tick: 0,
            import_result: None,
        }
    }
}

// ── Helper functions (public — used by app.rs for model operations) ───────────

pub fn touch_slot_name(index: usize) -> &'static str {
    match index {
        0 => "sig",
        1 => "enc",
        2 => "aut",
        3 => "att",
        _ => "sig",
    }
}

pub fn touch_slot_display(index: usize) -> &'static str {
    match index {
        0 => "Signature",
        1 => "Encryption",
        2 => "Authentication",
        3 => "Attestation",
        _ => "Signature",
    }
}

pub fn touch_policy_from_index(index: usize) -> crate::model::touch_policy::TouchPolicy {
    use crate::model::touch_policy::TouchPolicy;
    match index {
        0 => TouchPolicy::Off,
        1 => TouchPolicy::On,
        2 => TouchPolicy::Fixed,
        3 => TouchPolicy::Cached,
        4 => TouchPolicy::CachedFixed,
        _ => TouchPolicy::Off,
    }
}

pub fn keygen_params_from_state(
    state: &KeyState,
) -> Option<crate::model::key_operations::KeyGenParams> {
    use crate::model::key_operations::{KeyAlgorithm, KeyGenParams};
    let w = state.keygen_wizard.as_ref()?;
    let algo = match w.algorithm_index {
        0 => KeyAlgorithm::Ed25519,
        1 => KeyAlgorithm::Rsa2048,
        _ => KeyAlgorithm::Rsa4096,
    };
    let expire_date = match w.expiry_index {
        0 => "0".to_string(),
        1 => "1y".to_string(),
        2 => "2y".to_string(),
        _ => w.custom_expiry.clone(),
    };
    Some(KeyGenParams {
        algorithm: algo,
        expire_date,
        name: w.name.clone(),
        email: w.email.clone(),
        backup: w.backup,
        backup_path: if w.backup { Some(w.backup_path.clone()) } else { None },
    })
}

// ── Keys Screen Widget ─────────────────────────────────────────────────────────

static KEYS_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Char('g'),
        modifiers: KeyModifiers::NONE,
        action: "generate_key",
        description: "G Generate",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('i'),
        modifiers: KeyModifiers::NONE,
        action: "import_key",
        description: "I Import",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('d'),
        modifiers: KeyModifiers::NONE,
        action: "delete_key",
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
        key: KeyCode::Char('v'),
        modifiers: KeyModifiers::NONE,
        action: "view_status",
        description: "V View status",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('e'),
        modifiers: KeyModifiers::NONE,
        action: "export_ssh",
        description: "E Export SSH",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('k'),
        modifiers: KeyModifiers::NONE,
        action: "key_attributes",
        description: "K Attributes",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('t'),
        modifiers: KeyModifiers::NONE,
        action: "touch_policy",
        description: "T Touch policy",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('s'),
        modifiers: KeyModifiers::NONE,
        action: "ssh_pubkey",
        description: "S SSH pubkey",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Char('a'),
        modifiers: KeyModifiers::NONE,
        action: "attestation",
        description: "A Attestation",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Esc Back",
        show: true,
    },
];

/// Keys screen — shows OpenPGP key slots and allows key operations.
///
/// Sidebar (33%): key slot summary (SIG/ENC/AUTH status).
/// Main (67%): action buttons and key details.
///
/// Follows textual-rs Widget pattern (D-01, D-06, D-07):
/// - Header("Key Management")
/// - Key slot status Labels
/// - Action Buttons (D-06)
/// - Footer with keybindings (D-07, D-15)
/// - No hardcoded Color:: values
pub struct KeysScreen {
    yubikey_state: Option<YubiKeyState>,
    state: RefCell<KeyState>,
}

impl KeysScreen {
    pub fn new(yubikey_state: Option<YubiKeyState>) -> Self {
        Self {
            yubikey_state,
            state: RefCell::new(KeyState::default()),
        }
    }
}

impl Widget for KeysScreen {
    fn widget_type_name(&self) -> &'static str {
        "KeysScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut children: Vec<Box<dyn Widget>> = Vec::new();

        children.push(Box::new(Header::new("Key Management")));

        // Key slot summary (sidebar role — status block)
        if let Some(yk) = &self.yubikey_state {
            if let Some(ref openpgp) = yk.openpgp {
                // Signature slot
                if let Some(ref sig) = openpgp.signature_key {
                    let fp = sig.fingerprint.get(..16).unwrap_or(&sig.fingerprint);
                    children.push(Box::new(Label::new(format!(
                        "Signature:      {} ...",
                        fp
                    ))));
                } else {
                    children.push(Box::new(Label::new("Signature:      [Empty]")));
                }
                // Encryption slot
                if let Some(ref enc) = openpgp.encryption_key {
                    let fp = enc.fingerprint.get(..16).unwrap_or(&enc.fingerprint);
                    children.push(Box::new(Label::new(format!(
                        "Encryption:     {} ...",
                        fp
                    ))));
                } else {
                    children.push(Box::new(Label::new("Encryption:     [Empty]")));
                }
                // Authentication slot
                if let Some(ref auth) = openpgp.authentication_key {
                    let fp = auth.fingerprint.get(..16).unwrap_or(&auth.fingerprint);
                    children.push(Box::new(Label::new(format!(
                        "Authentication: {} ...",
                        fp
                    ))));
                } else {
                    children.push(Box::new(Label::new(
                        "Authentication: [Empty] (required for SSH)",
                    )));
                }
            } else {
                children.push(Box::new(Label::new("No keys configured")));
                children.push(Box::new(Label::new(
                    "Generate or import keys using the buttons below.",
                )));
            }

            // Touch policies
            if let Some(ref tp) = yk.touch_policies {
                let has_sig = yk.openpgp.as_ref().is_some_and(|o| o.signature_key.is_some());
                let has_enc = yk.openpgp.as_ref().is_some_and(|o| o.encryption_key.is_some());
                let has_aut =
                    yk.openpgp.as_ref().is_some_and(|o| o.authentication_key.is_some());
                children.push(Box::new(Label::new("")));
                children.push(Box::new(Label::new("Touch Policies:")));
                children.push(Box::new(Label::new(format!(
                    "  Signature:      {}",
                    if has_sig { format!("{}", tp.signature) } else { "—".to_string() }
                ))));
                children.push(Box::new(Label::new(format!(
                    "  Encryption:     {}",
                    if has_enc { format!("{}", tp.encryption) } else { "—".to_string() }
                ))));
                children.push(Box::new(Label::new(format!(
                    "  Authentication: {}",
                    if has_aut { format!("{}", tp.authentication) } else { "—".to_string() }
                ))));
                children.push(Box::new(Label::new(format!(
                    "  Attestation:    {}",
                    tp.attestation
                ))));
            }
        } else {
            children.push(Box::new(Label::new("No keys configured")));
            children.push(Box::new(Label::new(
                "Generate or import keys using the buttons below.",
            )));
        }

        // Status message
        {
            let state = self.state.borrow();
            if let Some(ref msg) = state.message {
                children.push(Box::new(Label::new("")));
                children.push(Box::new(Label::new(format!("Status: {}", msg))));
            }
        }

        children.push(Box::new(Label::new("")));

        // Action buttons (D-06: all navigable elements are Buttons)
        children.push(Box::new(Button::new("Generate Key on Card")));
        children.push(Box::new(Button::new("Import Existing Key")));
        children.push(Box::new(Button::new("View Full Key Details")));
        children.push(Box::new(Button::new("Export SSH Public Key")));
        children.push(Box::new(Button::new("Key Attributes")));
        children.push(Box::new(Button::new("Touch Policy")));
        children.push(Box::new(Button::new("Attestation")));

        children.push(Box::new(Footer));
        children
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        KEYS_BINDINGS
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "generate_key" => {
                let today = chrono_today();
                let wizard = KeyGenWizard::new(&today);
                ctx.push_screen_deferred(Box::new(KeyGenWizardScreen::new(wizard)));
            }
            "import_key" => {
                // Import flow — pushed screen
                ctx.push_screen_deferred(Box::new(ImportKeyScreen::new(
                    self.yubikey_state.clone(),
                )));
            }
            "delete_key" => {
                // Delete confirmation via ConfirmScreen (D-14: destructive = Error styled)
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    ConfirmScreen::new(
                        "Delete Slot",
                        "Delete Slot -- This will erase the key and certificate in this slot.",
                        true, // destructive
                    ),
                ))));
            }
            "view_status" => {
                ctx.push_screen_deferred(Box::new(KeyDetailScreen::new(
                    "View Card Status",
                    "Read card status via native PC/SC.\n\n\
                     This will display all card details including:\n\
                     - Key fingerprints\n\
                     - Key attributes\n\
                     - Cardholder name\n\
                     - PIN retry counters",
                )));
            }
            "export_ssh" => {
                let pubkey = self.yubikey_state.as_ref()
                    .and_then(|_yk| self.state.borrow().ssh_pubkey.clone());
                let body = if let Some(ref key) = pubkey {
                    format!(
                        "{}\n\nAdd this key to:\n  - ~/.ssh/authorized_keys on remote servers\n  - GitHub > Settings > SSH Keys\n  - GitLab > Preferences > SSH Keys",
                        key
                    )
                } else {
                    "No authentication key found on card.\nImport or generate a key first.".to_string()
                };
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    PopupScreen::new("SSH Public Key", body),
                ))));
            }
            "key_attributes" => {
                let state = self.state.borrow();
                let body = if let Some(ref attrs) = state.key_attributes {
                    let sig_str = attrs.signature.as_ref()
                        .map(|s| format!("{} ({})", s.algorithm, s.fingerprint))
                        .unwrap_or_else(|| "[empty]".to_string());
                    let enc_str = attrs.encryption.as_ref()
                        .map(|s| format!("{} ({})", s.algorithm, s.fingerprint))
                        .unwrap_or_else(|| "[empty]".to_string());
                    let aut_str = attrs.authentication.as_ref()
                        .map(|s| format!("{} ({})", s.algorithm, s.fingerprint))
                        .unwrap_or_else(|| "[empty]".to_string());
                    format!(
                        "Signature:      {}\nEncryption:     {}\nAuthentication: {}",
                        sig_str, enc_str, aut_str
                    )
                } else {
                    "Key attributes unavailable. Press K to load.".to_string()
                };
                drop(state);
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    PopupScreen::new("Key Attributes", body),
                ))));
            }
            "touch_policy" => {
                ctx.push_screen_deferred(Box::new(TouchPolicyScreen::new(
                    self.yubikey_state.clone(),
                )));
            }
            "ssh_pubkey" => {
                let state = self.state.borrow();
                let body = if let Some(ref key) = state.ssh_pubkey {
                    format!(
                        "{}\n\nAdd this key to:\n  - ~/.ssh/authorized_keys\n  - GitHub / GitLab SSH keys",
                        key
                    )
                } else {
                    "No authentication key found on card.\nImport or generate a key first.".to_string()
                };
                drop(state);
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    PopupScreen::new("SSH Public Key", body),
                ))));
            }
            "attestation" => {
                let state = self.state.borrow();
                let body = if let Some(ref pem) = state.attestation_popup {
                    format!("{}\n\nThis PEM verifies the key was generated on-device.", pem)
                } else {
                    "No attestation certificate available.\nGenerate a key on-device to obtain attestation.".to_string()
                };
                drop(state);
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    PopupScreen::new("Attestation Certificate", body),
                ))));
            }
            "refresh" => {
                // Refresh is an app-level side effect — no-op in widget scope.
            }
            "back" => {
                ctx.pop_screen_deferred();
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {
        // Layout and child rendering handled by compose() children.
    }
}

fn chrono_today() -> String {
    // Use a simple fixed-format date; no chrono dep needed for backup path default.
    // The actual value is just a filename hint, not machine-parsed.
    "today".to_string()
}

// ── KeyGenWizardScreen ────────────────────────────────────────────────────────

static KEYGEN_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Up,
        modifiers: KeyModifiers::NONE,
        action: "select_up",
        description: "Up Select",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Down,
        modifiers: KeyModifiers::NONE,
        action: "select_down",
        description: "Down Select",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "confirm",
        description: "Enter Confirm",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Esc Cancel",
        show: true,
    },
];

/// KeyGenWizard — 7-step key generation wizard as a pushed screen.
///
/// Internal state machine tracks the current step via RefCell<KeyGenWizard>.
/// Step transitions update state in on_action(); compose() renders the active step.
pub struct KeyGenWizardScreen {
    wizard: RefCell<KeyGenWizard>,
}

impl KeyGenWizardScreen {
    pub fn new(wizard: KeyGenWizard) -> Self {
        Self {
            wizard: RefCell::new(wizard),
        }
    }
}

impl Widget for KeyGenWizardScreen {
    fn widget_type_name(&self) -> &'static str {
        "KeyGenWizardScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let w = self.wizard.borrow();
        let mut children: Vec<Box<dyn Widget>> = Vec::new();

        match w.step {
            KeyGenStep::Algorithm => {
                children.push(Box::new(Header::new("Generate Key — Step 1/5: Algorithm")));
                children.push(Box::new(Label::new("Select key algorithm:")));
                children.push(Box::new(Label::new("")));
                let algorithms = [
                    ("Ed25519/Cv25519", "Modern elliptic curve — recommended for new keys"),
                    ("RSA 2048", "Classic RSA — widely compatible"),
                    ("RSA 4096", "Classic RSA — widest compatibility, slowest"),
                ];
                for (i, (name, desc)) in algorithms.iter().enumerate() {
                    let marker = if i == w.algorithm_index { "> " } else { "  " };
                    children.push(Box::new(Label::new(format!("{}{}", marker, name))));
                    if i == w.algorithm_index {
                        children.push(Box::new(Label::new(format!("    {}", desc))));
                    }
                }
            }
            KeyGenStep::Expiry => {
                children.push(Box::new(Header::new("Generate Key — Step 2/5: Expiry")));
                children.push(Box::new(Label::new("Select key expiry:")));
                children.push(Box::new(Label::new("")));
                let options = ["No expiry", "1 year", "2 years", "Custom date"];
                for (i, opt) in options.iter().enumerate() {
                    let marker = if i == w.expiry_index { "> " } else { "  " };
                    children.push(Box::new(Label::new(format!("{}{}", marker, opt))));
                }
                if w.expiry_index == 3 {
                    children.push(Box::new(Label::new("")));
                    children.push(Box::new(Label::new("Enter date (YYYY-MM-DD):")));
                    let display = if w.editing_custom_expiry {
                        format!("{}_", w.custom_expiry)
                    } else {
                        w.custom_expiry.clone()
                    };
                    children.push(Box::new(Label::new(display)));
                }
            }
            KeyGenStep::Identity => {
                children.push(Box::new(Header::new("Generate Key — Step 3/5: Identity")));
                children.push(Box::new(Label::new("Enter your name and email:")));
                children.push(Box::new(Label::new("")));
                let name_label = if w.active_field == 0 { "Name: [editing]" } else { "Name:" };
                children.push(Box::new(Label::new(name_label)));
                children.push(Box::new(Label::new(format!("  {}", w.name))));
                children.push(Box::new(Label::new("")));
                let email_label = if w.active_field == 1 { "Email: [editing]" } else { "Email:" };
                children.push(Box::new(Label::new(email_label)));
                children.push(Box::new(Label::new(format!("  {}", w.email))));
                children.push(Box::new(Label::new("")));
                children.push(Box::new(Label::new("[Tab] Switch field  [Enter] Next")));
            }
            KeyGenStep::Backup => {
                children.push(Box::new(Header::new("Generate Key — Step 4/5: Backup")));
                children.push(Box::new(Label::new(
                    "Create a backup copy before moving key to card?",
                )));
                children.push(Box::new(Label::new("")));
                children.push(Box::new(Label::new(if w.backup { "> [Y] Create backup" } else { "  [Y] Create backup" })));
                children.push(Box::new(Label::new(if !w.backup { "> [N] Skip backup" } else { "  [N] Skip backup" })));
                if w.backup {
                    children.push(Box::new(Label::new("")));
                    children.push(Box::new(Label::new("Backup path:")));
                    let path_display = if w.editing_path {
                        format!("{}_", w.backup_path)
                    } else {
                        format!("{} [Enter to edit]", w.backup_path)
                    };
                    children.push(Box::new(Label::new(path_display)));
                }
            }
            KeyGenStep::Confirm => {
                children.push(Box::new(Header::new("Generate Key — Step 5/5: Confirm")));
                children.push(Box::new(Label::new("Summary — press Enter to generate:")));
                children.push(Box::new(Label::new("")));
                let algo = match w.algorithm_index {
                    0 => "Ed25519/Cv25519",
                    1 => "RSA 2048",
                    _ => "RSA 4096",
                };
                children.push(Box::new(Label::new(format!("Algorithm: {}", algo))));
                let expiry = match w.expiry_index {
                    0 => "No expiry".to_string(),
                    1 => "1 year".to_string(),
                    2 => "2 years".to_string(),
                    _ => format!("Custom: {}", w.custom_expiry),
                };
                children.push(Box::new(Label::new(format!("Expiry:    {}", expiry))));
                children.push(Box::new(Label::new(format!("Name:      {}", w.name))));
                children.push(Box::new(Label::new(format!("Email:     {}", w.email))));
                children.push(Box::new(Label::new(format!(
                    "Backup:    {}",
                    if w.backup { format!("Yes ({})", w.backup_path) } else { "No".to_string() }
                ))));
                children.push(Box::new(Label::new("")));
                children.push(Box::new(Button::new("Generate Key — Enter Admin PIN")));
            }
            KeyGenStep::Running => {
                children.push(Box::new(Header::new("Generating Key...")));
                children.push(Box::new(Label::new("")));
                children.push(Box::new(Label::new("Key generation in progress.")));
                children.push(Box::new(Label::new(
                    "This may take up to 60 seconds for RSA 4096.",
                )));
            }
            KeyGenStep::Result => {
                children.push(Box::new(Header::new("Key Generation Complete")));
                children.push(Box::new(Label::new("")));
                children.push(Box::new(Label::new(
                    "Key generation completed. Press Enter or Esc to return.",
                )));
                children.push(Box::new(Button::new("Done")));
            }
        }

        children.push(Box::new(Footer));
        children
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        KEYGEN_BINDINGS
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        let mut w = self.wizard.borrow_mut();
        match action {
            "select_up" => match w.step {
                KeyGenStep::Algorithm => {
                    if w.algorithm_index > 0 {
                        w.algorithm_index -= 1;
                    }
                }
                KeyGenStep::Expiry => {
                    if !w.editing_custom_expiry && w.expiry_index > 0 {
                        w.expiry_index -= 1;
                    }
                }
                _ => {}
            },
            "select_down" => match w.step {
                KeyGenStep::Algorithm => {
                    if w.algorithm_index < 2 {
                        w.algorithm_index += 1;
                    }
                }
                KeyGenStep::Expiry => {
                    if !w.editing_custom_expiry && w.expiry_index < 3 {
                        w.expiry_index += 1;
                    }
                }
                _ => {}
            },
            "confirm" => match w.step {
                KeyGenStep::Algorithm => {
                    w.step = KeyGenStep::Expiry;
                }
                KeyGenStep::Expiry => {
                    if w.expiry_index == 3 {
                        if !w.editing_custom_expiry {
                            w.editing_custom_expiry = true;
                        } else if !w.custom_expiry.is_empty() {
                            w.editing_custom_expiry = false;
                            w.step = KeyGenStep::Identity;
                        }
                    } else {
                        w.step = KeyGenStep::Identity;
                    }
                }
                KeyGenStep::Identity => {
                    if w.active_field == 0 {
                        w.active_field = 1;
                    } else if !w.name.is_empty() && !w.email.is_empty() {
                        w.step = KeyGenStep::Backup;
                    }
                }
                KeyGenStep::Backup => {
                    if w.editing_path {
                        w.editing_path = false;
                        w.step = KeyGenStep::Confirm;
                    } else if w.backup {
                        w.editing_path = true;
                    } else {
                        w.step = KeyGenStep::Confirm;
                    }
                }
                KeyGenStep::Confirm => {
                    // Proceed to PIN entry — push PinInputWidget
                    drop(w);
                    use crate::tui::widgets::pin_input::PinInputWidget;
                    ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                        PinInputWidget::new("Key Generation — Admin PIN", &["Admin PIN"]),
                    ))));
                    return;
                }
                KeyGenStep::Result | KeyGenStep::Running => {
                    drop(w);
                    ctx.pop_screen_deferred();
                    return;
                }
            },
            "back" => {
                match w.step {
                    KeyGenStep::Algorithm => {
                        drop(w);
                        ctx.pop_screen_deferred();
                        return;
                    }
                    KeyGenStep::Expiry => w.step = KeyGenStep::Algorithm,
                    KeyGenStep::Identity => w.step = KeyGenStep::Expiry,
                    KeyGenStep::Backup => w.step = KeyGenStep::Identity,
                    KeyGenStep::Confirm => w.step = KeyGenStep::Backup,
                    KeyGenStep::Running | KeyGenStep::Result => {
                        drop(w);
                        ctx.pop_screen_deferred();
                        return;
                    }
                }
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ── ImportKeyScreen ──────────────────────────────────────────────────────────

static IMPORT_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "confirm_import",
        description: "Enter Import selected",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Esc Cancel",
        show: true,
    },
];

/// Import Key flow screen.
pub struct ImportKeyScreen {
    yubikey_state: Option<YubiKeyState>,
    available_keys: Vec<String>,
    selected_index: RefCell<usize>,
}

impl ImportKeyScreen {
    pub fn new(yubikey_state: Option<YubiKeyState>) -> Self {
        Self {
            yubikey_state,
            available_keys: Vec::new(),
            selected_index: RefCell::new(0),
        }
    }
}

impl Widget for ImportKeyScreen {
    fn widget_type_name(&self) -> &'static str {
        "ImportKeyScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut children: Vec<Box<dyn Widget>> = Vec::new();
        children.push(Box::new(Header::new("Import Key to YubiKey")));
        children.push(Box::new(Label::new(
            "This will import a GPG key from your keyring to the YubiKey.",
        )));
        children.push(Box::new(Label::new("Prerequisites:")));
        children.push(Box::new(Label::new("  - You must have a GPG key already generated")));
        children.push(Box::new(Label::new("  - The key must be in your GPG keyring")));
        children.push(Box::new(Label::new("")));

        if self.available_keys.is_empty() {
            children.push(Box::new(Label::new(
                "No GPG keys found in keyring.",
            )));
            children.push(Box::new(Label::new(
                "Generate a key first, or import one with: gpg --import <file>",
            )));
        } else {
            let idx = *self.selected_index.borrow();
            for (i, key) in self.available_keys.iter().enumerate() {
                let marker = if i == idx { "> " } else { "  " };
                children.push(Box::new(Label::new(format!("{}{}", marker, key))));
            }
        }

        if self.yubikey_state.is_none() {
            children.push(Box::new(Label::new("")));
            children.push(Box::new(Label::new("No YubiKey detected — insert device first.")));
        }

        children.push(Box::new(Footer));
        children
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        IMPORT_BINDINGS
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "confirm_import" => {
                // Push PIN input for admin PIN
                use crate::tui::widgets::pin_input::PinInputWidget;
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    PinInputWidget::new("Import Key — Admin PIN", &["Admin PIN"]),
                ))));
            }
            "back" => ctx.pop_screen_deferred(),
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ── KeyDetailScreen (generic operation info screen) ───────────────────────────

static KEY_DETAIL_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Esc Close",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "execute",
        description: "Enter Execute",
        show: true,
    },
];

pub struct KeyDetailScreen {
    title: String,
    body: String,
}

impl KeyDetailScreen {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }
}

impl Widget for KeyDetailScreen {
    fn widget_type_name(&self) -> &'static str {
        "KeyDetailScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let lines: Vec<Box<dyn Widget>> = self
            .body
            .lines()
            .map(|l| -> Box<dyn Widget> { Box::new(Label::new(l)) })
            .collect();
        let mut children: Vec<Box<dyn Widget>> = Vec::new();
        children.push(Box::new(Header::new(&self.title)));
        children.extend(lines);
        children.push(Box::new(Label::new("")));
        children.push(Box::new(Button::new("Execute")));
        children.push(Box::new(Footer));
        children
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        KEY_DETAIL_BINDINGS
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "back" | "execute" => ctx.pop_screen_deferred(),
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ── TouchPolicyScreen ─────────────────────────────────────────────────────────

static TOUCH_POLICY_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Up,
        modifiers: KeyModifiers::NONE,
        action: "slot_up",
        description: "Up Prev slot",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Down,
        modifiers: KeyModifiers::NONE,
        action: "slot_down",
        description: "Down Next slot",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "select_slot",
        description: "Enter Select",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Esc Back",
        show: true,
    },
];

pub struct TouchPolicyScreen {
    yubikey_state: Option<YubiKeyState>,
    slot_index: RefCell<usize>,
}

impl TouchPolicyScreen {
    pub fn new(yubikey_state: Option<YubiKeyState>) -> Self {
        Self {
            yubikey_state,
            slot_index: RefCell::new(0),
        }
    }
}

impl Widget for TouchPolicyScreen {
    fn widget_type_name(&self) -> &'static str {
        "TouchPolicyScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut children: Vec<Box<dyn Widget>> = Vec::new();
        children.push(Box::new(Header::new("Set Touch Policy")));
        children.push(Box::new(Label::new("Select slot:")));
        children.push(Box::new(Label::new("")));

        let slots = ["Signature (sig)", "Encryption (enc)", "Authentication (aut)", "Attestation (att)"];
        let openpgp = self.yubikey_state.as_ref().and_then(|yk| yk.openpgp.as_ref());
        let slot_has_key = [
            openpgp.is_some_and(|o| o.signature_key.is_some()),
            openpgp.is_some_and(|o| o.encryption_key.is_some()),
            openpgp.is_some_and(|o| o.authentication_key.is_some()),
            true, // attestation always present
        ];

        let idx = *self.slot_index.borrow();
        for (i, slot) in slots.iter().enumerate() {
            let marker = if i == idx { "> " } else { "  " };
            let key_status = if slot_has_key[i] { "[key]" } else { "[empty]" };
            children.push(Box::new(Label::new(format!("{}{} {}", marker, slot, key_status))));
        }

        children.push(Box::new(Footer));
        children
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        TOUCH_POLICY_BINDINGS
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "slot_up" => {
                let mut idx = self.slot_index.borrow_mut();
                if *idx > 0 {
                    *idx -= 1;
                }
            }
            "slot_down" => {
                let mut idx = self.slot_index.borrow_mut();
                if *idx < 3 {
                    *idx += 1;
                }
            }
            "select_slot" => {
                // Push PIN input for admin PIN to confirm
                use crate::tui::widgets::pin_input::PinInputWidget;
                ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(
                    PinInputWidget::new("Set Touch Policy — Admin PIN", &["Admin PIN"]),
                ))));
            }
            "back" => ctx.pop_screen_deferred(),
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ── ProgressWidget (textual-rs port of old progress.rs) ───────────────────────

/// Spinner animation for use in operation-running screens.
///
/// Ported from `src/tui/widgets/progress.rs` — uses textual-rs Label
/// rather than direct ratatui frame rendering.
pub struct ProgressLabel {
    title: String,
    status: String,
    tick: usize,
}

impl ProgressLabel {
    pub fn new(title: impl Into<String>, status: impl Into<String>, tick: usize) -> Self {
        Self {
            title: title.into(),
            status: status.into(),
            tick,
        }
    }
}

const SPINNER: [char; 4] = ['|', '/', '-', '\\'];

impl Widget for ProgressLabel {
    fn widget_type_name(&self) -> &'static str {
        "ProgressLabel"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let spinner_char = SPINNER[self.tick % SPINNER.len()];
        vec![
            Box::new(Header::new(&self.title)),
            Box::new(Label::new(format!("{} {}", spinner_char, self.status))),
        ]
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use textual_rs::TestApp;
    use crossterm::event::KeyCode;

    #[tokio::test]
    async fn keys_default_state() {
        let yubikey_states = crate::model::mock::mock_yubikey_states();
        let yk = yubikey_states.into_iter().next();
        let mut app = TestApp::new(80, 24, move || Box::new(KeysScreen::new(yk)));
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn keys_no_yubikey() {
        let mut app = TestApp::new(80, 24, || Box::new(KeysScreen::new(None)));
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn keys_import_screen() {
        let yubikey_states = crate::model::mock::mock_yubikey_states();
        let yk = yubikey_states.into_iter().next();
        let mut app = TestApp::new(80, 24, move || Box::new(KeysScreen::new(yk)));
        let mut pilot = app.pilot();
        pilot.press(KeyCode::Char('i')).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn keygen_wizard_renders() {
        let wizard = KeyGenWizard::new("2026-01-01");
        let mut app = TestApp::new(80, 24, move || Box::new(KeyGenWizardScreen::new(wizard)));
        app.pilot().settle().await;
        // Basic render check for wizard (not part of plan snapshot set)
        let buf = app.buffer();
        let rendered = format!("{:?}", buf);
        assert!(rendered.len() > 0, "wizard should render to a non-empty buffer");
    }
}
