#![allow(dead_code)]

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseButton,
        MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use std::io;

use crate::{diagnostics::Diagnostics, ui, yubikey::YubiKeyState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Diagnostics,
    Help,
    Keys,
    PinManagement,
    SshWizard,
}

pub struct App {
    should_quit: bool,
    current_screen: Screen,
    previous_screen: Screen,
    diagnostics: Diagnostics,
    yubikey_states: Vec<YubiKeyState>,
    selected_yubikey_idx: usize,
    pin_state: ui::pin::PinState,
    key_state: ui::keys::KeyState,
    ssh_state: ui::ssh::SshState,
    dashboard_state: ui::dashboard::DashboardState,
}

impl App {
    pub fn new() -> Result<Self> {
        let diagnostics = Diagnostics::run()?;
        let yubikey_states = YubiKeyState::detect_all().unwrap_or_default();

        Ok(Self {
            should_quit: false,
            current_screen: Screen::Dashboard,
            previous_screen: Screen::Dashboard,
            diagnostics,
            yubikey_states,
            selected_yubikey_idx: 0,
            pin_state: ui::pin::PinState::default(),
            key_state: ui::keys::KeyState::default(),
            ssh_state: ui::ssh::SshState::default(),
            dashboard_state: ui::dashboard::DashboardState::default(),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run the event loop
        let result = self.event_loop(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn event_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.render(f))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render(&self, frame: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(frame.area());

        // Render current screen
        match self.current_screen {
            Screen::Dashboard => {
                ui::dashboard::render(frame, chunks[0], self, &self.dashboard_state)
            }
            Screen::Diagnostics => ui::diagnostics::render(frame, chunks[0], &self.diagnostics),
            Screen::Help => ui::help::render(frame, chunks[0]),
            Screen::Keys => {
                let yk = self.yubikey_state().cloned();
                ui::keys::render(frame, chunks[0], &yk, &self.key_state)
            }
            Screen::PinManagement => {
                let yk = self.yubikey_state().cloned();
                ui::pin::render(frame, chunks[0], &yk, &self.pin_state)
            }
            Screen::SshWizard => ui::ssh::render(frame, chunks[0], self, &self.ssh_state),
        }

        // Render status bar
        ui::render_status_bar(frame, chunks[1], self);
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => self.handle_key_event(key)?,
                Event::Mouse(mouse) => self.handle_mouse_event(mouse)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if self.current_screen == Screen::Dashboard
                    && self.dashboard_state.show_context_menu
                {
                    self.dashboard_state.show_context_menu = false;
                }
            }
            MouseEventKind::ScrollUp => {
                if self.current_screen == Screen::Dashboard
                    && self.dashboard_state.show_context_menu
                {
                    if self.dashboard_state.menu_selected_index > 0 {
                        self.dashboard_state.menu_selected_index -= 1;
                    }
                } else if self.current_screen == Screen::Keys
                    && self.key_state.screen == ui::keys::KeyScreen::ImportKey
                    && self.key_state.selected_key_index > 0
                {
                    self.key_state.selected_key_index -= 1;
                }
            }
            MouseEventKind::ScrollDown => {
                if self.current_screen == Screen::Dashboard
                    && self.dashboard_state.show_context_menu
                {
                    if self.dashboard_state.menu_selected_index < 4 {
                        self.dashboard_state.menu_selected_index += 1;
                    }
                } else if self.current_screen == Screen::Keys
                    && self.key_state.screen == ui::keys::KeyScreen::ImportKey
                {
                    let max = self.key_state.available_keys.len().saturating_sub(1);
                    if self.key_state.selected_key_index < max {
                        self.key_state.selected_key_index += 1;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // On Windows, crossterm fires both Press and Release events.
        // Ignore everything except Press to prevent double-handling.
        if key.kind != crossterm::event::KeyEventKind::Press {
            return Ok(());
        }

        // Global ? handler — open help from any screen
        if key.code == KeyCode::Char('?') {
            if self.current_screen == Screen::Help {
                self.current_screen = self.previous_screen;
            } else {
                self.previous_screen = self.current_screen;
                self.current_screen = Screen::Help;
            }
            return Ok(());
        }

        // Handle Help screen — Esc closes it
        if self.current_screen == Screen::Help {
            if key.code == KeyCode::Esc {
                self.current_screen = self.previous_screen;
            }
            return Ok(());
        }

        // Handle Key management sub-screens
        if self.current_screen == Screen::Keys {
            use ui::keys::KeyScreen;

            match self.key_state.screen {
                KeyScreen::Main => {
                    // Attestation popup takes priority: Esc closes it
                    if self.key_state.attestation_popup.is_some() {
                        if key.code == KeyCode::Esc {
                            self.key_state.attestation_popup = None;
                        }
                        return Ok(());
                    }
                    match key.code {
                        KeyCode::Char('v') => {
                            self.key_state.screen = KeyScreen::ViewStatus;
                        }
                        KeyCode::Char('i') => {
                            self.key_state.screen = KeyScreen::ImportKey;
                            self.key_state.selected_key_index = 0;
                            // Load available keys
                            if let Ok(keys) = crate::yubikey::key_operations::list_gpg_keys() {
                                self.key_state.available_keys = keys;
                            }
                        }
                        KeyCode::Char('g') => {
                            self.key_state.screen = KeyScreen::GenerateKey;
                        }
                        KeyCode::Char('e') => {
                            self.key_state.screen = KeyScreen::ExportSSH;
                        }
                        KeyCode::Char('k') => {
                            // Fetch key attributes via ykman
                            self.key_state.screen = KeyScreen::KeyAttributes;
                            match crate::yubikey::key_operations::get_key_attributes() {
                                Ok(attrs) => self.key_state.key_attributes = Some(attrs),
                                Err(e) => {
                                    self.key_state.key_attributes = None;
                                    self.key_state.message =
                                        Some(format!("Could not fetch attributes: {}", e));
                                }
                            }
                        }
                        KeyCode::Char('s') => {
                            // Show SSH public key in popup
                            self.key_state.screen = KeyScreen::SshPubkeyPopup;
                            match crate::yubikey::key_operations::get_ssh_public_key_text() {
                                Ok(key) => self.key_state.ssh_pubkey = Some(key),
                                Err(e) => {
                                    self.key_state.ssh_pubkey = None;
                                    self.key_state.message = Some(format!("{}", e));
                                }
                            }
                        }
                        KeyCode::Char('t') => {
                            // Enter touch policy slot selection
                            self.key_state.screen = KeyScreen::SetTouchPolicy;
                            self.key_state.touch_slot_index = 0;
                        }
                        KeyCode::Char('a') => {
                            // Show attestation certificate for sig slot
                            let serial = self.yubikey_state().map(|yk| yk.info.serial);
                            match crate::yubikey::attestation::get_attestation_cert("sig", serial) {
                                Ok(pem) => {
                                    self.key_state.attestation_popup = Some(pem);
                                }
                                Err(e) => {
                                    self.key_state.message = Some(format!("Attestation: {}", e));
                                }
                            }
                        }
                        KeyCode::Esc => {
                            self.current_screen = Screen::Dashboard;
                        }
                        _ => {}
                    }
                }
                KeyScreen::KeyAttributes | KeyScreen::SshPubkeyPopup => {
                    if key.code == KeyCode::Esc {
                        self.key_state.screen = KeyScreen::Main;
                        self.key_state.message = None;
                    }
                }
                KeyScreen::SetTouchPolicy => {
                    match key.code {
                        KeyCode::Up => {
                            if self.key_state.touch_slot_index > 0 {
                                self.key_state.touch_slot_index -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if self.key_state.touch_slot_index < 3 {
                                self.key_state.touch_slot_index += 1;
                            }
                        }
                        KeyCode::Enter => {
                            self.key_state.touch_policy_index = 0;
                            self.key_state.screen = KeyScreen::SetTouchPolicySelect;
                        }
                        KeyCode::Esc => {
                            self.key_state.screen = KeyScreen::Main;
                            self.key_state.message = None;
                        }
                        _ => {}
                    }
                }
                KeyScreen::SetTouchPolicySelect => {
                    match key.code {
                        KeyCode::Up => {
                            if self.key_state.touch_policy_index > 0 {
                                self.key_state.touch_policy_index -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if self.key_state.touch_policy_index < 4 {
                                self.key_state.touch_policy_index += 1;
                            }
                        }
                        KeyCode::Enter => {
                            let policy = ui::keys::touch_policy_from_index(self.key_state.touch_policy_index);
                            if policy.is_irreversible() {
                                self.key_state.screen = KeyScreen::SetTouchPolicyConfirm;
                            } else {
                                let slot = ui::keys::touch_slot_name(self.key_state.touch_slot_index).to_string();
                                self.execute_touch_policy_set(&slot, &policy)?;
                            }
                        }
                        KeyCode::Esc => {
                            self.key_state.screen = KeyScreen::SetTouchPolicy;
                        }
                        _ => {}
                    }
                }
                KeyScreen::SetTouchPolicyConfirm => {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            let slot = ui::keys::touch_slot_name(self.key_state.touch_slot_index).to_string();
                            let policy = ui::keys::touch_policy_from_index(self.key_state.touch_policy_index);
                            self.execute_touch_policy_set(&slot, &policy)?;
                        }
                        _ => {
                            self.key_state.message = Some("Cancelled".to_string());
                            self.key_state.screen = KeyScreen::Main;
                        }
                    }
                }
                _ => match key.code {
                    KeyCode::Enter => {
                        self.execute_key_operation()?;
                    }
                    KeyCode::Up => {
                        if self.key_state.screen == KeyScreen::ImportKey
                            && self.key_state.selected_key_index > 0
                        {
                            self.key_state.selected_key_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.key_state.screen == KeyScreen::ImportKey {
                            let max = self.key_state.available_keys.len().saturating_sub(1);
                            if self.key_state.selected_key_index < max {
                                self.key_state.selected_key_index += 1;
                            }
                        }
                    }
                    KeyCode::Esc => {
                        self.key_state.screen = KeyScreen::Main;
                        self.key_state.message = None;
                    }
                    _ => {}
                },
            }
            return Ok(());
        }

        // Handle SSH wizard sub-screens
        if self.current_screen == Screen::SshWizard {
            use ui::ssh::SshScreen;

            match self.ssh_state.screen {
                SshScreen::Main => {
                    match key.code {
                        KeyCode::Char('1') => {
                            self.ssh_state.screen = SshScreen::EnableSSH;
                        }
                        KeyCode::Char('2') => {
                            self.ssh_state.screen = SshScreen::ConfigureShell;
                        }
                        KeyCode::Char('3') => {
                            self.ssh_state.screen = SshScreen::RestartAgent;
                        }
                        KeyCode::Char('4') => {
                            self.ssh_state.screen = SshScreen::ExportKey;
                        }
                        KeyCode::Char('5') => {
                            self.ssh_state.screen = SshScreen::TestConnection;
                        }
                        KeyCode::Char('r') => {
                            // Refresh status
                            self.refresh_ssh_status()?;
                        }
                        KeyCode::Esc => {
                            self.current_screen = Screen::Dashboard;
                        }
                        _ => {}
                    }
                }
                _ => match key.code {
                    KeyCode::Enter => {
                        self.execute_ssh_operation()?;
                    }
                    KeyCode::Esc => {
                        self.ssh_state.screen = SshScreen::Main;
                        self.ssh_state.message = None;
                    }
                    _ => {}
                },
            }
            return Ok(());
        }

        // Handle PIN management sub-screens
        if self.current_screen == Screen::PinManagement {
            use ui::pin::PinScreen;

            match self.pin_state.screen {
                PinScreen::Main => match key.code {
                    KeyCode::Char('c') => {
                        // Transition to TUI PIN input for Change User PIN
                        use crate::ui::widgets::pin_input::PinInputState;
                        self.pin_state.pending_operation = Some(PinScreen::ChangeUserPin);
                        self.pin_state.pin_input = Some(PinInputState::new(
                            "Change User PIN",
                            &["Current PIN", "New PIN", "Confirm New PIN"],
                        ));
                        self.pin_state.screen = PinScreen::PinInputActive;
                    }
                    KeyCode::Char('a') => {
                        // Transition to TUI PIN input for Change Admin PIN
                        use crate::ui::widgets::pin_input::PinInputState;
                        self.pin_state.pending_operation = Some(PinScreen::ChangeAdminPin);
                        self.pin_state.pin_input = Some(PinInputState::new(
                            "Change Admin PIN",
                            &["Current Admin PIN", "New Admin PIN", "Confirm New Admin PIN"],
                        ));
                        self.pin_state.screen = PinScreen::PinInputActive;
                    }
                    KeyCode::Char('r') => {
                        // Transition to TUI PIN input for Set Reset Code
                        use crate::ui::widgets::pin_input::PinInputState;
                        self.pin_state.pending_operation = Some(PinScreen::SetResetCode);
                        self.pin_state.pin_input = Some(PinInputState::new(
                            "Set Reset Code",
                            &["Admin PIN", "New Reset Code", "Confirm Reset Code"],
                        ));
                        self.pin_state.screen = PinScreen::PinInputActive;
                    }
                    KeyCode::Char('u') => {
                        // Launch the unblock wizard instead of direct passthrough
                        self.pin_state.screen = PinScreen::UnblockWizardCheck;
                        self.pin_state.ykman_available =
                            crate::yubikey::pin_operations::is_ykman_available();
                    }
                    KeyCode::Esc => {
                        self.current_screen = Screen::Dashboard;
                    }
                    _ => {}
                },
                PinScreen::PinInputActive => {
                    use crate::ui::widgets::pin_input::PinInputAction;
                    let action = if let Some(pin_input) = self.pin_state.pin_input.as_mut() {
                        pin_input.handle_key(key.code)
                    } else {
                        PinInputAction::Cancel
                    };
                    match action {
                        PinInputAction::Submit => {
                            self.execute_pin_operation_programmatic()?;
                        }
                        PinInputAction::Cancel => {
                            self.pin_state.pin_input = None;
                            self.pin_state.pending_operation = None;
                            self.pin_state.screen = PinScreen::Main;
                            self.pin_state.message = None;
                        }
                        PinInputAction::Continue => {}
                    }
                }
                PinScreen::OperationResult => {
                    // Any key returns to Main
                    self.pin_state.screen = PinScreen::Main;
                    self.pin_state.pin_input = None;
                    self.pin_state.pending_operation = None;
                    self.pin_state.operation_running = false;
                    self.pin_state.operation_status = None;
                }
                PinScreen::UnblockWizardCheck => match key.code {
                    KeyCode::Char('1') => {
                        if let Some(yk) = self.yubikey_state() {
                            if yk.pin_status.reset_code_retries > 0 {
                                self.pin_state.screen = PinScreen::UnblockWizardWithReset;
                                self.pin_state.unblock_path = Some(ui::pin::UnblockPath::ResetCode);
                            }
                        }
                    }
                    KeyCode::Char('2') => {
                        if let Some(yk) = self.yubikey_state() {
                            if yk.pin_status.admin_pin_retries > 0 {
                                self.pin_state.screen = PinScreen::UnblockWizardWithAdmin;
                                self.pin_state.unblock_path = Some(ui::pin::UnblockPath::AdminPin);
                            }
                        }
                    }
                    KeyCode::Char('3') => {
                        if let Some(yk) = self.yubikey_state() {
                            if yk.pin_status.reset_code_retries == 0
                                && yk.pin_status.admin_pin_retries == 0
                                && self.pin_state.ykman_available
                            {
                                self.pin_state.screen = PinScreen::UnblockWizardFactoryReset;
                                self.pin_state.unblock_path =
                                    Some(ui::pin::UnblockPath::FactoryReset);
                            }
                        }
                    }
                    KeyCode::Esc => {
                        self.pin_state.screen = PinScreen::Main;
                        self.pin_state.message = None;
                        self.pin_state.unblock_path = None;
                    }
                    _ => {}
                },
                PinScreen::UnblockWizardWithReset => {
                    match key.code {
                        KeyCode::Enter => {
                            // Transition to TUI PIN input for Unblock with Reset Code
                            use crate::ui::widgets::pin_input::PinInputState;
                            self.pin_state.pending_operation =
                                Some(PinScreen::UnblockWizardWithReset);
                            self.pin_state.pin_input = Some(PinInputState::new(
                                "Unblock with Reset Code",
                                &["Reset Code", "New User PIN", "Confirm New PIN"],
                            ));
                            self.pin_state.screen = PinScreen::PinInputActive;
                        }
                        KeyCode::Esc => {
                            self.pin_state.screen = PinScreen::UnblockWizardCheck;
                            self.pin_state.unblock_path = None;
                        }
                        _ => {}
                    }
                }
                PinScreen::UnblockWizardWithAdmin => {
                    match key.code {
                        KeyCode::Enter => {
                            // Transition to TUI PIN input for Unblock with Admin PIN
                            use crate::ui::widgets::pin_input::PinInputState;
                            self.pin_state.pending_operation =
                                Some(PinScreen::UnblockWizardWithAdmin);
                            self.pin_state.pin_input = Some(PinInputState::new(
                                "Unblock with Admin PIN",
                                &["Admin PIN", "New User PIN", "Confirm New PIN"],
                            ));
                            self.pin_state.screen = PinScreen::PinInputActive;
                        }
                        KeyCode::Esc => {
                            self.pin_state.screen = PinScreen::UnblockWizardCheck;
                            self.pin_state.unblock_path = None;
                        }
                        _ => {}
                    }
                }
                PinScreen::UnblockWizardFactoryReset => {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if self.pin_state.confirm_factory_reset {
                                // Second confirmation -- execute factory reset
                                let result =
                                    crate::yubikey::pin_operations::factory_reset_openpgp();
                                match result {
                                    Ok(msg) => {
                                        self.pin_state.message = Some(msg);
                                        self.yubikey_states = YubiKeyState::detect_all().unwrap_or_default();
                                        if self.selected_yubikey_idx >= self.yubikey_states.len() {
                                            self.selected_yubikey_idx = 0;
                                        }
                                    }
                                    Err(e) => {
                                        self.pin_state.message = Some(format!("Error: {}", e));
                                    }
                                }
                                self.pin_state.screen = PinScreen::Main;
                                self.pin_state.confirm_factory_reset = false;
                                self.pin_state.unblock_path = None;
                            } else {
                                // First Y press -- show confirmation overlay
                                self.pin_state.confirm_factory_reset = true;
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            if self.pin_state.confirm_factory_reset {
                                self.pin_state.confirm_factory_reset = false;
                            }
                        }
                        KeyCode::Esc => {
                            self.pin_state.confirm_factory_reset = false;
                            self.pin_state.screen = PinScreen::UnblockWizardCheck;
                            self.pin_state.unblock_path = None;
                        }
                        _ => {}
                    }
                }
                _ => {
                    if key.code == KeyCode::Esc {
                        self.pin_state.screen = PinScreen::Main;
                        self.pin_state.message = None;
                    }
                }
            }
            return Ok(());
        }

        // Handle Dashboard context menu navigation
        if self.current_screen == Screen::Dashboard && self.dashboard_state.show_context_menu {
            match key.code {
                KeyCode::Up => {
                    if self.dashboard_state.menu_selected_index > 0 {
                        self.dashboard_state.menu_selected_index -= 1;
                    }
                }
                KeyCode::Down => {
                    if self.dashboard_state.menu_selected_index < 4 {
                        self.dashboard_state.menu_selected_index += 1;
                    }
                }
                KeyCode::Enter => {
                    let target = match self.dashboard_state.menu_selected_index {
                        0 => Screen::Diagnostics,
                        1 => Screen::Keys,
                        2 => Screen::PinManagement,
                        3 => Screen::SshWizard,
                        4 => Screen::Help,
                        _ => Screen::Dashboard,
                    };
                    self.dashboard_state.show_context_menu = false;
                    self.dashboard_state.menu_selected_index = 0;
                    if target == Screen::PinManagement {
                        self.pin_state = ui::pin::PinState::default();
                    }
                    self.current_screen = target;
                }
                KeyCode::Esc => {
                    self.dashboard_state.show_context_menu = false;
                    self.dashboard_state.menu_selected_index = 0;
                }
                _ => {}
            }
            return Ok(());
        }

        // Regular navigation
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.current_screen == Screen::Dashboard {
                    self.should_quit = true;
                } else {
                    self.current_screen = Screen::Dashboard;
                }
            }
            KeyCode::Tab => {
                // Switch active YubiKey on Dashboard
                if self.current_screen == Screen::Dashboard && !self.yubikey_states.is_empty() {
                    self.selected_yubikey_idx = (self.selected_yubikey_idx + 1) % self.yubikey_states.len();
                }
            }
            KeyCode::Char('1') => self.current_screen = Screen::Dashboard,
            KeyCode::Char('2') => self.current_screen = Screen::Diagnostics,
            KeyCode::Char('3') => self.current_screen = Screen::Keys,
            KeyCode::Char('4') => {
                self.current_screen = Screen::PinManagement;
                self.pin_state = ui::pin::PinState::default();
            }
            KeyCode::Char('5') => self.current_screen = Screen::SshWizard,
            KeyCode::Char('r') => {
                // Refresh: re-run diagnostics and detect YubiKeys
                self.diagnostics = Diagnostics::run()?;
                self.yubikey_states = YubiKeyState::detect_all().unwrap_or_default();
                if self.selected_yubikey_idx >= self.yubikey_states.len() {
                    self.selected_yubikey_idx = 0;
                }
            }
            KeyCode::Enter | KeyCode::Char('m') => {
                if self.current_screen == Screen::Dashboard {
                    self.dashboard_state.show_context_menu = true;
                    self.dashboard_state.menu_selected_index = 0;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Execute the current PIN operation programmatically using PINs collected
    /// by the TUI PIN input widget.  No terminal escape occurs.
    fn execute_pin_operation_programmatic(&mut self) -> Result<()> {
        use crate::yubikey::pin_operations;
        use ui::pin::PinScreen;

        // Extract PIN values before mutably borrowing self.
        let values: Vec<String> = self
            .pin_state
            .pin_input
            .as_ref()
            .map(|p| p.values().into_iter().map(|s| s.to_owned()).collect())
            .unwrap_or_default();

        let pending = self.pin_state.pending_operation;

        // Show a brief "working" state (synchronous call — no actual async here).
        self.pin_state.screen = PinScreen::OperationRunning;
        self.pin_state.operation_running = true;
        self.pin_state.operation_status = Some("Verifying PIN...".to_string());

        let op_result = match pending {
            Some(PinScreen::ChangeUserPin) => {
                let (current, new_pin) = (
                    values.first().map(String::as_str).unwrap_or(""),
                    values.get(1).map(String::as_str).unwrap_or(""),
                );
                pin_operations::change_user_pin_programmatic(current, new_pin)
            }
            Some(PinScreen::ChangeAdminPin) => {
                let (current, new_pin) = (
                    values.first().map(String::as_str).unwrap_or(""),
                    values.get(1).map(String::as_str).unwrap_or(""),
                );
                pin_operations::change_admin_pin_programmatic(current, new_pin)
            }
            Some(PinScreen::SetResetCode) => {
                let (admin, reset) = (
                    values.first().map(String::as_str).unwrap_or(""),
                    values.get(1).map(String::as_str).unwrap_or(""),
                );
                pin_operations::set_reset_code_programmatic(admin, reset)
            }
            Some(PinScreen::UnblockWizardWithReset) => {
                let (code, new_pin) = (
                    values.first().map(String::as_str).unwrap_or(""),
                    values.get(1).map(String::as_str).unwrap_or(""),
                );
                pin_operations::unblock_user_pin_programmatic(code, new_pin)
            }
            Some(PinScreen::UnblockWizardWithAdmin) => {
                let (admin, new_pin) = (
                    values.first().map(String::as_str).unwrap_or(""),
                    values.get(1).map(String::as_str).unwrap_or(""),
                );
                pin_operations::unblock_user_pin_programmatic(admin, new_pin)
            }
            _ => {
                self.pin_state.screen = PinScreen::Main;
                self.pin_state.operation_running = false;
                return Ok(());
            }
        };

        // Clear input state
        self.pin_state.pin_input = None;
        self.pin_state.pending_operation = None;
        self.pin_state.operation_running = false;
        self.pin_state.operation_status = None;

        // Update state based on result
        match op_result {
            Ok(result) => {
                let msg = if result.success {
                    result.messages.join("\n")
                } else {
                    let mut lines = vec!["Operation failed:".to_string()];
                    lines.extend(result.messages);
                    lines.join("\n")
                };
                self.pin_state.message = Some(if msg.is_empty() {
                    if result.success {
                        "Operation completed successfully".to_string()
                    } else {
                        "Operation failed".to_string()
                    }
                } else {
                    msg
                });
                // Refresh YubiKey state to get updated PIN counters
                self.yubikey_states = YubiKeyState::detect_all().unwrap_or_default();
                if self.selected_yubikey_idx >= self.yubikey_states.len() {
                    self.selected_yubikey_idx = 0;
                }
            }
            Err(e) => {
                self.pin_state.message = Some(format!("Error: {}", e));
            }
        }

        self.pin_state.screen = PinScreen::OperationResult;
        Ok(())
    }

    fn execute_key_operation(&mut self) -> Result<()> {
        use crate::yubikey::key_operations;
        use ui::keys::KeyScreen;

        // Switch to alternate screen to run operations interactively
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

        let result = match self.key_state.screen {
            KeyScreen::ViewStatus => key_operations::view_card_status(),
            KeyScreen::ImportKey => {
                if self.key_state.available_keys.is_empty() {
                    Ok("No keys available to import".to_string())
                } else {
                    let idx = if self.key_state.selected_key_index
                        < self.key_state.available_keys.len()
                    {
                        self.key_state.selected_key_index
                    } else {
                        0
                    };
                    key_operations::import_key_to_card(&self.key_state.available_keys[idx])
                }
            }
            KeyScreen::GenerateKey => key_operations::generate_key_on_card(),
            KeyScreen::ExportSSH => key_operations::export_ssh_public_key(),
            _ => Ok("No operation".to_string()),
        };

        // Restore TUI
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
        crossterm::terminal::enable_raw_mode()?;

        // Update state
        match result {
            Ok(msg) => {
                self.key_state.message = Some(msg);
                // Refresh YubiKey state
                self.yubikey_states = YubiKeyState::detect_all().unwrap_or_default();
                if self.selected_yubikey_idx >= self.yubikey_states.len() {
                    self.selected_yubikey_idx = 0;
                }
            }
            Err(e) => {
                self.key_state.message = Some(format!("Error: {}", e));
            }
        }

        self.key_state.screen = KeyScreen::Main;
        Ok(())
    }

    fn execute_ssh_operation(&mut self) -> Result<()> {
        use crate::yubikey::ssh_operations;
        use ui::ssh::SshScreen;

        let result = match self.ssh_state.screen {
            SshScreen::EnableSSH => ssh_operations::enable_ssh_support(),
            SshScreen::ConfigureShell => ssh_operations::configure_shell_ssh(),
            SshScreen::RestartAgent => ssh_operations::restart_gpg_agent(),
            SshScreen::ExportKey => {
                // Switch to terminal for displaying key
                crossterm::terminal::disable_raw_mode()?;
                crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

                let key_result = crate::yubikey::key_operations::export_ssh_public_key();

                if let Ok(key) = &key_result {
                    println!("\n{}", "=".repeat(70));
                    println!("SSH Public Key:");
                    println!("{}", "=".repeat(70));
                    println!("{}", key);
                    println!("{}", "=".repeat(70));
                    println!("\nCopy this key and add it to:");
                    println!("  • ~/.ssh/authorized_keys on remote servers");
                    println!("  • GitHub/GitLab SSH keys");
                    println!("\nPress ENTER to continue...");

                    use std::io::Read;
                    let _ = std::io::stdin().read(&mut [0u8]).unwrap();
                }

                // Restore TUI
                crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
                crossterm::terminal::enable_raw_mode()?;

                key_result
            }
            SshScreen::TestConnection => {
                // This needs interactive input, so switch to terminal
                crossterm::terminal::disable_raw_mode()?;
                crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

                println!("Test SSH Connection");
                println!("==================");
                print!("Username: ");
                use std::io::{self, Write};
                io::stdout().flush()?;
                let mut user = String::new();
                io::stdin().read_line(&mut user)?;

                print!("Hostname: ");
                io::stdout().flush()?;
                let mut host = String::new();
                io::stdin().read_line(&mut host)?;

                let test_result = ssh_operations::test_ssh_connection(user.trim(), host.trim());

                // Restore TUI
                crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
                crossterm::terminal::enable_raw_mode()?;

                test_result
            }
            _ => Ok("No operation".to_string()),
        };

        // Update state
        match result {
            Ok(msg) => {
                self.ssh_state.message = Some(msg);
                self.refresh_ssh_status()?;
            }
            Err(e) => {
                self.ssh_state.message = Some(format!("Error: {}", e));
            }
        }

        self.ssh_state.screen = SshScreen::Main;
        Ok(())
    }

    fn refresh_ssh_status(&mut self) -> Result<()> {
        use crate::yubikey::ssh_operations;

        self.ssh_state.ssh_enabled = ssh_operations::check_ssh_support_enabled().unwrap_or(false);

        // Check if SSH_AUTH_SOCK is set correctly
        if let Ok(expected) = ssh_operations::get_gpg_ssh_socket() {
            if let Ok(current) = std::env::var("SSH_AUTH_SOCK") {
                self.ssh_state.shell_configured = current == expected;
            }
        }

        // Check if agent is running
        self.ssh_state.agent_running = std::process::Command::new("gpgconf")
            .arg("--list-dirs")
            .arg("agent-socket")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Ok(())
    }

    fn execute_touch_policy_set(
        &mut self,
        slot: &str,
        policy: &crate::yubikey::touch_policy::TouchPolicy,
    ) -> Result<()> {
        let serial = self.yubikey_state().map(|yk| yk.info.serial);

        // Drop to terminal for ykman Admin PIN entry
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

        let result = crate::yubikey::touch_policy::set_touch_policy(slot, policy, serial);
        match result {
            Ok(mut child) => {
                let status = child.wait();
                match status {
                    Ok(s) if s.success() => {
                        self.key_state.message =
                            Some(format!("Touch policy set to {} for {}", policy, slot));
                    }
                    Ok(_) => {
                        self.key_state.message =
                            Some("Touch policy change cancelled or failed".to_string());
                    }
                    Err(e) => {
                        self.key_state.message = Some(format!("Error: {}", e));
                    }
                }
            }
            Err(e) => {
                self.key_state.message = Some(format!("Error: {}", e));
            }
        }

        // Restore TUI
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
        crossterm::terminal::enable_raw_mode()?;

        // Refresh YubiKey state
        self.yubikey_states = YubiKeyState::detect_all().unwrap_or_default();
        if self.selected_yubikey_idx >= self.yubikey_states.len() {
            self.selected_yubikey_idx = 0;
        }
        self.key_state.screen = ui::keys::KeyScreen::Main;
        Ok(())
    }

    pub fn current_screen(&self) -> Screen {
        self.current_screen
    }

    pub fn yubikey_state(&self) -> Option<&YubiKeyState> {
        self.yubikey_states.get(self.selected_yubikey_idx)
    }

    pub fn yubikey_count(&self) -> usize {
        self.yubikey_states.len()
    }

    pub fn selected_yubikey_idx(&self) -> usize {
        self.selected_yubikey_idx
    }

    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }
}
