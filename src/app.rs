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

    let app_state = AppState {
        yubikey_states,
        mock_mode: mock,
        ..AppState::default()
    };

    let theme = load_theme_from_config();

    // Detect factory-default before moving app_state into the closure.
    let onboarding_yk = app_state
        .yubikey_state()
        .filter(|yk| crate::model::onboarding::is_factory_default(yk))
        .cloned();

    // textual-rs 0.3.8 renders all screens bottom-to-top for modal layering.
    // Screens without an explicit background let the Dashboard bleed through.
    // This CSS rule gives every pushed screen a solid background.
    const SCREEN_CSS: &str = "
DashboardScreen, OnboardingScreen,
KeysScreen, KeyGenWizardScreen, ImportKeyScreen, KeyDetailScreen, TouchPolicyScreen,
DiagnosticsScreen, PinManagementScreen, UnblockWizardScreen, FactoryResetScreen,
SshWizardScreen, PivScreen, HelpScreen, GlossaryScreen,
OathScreen, AddAccountScreen, ImportUriScreen, DeleteConfirmScreen,
OathUnlockScreen, OathPasswordMgmtScreen,
OathSetPasswordScreen, OathChangePasswordScreen, OathRemovePasswordScreen,
Fido2Screen, PinSetScreen, PinChangeScreen, PinAuthScreen,
DeleteCredentialScreen, ResetGuidanceScreen, ResetConfirmScreen,
OtpScreen, PopupScreen, ConfirmScreen
{ background: $background; }

HelpScreen Markdown { flex-grow: 1; }
GlossaryScreen Markdown { flex-grow: 1; }
";

    let mut app = App::new(move || {
        if let Some(ref yk) = onboarding_yk {
            // Factory-default key: show onboarding first; dismiss pushes DashboardScreen.
            return Box::new(crate::tui::onboarding::OnboardingScreen::new_startup(
                yk.clone(),
                app_state.clone(),
                diagnostics.clone(),
            ));
        }
        Box::new(crate::tui::dashboard::DashboardScreen::new(
            app_state.clone(),
            diagnostics.clone(),
        ))
    }).with_css(SCREEN_CSS);
    app.set_theme(theme);
    app.run()?;

    Ok(())
}
