use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Renders the user interface.
pub fn draw(frame: &mut Frame, _app: &super::app::App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(frame.area());

    let title = Paragraph::new(" BPC — Binary Protocol Workbench")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(title, chunks[0]);

    let body = Paragraph::new("No protocol loaded.")
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    frame.render_widget(body, chunks[1]);

    let footer = Paragraph::new(" q: quit")
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}
