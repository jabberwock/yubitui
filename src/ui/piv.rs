use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::yubikey::YubiKeyState;

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
