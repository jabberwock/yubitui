use anyhow::Result;
use textual_rs::App;

use crate::model::{AppState, YubiKeyState};
use crate::diagnostics::Diagnostics;
use crate::tui::theme::load_theme_from_config;

pub fn run(mock: bool) -> Result<()> {
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

    let _app_state = AppState {
        yubikey_states,
        mock_mode: mock,
        ..AppState::default()
    };

    let theme = load_theme_from_config();

    // TODO: RootScreen will be built in subsequent plans as screens are migrated.
    // For now, start with Help screen as the first migrated widget.
    let mut app = App::new(move || {
        Box::new(crate::tui::help::HelpScreen::new())
    });
    app.set_theme(theme);
    app.run()?;

    // diagnostics is kept alive through the run but not currently used
    drop(diagnostics);

    Ok(())
}
