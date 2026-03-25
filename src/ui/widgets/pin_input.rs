//! Reusable TUI PIN input widget with masked dot display.
//!
//! Provides a multi-field PIN form where each field shows entered characters
//! as masked dots (●). Supports Tab navigation between fields, Enter to submit,
//! and Esc to cancel.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crossterm::event::KeyCode;

/// A single PIN input field with a label and masked value buffer.
#[allow(dead_code)]
pub struct PinInputField {
    pub label: String,
    pub value: String,
    pub active: bool,
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

    /// Returns true if every field has at least one character.
    pub fn all_filled(&self) -> bool {
        self.fields.iter().all(|f| !f.value.is_empty())
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

/// Compute a centered rect from `area` using percentage width and fixed height.
fn centered_area(area: Rect, width_pct: u16, height: u16) -> Rect {
    let v_margin = (area.height.saturating_sub(height)) / 2;
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(v_margin),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_pct) / 2),
            Constraint::Percentage(width_pct),
            Constraint::Percentage((100 - width_pct) / 2),
        ])
        .split(vertical[1]);

    horizontal[1]
}

/// Render the PIN input form as a centered popup over `area`.
///
/// - Each field shows a label line and a masked value line (dots or dots + cursor).
/// - The active field is highlighted in yellow.
/// - A hint line shows available key actions.
/// - If `state.error_message` is set, it is shown in red below the hint.
#[allow(dead_code)]
pub fn render_pin_input(frame: &mut Frame, area: Rect, state: &PinInputState) {
    let field_count = state.fields.len() as u16;
    // Height: border top (1) + per-field label+value (2 each) + blank after fields (1) + hint (1) + error maybe (1) + border bottom (1)
    let height = 2 + field_count * 2 + 2 + if state.error_message.is_some() { 1 } else { 0 };
    let popup_area = centered_area(area, 50, height);
    frame.render_widget(Clear, popup_area);

    // Build body text
    let mut lines: Vec<Line> = Vec::new();
    for field in &state.fields {
        lines.push(Line::from(Span::raw(field.label.clone())));
        let dots: String = "●".repeat(field.value.len());
        let value_display = if field.active {
            format!("{}\u{2588}", dots) // cursor block
        } else {
            dots
        };
        let style = if field.active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(value_display, style)));
    }
    lines.push(Line::from("")); // blank separator

    let submit_hint = if state.all_filled() {
        "[Enter] Submit"
    } else {
        "[Enter] Next field"
    };
    lines.push(Line::from(format!(
        "[Tab] Next field  {}  [Esc] Cancel",
        submit_hint
    )));

    if let Some(err) = &state.error_message {
        lines.push(Line::from(Span::styled(
            err.clone(),
            Style::default().fg(Color::Red),
        )));
    }

    let text = Text::from(lines);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(state.title.clone());
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup_area);
}
