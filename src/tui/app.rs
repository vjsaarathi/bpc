use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::DefaultTerminal;
use std::io;

use super::layout_view::LayoutViewState;

pub struct App { running: bool, layout_view: Option<LayoutViewState> }

impl Default for App { fn default() -> Self { Self::new() } }

impl App {
    pub fn new() -> Self { Self { running: true, layout_view: None } }
    pub fn is_running(&self) -> bool { self.running }
    pub fn quit(&mut self) { self.running = false; }
    pub fn layout_view(&self) -> Option<&LayoutViewState> { self.layout_view.as_ref() }
    pub fn layout_view_mut(&mut self) -> Option<&mut LayoutViewState> { self.layout_view.as_mut() }
    pub fn set_layout_view(&mut self, view: LayoutViewState) { self.layout_view = Some(view); }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => self.quit(),
            KeyCode::Char('c') | KeyCode::Char('C') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit(),
            KeyCode::Down => if let Some(view) = self.layout_view.as_mut() { view.move_next_field(); },
            KeyCode::Up => if let Some(view) = self.layout_view.as_mut() { view.move_prev_field(); },
            KeyCode::Right | KeyCode::Enter => if let Some(view) = self.layout_view.as_mut() { view.toggle_selected_expansion(); },
            KeyCode::Left => if let Some(view) = self.layout_view.as_mut() { view.toggle_selected_expansion(); },
            KeyCode::Char('b') => if let Some(view) = self.layout_view.as_mut() { view.toggle_bit_mode(); },
            KeyCode::Char('f') => if let Some(view) = self.layout_view.as_mut() { view.toggle_selected_field_format(); },
            KeyCode::Char('F') => if let Some(view) = self.layout_view.as_mut() { view.toggle_global_format(); },
            KeyCode::Char('l') => if let Some(view) = self.layout_view.as_mut() { view.move_next_bit(); },
            KeyCode::Char('h') => if let Some(view) = self.layout_view.as_mut() { view.move_prev_bit(); },
            _ => {}
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while self.running {
            let size = terminal.size()?;
            if let Some(view) = self.layout_view.as_mut() { view.ensure_cursor_visible(size.height.saturating_sub(12)); }
            terminal.draw(|frame| super::ui::draw(frame, self))?;
            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? { self.handle_key_event(key); }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn new_app_is_running() { assert!(App::new().is_running()); }
    #[test] fn quit_app_stops_running() { let mut app = App::new(); app.quit(); assert!(!app.is_running()); }
    #[test] fn handle_key_event_q_quits() { let mut app = App::new(); app.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty())); assert!(!app.is_running()); }
    #[test] fn handle_key_event_ctrl_c_quits() { let mut app = App::new(); app.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)); assert!(!app.is_running()); }
    #[test] fn handle_key_event_other_key_continues() { let mut app = App::new(); app.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty())); assert!(app.is_running()); }
    #[test] fn new_app_has_no_layout() { assert!(App::new().layout_view().is_none()); }
    #[test] fn set_layout_view() { use crate::layout::BitLayout; let mut app = App::new(); let layout = BitLayout::builder().field("x", 8).build().unwrap(); app.set_layout_view(LayoutViewState::new(layout, vec![0xFF])); assert!(app.layout_view().is_some()); }
    #[test] fn handle_key_event_f_toggles_field_format() { use crate::layout::BitLayout; let mut app = App::new(); let layout = BitLayout::builder().field("x", 8).build().unwrap(); app.set_layout_view(LayoutViewState::new(layout, vec![0xFF])); assert_eq!(app.layout_view().unwrap().field_format("x").as_str(), "hex"); app.handle_key_event(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty())); assert_eq!(app.layout_view().unwrap().field_format("x").as_str(), "dec"); }
    #[test] fn handle_key_event_shift_f_toggles_global_format() { use crate::layout::BitLayout; let mut app = App::new(); let layout = BitLayout::builder().field("x", 8).field("y", 8).build().unwrap(); app.set_layout_view(LayoutViewState::new(layout, vec![0xFF, 0x00])); app.handle_key_event(KeyEvent::new(KeyCode::Char('F'), KeyModifiers::empty())); assert_eq!(app.layout_view().unwrap().global_format().as_str(), "dec"); }
}
