use bottom_text::BottomText;
use color_eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use objects_view::ObjectsView;
use ratatui::{
    Frame,
    layout::{Rect, Size},
};
use services_view::ServicesView;
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, components::{call_view::CallView, result_view::ResultsView}, config::Config, dbus_handler::{self, DbusActorHandle}, messages::AppMessage, tui::Event};

pub mod services_view;
pub mod objects_view;
pub mod bottom_text;
pub mod result_view;
pub mod call_view;

pub struct Components { 
    pub service_view: ServicesView,
    pub object_view: ObjectsView,
    pub bottom_text: BottomText,
    pub results_view: ResultsView,
    pub call_view: CallView

}

impl Components {
    pub fn new() -> Self {
        Components { service_view: ServicesView::new(), object_view: ObjectsView::new(), bottom_text: BottomText::new(), results_view: ResultsView::new(), call_view: CallView::new() }
    }
    
    pub fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.service_view.register_action_handler(tx.clone())?;
        self.object_view.register_action_handler(tx.clone())?;
        self.results_view.register_action_handler(tx.clone())?;
        self.call_view.register_action_handler(tx.clone())?;
        self.bottom_text.register_action_handler(tx)?;
        
        Ok(())
    }
    
    pub fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.service_view.register_config_handler(config.clone())?;
        self.object_view.register_config_handler(config.clone())?;
        self.results_view.register_config_handler(config.clone())?;
        self.call_view.register_config_handler(config.clone())?;
        self.bottom_text.register_config_handler(config)?;

        Ok(())
    }

    pub fn register_dbus_actor_handler(&mut self, dbus_handle: DbusActorHandle) -> Result<()> {
        self.service_view.register_dbus_actor_handle(dbus_handle.clone())?;
        self.object_view.register_dbus_actor_handle(dbus_handle.clone())?;
        self.results_view.register_dbus_actor_handle(dbus_handle.clone())?;
        self.call_view.register_dbus_actor_handle(dbus_handle.clone())?;

        Ok(())
    }
    
    pub fn init(&mut self, size: ratatui::prelude::Size) -> Result<()> {
        self.service_view.init(size)?;
        self.object_view.init(size)?;
        self.results_view.init(size)?;
        self.call_view.init(size)?;
        self.bottom_text.init(size)?;
        Ok(())
    }
    
    pub(crate) fn set_focus(&mut self, focus: crate::app::Focus)  {
        // TODO: Make call view and maybe results activable
        match focus {
            crate::app::Focus::Services => 
            {
                self.service_view.active(true);
                self.object_view.active(false);
            },
            crate::app::Focus::Objects => {
                self.service_view.active(false);
                self.object_view.active(true);
            },
            crate::app::Focus::All => (),
        }
    }
    
}

/// `Component` is a trait that represents a visual and interactive element of the user interface.
///
/// Implementors of this trait can be registered with the main application loop and will be able to
/// receive events, update state, and be rendered on the screen.
pub trait Component {
    /// Register an action handler that can send actions for processing if necessary.
    ///
    /// # Arguments
    ///
    /// * `tx` - An unbounded sender that can send actions.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        let _ = tx; // to appease clippy
        Ok(())
    }
    /// Register a configuration handler that provides configuration settings if necessary.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration settings.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        let _ = config; // to appease clippy
        Ok(())
    }
    /// Register a dbus_actor_handle that provides access to dbus
    ///
    /// # Arguments
    ///
    /// * `dbus_actor_handle` - DbusActorHandle that enables dbus access.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn register_dbus_actor_handle(&mut self, dbus_actor_handle: DbusActorHandle) -> Result<()> {
        let _ = dbus_actor_handle; // to appease clippy
        Ok(())
    }
    /// Initialize the component with a specified area if necessary.
    ///
    /// # Arguments
    ///
    /// * `area` - Rectangular area to initialize the component within.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn init(&mut self, area: Size) -> Result<()> {
        let _ = area; // to appease clippy
        Ok(())
    }
    /// Handle incoming events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `event` - An optional event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>> {
        let action = match event {
            Some(Event::Key(key_event)) => self.handle_key_event(key_event)?,
            Some(Event::Mouse(mouse_event)) => self.handle_mouse_event(mouse_event)?,
            _ => None,
        };
        Ok(action)
    }
    /// Handle key events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `key` - A key event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        let _ = key; // to appease clippy
        Ok(None)
    }
    /// Handle mouse events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `mouse` - A mouse event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        let _ = mouse; // to appease clippy
        Ok(None)
    }
    /// Update the state of the component based on a received action. (REQUIRED)
    ///
    /// # Arguments
    ///
    /// * `action` - An action that may modify the state of the component.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    async fn update(&mut self, action: Action) -> Result<Option<Action>> {
        let _ = action; // to appease clippy
        Ok(None)
    }
    /// Update the state of the component based on a received dbus action.
    ///
    /// # Arguments
    ///
    /// * `action` - A Dbus action that may modify the state of the component.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    /// 
    /// TODO: Should be merged with update
    fn update_from_dbus(&mut self, dbus_action: AppMessage) -> Result<Option<Action>> {
        let _ = dbus_action; // to appease clippy
        Ok(None)
    }
    /// Render the component on the screen. (REQUIRED)
    ///
    /// # Arguments
    ///
    /// * `f` - A frame used for rendering.
    /// * `area` - The area in which the component should be drawn.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()>;

    /// Set the component as active
    fn active(&mut self, active: bool);
}
