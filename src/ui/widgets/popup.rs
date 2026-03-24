use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

/// Compute a centered rectangle within `area` using percentage width and fixed height.
fn centered_rect(area: Rect, width_pct: u16, height: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_pct) / 2),
            Constraint::Percentage(width_pct),
            Constraint::Percentage((100 - width_pct) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Render a generic popup with a title and body text, centered on screen.
/// Clears the background area first to avoid visual artifacts.
#[allow(dead_code)]
pub fn render_popup(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    body: &str,
    width_pct: u16,
    height: u16,
) {
    let popup_area = centered_rect(area, width_pct, height);
    frame.render_widget(Clear, popup_area);
    let paragraph = Paragraph::new(body)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, popup_area);
}

/// Render a confirmation dialog with [Y]es / [N]o prompt.
/// When `destructive` is true, the title is styled in red bold.
#[allow(dead_code)]
pub fn render_confirm_dialog(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    message: &str,
    destructive: bool,
) {
    let popup_area = centered_rect(area, 60, 8);
    frame.render_widget(Clear, popup_area);

    let (display_title, body_text) = if destructive {
        (
            format!("WARNING: {}", title),
            format!("{}\n\n[Y] Yes  [N] No", message),
        )
    } else {
        (title.to_string(), format!("{}\n\n[Y] Yes  [N] No", message))
    };

    let title_style = if destructive {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let paragraph = Paragraph::new(body_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(display_title)
                .title_style(title_style),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, popup_area);
}

/// Render a context menu (floating list) centered on screen.
/// Items are &[&str]; `selected_index` highlights the current item in yellow bold.
pub fn render_context_menu(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: &[&str],
    selected_index: usize,
) {
    let height = (items.len() as u16) + 2; // +2 for borders
    let popup_area = centered_rect(area, 40, height);
    frame.render_widget(Clear, popup_area);

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            if i == selected_index {
                ListItem::new(format!("> {}", item)).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(format!("  {}", item))
            }
        })
        .collect();

    let list = List::new(list_items).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(list, popup_area);
}
