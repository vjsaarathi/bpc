//! TUI rendering for bit layout visualization.

use std::collections::{HashMap, HashSet};

use crate::bit::BitReader;
use crate::format::{FormatContext, FormatId, FormatRegistry};
use crate::layout::{BitLayout, LayoutField};
use ratatui::{layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Paragraph}, Frame};

#[derive(Debug, Clone)]
pub struct DisplayNode<'a> {
    pub field: &'a LayoutField,
    pub path: String,
    pub depth: usize,
    pub abs_offset: usize,
}

pub struct LayoutViewState {
    layout: BitLayout,
    data: Vec<u8>,
    cursor_bit: usize,
    scroll_x: u16,
    format_registry: FormatRegistry,
    global_format: FormatId,
    field_formats: HashMap<String, FormatId>,
    expanded: HashSet<String>,
    bit_mode_hex: bool,
}

impl LayoutViewState {
    pub fn new(layout: BitLayout, data: Vec<u8>) -> Self { Self::with_registry(layout, data, FormatRegistry::with_builtins()) }

    pub fn with_registry(layout: BitLayout, data: Vec<u8>, format_registry: FormatRegistry) -> Self {
        let global_format = format_registry.formatters().first().map(|f| f.id()).unwrap_or_else(|| FormatId::new("hex"));
        let mut expanded = HashSet::new();
        Self::collect_layout_paths(&layout, "", &mut expanded);
        Self { layout, data, cursor_bit: 0, scroll_x: 0, format_registry, global_format, field_formats: HashMap::new(), expanded, bit_mode_hex: false }
    }

    fn collect_layout_paths(layout: &BitLayout, prefix: &str, expanded: &mut HashSet<String>) {
        for field in layout.fields() {
            let path = if prefix.is_empty() { field.name().to_string() } else { format!("{}.{}", prefix, field.name()) };
            if let crate::layout::field::FieldType::Layout(nested) = field.field_type() {
                expanded.insert(path.clone());
                Self::collect_layout_paths(nested, &path, expanded);
            }
        }
    }

    pub fn layout(&self) -> &BitLayout { &self.layout }
    pub fn data(&self) -> &[u8] { &self.data }
    pub fn cursor_bit(&self) -> usize { self.cursor_bit }
    pub fn format_registry(&self) -> &FormatRegistry { &self.format_registry }
    pub fn format_registry_mut(&mut self) -> &mut FormatRegistry { &mut self.format_registry }
    pub fn global_format(&self) -> &FormatId { &self.global_format }

    pub fn flatten_nodes(&self) -> Vec<DisplayNode<'_>> {
        let mut nodes = Vec::new();
        self.flatten_recursive(&self.layout, "", 0, 0, &mut nodes);
        nodes
    }

    fn flatten_recursive<'a>(&self, layout: &'a BitLayout, prefix: &str, depth: usize, abs_offset: usize, nodes: &mut Vec<DisplayNode<'a>>) {
        for field in layout.fields() {
            let path = if prefix.is_empty() { field.name().to_string() } else { format!("{}.{}", prefix, field.name()) };
            let field_abs_offset = abs_offset + field.offset();
            nodes.push(DisplayNode { field, path: path.clone(), depth, abs_offset: field_abs_offset });
            if let crate::layout::field::FieldType::Layout(nested) = field.field_type() {
                if self.expanded.contains(&path) { self.flatten_recursive(nested, &path, depth + 1, field_abs_offset, nodes); }
            }
        }
    }

    fn all_nodes(&self) -> Vec<DisplayNode<'_>> {
        let mut nodes = Vec::new();
        self.flatten_all_recursive(&self.layout, "", 0, 0, &mut nodes);
        nodes
    }

    fn flatten_all_recursive<'a>(&self, layout: &'a BitLayout, prefix: &str, depth: usize, abs_offset: usize, nodes: &mut Vec<DisplayNode<'a>>) {
        for field in layout.fields() {
            let path = if prefix.is_empty() { field.name().to_string() } else { format!("{}.{}", prefix, field.name()) };
            let field_abs_offset = abs_offset + field.offset();
            nodes.push(DisplayNode { field, path: path.clone(), depth, abs_offset: field_abs_offset });
            if let crate::layout::field::FieldType::Layout(nested) = field.field_type() { self.flatten_all_recursive(nested, &path, depth + 1, field_abs_offset, nodes); }
        }
    }

    pub fn selected_field_node(&self) -> Option<DisplayNode<'_>> {
        self.all_nodes().into_iter().find(|n| self.cursor_bit >= n.abs_offset && self.cursor_bit < n.abs_offset + n.field.width())
    }

    pub fn selected_field_path(&self) -> Option<String> { self.selected_field_node().map(|n| n.path) }

    pub fn toggle_selected_expansion(&mut self) {
        if let Some(path) = self.selected_field_path() {
            if let Some(node) = self.all_nodes().into_iter().find(|n| n.path == path) {
                if matches!(node.field.field_type(), crate::layout::field::FieldType::Layout(_)) {
                    if !self.expanded.remove(&path) { self.expanded.insert(path); }
                }
            }
        }
    }

    pub fn set_global_format(&mut self, format_id: FormatId) { self.global_format = format_id; }
    pub fn field_format(&self, path: &str) -> &FormatId { self.field_formats.get(path).unwrap_or(&self.global_format) }
    pub fn set_field_format(&mut self, path: &str, format_id: FormatId) { self.field_formats.insert(path.to_string(), format_id); }

    pub fn toggle_selected_field_format(&mut self) {
        if let Some(path) = self.selected_field_path() {
            let current = self.field_format(&path).clone();
            if let Some(next) = self.format_registry.next_format_id(&current) { self.set_field_format(&path, next); }
        }
    }

    pub fn toggle_global_format(&mut self) {
        let current = self.global_format.clone();
        if let Some(next) = self.format_registry.next_format_id(&current) { self.global_format = next; self.field_formats.clear(); }
    }

    pub fn toggle_bit_mode(&mut self) { self.bit_mode_hex = !self.bit_mode_hex; }

    pub fn move_next_field(&mut self) {
        let nodes = self.flatten_nodes();
        let current = self.selected_field_path();
        if let Some(pos) = current.and_then(|p| nodes.iter().position(|n| n.path == p)) { if pos + 1 < nodes.len() { self.cursor_bit = nodes[pos + 1].abs_offset; } }
        else if let Some(first) = nodes.first() { self.cursor_bit = first.abs_offset; }
    }

    pub fn move_prev_field(&mut self) {
        let nodes = self.flatten_nodes();
        let current = self.selected_field_path();
        if let Some(pos) = current.and_then(|p| nodes.iter().position(|n| n.path == p)) { if pos > 0 { self.cursor_bit = nodes[pos - 1].abs_offset; } }
        else if let Some(last) = nodes.last() { self.cursor_bit = last.abs_offset; }
    }

    pub fn move_next_bit(&mut self) {
        if let Some(node) = self.selected_field_node() { if self.cursor_bit + 1 < node.abs_offset + node.field.width() { self.cursor_bit += 1; } }
    }

    pub fn move_prev_bit(&mut self) {
        if let Some(node) = self.selected_field_node() { if self.cursor_bit > node.abs_offset { self.cursor_bit -= 1; } }
    }

    pub fn read_bit_value(&self, bit_offset: usize) -> Option<bool> {
        if bit_offset >= self.data.len() * 8 { return None; }
        let mut reader = BitReader::from_bytes(&self.data);
        reader.skip(bit_offset).ok()?;
        reader.read_bit().ok()
    }

    pub fn format_field_value(&self, path: &str) -> String {
        let (field, abs_offset) = match self.layout.find_by_path(path) { Some(v) => v, None => return "—".into() };
        let formatter = match self.format_registry.get(self.field_format(path)) { Some(v) => v, None => return "—".into() };
        let parsed_value = crate::format::extract_value(field, &self.data);
        formatter.format(&FormatContext { data: &self.data, offset: abs_offset, width: field.width(), parsed_value, field_type: Some(field.field_type()) })
    }

    pub fn ensure_cursor_visible(&mut self, visible_height: u16) {
        let nodes = self.flatten_nodes();
        let Some(path) = self.selected_field_path() else { self.scroll_x = 0; return; };
        let Some(idx) = nodes.iter().position(|n| n.path == path) else { return; };
        let height = visible_height.max(1) as usize;
        let scroll = self.scroll_x as usize;
        if idx < scroll { self.scroll_x = idx as u16; }
        else if idx >= scroll + height.saturating_sub(1) { self.scroll_x = idx.saturating_sub(height.saturating_sub(2)) as u16; }
    }

    fn type_label(field: &LayoutField) -> String {
        match field.field_type() {
            crate::layout::field::FieldType::Primitive(_) => format!("bits<{}>", field.width()),
            crate::layout::field::FieldType::Layout(_) => "layout".into(),
            crate::layout::field::FieldType::Enum(_) => "enum".into(),
        }
    }
}

pub fn draw_layout_view(frame: &mut Frame, state: &LayoutViewState, area: Rect) {
    if state.layout.is_empty() {
        frame.render_widget(Paragraph::new("No fields defined.").block(Block::default().borders(Borders::ALL).title(" Inspector ")), area);
        return;
    }
    let rows = Layout::default().direction(Direction::Vertical).constraints([Constraint::Min(8), Constraint::Length(8), Constraint::Length(2)]).split(area);
    draw_structure_and_fields(frame, state, rows[0]);
    draw_raw_data(frame, state, rows[1]);
    draw_context(frame, state, rows[2]);
}

fn draw_structure_and_fields(frame: &mut Frame, state: &LayoutViewState, area: Rect) {
    let cols = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(28), Constraint::Percentage(72)]).split(area);
    draw_tree(frame, state, cols[0]);
    draw_field_table(frame, state, cols[1]);
}

fn draw_tree(frame: &mut Frame, state: &LayoutViewState, area: Rect) {
    let selected = state.selected_field_path();
    let mut lines = Vec::new();
    for node in state.flatten_nodes() {
        let is_selected = selected.as_deref() == Some(node.path.as_str());
        let is_layout = matches!(node.field.field_type(), crate::layout::field::FieldType::Layout(_));
        let marker = if is_layout { if state.expanded.contains(&node.path) { "▾" } else { "▸" } } else { "·" };
        let label = format!("{}{} {}", "  ".repeat(node.depth), marker, node.field.name());
        let style = if is_selected { Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD) } else if is_layout { Style::default().add_modifier(Modifier::BOLD) } else { Style::default() };
        lines.push(Line::from(Span::styled(label, style)));
    }
    frame.render_widget(Paragraph::new(lines).block(Block::default().title(" Structure ").borders(Borders::ALL)).scroll((0, state.scroll_x)), area);
}

fn draw_field_table(frame: &mut Frame, state: &LayoutViewState, area: Rect) {
    let selected = state.selected_field_path();
    let header = Line::from(vec![Span::styled(format!("{:<24}", "NAME"), Style::default().add_modifier(Modifier::BOLD)), Span::styled(format!("{:<14}", "TYPE"), Style::default().add_modifier(Modifier::BOLD)), Span::styled(format!("{:<14}", "RANGE"), Style::default().add_modifier(Modifier::BOLD)), Span::styled("VALUE", Style::default().add_modifier(Modifier::BOLD))]);
    let mut lines = vec![header, Line::from("────────────────────────────────────────────────────────────────────────")];
    for node in state.flatten_nodes() {
        let selected_row = selected.as_deref() == Some(node.path.as_str());
        let name_width = 24usize.saturating_sub(node.depth * 2).max(8);
        let name = format!("{}{}", "  ".repeat(node.depth), node.field.name());
        let value = if matches!(node.field.field_type(), crate::layout::field::FieldType::Layout(_)) { "—".to_string() } else { state.format_field_value(&node.path) };
        let range = if node.field.width() == 0 { "—".to_string() } else { format!("{}..{}", node.abs_offset, node.abs_offset + node.field.width() - 1) };
        let row = format!("{:<width$} {:<14} {:<14} {}", name, LayoutViewState::type_label(node.field), range, value, width = name_width);
        let style = if selected_row { Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default() };
        lines.push(Line::from(Span::styled(row, style)));
    }
    frame.render_widget(Paragraph::new(lines).block(Block::default().title(" Fields ").borders(Borders::ALL)).scroll((0, state.scroll_x)), area);
}

fn draw_raw_data(frame: &mut Frame, state: &LayoutViewState, area: Rect) {
    let layout_bits = state.layout.bit_len();
    let readable = layout_bits.min(state.data.len() * 8);
    let selected = state.selected_field_node();
    let mut spans = Vec::new();
    if state.bit_mode_hex {
        for (i, byte) in state.data.iter().enumerate() {
            if i * 8 >= layout_bits { break; }
            let start = i * 8;
            let end = (start + 8).min(layout_bits);
            let highlighted = selected.as_ref().is_some_and(|n| start < n.abs_offset + n.field.width() && end > n.abs_offset);
            if i > 0 { spans.push(Span::raw(" ")); }
            spans.push(Span::styled(format!("{:02X}", byte), if highlighted { Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD) } else { Style::default() }));
        }
    } else if readable > 0 {
        let mut reader = BitReader::new(&state.data, readable);
        for bit in 0..readable {
            if bit > 0 && bit % 8 == 0 { spans.push(Span::raw(" ")); }
            let value = reader.read_bit().unwrap_or(false);
            let highlighted = selected.as_ref().is_some_and(|n| bit >= n.abs_offset && bit < n.abs_offset + n.field.width());
            let style = if bit == state.cursor_bit { Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD) } else if highlighted { Style::default().bg(Color::Yellow).fg(Color::Black) } else { Style::default() };
            spans.push(Span::styled(if value { "1" } else { "0" }, style));
        }
    } else { spans.push(Span::styled("No data", Style::default().add_modifier(Modifier::DIM))); }
    let mode = if state.bit_mode_hex { "HEX" } else { "BITS" };
    frame.render_widget(Paragraph::new(Line::from(spans)).block(Block::default().title(format!(" Wire · {} ", mode)).borders(Borders::ALL)), area);
}

fn draw_context(frame: &mut Frame, state: &LayoutViewState, area: Rect) {
    let text = if let Some(node) = state.selected_field_node() {
        let rendered = if matches!(node.field.field_type(), crate::layout::field::FieldType::Layout(_)) { "—".to_string() } else { state.format_field_value(&node.path) };
        format!("● {}  ·  {}  ·  bits {}..{}  ·  {}", node.path, LayoutViewState::type_label(node.field), node.abs_offset, node.abs_offset + node.field.width().saturating_sub(1), rendered)
    } else { "● No field selected".into() };
    frame.render_widget(Paragraph::new(text).block(Block::default().borders(Borders::ALL)), area);
}
