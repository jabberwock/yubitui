pub mod pin_input;
pub mod popup;
pub mod progress;

/// Fill the entire area with the theme's background color, overwriting any
/// previously rendered content (e.g. Dashboard bleed-through in textual-rs 0.3.8+).
/// Call this at the start of every pushed screen's `render()` method.
pub fn fill_screen_background(
    ctx: &textual_rs::widget::context::AppContext,
    area: ratatui::layout::Rect,
    buf: &mut ratatui::buffer::Buffer,
) {
    let (r, g, b) = ctx.theme.background;
    let bg = ratatui::style::Color::Rgb(r, g, b);
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(" ");
                cell.set_bg(bg);
            }
        }
    }
}
