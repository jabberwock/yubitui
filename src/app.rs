#![allow(dead_code)]

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEvent,
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

use crate::{diagnostics::Diagnostics, model::YubiKeyState};
use crate::model::{AppState, Screen};

pub struct App {
    state: AppState,
    diagnostics: Diagnostics,
    pin_state: crate::tui::pin::PinState,
    key_state: crate::tui::keys::KeyState,
    ssh_state: crate::tui::ssh::SshState,
    dashboard_state: crate::tui::dashboard::DashboardState,
    import_task: Option<
        std::sync::mpsc::Receiver<anyhow::Result<crate::model::key_operations::ImportResult>>,
    >,
}

impl App {
    pub fn new(mock: bool) -> Result<Self> {
        let diagnostics = if mock {
            Diagnostics::default()
        } else {
            Diagnostics::run()?
        };

        let yubikey_states = if mock {
            crate::model::mock::mock_yubikey_states()
        } else {
            YubiKeyState::detect_all().unwrap_or_default()
        };

        Ok(Self {
            state: AppState {
                yubikey_states,
                mock_mode: mock,
                ..AppState::default()
            },
            diagnostics,
            pin_state: crate::tui::pin::PinState::default(),
            key_state: crate::tui::keys::KeyState::default(),
            ssh_state: crate::tui::ssh::SshState::default(),
            dashboard_state: crate::tui::dashboard::DashboardState::default(),
            import_task: None,
        })
    }

    pub fn is_mock(&self) -> bool {
        self.state.mock_mode
    }

    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        if let Err(e) = execute!(stdout, EnableMouseCapture) {
            tracing::debug!("Mouse capture unavailable (likely ConPTY): {}", e);
        }
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run the event loop
        let result = self.event_loop(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    fn event_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        while !self.state.should_quit {
            terminal.draw(|f| self.render(f))?;
            self.poll_import_task()?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(frame.area());

        // Extract click_regions to satisfy borrow checker during render
        let mut click_regions = std::mem::take(&mut self.state.click_regions);

        // Render current screen
        match self.state.current_screen {
            Screen::Dashboard => {
                crate::tui::dashboard::render(frame, chunks[0], self, &self.dashboard_state, &mut click_regions)
            }
            Screen::Diagnostics => {
                crate::tui::diagnostics::render(frame, chunks[0], &self.diagnostics, &mut click_regions)
            }
            Screen::Help => crate::tui::help::render(frame, chunks[0], &mut click_regions),
            Screen::Keys => {
                let yk = self.yubikey_state().cloned();
                crate::tui::keys::render(frame, chunks[0], &yk, &self.key_state, &mut click_regions)
            }
            Screen::PinManagement => {
                let yk = self.yubikey_state().cloned();
                crate::tui::pin::render(frame, chunks[0], &yk, &self.pin_state, &mut click_regions)
            }
            Screen::SshWizard => {
                crate::tui::ssh::render(frame, chunks[0], self, &self.ssh_state, &mut click_regions)
            }
            Screen::Piv => {
                let yk = self.yubikey_state().cloned();
                crate::tui::piv::render(frame, chunks[0], &yk, &mut click_regions)
            }
        }

        self.state.click_regions = click_regions;

        // Render status bar
        crate::tui::render_status_bar(frame, chunks[1], self);
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
        match self.state.current_screen {
            Screen::Dashboard => {
                let action =
                    crate::tui::dashboard::handle_mouse(&mut self.dashboard_state, mouse);
                self.execute_dashboard_action(action)?;
            }
            Screen::Keys => {
                let action = crate::tui::keys::handle_mouse(&mut self.key_state, mouse);
                self.execute_key_action(action)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // On Windows, crossterm fires both Press and Release events.
        if key.kind != crossterm::event::KeyEventKind::Press {
            return Ok(());
        }

        // Global ? handler — open/close help from any screen
        if key.code == KeyCode::Char('?') {
            if self.state.current_screen == Screen::Help {
                self.state.current_screen = self.state.previous_screen;
            } else {
                self.state.previous_screen = self.state.current_screen;
                self.state.current_screen = Screen::Help;
            }
            return Ok(());
        }

        // Dispatch to per-screen handler
        match self.state.current_screen {
            Screen::Help => {
                match crate::tui::help::handle_key(key) {
                    crate::tui::help::HelpAction::Close => {
                        self.state.current_screen = self.state.previous_screen;
                    }
                    crate::tui::help::HelpAction::None => {}
                }
            }
            Screen::Dashboard => {
                let yk_count = self.state.yubikey_states.len();
                let action = crate::tui::dashboard::handle_key(
                    &mut self.dashboard_state,
                    key,
                    yk_count,
                );
                self.execute_dashboard_action(action)?;
            }
            Screen::Keys => {
                let yk = self.yubikey_state().cloned();
                let action =
                    crate::tui::keys::handle_key(&mut self.key_state, key, yk.as_ref());
                self.execute_key_action(action)?;
            }
            Screen::PinManagement => {
                let yk = self.yubikey_state().cloned();
                let action =
                    crate::tui::pin::handle_key(&mut self.pin_state, key, yk.as_ref());
                self.execute_pin_action(action)?;
            }
            Screen::SshWizard => {
                let action = crate::tui::ssh::handle_key(&mut self.ssh_state, key);
                self.execute_ssh_action(action)?;
            }
            Screen::Piv => {
                let action = crate::tui::piv::handle_key(key);
                self.execute_piv_action(action)?;
            }
            Screen::Diagnostics => {
                let action = crate::tui::diagnostics::handle_key(key);
                self.execute_diagnostics_action(action)?;
            }
        }
        Ok(())
    }

    // --- Action executors ---

    fn execute_dashboard_action(
        &mut self,
        action: crate::tui::dashboard::DashboardAction,
    ) -> Result<()> {
        use crate::tui::dashboard::DashboardAction;
        match action {
            DashboardAction::Quit => {
                self.state.should_quit = true;
            }
            DashboardAction::NavigateTo(screen) => {
                self.navigate_to(screen)?;
            }
            DashboardAction::SelectMenuItem(idx) => {
                let target = match idx {
                    0 => Screen::Diagnostics,
                    1 => Screen::Keys,
                    2 => Screen::PinManagement,
                    3 => Screen::SshWizard,
                    4 => Screen::Piv,
                    5 => Screen::Help,
                    _ => Screen::Dashboard,
                };
                self.navigate_to(target)?;
            }
            DashboardAction::Refresh => {
                if self.state.mock_mode {
                    self.state.yubikey_states = crate::model::mock::mock_yubikey_states();
                } else {
                    self.diagnostics = Diagnostics::run()?;
                    self.state.yubikey_states = YubiKeyState::detect_all().unwrap_or_default();
                }
                if self.state.selected_yubikey_idx >= self.state.yubikey_states.len() {
                    self.state.selected_yubikey_idx = 0;
                }
            }
            DashboardAction::SwitchYubiKey => {
                if !self.state.yubikey_states.is_empty() {
                    self.state.selected_yubikey_idx =
                        (self.state.selected_yubikey_idx + 1) % self.state.yubikey_states.len();
                }
            }
            DashboardAction::OpenContextMenu => {
                self.dashboard_state.show_context_menu = true;
                self.dashboard_state.menu_selected_index = 0;
            }
            DashboardAction::CloseContextMenu => {
                self.dashboard_state.show_context_menu = false;
            }
            DashboardAction::MenuUp => {
                if self.dashboard_state.menu_selected_index > 0 {
                    self.dashboard_state.menu_selected_index -= 1;
                }
            }
            DashboardAction::MenuDown => {
                if self.dashboard_state.menu_selected_index < 5 {
                    self.dashboard_state.menu_selected_index += 1;
                }
            }
            DashboardAction::None => {}
        }
        Ok(())
    }

    fn execute_key_action(&mut self, action: crate::tui::keys::KeyAction) -> Result<()> {
        use crate::tui::keys::KeyAction;
        match action {
            KeyAction::NavigateTo(screen) => {
                self.navigate_to(screen)?;
            }
            KeyAction::ExecuteViewStatus => {
                self.execute_key_operation()?;
            }
            KeyAction::ExecuteExportSSH => {
                self.execute_key_operation()?;
            }
            KeyAction::ExecuteKeyImport => {
                self.execute_key_import()?;
            }
            KeyAction::ExecuteKeyGen => {
                self.execute_keygen_batch()?;
            }
            KeyAction::ExecuteTouchPolicySet {
                slot,
                policy,
                admin_pin,
            } => {
                self.execute_touch_policy_set(&slot, &policy, &admin_pin)?;
            }
            KeyAction::LoadGpgKeys => {
                if let Ok(keys) = crate::model::key_operations::list_gpg_keys() {
                    self.key_state.available_keys = keys;
                }
                if !self.key_state.available_keys.is_empty() {
                    use crate::tui::widgets::pin_input::PinInputState;
                    let mut pin_input = PinInputState::new(
                        "Import Key",
                        &["Key Passphrase (blank if none)", "Admin PIN"],
                    );
                    pin_input.set_optional(0);
                    self.key_state.pin_input = Some(pin_input);
                    self.key_state.screen = crate::tui::keys::KeyScreen::KeyImportPinInput;
                } else {
                    self.key_state.message = Some("No GPG keys found in keyring.".to_string());
                    self.key_state.screen = crate::tui::keys::KeyScreen::ImportKey;
                }
            }
            KeyAction::LoadKeyAttributes => {
                match crate::model::key_operations::get_key_attributes() {
                    Ok(attrs) => self.key_state.key_attributes = Some(attrs),
                    Err(e) => {
                        self.key_state.key_attributes = None;
                        self.key_state.message =
                            Some(format!("Could not fetch attributes: {}", e));
                    }
                }
            }
            KeyAction::LoadSshPubkey => {
                match crate::model::key_operations::get_ssh_public_key_text() {
                    Ok(key) => self.key_state.ssh_pubkey = Some(key),
                    Err(e) => {
                        self.key_state.ssh_pubkey = None;
                        self.key_state.message = Some(format!("{}", e));
                    }
                }
            }
            KeyAction::LoadAttestation { serial } => {
                match crate::model::attestation::get_attestation_cert("sig", serial) {
                    Ok(pem) => {
                        self.key_state.attestation_popup = Some(pem);
                    }
                    Err(e) => {
                        self.key_state.message = Some(format!("Attestation: {}", e));
                    }
                }
            }
            KeyAction::None => {}
        }
        Ok(())
    }

    fn execute_pin_action(&mut self, action: crate::tui::pin::PinAction) -> Result<()> {
        use crate::tui::pin::PinAction;
        match action {
            PinAction::NavigateTo(screen) => {
                self.navigate_to(screen)?;
            }
            PinAction::ExecutePinOperation => {
                self.execute_pin_operation_programmatic()?;
            }
            PinAction::ExecuteFactoryReset => {
                let result = crate::model::pin_operations::factory_reset_openpgp();
                match result {
                    Ok(msg) => {
                        self.pin_state.message = Some(msg);
                        if !self.state.mock_mode {
                            self.state.yubikey_states =
                                YubiKeyState::detect_all().unwrap_or_default();
                            if self.state.selected_yubikey_idx >= self.state.yubikey_states.len() {
                                self.state.selected_yubikey_idx = 0;
                            }
                        }
                    }
                    Err(e) => {
                        self.pin_state.message = Some(format!("Error: {}", e));
                    }
                }
                self.pin_state.screen = crate::tui::pin::PinScreen::Main;
                self.pin_state.confirm_factory_reset = false;
                self.pin_state.unblock_path = None;
            }
            PinAction::None => {}
        }
        Ok(())
    }

    fn execute_ssh_action(&mut self, action: crate::tui::ssh::SshAction) -> Result<()> {
        use crate::tui::ssh::SshAction;
        match action {
            SshAction::NavigateTo(screen) => {
                self.navigate_to(screen)?;
            }
            SshAction::ExecuteSshOperation => {
                self.execute_ssh_operation()?;
            }
            SshAction::RefreshSshStatus => {
                self.refresh_ssh_status()?;
            }
            SshAction::None => {}
        }
        Ok(())
    }

    fn execute_piv_action(&mut self, action: crate::tui::piv::PivAction) -> Result<()> {
        use crate::tui::piv::PivAction;
        match action {
            PivAction::NavigateTo(screen) => {
                self.navigate_to(screen)?;
            }
            PivAction::None => {}
        }
        Ok(())
    }

    fn execute_diagnostics_action(
        &mut self,
        action: crate::tui::diagnostics::DiagnosticsAction,
    ) -> Result<()> {
        use crate::tui::diagnostics::DiagnosticsAction;
        match action {
            DiagnosticsAction::NavigateTo(screen) => {
                self.navigate_to(screen)?;
            }
            DiagnosticsAction::None => {}
        }
        Ok(())
    }

    /// Navigate to a screen, applying any screen-entry side effects.
    fn navigate_to(&mut self, screen: Screen) -> Result<()> {
        if screen == Screen::PinManagement {
            self.pin_state = crate::tui::pin::PinState::default();
        }
        if screen == Screen::SshWizard {
            self.refresh_ssh_status()?;
        }
        self.state.current_screen = screen;
        Ok(())
    }

    // --- Hardware operation functions (unchanged) ---

    fn execute_pin_operation_programmatic(&mut self) -> Result<()> {
        use crate::model::pin_operations;
        use crate::tui::pin::PinScreen;

        let values: Vec<String> = self
            .pin_state
            .pin_input
            .as_ref()
            .map(|p| p.values().into_iter().map(|s| s.to_owned()).collect())
            .unwrap_or_default();

        let pending = self.pin_state.pending_operation;

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

        self.pin_state.pin_input = None;
        self.pin_state.pending_operation = None;
        self.pin_state.operation_running = false;
        self.pin_state.operation_status = None;

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
                if !self.state.mock_mode {
                    self.state.yubikey_states = YubiKeyState::detect_all().unwrap_or_default();
                    if self.state.selected_yubikey_idx >= self.state.yubikey_states.len() {
                        self.state.selected_yubikey_idx = 0;
                    }
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
        use crate::model::key_operations;
        use crate::tui::keys::KeyScreen;

        match self.key_state.screen {
            KeyScreen::ViewStatus => {
                let result = key_operations::view_card_status();
                match result {
                    Ok(msg) => {
                        self.key_state.message = Some(msg);
                        if !self.state.mock_mode {
                            self.state.yubikey_states = YubiKeyState::detect_all().unwrap_or_default();
                            if self.state.selected_yubikey_idx >= self.state.yubikey_states.len() {
                                self.state.selected_yubikey_idx = 0;
                            }
                        }
                        self.key_state.screen = KeyScreen::KeyOperationResult;
                    }
                    Err(e) => {
                        self.key_state.message = Some(format!("Error: {}", e));
                        self.key_state.screen = KeyScreen::Main;
                    }
                }
            }
            KeyScreen::ExportSSH => {
                match key_operations::get_ssh_public_key_text() {
                    Ok(key) => {
                        self.key_state.ssh_pubkey = Some(key);
                        self.key_state.screen = KeyScreen::SshPubkeyPopup;
                    }
                    Err(_) => {
                        self.key_state.ssh_pubkey = None;
                        self.key_state.screen = KeyScreen::SshPubkeyPopup;
                    }
                }
            }
            _ => {
                self.key_state.screen = KeyScreen::Main;
            }
        }

        Ok(())
    }

    fn execute_keygen_batch(&mut self) -> Result<()> {
        use crate::tui::keys::{KeyGenStep, KeyScreen};
        use crate::model::key_operations::{generate_key_batch, import_key_programmatic};

        let admin_pin = self
            .key_state
            .pin_input
            .as_ref()
            .and_then(|p| p.values().into_iter().next().map(|s| s.to_owned()))
            .unwrap_or_default();

        let params = match crate::tui::keys::keygen_params_from_state(&self.key_state) {
            Some(p) => p,
            None => {
                // No wizard yet — launch it
                let date_str = current_date_ymd();
                self.key_state.keygen_wizard =
                    Some(crate::tui::keys::KeyGenWizard::new(&date_str));
                self.key_state.message = None;
                self.key_state.screen = KeyScreen::KeyGenWizardActive;
                return Ok(());
            }
        };

        if let Some(ref mut w) = self.key_state.keygen_wizard {
            w.step = KeyGenStep::Running;
        }
        self.key_state.operation_status = Some("Generating key...".to_string());
        self.key_state.pin_input = None;

        match generate_key_batch(&params, &admin_pin) {
            Ok(gen_result) => {
                let mut parts = gen_result.messages.clone();

                if gen_result.success {
                    if let Some(ref fp) = gen_result.fingerprint {
                        // Key created in local keyring with %no-protection.
                        // Transfer it to the card now. The key has no passphrase
                        // (empty string) because %no-protection was used. The admin
                        // PIN collected by the wizard is the card admin PIN.
                        self.key_state.operation_status =
                            Some("Transferring key to card...".to_string());
                        match import_key_programmatic(fp, "", &admin_pin) {
                            Ok(import_result) => {
                                parts.push(format!(
                                    "Card: {}",
                                    import_result.format_slots()
                                ));
                                let all_failed = !import_result.sig_filled
                                    && !import_result.enc_filled
                                    && !import_result.aut_filled;
                                if all_failed {
                                    parts.push(
                                        "Key transfer to card failed — key remains in local keyring."
                                            .to_string(),
                                    );
                                }
                                parts.extend(import_result.messages);
                            }
                            Err(e) => {
                                parts.push(format!("Key transfer error: {}", e));
                            }
                        }
                        parts.push(format!("Fingerprint: {}", fp));
                    }
                } else if parts.is_empty() {
                    parts.push("Key generation failed.".to_string());
                }

                self.key_state.message =
                    Some(if parts.is_empty() { "Key generated and transferred to card.".to_string() } else { parts.join("\n") });
                if let Some(ref mut w) = self.key_state.keygen_wizard { w.step = KeyGenStep::Result; }
                self.key_state.screen = KeyScreen::KeyOperationResult;
                if !self.state.mock_mode {
                    self.state.yubikey_states = YubiKeyState::detect_all().unwrap_or_default();
                    if self.state.selected_yubikey_idx >= self.state.yubikey_states.len() {
                        self.state.selected_yubikey_idx = 0;
                    }
                }
            }
            Err(e) => {
                self.key_state.message = Some(format!("Key generation error: {}", e));
                self.key_state.screen = KeyScreen::KeyOperationResult;
            }
        }
        self.key_state.operation_status = None;
        Ok(())
    }

    fn execute_key_import(&mut self) -> Result<()> {
        use crate::tui::keys::KeyScreen;
        use crate::model::key_operations::import_key_programmatic;

        let (key_passphrase, admin_pin) = self
            .key_state
            .pin_input
            .as_ref()
            .map(|p| {
                let vals = p.values();
                (
                    vals.first().copied().unwrap_or("").to_owned(),
                    vals.get(1).copied().unwrap_or("").to_owned(),
                )
            })
            .unwrap_or_default();

        let idx = self
            .key_state
            .selected_key_index
            .min(self.key_state.available_keys.len().saturating_sub(1));
        let key_id = match self.key_state.available_keys.get(idx) {
            Some(k) => k.clone(),
            None => {
                self.key_state.message = Some("No key selected.".to_string());
                self.key_state.screen = KeyScreen::Main;
                return Ok(());
            }
        };

        self.key_state.screen = KeyScreen::KeyImportRunning;
        self.key_state.operation_status =
            Some("Importing key to card... (touch YubiKey if it is flashing)".to_string());
        self.key_state.pin_input = None;

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = import_key_programmatic(&key_id, &key_passphrase, &admin_pin);
            let _ = tx.send(result);
        });
        self.import_task = Some(rx);
        Ok(())
    }

    fn poll_import_task(&mut self) -> Result<()> {
        use crate::tui::keys::KeyScreen;
        use std::sync::mpsc::TryRecvError;

        let result = match self.import_task.as_ref() {
            Some(rx) => match rx.try_recv() {
                Ok(result) => result,
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => {
                    self.import_task = None;
                    return Ok(());
                }
            },
            None => return Ok(()),
        };
        self.import_task = None;

        match result {
            Ok(import_result) => {
                let slots = import_result.format_slots();
                let msg = if import_result.messages.is_empty() {
                    "Key imported successfully.".to_string()
                } else {
                    import_result.messages.join("\n")
                };
                self.key_state.message = Some(msg);
                self.key_state.import_result = Some(slots);
                self.key_state.screen = KeyScreen::KeyOperationResult;
                if !self.state.mock_mode {
                    self.state.yubikey_states = YubiKeyState::detect_all().unwrap_or_default();
                    if self.state.selected_yubikey_idx >= self.state.yubikey_states.len() {
                        self.state.selected_yubikey_idx = 0;
                    }
                }
            }
            Err(e) => {
                self.key_state.message = Some(format!("Import error: {}", e));
                self.key_state.screen = KeyScreen::KeyOperationResult;
            }
        }
        self.key_state.operation_status = None;
        Ok(())
    }

    fn execute_ssh_operation(&mut self) -> Result<()> {
        use crate::model::ssh_operations;
        use crate::tui::ssh::SshScreen;

        match self.ssh_state.screen {
            SshScreen::EnableSSH => {
                let result = ssh_operations::enable_ssh_support();
                match result {
                    Ok(msg) => self.ssh_state.message = Some(msg),
                    Err(e) => self.ssh_state.message = Some(format!("Error: {}", e)),
                }
                self.refresh_ssh_status()?;
                self.ssh_state.screen = SshScreen::Main;
            }
            SshScreen::ConfigureShell => {
                let result = ssh_operations::configure_shell_ssh();
                match result {
                    Ok(msg) => self.ssh_state.message = Some(msg),
                    Err(e) => self.ssh_state.message = Some(format!("Error: {}", e)),
                }
                self.refresh_ssh_status()?;
                self.ssh_state.screen = SshScreen::Main;
            }
            SshScreen::RestartAgent => {
                let result = ssh_operations::restart_gpg_agent();
                match result {
                    Ok(msg) => self.ssh_state.message = Some(msg),
                    Err(e) => self.ssh_state.message = Some(format!("Error: {}", e)),
                }
                self.refresh_ssh_status()?;
                self.ssh_state.screen = SshScreen::Main;
            }
            SshScreen::ExportKey => {
                match crate::model::key_operations::get_ssh_public_key_text() {
                    Ok(key) => {
                        self.key_state.ssh_pubkey = Some(key);
                        self.state.current_screen = Screen::Keys;
                        self.key_state.screen = crate::tui::keys::KeyScreen::SshPubkeyPopup;
                    }
                    Err(e) => {
                        self.ssh_state.message = Some(format!("Error: {}", e));
                        self.ssh_state.screen = SshScreen::Main;
                    }
                }
            }
            SshScreen::TestConnection => {
                let user = self.ssh_state.test_conn_user.trim().to_string();
                let host = self.ssh_state.test_conn_host.trim().to_string();
                let result = ssh_operations::test_ssh_connection(&user, &host);
                match result {
                    Ok(msg) => self.ssh_state.message = Some(msg),
                    Err(e) => self.ssh_state.message = Some(format!("Error: {}", e)),
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn refresh_ssh_status(&mut self) -> Result<()> {
        use crate::model::ssh_operations;

        self.ssh_state.ssh_enabled =
            ssh_operations::check_ssh_support_enabled().unwrap_or(false);

        if let Ok(expected) = ssh_operations::get_gpg_ssh_socket() {
            if let Ok(current) = std::env::var("SSH_AUTH_SOCK") {
                self.ssh_state.shell_configured = current == expected;
            }
        }

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
        policy: &crate::model::touch_policy::TouchPolicy,
        admin_pin: &str,
    ) -> Result<()> {
        let serial = self.yubikey_state().map(|yk| yk.info.serial);

        match crate::model::touch_policy::set_touch_policy(slot, policy, serial, admin_pin) {
            Ok(msg) => {
                self.key_state.message = Some(msg);
            }
            Err(e) => {
                self.key_state.message = Some(format!("Error: {}", e));
            }
        }

        if !self.state.mock_mode {
            self.state.yubikey_states = YubiKeyState::detect_all().unwrap_or_default();
            if self.state.selected_yubikey_idx >= self.state.yubikey_states.len() {
                self.state.selected_yubikey_idx = 0;
            }
        }
        self.key_state.screen = crate::tui::keys::KeyScreen::Main;
        Ok(())
    }

    // --- Public accessors ---

    pub fn current_screen(&self) -> Screen {
        self.state.current_screen
    }

    pub fn yubikey_state(&self) -> Option<&YubiKeyState> {
        self.state.yubikey_states.get(self.state.selected_yubikey_idx)
    }

    pub fn yubikey_count(&self) -> usize {
        self.state.yubikey_states.len()
    }

    pub fn selected_yubikey_idx(&self) -> usize {
        self.state.selected_yubikey_idx
    }

    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }
}

/// Return current date as "YYYY-MM-DD" string using only std::time (no chrono).
fn current_date_ymd() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;

    // Algorithm: https://howardhinnant.github.io/date_algorithms.html
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}", y, m, d)
}
