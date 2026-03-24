#![allow(dead_code)]

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
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
    yubikey_state: Option<YubiKeyState>,
    pin_state: ui::pin::PinState,
    key_state: ui::keys::KeyState,
    ssh_state: ui::ssh::SshState,
}

impl App {
    pub fn new() -> Result<Self> {
        let diagnostics = Diagnostics::run()?;
        let yubikey_state = YubiKeyState::detect()?;

        Ok(Self {
            should_quit: false,
            current_screen: Screen::Dashboard,
            previous_screen: Screen::Dashboard,
            diagnostics,
            yubikey_state,
            pin_state: ui::pin::PinState::default(),
            key_state: ui::keys::KeyState::default(),
            ssh_state: ui::ssh::SshState::default(),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        // Note: We deliberately DON'T enable mouse capture to allow text selection
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run the event loop
        let result = self.event_loop(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
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
            Screen::Dashboard => ui::dashboard::render(frame, chunks[0], self),
            Screen::Diagnostics => ui::diagnostics::render(frame, chunks[0], &self.diagnostics),
            Screen::Help => ui::help::render(frame, chunks[0]),
            Screen::Keys => {
                ui::keys::render(frame, chunks[0], &self.yubikey_state, &self.key_state)
            }
            Screen::PinManagement => {
                ui::pin::render(frame, chunks[0], &self.yubikey_state, &self.pin_state)
            }
            Screen::SshWizard => ui::ssh::render(frame, chunks[0], self, &self.ssh_state),
        }

        // Render status bar
        ui::render_status_bar(frame, chunks[1], self);
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                self.handle_key_event(key)?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
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
                        KeyCode::Esc => {
                            self.current_screen = Screen::Dashboard;
                        }
                        _ => {}
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
                        self.pin_state.screen = PinScreen::ChangeUserPin;
                    }
                    KeyCode::Char('a') => {
                        self.pin_state.screen = PinScreen::ChangeAdminPin;
                    }
                    KeyCode::Char('r') => {
                        self.pin_state.screen = PinScreen::SetResetCode;
                    }
                    KeyCode::Char('u') => {
                        self.pin_state.screen = PinScreen::UnblockUserPin;
                    }
                    KeyCode::Esc => {
                        self.current_screen = Screen::Dashboard;
                    }
                    _ => {}
                },
                _ => {
                    match key.code {
                        KeyCode::Enter => {
                            // Execute the operation
                            self.execute_pin_operation()?;
                        }
                        KeyCode::Esc => {
                            self.pin_state.screen = PinScreen::Main;
                            self.pin_state.message = None;
                        }
                        _ => {}
                    }
                }
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
            KeyCode::Char('1') => self.current_screen = Screen::Dashboard,
            KeyCode::Char('2') => self.current_screen = Screen::Diagnostics,
            KeyCode::Char('3') => self.current_screen = Screen::Keys,
            KeyCode::Char('4') => {
                self.current_screen = Screen::PinManagement;
                self.pin_state = ui::pin::PinState::default();
            }
            KeyCode::Char('5') => self.current_screen = Screen::SshWizard,
            KeyCode::Char('r') => {
                // Refresh: re-run diagnostics and detect YubiKey
                self.diagnostics = Diagnostics::run()?;
                self.yubikey_state = YubiKeyState::detect()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_pin_operation(&mut self) -> Result<()> {
        use crate::yubikey::pin_operations;
        use ui::pin::PinScreen;

        // Switch to alternate screen to run GPG interactively
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

        let result = match self.pin_state.screen {
            PinScreen::ChangeUserPin => pin_operations::change_user_pin(),
            PinScreen::ChangeAdminPin => pin_operations::change_admin_pin(),
            PinScreen::SetResetCode => pin_operations::set_reset_code(),
            PinScreen::UnblockUserPin => pin_operations::unblock_user_pin(),
            _ => Ok("No operation".to_string()),
        };

        // Restore TUI
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
        crossterm::terminal::enable_raw_mode()?;

        // Update state
        match result {
            Ok(msg) => {
                self.pin_state.message = Some(msg);
                // Refresh YubiKey state to get updated PIN counters
                self.yubikey_state = YubiKeyState::detect()?;
            }
            Err(e) => {
                self.pin_state.message = Some(format!("Error: {}", e));
            }
        }

        self.pin_state.screen = PinScreen::Main;
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
                self.yubikey_state = YubiKeyState::detect()?;
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

    pub fn current_screen(&self) -> Screen {
        self.current_screen
    }

    pub fn yubikey_state(&self) -> Option<&YubiKeyState> {
        self.yubikey_state.as_ref()
    }

    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }
}
