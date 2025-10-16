use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::{Action, DbusInvocationAction, EditorMode},
    app::Focus,
    config::Config,
};

#[derive(Default)]
pub struct HelpView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    help_text: Vec<Line<'static>>,
    vertical_scroll: usize,
}

impl HelpView {
    pub fn new() -> Self {
        let mut new = Self::default();
        new.help_text = build_help_text_lines();
        new
    }

    fn scroll_up(&mut self) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        self.vertical_scroll = self.vertical_scroll.saturating_add(1);
    }
}

impl Component for HelpView {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    async fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Up => self.scroll_up(),
            Action::Down => self.scroll_down(),
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        frame.render_widget(Clear, area);
        let helper_paragraph = Paragraph::new(self.help_text.clone())
            .alignment(Alignment::Left)
            .scroll((self.vertical_scroll as u16, 0));
        let popup_block = Block::bordered()
            .title("Help")
            .border_type(BorderType::Rounded);
        helper_paragraph.render(popup_block.inner(area), frame.buffer_mut());

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .thumb_style(Color::LightBlue);
        let mut scrollbar_state =
            ScrollbarState::new(self.help_text.len()).position(self.vertical_scroll);

        frame.render_widget(popup_block, area);
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
        Ok(())
    }
}

/// Translates the markdown content into a vector of styled `ratatui::text::Line`.
fn build_help_text_lines() -> Vec<Line<'static>> {
    // Define styles for different elements to maintain consistency.
    let title_style = Style::new().bold().underlined();
    let heading_style = Style::new().bold();
    let key_style = Style::new().fg(Color::Cyan).bold();
    let code_style = Style::new().bg(Color::Rgb(60, 60, 60));
    let sig_style = Style::default();
    let value_style = Style::default().fg(Color::Green);

    vec![
        Line::from(Span::styled("Basic Usage", title_style)),
        Line::from(""),
        Line::from(Span::styled("Views & Navigation", heading_style)),
        Line::from("There are 4 views: Services, Objects, Results, and Call view."),
        Line::from("- The Service view will show the services available on your bus."),
        Line::from("- The Object view will show objects on the selected service."),
        Line::from("- The Result view shows results from D-Bus, like method calls or properties."),
        Line::from("- The Call view shows the active Method call or Property Set."),
        Line::from(""),
        // --- General Navigation Table ---
        Line::from(Span::styled("General Navigation", heading_style)),
        Line::from(
            "┌──────────────────┬──────────────────┬────────────────────────────────────────┐",
        ),
        Line::from(
            "│ Key              │ Action           │ Description                            │",
        ),
        Line::from(
            "├──────────────────┼──────────────────┼────────────────────────────────────────┤",
        ),
        Line::from(vec![
            "│ ".into(),
            Span::styled("Arrows/h j k l", key_style),
            "   │ ".into(),
            "Move Cursor".into(),
            "      │ ".into(),
            "Navigate menus, lists, and tables.".into(),
            "     │".into(),
        ]),
        Line::from(vec![
            "│ ".into(),
            Span::styled("Tab", key_style),
            "              │ ".into(),
            "Next view".into(),
            "        │ ".into(),
            "Switch focus to the next view.".into(),
            "         │".into(),
        ]),
        Line::from(vec![
            "│ ".into(),
            Span::styled("?", key_style),
            "                │ ".into(),
            "Help".into(),
            "             │ ".into(),
            "Show this help screen.".into(),
            "                 │".into(),
        ]),
        Line::from(vec![
            "│ ".into(),
            Span::styled("q", key_style),
            "                │ ".into(),
            "Quit".into(),
            "             │ ".into(),
            "Quit the application.".into(),
            "                  │".into(),
        ]),
        Line::from(
            "└──────────────────┴──────────────────┴────────────────────────────────────────┘",
        ),
        Line::from(""),
        // --- Services ---
        Line::from(Span::styled("Services View Navigation", heading_style)),
        Line::from(
            "┌──────────────────┬──────────────────┬────────────────────────────────────────┐",
        ),
        Line::from(
            "│ Key              │ Action           │ Description                            │",
        ),
        Line::from(
            "├──────────────────┼──────────────────┼────────────────────────────────────────┤",
        ),
        Line::from(vec![
            "│ ".into(),
            Span::styled("Enter", key_style),
            "            │ ".into(),
            "Select".into(),
            "           │ ".into(),
            "Select a service and get its objects.".into(),
            "  │".into(),
        ]),
        Line::from(
            "└──────────────────┴──────────────────┴────────────────────────────────────────┘",
        ),
        Line::from(""),
        // --- Services ---
        Line::from(Span::styled("Objects View Navigation", heading_style)),
        Line::from(
            "┌──────────────────┬──────────────────┬────────────────────────────────────────┐",
        ),
        Line::from(
            "│ Key              │ Action           │ Description                            │",
        ),
        Line::from(
            "├──────────────────┼──────────────────┼────────────────────────────────────────┤",
        ),
        Line::from(vec![
            "│ ".into(),
            Span::styled("Right/Left or l/h", key_style),
            "│ ".into(),
            "Expand/Collapse".into(),
            "  │ ".into(),
            "Expand/Collapse a node.".into(),
            "                │".into(),
        ]),
        Line::from(vec![
            "│ ".into(),
            Span::styled("Enter", key_style),
            "            │ ".into(),
            "Call".into(),
            "             │ ".into(),
            "Call a method or get a property.".into(),
            "       │".into(),
        ]),
        Line::from(vec![
            "│ ".into(),
            Span::styled("s", key_style),
            "                │ ".into(),
            "Set Property".into(),
            "     │ ".into(),
            "Set a selected property.".into(),
            "               │".into(),
        ]),
        Line::from(vec![
            "│ ".into(),
            Span::styled("g", key_style),
            "                │ ".into(),
            "Get Property".into(),
            "     │ ".into(),
            "Get a selected property.".into(),
            "               │".into(),
        ]),
        Line::from(
            "└──────────────────┴──────────────────┴────────────────────────────────────────┘",
        ),
        Line::from(""),
        // --- Call view ---
        Line::from(Span::styled("Call View Navigation", heading_style)),
        Line::from(
            "┌──────────────────┬──────────────────┬────────────────────────────────────────┐",
        ),
        Line::from(
            "│ Key              │ Action           │ Description                            │",
        ),
        Line::from(
            "├──────────────────┼──────────────────┼────────────────────────────────────────┤",
        ),
        Line::from(vec![
            "│ ".into(),
            Span::styled("i", key_style),
            "                │ ".into(),
            "Insert mode".into(),
            "      │ ".into(),
            "Insert mode for text fields".into(),
            "            │".into(),
        ]),
        Line::from(vec![
            "│ ".into(),
            Span::styled("Esc", key_style),
            "              │ ".into(),
            "Normal Mode".into(),
            "      │ ".into(),
            "Normal mode to navigate or call".into(),
            "        │".into(),
        ]),
        Line::from(vec![
            "│ ".into(),
            Span::styled("Enter", key_style),
            "            │ ".into(),
            "Call".into(),
            "             │ ".into(),
            "Call the method or set property.".into(),
            "       │".into(),
        ]),
        Line::from(
            "└──────────────────┴──────────────────┴────────────────────────────────────────┘",
        ),
        Line::from(""),
        Line::from("──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────".dim()),
        // --- Parsing Section ---
        Line::from(Span::styled("Parsing", title_style)),
        Line::from("Arguments in the Call view are parsed based on their D-Bus type signature."),
        Line::from("A text field turns green when the input is valid."),
        Line::from(""),
        Line::from(Span::styled("D-Bus Type Signatures", heading_style)),
        Line::from(
            "┌─────────────────┬──────────────────────────────────┬─────────────────────────────┐",
        ),
        Line::from(
            "│ D-Bus Type      │ Input Format                     │ Example                     │",
        ),
        Line::from(
            "├─────────────────┼──────────────────────────────────┼─────────────────────────────┤",
        ),
        Line::from(vec![
            "│ Basic Types     │ Value (e.g., ".into(),
            Span::styled("5", code_style),
            ", ".into(),
            Span::styled("true", code_style),
            ")            │ ".into(),
            Span::styled("5", code_style),
            " for ".into(),
            Span::styled("i", code_style),
            ", ".into(),
            Span::styled("true", code_style),
            " for ".into(),
            Span::styled("b", code_style),
            Span::raw("         │"),
        ]),
        Line::from(vec![
            "│ Array ".into(),
            Span::styled("a...", code_style),
            "      │ Elements in ".into(),
            Span::styled("[]", code_style),
            ", separated by ".into(),
            Span::styled(",", code_style),
            "   │ ".into(),
            Span::styled("[\"a\", \"b\"]", code_style),
            Span::raw("                  │"),
        ]),
        Line::from(vec![
            "│ Struct ".into(),
            Span::styled("(...)", code_style),
            "    │ Elements in ".into(),
            Span::styled("()", code_style),
            ", separated by ".into(),
            Span::styled(",", code_style),
            "   │ ".into(),
            Span::styled("(\"x\", 1)", code_style),
            " for ".into(),
            Span::styled("(su)", code_style),
            Span::raw("           │"),
        ]),
        Line::from(vec![
            "│ Dictionary ".into(),
            Span::styled("a{}", code_style),
            "  │ Key-value pairs in ".into(),
            Span::styled("{}", code_style),
            "            │ ".into(),
            Span::styled("{\"k\": \"v\"}", code_style),
            Span::raw("                  │"),
        ]),
        Line::from(vec![
            "│ Variant ".into(),
            Span::styled("v", code_style),
            "       │ ".into(),
            Span::styled("sig->val", code_style),
            "                         │ ".into(),
            Span::styled("\"u\"->5", code_style),
            Span::raw(" (u32 with value 5)   │"),
        ]),
        Line::from(
            "└─────────────────┴──────────────────────────────────┴─────────────────────────────┘",
        ),
        Line::from(""),
        // --- Basic types ---
                Line::from(Span::styled("D-Bus Type Signatures", heading_style)),
        Line::from(
            "┌─────────────────┬──────────────────────────────────┬─────────────────────────────┐",
        ),
        Line::from(
            "│ D-Bus Type      │ Input Format                     │ Example                     │",
        ),
        Line::from(
            "├─────────────────┼──────────────────────────────────┼─────────────────────────────┤",
        ),
    Line::from(vec![
        Span::raw("│ "),
        Span::styled("`y` byte/u8", sig_style),
        Span::raw("     │ "),
        Span::styled("`255`", value_style),
        Span::raw("                            │ "),
        Span::styled("255", value_style),
        Span::raw("                         │"),
    ]),

    Line::from(vec![
        Span::raw("│ "),
        Span::styled("`i` integer/i32", sig_style),
        Span::raw(" │ "),
        Span::styled("`-42`", value_style),
        Span::raw("                            │ "),
        Span::styled("-42", value_style),
        Span::raw("                         │"),
    ]),

    Line::from(vec![
        Span::raw("│ "),
        Span::styled("`d` double/f64", sig_style),
        Span::raw("  │ "),
        Span::styled("`3.1415`", value_style),
        Span::raw("                         │ "),
        Span::styled("3.1415", value_style),
        Span::raw("                      │"),
    ]),

    Line::from(vec![
        Span::raw("│ "),
        Span::styled("`b` boolean", sig_style),
        Span::raw("     │ "),
        Span::styled("`true` or `false`", value_style),
        Span::raw("                │ "),
        Span::styled("True or False", value_style),
        Span::raw("               │"),
    ]),

    Line::from(vec![
        Span::raw("│ "),
        Span::styled("`s` string", sig_style),
        Span::raw("      │ "),
        Span::styled("`\"Hello World\"`", value_style),
        Span::raw("                  │ "),
        Span::styled("\"Hello World\"", value_style),
        Span::raw("               │"),
    ]),

    Line::from(vec![
        Span::raw("│ "),
        Span::styled("`o` object path", sig_style),
        Span::raw(" │ "),
        Span::styled("`\"/org/example/path\"`", value_style),
        Span::raw("            │ "),
        Span::styled("ObjectPath", value_style),
        Span::raw("                  │"),
    ]),

    Line::from(vec![
        Span::raw("│ "),
        Span::styled("`g` signature", sig_style),
        Span::raw("   │ "),
        Span::styled("`\"as\"`", value_style),
        Span::raw("                           │ "),
        Span::styled("Signature value", value_style),
        Span::raw("             │"),
    ]),

        Line::from(
            "└─────────────────┴──────────────────────────────────┴─────────────────────────────┘",
        ),
        Line::from(""),
        // --- Complex Type Examples ---
        Line::from(Span::styled("Complex Type Examples", heading_style)),
        Line::from(vec![
            "• ".into(),
            Span::styled("a{si}", code_style),
            ": A dictionary mapping strings to integers. e.g., ".into(),
            Span::styled("{\"count\": 5}", code_style),
        ]),
        Line::from(vec![
            "• ".into(),
            Span::styled("(ssu)", code_style),
            ": A struct of two strings and a u32. e.g., ".into(),
            Span::styled("(\"user\", \"john\", 42)", code_style),
        ]),
        Line::from(vec![
            "• ".into(),
            Span::styled("a(is)", code_style),
            ": An array of (integer, string) structs. e.g., ".into(),
            Span::styled("[(1, \"a\"), (2, \"b\")]", code_style),
        ]),
        Line::from(""),
        Line::from(vec![
            "Note: File Descriptors (".into(),
            Span::styled("h", code_style),
            ") cannot be parsed from strings.".into(),
        ]),
    ]
}
