use std::cell::{Cell, RefCell};

use textual_rs::{Widget, WidgetId, Header, Label, Button, DataTable, ColumnDef, Footer};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::model::YubiKeyState;
use crate::tui::widgets::popup::PopupScreen;

const KEYS_HELP_TEXT: &str = "\
OpenPGP Keys\n\
\n\
OpenPGP lets your YubiKey store private keys for encryption, signing,\n\
and authentication. Keys never leave the hardware.\n\
\n\
Three key slots:\n\
- Signature: git commit signing, email signing\n\
- Encryption: decrypt files and emails\n\
- Authentication: SSH login via gpg-agent\n\
\n\
You can view key info, import existing keys, generate new ones on-card,\n\
or export the SSH public key derived from the authentication subkey.";

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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

#[allow(dead_code)]
pub fn touch_slot_name(index: usize) -> &'static str {
    match index {
        0 => "sig",
        1 => "enc",
        2 => "aut",
        3 => "att",
        _ => "sig",
    }
}

#[allow(dead_code)]
pub fn touch_slot_display(index: usize) -> &'static str {
    match index {
        0 => "Signature",
        1 => "Encryption",
        2 => "Authentication",
        3 => "Attestation",
        _ => "Signature",
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
    KeyBinding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "",
        show: false,
    },
    KeyBinding {
        key: KeyCode::Char('?'),
        modifiers: KeyModifiers::NONE,
        action: "help",
        description: "? Help",
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

    fn can_focus(&self) -> bool {
        true
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut children: Vec<Box<dyn Widget>> = Vec::new();

        children.push(Box::new(Header::new("Key Management")));

        // Key slot summary — DataTable with 3 OpenPGP slots
        if let Some(yk) = &self.yubikey_state {
            if let Some(ref openpgp) = yk.openpgp {
                let columns = vec![
                    ColumnDef::new("Slot").with_width(18),
                    ColumnDef::new("Status").with_width(7),
                    ColumnDef::new("Fingerprint").with_width(40),
                ];
                let mut table = DataTable::new(columns);

                // Signature slot
                let (sig_status, sig_fp) = if let Some(ref sig) = openpgp.signature_key {
                    ("[SET]".to_string(), sig.fingerprint.get(..16).unwrap_or(&sig.fingerprint).to_string())
                } else {
                    ("[EMPTY]".to_string(), "—".to_string())
                };
                table.add_row(vec!["Signature".to_string(), sig_status, sig_fp]);

                // Encryption slot
                let (enc_status, enc_fp) = if let Some(ref enc) = openpgp.encryption_key {
                    ("[SET]".to_string(), enc.fingerprint.get(..16).unwrap_or(&enc.fingerprint).to_string())
                } else {
                    ("[EMPTY]".to_string(), "—".to_string())
                };
                table.add_row(vec!["Encryption".to_string(), enc_status, enc_fp]);

                // Authentication slot
                let (aut_status, aut_fp) = if let Some(ref auth) = openpgp.authentication_key {
                    ("[SET]".to_string(), auth.fingerprint.get(..16).unwrap_or(&auth.fingerprint).to_string())
                } else {
                    ("[EMPTY]".to_string(), "—".to_string())
                };
                table.add_row(vec!["Authentication".to_string(), aut_status, aut_fp]);

                children.push(Box::new(table));

                // Touch policies — secondary info, keep as indented Labels with bracket notation
                if let Some(ref tp) = yk.touch_policies {
                    let has_sig = openpgp.signature_key.is_some();
                    let has_enc = openpgp.encryption_key.is_some();
                    let has_aut = openpgp.authentication_key.is_some();
                    children.push(Box::new(Label::new("")));
                    children.push(Box::new(Label::new("Touch Policies:")));
                    children.push(Box::new(Label::new(format!(
                        "  Signature:      [{}]",
                        if has_sig { format!("{}", tp.signature) } else { "—".to_string() }
                    ))));
                    children.push(Box::new(Label::new(format!(
                        "  Encryption:     [{}]",
                        if has_enc { format!("{}", tp.encryption) } else { "—".to_string() }
                    ))));
                    children.push(Box::new(Label::new(format!(
                        "  Authentication: [{}]",
                        if has_aut { format!("{}", tp.authentication) } else { "—".to_string() }
                    ))));
                    children.push(Box::new(Label::new(format!(
                        "  Attestation:    [{}]",
                        tp.attestation
                    ))));
                }
            } else {
                children.push(Box::new(Label::new("No keys configured.")));
                children.push(Box::new(Label::new(
                    "Generate or import keys using the buttons below.",
                )));
            }
        } else {
            children.push(Box::new(Label::new("No YubiKey detected.")));
            children.push(Box::new(Label::new(
                "Insert your YubiKey and press R to refresh.",
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

        // Action buttons
        if self.yubikey_state.is_none() {
            children.push(Box::new(Button::new("[R] Refresh")));
        } else {
            children.push(Box::new(Button::new("[G] Generate Key on Card")));
            children.push(Box::new(Button::new("[I] Import Existing Key")));
            children.push(Box::new(Button::new("[D] Delete Key Slot")));
            children.push(Box::new(Button::new("[V] View Full Key Details")));
            children.push(Box::new(Button::new("[E] Export SSH Public Key")));
            children.push(Box::new(Button::new("[K] Key Attributes")));
            children.push(Box::new(Button::new("[T] Touch Policy")));
            children.push(Box::new(Button::new("[A] Attestation")));
        }

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
                use crate::model::openpgp_delete::OpenPgpKeySlot;

                // Map selected_key_index to the appropriate slot (0=Sig, 1=Enc, 2=Aut).
                let selected_index = self.state.borrow().selected_key_index;
                let slot = match selected_index {
                    0 => OpenPgpKeySlot::Sig,
                    1 => OpenPgpKeySlot::Enc,
                    _ => OpenPgpKeySlot::Aut,
                };

                // Only allow delete when the slot is occupied.
                let key_present = self.yubikey_state.as_ref()
                    .and_then(|yk| yk.openpgp.as_ref())
                    .map(|pgp| match slot {
                        OpenPgpKeySlot::Sig => pgp.signature_key.is_some(),
                        OpenPgpKeySlot::Enc => pgp.encryption_key.is_some(),
                        OpenPgpKeySlot::Aut => pgp.authentication_key.is_some(),
                    })
                    .unwrap_or(false);

                if !key_present {
                    ctx.push_screen_deferred(Box::new(PopupScreen::new(
                        "No Key",
                        format!("No key in the {} slot to delete.", slot.display_name()),
                    )));
                } else {
                    ctx.push_screen_deferred(Box::new(PinThenDeleteScreen::new(slot)));
                }
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
                ctx.push_screen_deferred(Box::new(PopupScreen::new("SSH Public Key", body)));
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
                ctx.push_screen_deferred(Box::new(PopupScreen::new("Key Attributes", body)));
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
                ctx.push_screen_deferred(Box::new(PopupScreen::new("SSH Public Key", body)));
            }
            "attestation" => {
                let state = self.state.borrow();
                let body = if let Some(ref pem) = state.attestation_popup {
                    format!("{}\n\nThis PEM verifies the key was generated on-device.", pem)
                } else {
                    "No attestation certificate available.\nGenerate a key on-device to obtain attestation.".to_string()
                };
                drop(state);
                ctx.push_screen_deferred(Box::new(PopupScreen::new("Attestation Certificate", body)));
            }
            "refresh" => {
                // Re-detect YubiKey state from hardware and push fresh KeysScreen
                let fresh_yk = crate::model::YubiKeyState::detect_all()
                    .ok()
                    .and_then(|mut v| if v.is_empty() { None } else { Some(v.remove(0)) });
                ctx.pop_screen_deferred();
                ctx.push_screen_deferred(Box::new(KeysScreen::new(fresh_yk)));
            }
            "back" => {
                ctx.pop_screen_deferred();
            }
            "help" => {
                ctx.push_screen_deferred(Box::new(PopupScreen::new("OpenPGP Keys Help", KEYS_HELP_TEXT)));
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
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
    KeyBinding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "",
        show: false,
    },
];

/// KeyGenWizard — 7-step key generation wizard as a pushed screen.
///
/// Internal state machine tracks the current step via RefCell<KeyGenWizard>.
/// Step transitions update state in on_action(); compose() renders the active step.
pub struct KeyGenWizardScreen {
    wizard: RefCell<KeyGenWizard>,
    own_id: Cell<Option<WidgetId>>,
}

impl KeyGenWizardScreen {
    pub fn new(wizard: KeyGenWizard) -> Self {
        Self {
            wizard: RefCell::new(wizard),
            own_id: Cell::new(None),
        }
    }
}

impl Widget for KeyGenWizardScreen {
    fn widget_type_name(&self) -> &'static str {
        "KeyGenWizardScreen"
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn on_mount(&self, id: WidgetId) {
        self.own_id.set(Some(id));
    }

    fn on_unmount(&self, _id: WidgetId) {
        self.own_id.set(None);
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
                children.push(Box::new(Label::new("  Press Enter to continue.")));
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
                children.push(Box::new(Header::new("Generate Key")));
                children.push(Box::new(Label::new("")));
                children.push(Box::new(Label::new(
                    "Key generation requires Admin PIN entry.",
                )));
                children.push(Box::new(Label::new(
                    "Full implementation coming soon.",
                )));
                children.push(Box::new(Label::new("")));
                children.push(Box::new(Label::new("Press Enter or Esc to close.")));
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
                    // Transition to Result step (no modal — avoids Esc double-pop).
                    // Full PIN+keygen flow is a TODO.
                    w.step = KeyGenStep::Result;
                    drop(w);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
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
        // Request recompose for all state-mutating paths (early returns handle pop/push cases).
        drop(w);
        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
    }

    fn on_event(&self, event: &dyn std::any::Any, ctx: &AppContext) -> textual_rs::widget::EventPropagation {
        use textual_rs::widget::EventPropagation;
        use crossterm::event::KeyEvent;
        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            let mut w = self.wizard.borrow_mut();
            match key.code {
                KeyCode::Backspace => {
                    match w.step {
                        KeyGenStep::Identity => {
                            if w.active_field == 0 { w.name.pop(); }
                            else { w.email.pop(); }
                        }
                        KeyGenStep::Expiry if w.editing_custom_expiry => { w.custom_expiry.pop(); }
                        KeyGenStep::Backup if w.editing_path => { w.backup_path.pop(); }
                        _ => return EventPropagation::Continue,
                    }
                    drop(w);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c) => {
                    match w.step {
                        KeyGenStep::Identity => {
                            if w.active_field == 0 { w.name.push(c); }
                            else { w.email.push(c); }
                        }
                        KeyGenStep::Expiry if w.editing_custom_expiry => { w.custom_expiry.push(c); }
                        KeyGenStep::Backup if w.editing_path => { w.backup_path.push(c); }
                        _ => return EventPropagation::Continue,
                    }
                    drop(w);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return EventPropagation::Stop;
                }
                KeyCode::Tab => {
                    if w.step == KeyGenStep::Identity {
                        w.active_field = if w.active_field == 0 { 1 } else { 0 };
                        drop(w);
                        if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                        return EventPropagation::Stop;
                    }
                }
                _ => {}
            }
        }
        textual_rs::widget::EventPropagation::Continue
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
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
    KeyBinding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "",
        show: false,
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

    fn can_focus(&self) -> bool {
        true
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut children: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new("Import Key to YubiKey")),
            Box::new(Label::new(
                "This will import a GPG key from your keyring to the YubiKey.",
            )),
            Box::new(Label::new("Prerequisites:")),
            Box::new(Label::new("  - You must have a GPG key already generated")),
            Box::new(Label::new("  - The key must be in your GPG keyring")),
            Box::new(Label::new("")),
        ];

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
                ctx.push_screen_deferred(Box::new(PinInputWidget::new("Import Key — Admin PIN", &["Admin PIN"])));
            }
            "back" => ctx.pop_screen_deferred(),
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
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
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "",
        show: false,
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

    fn can_focus(&self) -> bool {
        true
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

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
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
    KeyBinding {
        key: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "",
        show: false,
    },
];

pub struct TouchPolicyScreen {
    yubikey_state: Option<YubiKeyState>,
    slot_index: RefCell<usize>,
    own_id: Cell<Option<WidgetId>>,
}

impl TouchPolicyScreen {
    pub fn new(yubikey_state: Option<YubiKeyState>) -> Self {
        Self {
            yubikey_state,
            slot_index: RefCell::new(0),
            own_id: Cell::new(None),
        }
    }
}

impl Widget for TouchPolicyScreen {
    fn widget_type_name(&self) -> &'static str {
        "TouchPolicyScreen"
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn on_mount(&self, id: WidgetId) {
        self.own_id.set(Some(id));
    }

    fn on_unmount(&self, _id: WidgetId) {
        self.own_id.set(None);
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
                    drop(idx);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                }
            }
            "slot_down" => {
                let mut idx = self.slot_index.borrow_mut();
                if *idx < 3 {
                    *idx += 1;
                    drop(idx);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                }
            }
            "select_slot" => {
                // Push value selection screen before the PIN screen
                let slot_idx = *self.slot_index.borrow();
                ctx.push_screen_deferred(Box::new(TouchPolicyValueScreen::new(slot_idx)));
            }
            "back" => ctx.pop_screen_deferred(),
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ── TouchPolicyValueScreen ─────────────────────────────────────────────────────

static TOUCH_POLICY_VALUE_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        key: KeyCode::Up,
        modifiers: KeyModifiers::NONE,
        action: "value_up",
        description: "Up  Prev",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Down,
        modifiers: KeyModifiers::NONE,
        action: "value_down",
        description: "Down  Next",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        action: "select_value",
        description: "Enter  Select",
        show: true,
    },
    KeyBinding {
        key: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        action: "back",
        description: "Esc  Back",
        show: true,
    },
];

const TOUCH_POLICY_VALUES: &[(&str, &str)] = &[
    ("Off",          "No touch required"),
    ("On",           "Touch required for every operation"),
    ("Fixed",        "Touch required; cannot be changed without admin PIN"),
    ("Cached",       "Touch required; cached for 15 seconds"),
    ("CachedFixed",  "Cached touch; cannot be changed without admin PIN"),
];

pub struct TouchPolicyValueScreen {
    slot_index: usize,
    value_index: RefCell<usize>,
}

impl TouchPolicyValueScreen {
    pub fn new(slot_index: usize) -> Self {
        Self {
            slot_index,
            value_index: RefCell::new(1), // default to "On"
        }
    }
}

impl Widget for TouchPolicyValueScreen {
    fn widget_type_name(&self) -> &'static str {
        "TouchPolicyValueScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let slot_names = ["Signature", "Encryption", "Authentication", "Attestation"];
        let slot_name = slot_names.get(self.slot_index).copied().unwrap_or("Unknown");
        let title = format!("Set Touch Policy — {} Slot", slot_name);

        let mut children: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new(&title)),
            Box::new(Label::new("Select touch policy:")),
            Box::new(Label::new("")),
        ];

        let sel = *self.value_index.borrow();
        for (i, (name, desc)) in TOUCH_POLICY_VALUES.iter().enumerate() {
            let marker = if i == sel { "> " } else { "  " };
            children.push(Box::new(Label::new(format!("{}{:<14}  {}", marker, name, desc))));
        }

        children.push(Box::new(Footer));
        children
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        TOUCH_POLICY_VALUE_BINDINGS
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "value_up" => {
                let mut idx = self.value_index.borrow_mut();
                if *idx > 0 { *idx -= 1; }
            }
            "value_down" => {
                let mut idx = self.value_index.borrow_mut();
                if *idx < TOUCH_POLICY_VALUES.len() - 1 { *idx += 1; }
            }
            "select_value" => {
                use crate::tui::widgets::pin_input::PinInputWidget;
                let slot_names = ["Signature", "Encryption", "Authentication", "Attestation"];
                let slot = slot_names.get(self.slot_index).copied().unwrap_or("Unknown");
                let (value_name, _) = TOUCH_POLICY_VALUES[*self.value_index.borrow()];
                let title = format!("Set Touch Policy — {} → {} — Admin PIN", slot, value_name);
                ctx.push_screen_deferred(Box::new(PinInputWidget::new(&title, &["Admin PIN"])));
            }
            "back" => ctx.pop_screen_deferred(),
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ── ProgressWidget (textual-rs port of old progress.rs) ───────────────────────

/// Spinner animation for use in operation-running screens.
///
/// Ported from `src/tui/widgets/progress.rs` — uses textual-rs Label
/// rather than direct ratatui frame rendering.
#[allow(dead_code)]
pub struct ProgressLabel {
    title: String,
    status: String,
    tick: usize,
}

impl ProgressLabel {
    #[allow(dead_code)]
    pub fn new(title: impl Into<String>, status: impl Into<String>, tick: usize) -> Self {
        Self {
            title: title.into(),
            status: status.into(),
            tick,
        }
    }
}

#[allow(dead_code)]
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

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
}

// ── DeleteKeyScreen ───────────────────────────────────────────────────────────

/// Confirmation screen wrapping ConfirmScreen to delete a single OpenPGP key slot.
///
/// Follows the DeleteCredentialScreen pattern from fido2.rs exactly.
/// Receives the slot and admin_pin from PinThenDeleteScreen.
pub struct DeleteKeyScreen {
    slot: crate::model::openpgp_delete::OpenPgpKeySlot,
    admin_pin: String,
    inner: crate::tui::widgets::popup::ConfirmScreen,
}

impl DeleteKeyScreen {
    pub fn new(slot: crate::model::openpgp_delete::OpenPgpKeySlot, admin_pin: String) -> Self {
        let body = format!(
            "Permanently delete the {} key?\n\nThis destroys the key material and cannot be undone.",
            slot.display_name()
        );
        Self {
            slot,
            admin_pin,
            inner: crate::tui::widgets::popup::ConfirmScreen::new("Delete Key Slot", body, true),
        }
    }
}

impl Widget for DeleteKeyScreen {
    fn widget_type_name(&self) -> &'static str {
        "DeleteKeyScreen"
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
                match crate::model::openpgp_delete::delete_openpgp_key(self.slot, &self.admin_pin) {
                    Ok(()) => {
                        ctx.pop_screen_deferred();
                        ctx.push_screen_deferred(Box::new(crate::tui::widgets::popup::PopupScreen::new(
                            "Success",
                            format!("{} key deleted.", self.slot.display_name()),
                        )));
                    }
                    Err(e) => {
                        ctx.pop_screen_deferred();
                        ctx.push_screen_deferred(Box::new(crate::tui::widgets::popup::PopupScreen::new(
                            "Error",
                            format!("Delete failed: {}", e),
                        )));
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

// ── PinThenDeleteScreen ───────────────────────────────────────────────────────

/// Admin PIN collection screen for OpenPGP slot deletion.
///
/// Follows the PinAuthScreen pattern from fido2.rs: on_event captures character
/// input into a RefCell<String>, Enter submits, Esc cancels.
/// On submit: pops self, pushes DeleteKeyScreen.
pub struct PinThenDeleteScreen {
    slot: crate::model::openpgp_delete::OpenPgpKeySlot,
    pin_input: RefCell<String>,
    error_message: RefCell<Option<String>>,
    own_id: Cell<Option<WidgetId>>,
}

impl PinThenDeleteScreen {
    pub fn new(slot: crate::model::openpgp_delete::OpenPgpKeySlot) -> Self {
        Self {
            slot,
            pin_input: RefCell::new(String::new()),
            error_message: RefCell::new(None),
            own_id: Cell::new(None),
        }
    }
}

static PIN_THEN_DELETE_BINDINGS: &[KeyBinding] = &[
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
        action: "submit",
        description: "Enter Continue",
        show: true,
    },
];

impl Widget for PinThenDeleteScreen {
    fn widget_type_name(&self) -> &'static str {
        "PinThenDeleteScreen"
    }

    fn on_mount(&self, id: WidgetId) { self.own_id.set(Some(id)); }
    fn on_unmount(&self, _id: WidgetId) { self.own_id.set(None); }

    fn can_focus(&self) -> bool {
        true
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let error = self.error_message.borrow().clone();
        let masked = "*".repeat(self.pin_input.borrow().len());

        let mut widgets: Vec<Box<dyn Widget>> = vec![
            Box::new(Header::new(
                format!("Delete {} Key — Enter Admin PIN", self.slot.display_name()).as_str()
            )),
            Box::new(Label::new("")),
            Box::new(Label::new(format!(
                "Enter Admin PIN to delete the {} key:",
                self.slot.display_name()
            ))),
            Box::new(Label::new(format!("> {}_", masked))),
        ];

        if let Some(ref err) = error {
            widgets.push(Box::new(Label::new("")));
            widgets.push(Box::new(Label::new(format!("Error: {}", err))));
        }

        widgets.push(Box::new(Label::new("")));
        widgets.push(Box::new(Footer));
        widgets
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        PIN_THEN_DELETE_BINDINGS
    }

    fn on_event(
        &self,
        event: &dyn std::any::Any,
        ctx: &AppContext,
    ) -> textual_rs::widget::EventPropagation {
        use crossterm::event::KeyEvent;
        use textual_rs::widget::EventPropagation;

        if let Some(key) = event.downcast_ref::<KeyEvent>() {
            match key.code {
                KeyCode::Esc => {
                    ctx.pop_screen_deferred();
                    return EventPropagation::Stop;
                }
                KeyCode::Backspace => {
                    self.pin_input.borrow_mut().pop();
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return EventPropagation::Stop;
                }
                KeyCode::Enter => {
                    self.on_action("submit", ctx);
                    return EventPropagation::Stop;
                }
                KeyCode::Char(c) => {
                    self.pin_input.borrow_mut().push(c);
                    if let Some(id) = self.own_id.get() { ctx.request_recompose(id); }
                    return EventPropagation::Stop;
                }
                _ => {}
            }
        }
        textual_rs::widget::EventPropagation::Continue
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "cancel" => ctx.pop_screen_deferred(),
            "submit" => {
                let pin = self.pin_input.borrow().clone();
                if pin.is_empty() {
                    *self.error_message.borrow_mut() = Some("Admin PIN cannot be empty".to_string());
                    return;
                }
                ctx.pop_screen_deferred();
                ctx.push_screen_deferred(Box::new(DeleteKeyScreen::new(self.slot, pin)));
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &AppContext, area: Rect, buf: &mut Buffer) {
        crate::tui::widgets::fill_screen_background(ctx, area, buf);
    }
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
        let mut app = TestApp::new_styled(80, 24, "", move || Box::new(KeysScreen::new(yk)));
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn keys_no_yubikey() {
        let mut app = TestApp::new_styled(80, 24, "", || Box::new(KeysScreen::new(None)));
        app.pilot().settle().await;
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn keys_import_screen() {
        let yubikey_states = crate::model::mock::mock_yubikey_states();
        let yk = yubikey_states.into_iter().next();
        let mut app = TestApp::new_styled(80, 24, "", move || Box::new(KeysScreen::new(yk)));
        let mut pilot = app.pilot();
        pilot.press(KeyCode::Char('i')).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_display_snapshot!(app.backend());
    }

    #[tokio::test]
    async fn keygen_wizard_renders() {
        let wizard = KeyGenWizard::new("2026-01-01");
        let mut app = TestApp::new_styled(80, 24, "", move || Box::new(KeyGenWizardScreen::new(wizard)));
        app.pilot().settle().await;
        // Check that the wizard actually rendered content (header/footer text visible)
        let buf = app.buffer();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Generate"), "wizard should render header with 'Generate'");
    }

    #[tokio::test]
    async fn keygen_wizard_step5_enter_transitions_to_result() {
        let mut wizard = KeyGenWizard::new("2026-01-01");
        wizard.step = KeyGenStep::Confirm;
        wizard.name = "Test User".to_string();
        wizard.email = "test@example.com".to_string();
        let mut app = TestApp::new_styled(80, 24, "", move || Box::new(KeyGenWizardScreen::new(wizard)));
        {
            let mut pilot = app.pilot();
            pilot.settle().await;
        }
        let content: String = app.buffer().content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Step 5/5"), "should be on Confirm step");
        {
            let mut pilot = app.pilot();
            pilot.press(KeyCode::Enter).await;
            pilot.settle().await;
        }
        let content: String = app.buffer().content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Generating Key") || content.contains("Generate Key"), "Enter should advance past Confirm to Result");
        assert!(!content.contains("Step 5/5"), "should no longer be on Confirm step after Enter");
    }

    #[tokio::test]
    async fn keys_help_popup_opens_and_closes() {
        let yubikey_states = crate::model::mock::mock_yubikey_states();
        let yk = yubikey_states.into_iter().next();
        let mut app = TestApp::new_styled(80, 24, "", move || Box::new(KeysScreen::new(yk)));
        // Open help popup
        {
            let mut pilot = app.pilot();
            pilot.press(KeyCode::Char('?')).await;
            pilot.settle().await;
        }
        let content: String = app.buffer().content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("OpenPGP Keys Help"), "help popup should be visible");
        // Dismiss with Esc
        {
            let mut pilot = app.pilot();
            pilot.press(KeyCode::Esc).await;
            pilot.settle().await;
        }
        let content: String = app.buffer().content().iter().map(|c| c.symbol()).collect();
        assert!(!content.contains("OpenPGP Keys Help"), "help popup should be dismissed after Esc");
    }

    #[tokio::test]
    async fn keys_touch_policy_value_screen() {
        // Navigate: KeysScreen → t (TouchPolicyScreen) → Enter (TouchPolicyValueScreen)
        let yubikey_states = crate::model::mock::mock_yubikey_states();
        let yk = yubikey_states.into_iter().next();
        let mut app = TestApp::new_styled(80, 24, "", move || Box::new(KeysScreen::new(yk)));
        let mut pilot = app.pilot();
        pilot.press(KeyCode::Char('t')).await;
        pilot.settle().await;
        pilot.press(KeyCode::Enter).await;
        pilot.settle().await;
        drop(pilot);
        insta::assert_display_snapshot!(app.backend());
    }
}
