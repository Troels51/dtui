use itertools::repeat_n;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{self, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use tui_textarea::TextArea;
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
    let area = frame.size();
    let full = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Max(2)])
        .split(area);
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

    let objects_view = Tree::new(&app.objects.items)
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

    // Render a potential pop up
    if let WorkingArea::PopUp(ref m) = app.working_area {
        let selected = app.objects.state.selected();

        match selected.last() {
            Some(method) => match method {
                crate::stateful_tree::DbusIdentifier::Object(_) => (),
                crate::stateful_tree::DbusIdentifier::Interface(_) => (),
                crate::stateful_tree::DbusIdentifier::Member(_) => (),
                crate::stateful_tree::DbusIdentifier::Method(m) => {
                    let block = Block::bordered()
                        .title(m.0.name().to_string())
                        .border_style(Style::default().fg(Color::Blue));
                    let area = centered_rect(80, 50, area);
                    let args = m.0.args();
                    let single_line_layout = Layout::vertical(
                        repeat_n(Constraint::Length(3), args.len()).chain([Constraint::Min(1)]),
                    );

                    let segments = single_line_layout.split(block.inner(area));
                    frame.render_widget(Clear, area); //this clears out the background
                    frame.render_widget(block, area);

                    for (i, arg) in m.0.args().iter().take(segments.len()).enumerate() {
                        let mut text_area = TextArea::default();
                        text_area.set_cursor_line_style(Style::default());
                        text_area.set_cursor_style(Style::default());
                        text_area.insert_str(arg.ty().to_string());
                        text_area.set_block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(arg.name().unwrap().to_string()),
                        );
                        frame.render_widget(text_area.widget(), segments[i]);
                    }
                }
                crate::stateful_tree::DbusIdentifier::Property(_) => (),
                crate::stateful_tree::DbusIdentifier::Signal(_) => (),
            },
            None => (),
        }
    }

    let bottom_text =
        Span::raw("Change focus: Tab | Navigation: ← ↓ ↑ → | Get Service: Enter | Quit: q");
    let helper_paragraph = Paragraph::new(bottom_text).alignment(Alignment::Center);
    frame.render_widget(helper_paragraph, full[1]);
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
