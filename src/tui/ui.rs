use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Renders the user interface.
pub fn draw(frame: &mut Frame, app: &super::app::App) {
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

    // Body: show the layout view if available, otherwise a placeholder.
    if let Some(view) = app.layout_view() {
        super::layout_view::draw_layout_view(frame, view, chunks[1]);
    } else {
        let body = Paragraph::new("No fields defined.")
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);
        frame.render_widget(body, chunks[1]);
    }

    let footer_text = if app.layout_view().is_some() {
        " ←/→: field  ↑/↓: bit  f: toggle field format  F: toggle all formats  q: quit"
    } else {
        " q: quit"
    };
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}
