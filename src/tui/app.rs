use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::DefaultTerminal;
use std::io;

use super::layout_view::LayoutViewState;

/// Application state for BPC.
pub struct App {
    /// Whether the application is still running.
    running: bool,
    /// Optional layout view state.
    layout_view: Option<LayoutViewState>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates a new App with running=true and no layout.
    pub fn new() -> Self {
        Self {
            running: true,
            layout_view: None,
        }
    }

    /// Returns whether the application is still running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Sets the application state to stop running.
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Returns a reference to the layout view state, if any.
    pub fn layout_view(&self) -> Option<&LayoutViewState> {
        self.layout_view.as_ref()
    }

    /// Returns a mutable reference to the layout view state, if any.
    pub fn layout_view_mut(&mut self) -> Option<&mut LayoutViewState> {
        self.layout_view.as_mut()
    }

    /// Sets the layout view state.
    pub fn set_layout_view(&mut self, view: LayoutViewState) {
        self.layout_view = Some(view);
    }

    /// Handles a key event.
    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => self.quit(),
            KeyCode::Char('c') | KeyCode::Char('C')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.quit()
            }
            KeyCode::Right => {
                if let Some(ref mut view) = self.layout_view {
                    view.move_next_field();
                }
            }
            KeyCode::Left => {
                if let Some(ref mut view) = self.layout_view {
                    view.move_prev_field();
                }
            }
            KeyCode::Down => {
                if let Some(ref mut view) = self.layout_view {
                    view.move_next_bit();
                }
            }
            KeyCode::Up => {
                if let Some(ref mut view) = self.layout_view {
                    view.move_prev_bit();
                }
            }
            // Toggle format for current selected field
            KeyCode::Char('f') => {
                if let Some(ref mut view) = self.layout_view {
                    view.toggle_selected_field_format();
                }
            }
            // Toggle format for entire layout (global)
            KeyCode::Char('F') => {
                if let Some(ref mut view) = self.layout_view {
                    view.toggle_global_format();
                }
            }
            _ => {}
        }
    }

    /// Main event loop.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while self.running {
            // Pre-compute scroll based on terminal size.
            let size = terminal.size()?;
            if let Some(ref mut view) = self.layout_view {
                view.ensure_cursor_visible(size.width.saturating_sub(4));
            }

            terminal.draw(|frame| super::ui::draw(frame, self))?;

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key_event(key);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_app_is_running() {
        let app = App::new();
        assert!(app.is_running());
    }

    #[test]
    fn quit_app_stops_running() {
        let mut app = App::new();
        app.quit();
        assert!(!app.is_running());
    }

    #[test]
    fn handle_key_event_q_quits() {
        let mut app = App::new();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()));
        assert!(!app.is_running());
    }

    #[test]
    fn handle_key_event_ctrl_c_quits() {
        let mut app = App::new();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(!app.is_running());
    }

    #[test]
    fn handle_key_event_other_key_continues() {
        let mut app = App::new();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        assert!(app.is_running());
    }

    #[test]
    fn new_app_has_no_layout() {
        let app = App::new();
        assert!(app.layout_view().is_none());
    }

    #[test]
    fn set_layout_view() {
        use crate::layout::BitLayout;

        let mut app = App::new();
        let layout = BitLayout::builder().field("x", 8).build().unwrap();
        app.set_layout_view(LayoutViewState::new(layout, vec![0xFF]));
        assert!(app.layout_view().is_some());
    }

    #[test]
    fn handle_key_event_f_toggles_field_format() {
        use crate::layout::BitLayout;

        let mut app = App::new();
        let layout = BitLayout::builder().field("x", 8).build().unwrap();
        app.set_layout_view(LayoutViewState::new(layout, vec![0xFF]));
        assert_eq!(app.layout_view().unwrap().field_format(0).as_str(), "hex");

        app.handle_key_event(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()));
        assert_eq!(app.layout_view().unwrap().field_format(0).as_str(), "dec");
    }

    #[test]
    fn handle_key_event_shift_f_toggles_global_format() {
        use crate::layout::BitLayout;

        let mut app = App::new();
        let layout = BitLayout::builder().field("x", 8).field("y", 8).build().unwrap();
        app.set_layout_view(LayoutViewState::new(layout, vec![0xFF, 0x00]));
        assert_eq!(app.layout_view().unwrap().global_format().as_str(), "hex");

        app.handle_key_event(KeyEvent::new(KeyCode::Char('F'), KeyModifiers::empty()));
        assert_eq!(app.layout_view().unwrap().global_format().as_str(), "dec");
        assert_eq!(app.layout_view().unwrap().field_format(0).as_str(), "dec");
        assert_eq!(app.layout_view().unwrap().field_format(1).as_str(), "dec");
    }
}
