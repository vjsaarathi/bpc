use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::DefaultTerminal;
use std::io;

/// Application state for BPC.
pub struct App {
    /// Whether the application is still running.
    running: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates a new App with running=true.
    pub fn new() -> Self {
        Self { running: true }
    }

    /// Returns whether the application is still running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Sets the application state to stop running.
    pub fn quit(&mut self) {
        self.running = false;
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
            _ => {}
        }
    }

    /// Main event loop.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while self.running {
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
}
