use ratatui::style::Color;

// TODO: Rename this module to something else
pub fn active_area_border_color(is_active: bool) -> Color {
    if is_active {
        Color::LightBlue
    } else {
        Color::White
    }
}