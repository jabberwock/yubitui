use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
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
    Keys,
    PinManagement,
    SshWizard,
}

pub struct App {
    should_quit: bool,
    current_screen: Screen,
    diagnostics: Diagnostics,
    yubikey_state: Option<YubiKeyState>,
}

impl App {
    pub fn new() -> Result<Self> {
        let diagnostics = Diagnostics::run()?;
        let yubikey_state = YubiKeyState::detect()?;

        Ok(Self {
            should_quit: false,
            current_screen: Screen::Dashboard,
            diagnostics,
            yubikey_state,
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
            Screen::Dashboard => ui::dashboard::render(frame, chunks[0], self),
            Screen::Diagnostics => ui::diagnostics::render(frame, chunks[0], &self.diagnostics),
            Screen::Keys => ui::keys::render(frame, chunks[0], &self.yubikey_state),
            Screen::PinManagement => ui::pin::render(frame, chunks[0], &self.yubikey_state),
            Screen::SshWizard => ui::ssh::render(frame, chunks[0], self),
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
            KeyCode::Char('4') => self.current_screen = Screen::PinManagement,
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
