use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};
use tui_tree_widget::Tree;

use crate::app::{App, WorkingArea};

fn working_area_border(app: &App, working_area: WorkingArea) -> Color {
    if app.working_area == working_area {
        Color::LightBlue
    } else {
        Color::White
    }
}

pub fn ui<B: Backend>(frame: &mut Frame, app: &mut App) {
    // Create two chunks with equal horizontal screen space
    let full = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Max(2)])
        .split(frame.size());
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)].as_ref())
        .split(full[0]);
    let items: Vec<ListItem> = app
        .services
        .items
        .iter()
        .map(|i| {
            let lines = Span::from(i.as_str());
            ListItem::new(lines).style(Style::default())
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Services")
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(working_area_border(app, WorkingArea::Services))),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    // We can now render the item list
    frame.render_stateful_widget(items, chunks[0], &mut app.services.state);

    let objects_view = Tree::new(app.objects.items.clone())
        .unwrap()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(working_area_border(app, WorkingArea::Objects)))
                .title("Objects"),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    frame.render_stateful_widget(objects_view, chunks[1], &mut app.objects.state);
    let bottom_text =
        Span::raw("Change focus: Tab | Navigation: ← ↓ ↑ → | Get Service: Enter | Quit: q");
    let helper_paragraph = Paragraph::new(bottom_text).alignment(Alignment::Center);
    frame.render_widget(helper_paragraph, full[1]);
}
