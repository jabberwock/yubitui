use textual_rs::css::theme::{theme_by_name, default_dark_theme, Theme};

/// Available theme names matching textual-rs built-ins (D-11).
pub const THEME_NAMES: &[&str] = &[
    "tokyo-night",
    "nord",
    "gruvbox-dark",
    "dracula",
    "catppuccin-mocha",
];

/// Default theme when none configured (D-12: Claude's discretion = tokyo-night).
pub const DEFAULT_THEME: &str = "tokyo-night";

pub fn load_theme_from_config() -> Theme {
    let name = crate::tui::config::read_theme_name();
    name.as_deref()
        .and_then(theme_by_name)
        .unwrap_or_else(|| {
            theme_by_name(DEFAULT_THEME).unwrap_or_else(default_dark_theme)
        })
}

pub fn next_theme_name(current: &str) -> &'static str {
    let idx = THEME_NAMES.iter().position(|&n| n == current).unwrap_or(0);
    THEME_NAMES[(idx + 1) % THEME_NAMES.len()]
}
