use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::model::YubiKeyState;

#[derive(Default)]
pub struct PivTuiState {
    pub scroll_offset: usize,
}

#[derive(Clone, Debug)]
pub enum PivAction {
    None,
    NavigateTo(crate::model::Screen),
}

pub fn handle_key(key: KeyEvent) -> PivAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => PivAction::NavigateTo(crate::model::Screen::Dashboard),
        _ => PivAction::None,
    }
}

/// Render the PIV certificates screen.
///
/// Shows each standard PIV slot and whether it is occupied or empty,
/// based on YubiKeyState.piv populated by detect_all().
pub fn render(frame: &mut Frame, area: Rect, yubikey_state: &Option<YubiKeyState>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let title_widget = Paragraph::new("PIV Certificates")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title_widget, chunks[0]);

    let mut lines: Vec<Line> = Vec::new();

    let slot_labels: &[(&str, &str)] = &[
        ("9a", "Authentication (9a)"),
        ("9c", "Digital Signature (9c)"),
        ("9d", "Key Management (9d)"),
        ("9e", "Card Authentication (9e)"),
    ];

    match yubikey_state {
        Some(yk) => {
            match &yk.piv {
                Some(piv_state) => {
                    lines.push(Line::from(vec![Span::styled(
                        "PIV Slot Status",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )]));
                    lines.push(Line::from(""));

                    for (slot_id, label) in slot_labels {
                        let occupied = piv_state.slots.iter().any(|s| s.slot == *slot_id);
                        if occupied {
                            lines.push(Line::from(vec![
                                Span::styled("  [OK] ", Style::default().fg(Color::Green)),
                                Span::styled(*label, Style::default().fg(Color::Green)),
                                Span::raw(" -- Occupied"),
                            ]));
                        } else {
                            lines.push(Line::from(vec![
                                Span::styled("  [  ] ", Style::default().fg(Color::DarkGray)),
                                Span::styled(*label, Style::default().fg(Color::DarkGray)),
                                Span::raw(" -- Empty"),
                            ]));
                        }
                    }
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![Span::styled(
                        "Press [R] to refresh or [Esc] to return.",
                        Style::default().fg(Color::DarkGray),
                    )]));
                }
                None => {
                    lines.push(Line::from(vec![Span::styled(
                        "PIV data unavailable for this YubiKey.",
                        Style::default().fg(Color::Yellow),
                    )]));
                }
            }
        }
        None => {
            lines.push(Line::from(vec![Span::styled(
                "No YubiKey detected.",
                Style::default().fg(Color::Red),
            )]));
        }
    }

    let content = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(content, chunks[1]);

}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};
    use crate::model::mock::mock_yubikey_states;

    #[test]
    fn piv_default_state() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let yk = mock_yubikey_states().into_iter().next();
        terminal.draw(|frame| {
            render(frame, frame.area(), &yk);
        }).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn piv_no_yubikey() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| {
            render(frame, frame.area(), &None);
        }).unwrap();
        assert_snapshot!(terminal.backend());
    }
}
