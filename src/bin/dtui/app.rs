use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout},
    prelude::Rect,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::info;
use zbus::{Connection, conn};

use crate::{
    Args, BusType,
    action::{Action, EditorMode},
    components::{Component, Components},
    config::Config,
    dbus_handler::DbusActorHandle,
    messages::AppMessage,
    tui::{Event, Tui},
};

pub struct App {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    components: Components,
    should_quit: bool,
    should_suspend: bool,
    focus: Focus,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
    dbus_handler: DbusActorHandle,
    dbus_receiver: mpsc::UnboundedReceiver<AppMessage>,
    editor_mode: crate::action::EditorMode,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Focus {
    #[default]
    Services,
    Objects,
    Call,
    Help,
    All, // For keybindings or interactions that are always active
}

impl Focus {
    fn next(&self) -> Focus {
        match self {
            Focus::Services => Focus::Objects,
            Focus::Objects => Focus::Call,
            Focus::Call => Focus::Services,
            Focus::All => Focus::Services,
            Focus::Help => Focus::Services,
        }
    }
}

impl App {
    pub async fn new(tick_rate: f64, frame_rate: f64, args: Args) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        // Create Dbus actor
        let mut connection = match args.bus {
            BusType::System => Connection::system().await?,
            BusType::Session => Connection::session().await?,
        };

        if let Some(address) = args.address {
            connection = conn::Builder::address(address.as_str())?.build().await?;
        }
        let (dbus_handler_sender, dbus_receiver) = mpsc::unbounded_channel();
        let dbus_handler = DbusActorHandle::new(dbus_handler_sender, connection);

        Ok(Self {
            tick_rate,
            frame_rate,
            components: Components::new(),
            should_quit: false,
            should_suspend: false,
            config: Config::new()?,
            focus: Focus::Services,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
            dbus_handler,
            dbus_receiver,
            editor_mode: EditorMode::Normal,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            // .mouse(true) // uncomment this line to enable mouse support
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        self.components
            .register_action_handler(self.action_tx.clone())?;
        self.components
            .register_dbus_actor_handler(self.dbus_handler.clone())?;
        self.components
            .register_config_handler(self.config.clone())?;
        self.components.init(tui.size()?)?;
        let _ = self.action_tx.send(Action::Focus(self.focus));
        let action_tx = self.action_tx.clone();

        self.dbus_handler.request_services().await;

        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui).await?;
            self.handle_dbus_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                // tui.mouse(true);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }
        if let Some(action) = self
            .components
            .service_view
            .handle_events(Some(event.clone()))?
        {
            action_tx.send(action)?;
        }
        if let Some(action) = self
            .components
            .object_view
            .handle_events(Some(event.clone()))?
        {
            action_tx.send(action)?;
        }
        if let Some(action) = self
            .components
            .results_view
            .handle_events(Some(event.clone()))?
        {
            action_tx.send(action)?;
        }
        if let Some(action) = self
            .components
            .call_view
            .handle_events(Some(event.clone()))?
        {
            action_tx.send(action)?;
        }
        if let Some(action) = self
            .components
            .bottom_text
            .handle_events(Some(event.clone()))?
        {
            action_tx.send(action)?;
        }
        if let Some(action) = self
            .components
            .help_view
            .handle_events(Some(event.clone()))?
        {
            action_tx.send(action)?;
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        let action_tx = self.action_tx.clone();
        let Some(generic_keymap) = self.config.keybindings.get(&Focus::All) else {
            return Ok(());
        };

        let Some(focus_keymap) = self.config.keybindings.get(&self.focus) else {
            return Ok(());
        };

        if self.editor_mode == EditorMode::Insert {
            if let Some(action) = focus_keymap.get(&vec![key])
                && let Action::EditorMode(..) = action
            {
                action_tx.send(action.clone())?
            }
            return Ok(());
        }
        for keymap in [generic_keymap, focus_keymap] {
            match keymap.get(&vec![key]) {
                Some(action) => {
                    info!("Got action: {action:?}");
                    action_tx.send(action.clone())?;
                }
                _ => {
                    // If the key was not handled as a single key action,
                    // then consider it for multi-key combinations.
                    self.last_tick_key_events.push(key);

                    // Check for multi-key combinations
                    if let Some(action) = keymap.get(&self.last_tick_key_events) {
                        info!("Multi-key action: {action:?}");
                        action_tx.send(action.clone())?;
                    }
                }
            }
        }
        let _ = self.components.handle_key_event(key);

        Ok(())
    }

    async fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            match action {
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                Action::NextFocus => {
                    let next_focus = self.focus.next();
                    info!("next focus {:?}", next_focus);
                    let _ = self.action_tx.send(Action::Focus(next_focus));
                }
                Action::Focus(focus) => {
                    info!("Setting focus {:?}", focus);
                    self.focus = focus;
                }
                Action::EditorMode(mode) => {
                    // It only makes sense to change the editor mode when the call view is in focus
                    if self.focus == Focus::Call {
                        self.editor_mode = mode;
                    }
                }
                Action::Help => {
                    info!("Help requested");
                    let _ = self.action_tx.send(Action::Focus(Focus::Help));
                }
                _ => {}
            }
            if let Some(action) = self.components.service_view.update(action.clone()).await? {
                self.action_tx.send(action)?
            };
            if let Some(action) = self.components.object_view.update(action.clone()).await? {
                self.action_tx.send(action)?
            };
            if let Some(action) = self.components.results_view.update(action.clone()).await? {
                self.action_tx.send(action)?
            };
            if let Some(action) = self.components.call_view.update(action.clone()).await? {
                self.action_tx.send(action)?
            };
            if let Some(action) = self.components.bottom_text.update(action.clone()).await? {
                self.action_tx.send(action)?
            };
            if let Some(action) = self.components.help_view.update(action.clone()).await? {
                self.action_tx.send(action)?
            };
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        tui.draw(|frame| {
            /*
                +----------------+----------------+
                |                |                |
                |                |                |
                |                |                |
                |                |                |
                |     Services   |     Objects    |
                |                |                |
                |                |                |
                |                |                |
                |                |                |
                +----------------+----------------+
                |                |                |
                |                |                |
                |   Result log   |       Call     |
                |                |                |
                |                |                |
                +---------------------------------+
                |           Bottom help text      |
                +---------------------------------+
            */

            let vertical_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(5), Constraint::Min(5), Constraint::Max(2)])
                .split(frame.area());
            let service_object_split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(25), Constraint::Percentage(75)].as_ref())
                .split(vertical_split[0]);
            let result_call_split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(vertical_split[1]);

            if let Err(err) = self
                .components
                .service_view
                .draw(frame, service_object_split[0])
            {
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("Failed to draw: {:?}", err)));
            }
            if let Err(err) = self
                .components
                .object_view
                .draw(frame, service_object_split[1])
            {
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("Failed to draw: {:?}", err)));
            }
            if let Err(err) = self
                .components
                .results_view
                .draw(frame, result_call_split[0])
            {
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("Failed to draw: {:?}", err)));
            }
            if let Err(err) = self.components.call_view.draw(frame, result_call_split[1]) {
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("Failed to draw: {:?}", err)));
            }
            if let Err(err) = self.components.bottom_text.draw(frame, vertical_split[2]) {
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("Failed to draw: {:?}", err)));
            }
            // We show the Help view overlaid on the other components as a popup
            if self.focus == Focus::Help {
                let pop_up = centered_area(frame.area(), 75, 75);
                if let Err(err) = self.components.help_view.draw(frame, pop_up) {
                    let _ = self
                        .action_tx
                        .send(Action::Error(format!("Failed to draw: {:?}", err)));
                }
            }
        })?;
        Ok(())
    }

    fn handle_dbus_actions(&mut self, _tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.dbus_receiver.try_recv() {
            if let Some(action) = self
                .components
                .service_view
                .update_from_dbus(action.clone())?
            {
                self.action_tx.send(action)?
            };
            if let Some(action) = self
                .components
                .object_view
                .update_from_dbus(action.clone())?
            {
                self.action_tx.send(action)?
            };
            if let Some(action) = self
                .components
                .bottom_text
                .update_from_dbus(action.clone())?
            {
                self.action_tx.send(action)?
            };
            if let Some(action) = self
                .components
                .results_view
                .update_from_dbus(action.clone())?
            {
                self.action_tx.send(action)?
            };
            if let Some(action) = self.components.call_view.update_from_dbus(action.clone())? {
                self.action_tx.send(action)?
            };
        }
        Ok(())
    }
}

fn centered_area(area: Rect, percent_y: u16, percent_x: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
