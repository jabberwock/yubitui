//! Spinner + status line progress popup.
//!
//! Shows an animated spinner and a current status message while a gpg/ykman
//! operation is running in the background. Caller increments `tick` on each
//! TUI render cycle to advance the spinner.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Spinner character set indexed by `tick % 4`.
const SPINNER: [char; 4] = ['|', '/', '-', '\\'];

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

/// Render a progress popup with an animated spinner and status message.
///
/// - `title` is shown in the popup border title.
/// - `status` is the current operation description (e.g. "Verifying current PIN...").
/// - `tick` advances the spinner; increment it on each render frame.
#[allow(dead_code)]
pub fn render_progress_popup(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    status: &str,
    tick: usize,
) {
    let popup_area = centered_area(area, 50, 6);
    frame.render_widget(Clear, popup_area);
    let spinner_char = SPINNER[tick % SPINNER.len()];
    let body = format!("{} {}", spinner_char, status);
    let paragraph = Paragraph::new(body)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, popup_area);
}
