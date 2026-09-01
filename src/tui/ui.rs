use ratatui::{layout::{Alignment, Constraint, Layout}, style::{Modifier, Style}, widgets::{Block, Borders, Paragraph}, Frame};

/// Renders the BPC workbench.
pub fn draw(frame: &mut Frame, app: &super::app::App) {
    let chunks = Layout::vertical([Constraint::Length(2), Constraint::Min(1), Constraint::Length(2)]).split(frame.area());

    let header = Paragraph::new(" BPC  ·  Binary Protocol Workbench")
        .block(Block::default().borders(Borders::BOTTOM))
        .style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(header, chunks[0]);

    if let Some(view) = app.layout_view() {
        super::layout_view::draw_layout_view(frame, view, chunks[1]);
    } else {
        let body = Paragraph::new("No layout loaded.").alignment(Alignment::Center).block(Block::default().borders(Borders::ALL).title(" Inspector "));
        frame.render_widget(body, chunks[1]);
    }

    let footer_text = if app.layout_view().is_some() {
        " ↑↓ Navigate   ←→ Expand   Enter Expand   b Bits/Hex   f Format   F Global   h/l Bit   q Quit "
    } else { " q Quit " };
    let footer = Paragraph::new(footer_text).block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, chunks[2]);
}
