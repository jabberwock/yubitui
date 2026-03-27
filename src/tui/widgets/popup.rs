//! Popup / modal dialog widgets for textual-rs screen stack.
//!
//! In textual-rs, popups become pushed screens — overlay dialogs are implemented
//! via `ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(PopupScreen::new(...)))))`.
//!
//! This module provides:
//! - `PopupScreen` — a generic titled popup with body text and a Close button.
//! - `ConfirmScreen` — a confirmation dialog with Cancel (default) and Confirm buttons.

use std::cell::Cell;

use textual_rs::{Widget, Label, Button, ButtonVariant, Footer};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
pub use textual_rs::widget::screen::ModalScreen;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// A generic informational popup with a title, body text, and a Close button.
///
/// Push it via:
/// ```no_run
/// ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(PopupScreen::new("Title", "Body")))));
/// ```
pub struct PopupScreen {
    title: String,
    body: String,
    /// Set to true by on_action("close") so the parent can check after pop.
    pub closed: Cell<bool>,
}

impl PopupScreen {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            closed: Cell::new(false),
        }
    }
}

impl Widget for PopupScreen {
    fn widget_type_name(&self) -> &'static str {
        "PopupScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        vec![
            Box::new(textual_rs::Header::new(&self.title)),
            Box::new(Label::new(&self.body)),
            Box::new(Button::new("Close")),
            Box::new(Footer),
        ]
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        &[
            KeyBinding {
                key: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                action: "close",
                description: "Close",
                show: true,
            },
            KeyBinding {
                key: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                action: "close",
                description: "Close",
                show: false,
            },
        ]
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "close" => {
                self.closed.set(true);
                ctx.pop_screen_deferred();
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}

/// A confirmation dialog with Cancel (default) and Confirm buttons.
///
/// Per UI-SPEC Modal/Overlay Rules:
/// - Cancel is auto-focused as the default (safe) option.
/// - Confirm uses `ButtonVariant::Error` for destructive actions.
///
/// Push it via:
/// ```no_run
/// ctx.push_screen_deferred(Box::new(ModalScreen::new(Box::new(ConfirmScreen::new(
///     "Confirm Reset",
///     "Are you sure? This cannot be undone.",
///     true, // destructive
/// )))));
/// ```
pub struct ConfirmScreen {
    title: String,
    message: String,
    destructive: bool,
    /// True after confirmed; false after cancelled (or not yet acted on).
    pub confirmed: Cell<bool>,
}

impl ConfirmScreen {
    pub fn new(
        title: impl Into<String>,
        message: impl Into<String>,
        destructive: bool,
    ) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            destructive,
            confirmed: Cell::new(false),
        }
    }
}

impl Widget for ConfirmScreen {
    fn widget_type_name(&self) -> &'static str {
        "ConfirmScreen"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let header_text = if self.destructive {
            format!("WARNING: {}", self.title)
        } else {
            self.title.clone()
        };

        let confirm_button = if self.destructive {
            Button::new("Confirm").with_variant(ButtonVariant::Error)
        } else {
            Button::new("Confirm")
        };

        vec![
            Box::new(textual_rs::Header::new(&header_text)),
            Box::new(Label::new(&self.message)),
            // Cancel first — it is the default safe option (focused first by Tab order).
            Box::new(Button::new("Cancel")),
            Box::new(confirm_button),
            Box::new(Footer),
        ]
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        &[
            KeyBinding {
                key: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                action: "cancel",
                description: "Cancel",
                show: true,
            },
            KeyBinding {
                key: KeyCode::Char('y'),
                modifiers: KeyModifiers::NONE,
                action: "confirm",
                description: "Confirm",
                show: true,
            },
            KeyBinding {
                key: KeyCode::Char('n'),
                modifiers: KeyModifiers::NONE,
                action: "cancel",
                description: "Cancel",
                show: false,
            },
        ]
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        match action {
            "confirm" => {
                self.confirmed.set(true);
                ctx.pop_screen_deferred();
            }
            "cancel" => {
                self.confirmed.set(false);
                ctx.pop_screen_deferred();
            }
            _ => {}
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}
