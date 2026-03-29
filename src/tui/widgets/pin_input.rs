//! Reusable TUI PIN input widget with masked dot display.
//!
//! Provides a multi-field PIN form where each field shows entered characters
//! as masked dots (●). Supports Tab navigation between fields, Enter to submit,
//! and Esc to cancel.

#![allow(dead_code)]

use std::cell::RefCell;

use textual_rs::{Widget, Input, Label, Footer};
use textual_rs::widget::context::AppContext;
use textual_rs::event::keybinding::KeyBinding;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// A single PIN input field with a label and masked value buffer.
#[allow(dead_code)]
pub struct PinInputField {
    pub label: String,
    pub value: String,
    pub active: bool,
    /// If true, field may be left empty and still allow Submit.
    pub optional: bool,
}

/// Multi-field PIN form state.
///
/// Used for PIN change operations that need current PIN → new PIN → confirm new PIN
/// all on a single screen, as well as any other multi-field masked input.
#[allow(dead_code)]
pub struct PinInputState {
    pub fields: Vec<PinInputField>,
    pub active_field: usize,
    pub error_message: Option<String>,
    pub title: String,
}

/// Result of processing a key event in the PIN input widget.
#[allow(dead_code)]
pub enum PinInputAction {
    /// Key consumed, no state transition needed.
    Continue,
    /// User pressed Enter on last field with all fields filled — values ready.
    Submit,
    /// User pressed Esc — cancel the operation.
    Cancel,
}

#[allow(dead_code)]
impl PinInputState {
    /// Create a new PIN input form with `title` and one field per label.
    /// The first field starts as active.
    pub fn new(title: &str, labels: &[&str]) -> Self {
        let fields = labels
            .iter()
            .enumerate()
            .map(|(i, label)| PinInputField {
                label: label.to_string(),
                value: String::new(),
                active: i == 0,
                optional: false,
            })
            .collect();
        Self {
            fields,
            active_field: 0,
            error_message: None,
            title: title.to_string(),
        }
    }

    /// Process a single key event and return the resulting action.
    pub fn handle_key(&mut self, key: KeyCode) -> PinInputAction {
        let last = self.fields.len().saturating_sub(1);
        match key {
            KeyCode::Char(c) if c.is_ascii_graphic() => {
                if let Some(field) = self.fields.get_mut(self.active_field) {
                    field.value.push(c);
                }
                self.error_message = None;
                PinInputAction::Continue
            }
            KeyCode::Backspace => {
                if let Some(field) = self.fields.get_mut(self.active_field) {
                    field.value.pop();
                }
                self.error_message = None;
                PinInputAction::Continue
            }
            KeyCode::Tab => {
                self.advance_field();
                PinInputAction::Continue
            }
            KeyCode::BackTab => {
                self.retreat_field();
                PinInputAction::Continue
            }
            KeyCode::Enter => {
                if self.active_field < last {
                    // Not on last field — advance
                    self.advance_field();
                    PinInputAction::Continue
                } else if self.all_filled() {
                    PinInputAction::Submit
                } else {
                    self.error_message = Some("All fields required".to_string());
                    PinInputAction::Continue
                }
            }
            KeyCode::Esc => PinInputAction::Cancel,
            _ => PinInputAction::Continue,
        }
    }

    /// Return field values in order.
    pub fn values(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.value.as_str()).collect()
    }

    /// Returns true if every required field has at least one character.
    pub fn all_filled(&self) -> bool {
        self.fields.iter().all(|f| f.optional || !f.value.is_empty())
    }

    /// Mark the field at `idx` as optional (may be submitted empty).
    pub fn set_optional(&mut self, idx: usize) {
        if let Some(f) = self.fields.get_mut(idx) {
            f.optional = true;
        }
    }

    fn advance_field(&mut self) {
        let next = (self.active_field + 1).min(self.fields.len().saturating_sub(1));
        self.set_active(next);
    }

    fn retreat_field(&mut self) {
        let prev = self.active_field.saturating_sub(1);
        self.set_active(prev);
    }

    fn set_active(&mut self, idx: usize) {
        for (i, field) in self.fields.iter_mut().enumerate() {
            field.active = i == idx;
        }
        self.active_field = idx;
    }
}

/// textual-rs Widget wrapping a multi-field masked PIN input form.
///
/// Used by sub-screens in PinScreen (ChangeUserPin, ChangeAdminPin, etc.).
/// The widget renders a vertical list of labelled password Input fields plus
/// a Footer showing Tab/Enter/Esc bindings.
pub struct PinInputWidget {
    /// Immutable form spec: field labels and form title.
    field_labels: Vec<String>,
    title: String,
    /// Error message to show below the form (e.g. "All fields required").
    error_message: RefCell<Option<String>>,
    /// True after on_action("cancel") — parent checks this to dismiss.
    pub cancelled: RefCell<bool>,
}

impl PinInputWidget {
    pub fn new(title: &str, field_labels: &[&str]) -> Self {
        Self {
            field_labels: field_labels.iter().map(|s| s.to_string()).collect(),
            title: title.to_string(),
            error_message: RefCell::new(None),
            cancelled: RefCell::new(false),
        }
    }
}

impl Widget for PinInputWidget {
    fn widget_type_name(&self) -> &'static str {
        "PinInputWidget"
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        let mut children: Vec<Box<dyn Widget>> = Vec::new();

        children.push(Box::new(
            textual_rs::Header::new(&self.title)
        ));

        // Push Label and Input as separate top-level children rather than wrapping
        // in a Vertical container. Vertical's default height is `1fr`, which resolves
        // to 0 when PinInputWidget has no explicit height in the screen-stack context,
        // causing the children to be invisible. Input has `height: 3` (absolute), so
        // it renders correctly when laid out directly by the root vertical layout.
        for label in &self.field_labels {
            children.push(Box::new(Label::new(label.as_str())));
            children.push(Box::new(Input::new("").with_password()));
        }

        // Error message line — shown when validation fails.
        if let Some(err) = self.error_message.borrow().as_ref() {
            children.push(Box::new(Label::new(format!("Error: {}", err).as_str())));
        }

        children.push(Box::new(Footer));
        children
    }

    fn key_bindings(&self) -> &[KeyBinding] {
        &[
            KeyBinding {
                key: KeyCode::Tab,
                modifiers: KeyModifiers::NONE,
                action: "next_field",
                description: "Next field",
                show: true,
            },
            KeyBinding {
                key: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                action: "submit",
                description: "Submit",
                show: true,
            },
            KeyBinding {
                key: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                action: "cancel",
                description: "Cancel",
                show: true,
            },
        ]
    }

    fn on_action(&self, action: &str, ctx: &AppContext) {
        if action == "cancel" {
            *self.cancelled.borrow_mut() = true;
            ctx.pop_screen_deferred();
        }
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {
        // Layout and content are handled by compose() children.
    }
}

// ---------------------------------------------------------------------------
// Legacy ratatui free function — used by unmigrated keys.rs screen.
// Removed in plan 08-05 when keys.rs is migrated to textual-rs.
// ---------------------------------------------------------------------------

/// Compute a centered rect from `area` using percentage width and fixed height.
fn centered_area_legacy(area: ratatui::layout::Rect, width_pct: u16, height: u16) -> ratatui::layout::Rect {
    let v_margin = (area.height.saturating_sub(height)) / 2;
    let vertical = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(v_margin),
            ratatui::layout::Constraint::Length(height),
            ratatui::layout::Constraint::Min(0),
        ])
        .split(area);

    let horizontal = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - width_pct) / 2),
            ratatui::layout::Constraint::Percentage(width_pct),
            ratatui::layout::Constraint::Percentage((100 - width_pct) / 2),
        ])
        .split(vertical[1]);

    horizontal[1]
}

/// Render the PIN input form as a centered popup over `area` (legacy ratatui function).
/// Used by keys.rs until it is migrated in plan 08-05.
pub fn render_pin_input(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &PinInputState) {
    use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

    let field_count = state.fields.len() as u16;
    let height = 2 + field_count * 2 + 2 + if state.error_message.is_some() { 1 } else { 0 };
    let popup_area = centered_area_legacy(area, 50, height);
    frame.render_widget(Clear, popup_area);

    let mut lines: Vec<ratatui::text::Line> = Vec::new();
    for field in &state.fields {
        lines.push(ratatui::text::Line::from(ratatui::text::Span::raw(field.label.clone())));
        let dots: String = "\u{25cf}".repeat(field.value.len());
        let value_display = if field.active {
            format!("{}\u{2588}", dots)
        } else {
            dots
        };
        let style = if field.active {
            ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)
        } else {
            ratatui::style::Style::default()
        };
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(value_display, style)));
    }
    lines.push(ratatui::text::Line::from(""));

    let submit_hint = if state.all_filled() {
        "[Enter] Submit"
    } else {
        "[Enter] Next field"
    };
    lines.push(ratatui::text::Line::from(format!(
        "[Tab] Next field  {}  [Esc] Cancel",
        submit_hint
    )));

    if let Some(err) = &state.error_message {
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            err.clone(),
            ratatui::style::Style::default().fg(ratatui::style::Color::Red),
        )));
    }

    let text = ratatui::text::Text::from(lines);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(state.title.clone());
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup_area);
}
