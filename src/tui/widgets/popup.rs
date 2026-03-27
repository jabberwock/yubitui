#![allow(dead_code)]

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

/// Compute a centered rect from the given area using percentage width and fixed height.
#[allow(dead_code)]
fn centered_area(area: Rect, width_pct: u16, height: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height.min(100)) / 2),
            Constraint::Length(height),
            Constraint::Percentage((100 - height.min(100)) / 2),
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

/// Render a generic popup with a title and body text, centered on screen.
/// Clears background with `Clear` widget first.
pub fn render_popup(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    body: &str,
    width_pct: u16,
    height: u16,
) {
    let popup_area = centered_area(area, width_pct, height);
    frame.render_widget(Clear, popup_area);
    let paragraph = Paragraph::new(body)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, popup_area);
}

/// Render a confirmation dialog with [Y]es / [N]o prompt.
/// Red BOLD title with "WARNING" prefix for destructive actions.
/// `destructive` param controls whether WARNING styling is applied.
pub fn render_confirm_dialog(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    message: &str,
    destructive: bool,
) {
    let popup_area = centered_area(area, 60, 8);
    frame.render_widget(Clear, popup_area);

    let (block_title, body_text) = if destructive {
        let styled_title = format!("WARNING: {}", title);
        let body = format!("{}\n\n[Y] Yes  [N] No", message);
        (styled_title, body)
    } else {
        let body = format!("{}\n\n[Y] Yes  [N] No", message);
        (title.to_string(), body)
    };

    let title_style = if destructive {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let paragraph = Paragraph::new(body_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(block_title)
                .title_style(title_style),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, popup_area);
}

/// Render a context menu (floating list) at a given position.
/// Items are `&[&str]`, selected_index highlights current item.
/// Yellow bold for selected item, white for others.
/// Returns the popup Rect so callers can register click regions.
pub fn render_context_menu(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: &[&str],
    selected_index: usize,
) -> Rect {
    let height = (items.len() as u16).saturating_add(2);
    let popup_area = centered_area(area, 40, height);
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
    popup_area
}
