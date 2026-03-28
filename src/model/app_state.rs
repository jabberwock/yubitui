use serde::Serialize;

/// Screen navigation -- pure enum, no TUI types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Screen {
    Dashboard,
    Diagnostics,
    Fido2,
    Help,
    Keys,
    Oath,
    PinManagement,
    SshWizard,
    Piv,
}

/// Application state that is Tauri-serializable.
/// Contains all data a GUI front-end would need.
/// The TUI runtime (`App` in app.rs) owns this alongside the terminal handle.
#[derive(Debug, Clone, Serialize)]
pub struct AppState {
    pub current_screen: Screen,
    pub previous_screen: Screen,
    pub should_quit: bool,
    pub yubikey_states: Vec<super::YubiKeyState>,
    pub selected_yubikey_idx: usize,
    pub mock_mode: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_screen: Screen::Dashboard,
            previous_screen: Screen::Dashboard,
            should_quit: false,
            yubikey_states: Vec::new(),
            selected_yubikey_idx: 0,
            mock_mode: false,
        }
    }
}

impl AppState {
    pub fn yubikey_state(&self) -> Option<&super::YubiKeyState> {
        self.yubikey_states.get(self.selected_yubikey_idx)
    }

    pub fn yubikey_count(&self) -> usize {
        self.yubikey_states.len()
    }
}
