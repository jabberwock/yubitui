use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub enum HelpAction {
    None,
    Close,
}

pub fn handle_key(key: KeyEvent) -> HelpAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') => HelpAction::Close,
        _ => HelpAction::None,
    }
}

pub fn render(frame: &mut Frame, area: Rect) {
    let lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            " Global",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "1-5"), Style::default().fg(Color::Yellow)),
            Span::styled(
                "Switch screen (Dashboard / Diagnostics / Keys / PIN / SSH)",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "r"), Style::default().fg(Color::Yellow)),
            Span::styled(
                "Refresh YubiKey status and diagnostics",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "?"), Style::default().fg(Color::Yellow)),
            Span::styled("Toggle this help screen", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<12}", "q / Esc"),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "Quit (from Dashboard) or go back",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<12}", "m / Enter"),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "Open navigation menu (Dashboard)",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " Key Management (Screen 3)",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "v"), Style::default().fg(Color::Yellow)),
            Span::styled("View full card status", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "i"), Style::default().fg(Color::Yellow)),
            Span::styled(
                "Import existing key to card",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "g"), Style::default().fg(Color::Yellow)),
            Span::styled(
                "Generate new key on card",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "e"), Style::default().fg(Color::Yellow)),
            Span::styled("Export SSH public key", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<12}", "Up/Down"),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "Select key (in import view)",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<12}", "Enter"),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "Execute selected operation",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " PIN Management (Screen 4)",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "c"), Style::default().fg(Color::Yellow)),
            Span::styled("Change user PIN", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "a"), Style::default().fg(Color::Yellow)),
            Span::styled("Change admin PIN", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "r"), Style::default().fg(Color::Yellow)),
            Span::styled("Set reset code", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "u"), Style::default().fg(Color::Yellow)),
            Span::styled("Unblock user PIN", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<12}", "Enter"),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "Execute selected operation",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " SSH Wizard (Screen 5)",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "1-5"), Style::default().fg(Color::Yellow)),
            Span::styled("Select wizard step", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<12}", "r"), Style::default().fg(Color::Yellow)),
            Span::styled("Refresh SSH status", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:<12}", "Enter"),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("Execute selected step", Style::default().fg(Color::White)),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help - Keybindings ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}
