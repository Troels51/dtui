use itertools::repeat_n;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{self, Span},
    widgets::{
        block::{Position, Title},
        Block, BorderType, Borders, Clear, List, ListItem, Paragraph,
    },
    Frame,
};
use tui_textarea::TextArea;
use tui_tree_widget::Tree;
use zbus::zvariant::{DynamicDeserialize, DynamicType, Value};

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
    if let WorkingArea::PopUp(ref mut popup) = app.working_area {
        // TODO: Big ass block, lets refactor to smaller functions
        let method = &popup.method_description.0;

        let bottom_text = Span::raw("Navigation: ↓ ↑ | Call: Enter | Quit: esq");
        let block = Block::bordered()
            .title(Title::from(method.name().to_string()).position(Position::Top))
            .title(
                Title::from(bottom_text)
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .border_style(Style::default().fg(Color::Blue));
        let area = centered_rect(80, 50, area);
        let args = popup.method_description.0.args();
        let single_line_layout = Layout::vertical(
            repeat_n(Constraint::Length(3), args.len()).chain([Constraint::Min(1)]),
        );

        let segments = single_line_layout.split(block.inner(area));
        frame.render_widget(Clear, area); //this clears out the background
        frame.render_widget(block, area);
        if popup.inputs.is_empty() {
            // First time init of text areas
            for arg in method.args().iter().take(segments.len()) {
                let mut text_area = TextArea::default();
                text_area.set_cursor_line_style(Style::default());
                text_area.set_cursor_style(Style::default());
                text_area.set_block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("name: {}", arg.name().unwrap().to_string()))
                        .title_bottom(format!("type: {}", arg.ty().to_string())),
                );
                popup.inputs.push(text_area);
            }
        }
        for (i, input) in popup.inputs.iter_mut().enumerate() {
            let emphasis = if i == popup.selected {
                let ty = popup.method_description.0.args()[i].ty();
                // NEXT: From a type/signature create a string parser that outputs a body?
                // How does that body look like? It doesnt look like zbus has a serializer for a completely dynamic thing
                // Maybe https://docs.rs/zvariant/latest/zvariant/enum.Value.html
                // Let's figure it out next time, in this installment of Troels does programming in the evening while Eva sleeps
                let deser = Value::deserializer_for_signature(ty.signature()).unwrap();
                if true {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default().fg(Color::Red)
                }
            } else {
                Style::default()
            };
            input.set_block(input.block().unwrap().clone().border_style(emphasis));
            input.set_cursor_line_style(emphasis);
            frame.render_widget(input.widget(), segments[i]);
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
