//! TUI rendering for bit layout visualization.
//!
//! Provides [`LayoutViewState`] for managing layout view state (selection,
//! scrolling, and format toggles) and rendering functions for drawing the layout in the terminal.

use std::collections::HashMap;

use crate::bit::BitReader;
use crate::format::{FormatContext, FormatId, FormatRegistry};
use crate::layout::{BitLayout, LayoutField};
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// A flattened node representing a field in the hierarchical layout.
#[derive(Debug, Clone)]
pub struct DisplayNode<'a> {
    pub field: &'a LayoutField,
    pub path: String,
    pub depth: usize,
    pub abs_offset: usize,
}

/// State for the layout view in the TUI.
///
/// Tracks the current layout, raw data, cursor position, scroll offset,
/// format registry, and active formatting options per-field and globally.
pub struct LayoutViewState {
    layout: BitLayout,
    data: Vec<u8>,
    cursor_bit: usize,
    scroll_x: u16,
    format_registry: FormatRegistry,
    global_format: FormatId,
    field_formats: HashMap<String, FormatId>,
}

impl LayoutViewState {
    /// Creates a new layout view with the given layout and data using default formatters.
    pub fn new(layout: BitLayout, data: Vec<u8>) -> Self {
        Self::with_registry(layout, data, FormatRegistry::with_builtins())
    }

    /// Creates a new layout view with a customized format registry.
    pub fn with_registry(layout: BitLayout, data: Vec<u8>, format_registry: FormatRegistry) -> Self {
        let global_format = format_registry
            .formatters()
            .first()
            .map(|f| f.id())
            .unwrap_or_else(|| FormatId::new("hex"));

        Self {
            layout,
            data,
            cursor_bit: 0,
            scroll_x: 0,
            format_registry,
            global_format,
            field_formats: HashMap::new(),
        }
    }

    /// Returns a reference to the layout.
    pub fn layout(&self) -> &BitLayout {
        &self.layout
    }

    /// Recursively flattens the layout into a list of display nodes.
    pub fn flatten_nodes(&self) -> Vec<DisplayNode<'_>> {
        let mut nodes = Vec::new();
        self.flatten_recursive(&self.layout, "", 0, 0, &mut nodes);
        nodes
    }

    fn flatten_recursive<'a>(
        &self,
        layout: &'a BitLayout,
        prefix: &str,
        depth: usize,
        abs_offset: usize,
        nodes: &mut Vec<DisplayNode<'a>>,
    ) {
        for field in layout.fields() {
            let path = if prefix.is_empty() {
                field.name().to_string()
            } else {
                format!("{}.{}", prefix, field.name())
            };

            let field_abs_offset = abs_offset + field.offset();

            nodes.push(DisplayNode {
                field,
                path: path.clone(),
                depth,
                abs_offset: field_abs_offset,
            });

            if let crate::layout::field::FieldType::Layout(nested) = field.field_type() {
                self.flatten_recursive(nested, &path, depth + 1, field_abs_offset, nodes);
            }
        }
    }

    /// Returns a reference to the raw data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns the current cursor bit position.
    pub fn cursor_bit(&self) -> usize {
        self.cursor_bit
    }

    /// Returns the format registry.
    pub fn format_registry(&self) -> &FormatRegistry {
        &self.format_registry
    }

    /// Returns a mutable reference to the format registry.
    pub fn format_registry_mut(&mut self) -> &mut FormatRegistry {
        &mut self.format_registry
    }

    /// Returns the global default format ID.
    pub fn global_format(&self) -> &FormatId {
        &self.global_format
    }

    /// Sets the global format ID for all fields.
    pub fn set_global_format(&mut self, format_id: FormatId) {
        self.global_format = format_id;
    }

    /// Returns the active format ID for the given field path.
    pub fn field_format(&self, path: &str) -> &FormatId {
        self.field_formats
            .get(path)
            .unwrap_or(&self.global_format)
    }

    /// Sets the format ID for a specific field by path.
    pub fn set_field_format(&mut self, path: &str, format_id: FormatId) {
        self.field_formats.insert(path.to_string(), format_id);
    }

    /// Cycles the active format for the currently selected field.
    pub fn toggle_selected_field_format(&mut self) {
        if let Some(path) = self.selected_field_path() {
            let current = self.field_format(&path).clone();
            if let Some(next) = self.format_registry.next_format_id(&current) {
                self.set_field_format(&path, next);
            }
        }
    }

    /// Cycles the active format globally across all fields and resets per-field overrides.
    pub fn toggle_global_format(&mut self) {
        let current = self.global_format.clone();
        if let Some(next) = self.format_registry.next_format_id(&current) {
            self.global_format = next;
            // Clear per-field overrides so entire layout syncs to the new global format.
            self.field_formats.clear();
        }
    }

    /// Returns the currently selected node containing the cursor.
    pub fn selected_field_node(&self) -> Option<DisplayNode<'_>> {
        let nodes = self.flatten_nodes();
        nodes.into_iter().find(|n| {
            self.cursor_bit >= n.abs_offset && self.cursor_bit < n.abs_offset + n.field.width()
        })
    }

    /// Returns the path of the currently selected field.
    pub fn selected_field_path(&self) -> Option<String> {
        self.selected_field_node().map(|n| n.path)
    }

    /// Moves the cursor to the start of the next field.
    pub fn move_next_field(&mut self) {
        let nodes = self.flatten_nodes();
        if let Some(pos) = nodes.iter().position(|n| n.path == self.selected_field_path().unwrap_or_default()) {
            if pos + 1 < nodes.len() {
                self.cursor_bit = nodes[pos + 1].abs_offset;
            }
        } else if !nodes.is_empty() {
            self.cursor_bit = 0;
        }
    }

    /// Moves the cursor to the start of the previous field.
    pub fn move_prev_field(&mut self) {
        let nodes = self.flatten_nodes();
        if let Some(pos) = nodes.iter().position(|n| n.path == self.selected_field_path().unwrap_or_default()) {
            if pos > 0 {
                self.cursor_bit = nodes[pos - 1].abs_offset;
            }
        } else if !nodes.is_empty() {
            self.cursor_bit = nodes.last().unwrap().abs_offset;
        }
    }

    /// Moves the cursor to the next bit within the current field.
    pub fn move_next_bit(&mut self) {
        if let Some(node) = self.selected_field_node() {
            if self.cursor_bit + 1 < node.abs_offset + node.field.width() {
                self.cursor_bit += 1;
            }
        }
    }

    /// Moves the cursor to the previous bit within the current field.
    pub fn move_prev_bit(&mut self) {
        if let Some(node) = self.selected_field_node() {
            if self.cursor_bit > node.abs_offset {
                self.cursor_bit -= 1;
            }
        }
    }

    /// Reads the value of a single bit from the data.
    ///
    /// Returns `None` if the bit is outside the available data.
    pub fn read_bit_value(&self, bit_offset: usize) -> Option<bool> {
        if bit_offset >= self.data.len() * 8 {
            return None;
        }
        let mut reader = BitReader::from_bytes(&self.data);
        reader.skip(bit_offset).ok()?;
        reader.read_bit().ok()
    }

    /// Returns the formatted value of the field at `path`.
    pub fn format_field_value(&self, path: &str) -> String {
        let (field, abs_offset) = match self.layout.find_by_path(path) {
            Some(res) => res,
            None => return "(unknown field)".to_string(),
        };

        let format_id = self.field_format(path);
        let formatter = match self.format_registry.get(format_id) {
            Some(f) => f,
            None => return format!("(unknown format {})", format_id.as_str()),
        };

        let parsed_value = crate::format::extract_value(field, &self.data);

        let ctx = FormatContext {
            data: &self.data,
            offset: abs_offset,
            width: field.width(),
            parsed_value,
            field_type: Some(field.field_type()),
        };

        formatter.format(&ctx)
    }

    /// Adjusts scroll to keep the selected field visible.
    pub fn ensure_cursor_visible(&mut self, visible_height: u16) {
        let nodes = self.flatten_nodes();
        if nodes.is_empty() {
            self.scroll_x = 0; // Using scroll_x as vertical scroll
            return;
        }

        let selected_path = self.selected_field_path().unwrap_or_default();
        let selected_idx = nodes.iter().position(|n| n.path == selected_path).unwrap_or(0);

        let vh = visible_height as usize;
        let sx = self.scroll_x as usize;

        if selected_idx < sx {
            self.scroll_x = selected_idx as u16;
        } else if selected_idx >= sx + vh.saturating_sub(2) {
            self.scroll_x = (selected_idx + 3).saturating_sub(vh) as u16;
        }
    }
}

/// Draws the complete layout view into the given area.
pub fn draw_layout_view(frame: &mut ratatui::Frame, state: &LayoutViewState, area: ratatui::layout::Rect) {
    if state.layout.is_empty() {
        let p = Paragraph::new("No fields defined.")
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);
        frame.render_widget(p, area);
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Min(10), // field tree (border + 5 lines + border)
        Constraint::Length(3), // bit values (border + 1 line + border)
        Constraint::Min(4),   // details panel
    ])
    .split(area);

    draw_tree(frame, state, chunks[0]);
    draw_bit_values(frame, state, chunks[1]);
    draw_details(frame, state, chunks[2]);
}

/// Draws the hierarchical field tree.
fn draw_tree(frame: &mut ratatui::Frame, state: &LayoutViewState, area: ratatui::layout::Rect) {
    let nodes = state.flatten_nodes();
    let selected_path = state.selected_field_path();

    let mut lines = Vec::new();

    if nodes.is_empty() {
        lines.push(Line::from(Span::styled("(no fields)", Style::default().fg(Color::DarkGray))));
    }

    for node in nodes {
        let is_selected = Some(&node.path) == selected_path.as_ref();
        
        // Depth indentation
        let indent = "  ".repeat(node.depth);
        let prefix = if node.field.is_variable() { "~" } else { "-" };
        
        let type_marker = match node.field.field_type() {
            crate::layout::field::FieldType::Layout(_) => "[Layout]",
            crate::layout::field::FieldType::Enum(_) => "[Enum]",
            crate::layout::field::FieldType::Primitive(_) => "",
        };

        let val_str = state.format_field_value(&node.path);
        let fmt_id = state.field_format(&node.path).as_str();

        let label = format!("{indent}{prefix} {} {} ({} bits) = {} [{}]", node.field.name(), type_marker, node.field.width(), val_str, fmt_id);
        
        let style = if is_selected {
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        lines.push(Line::from(Span::styled(label, style)));
    }

    let block = Block::default().title(" Layout Tree ").borders(Borders::ALL);
    // Since it's a vertical list, we might want to slice `lines` by scroll, but Paragraph handles some of this.
    // Actually scroll_x might need to be scroll_y for vertical scrolling.
    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((0, state.scroll_x)); // scroll_x is a bit badly named now, but we'll use it as scroll_y.

    frame.render_widget(paragraph, area);
}

/// Draws the bit values
fn draw_bit_values(frame: &mut ratatui::Frame, state: &LayoutViewState, area: ratatui::layout::Rect) {
    let layout = &state.layout;
    let layout_bits = layout.bit_len();
    let data_bits = state.data.len() * 8;
    let readable = layout_bits.min(data_bits);

    let mut spans: Vec<Span> = vec![Span::raw("  ")];

    if layout_bits == 0 {
        spans.push(Span::styled("(no data)", Style::default().fg(Color::DarkGray)));
    } else {
        let mut reader = BitReader::new(&state.data, readable);
        let selected_node = state.selected_field_node();

        let nodes = state.flatten_nodes();

        for bit_idx in 0..layout_bits {
            // Separator before this bit.
            if bit_idx > 0 {
                let is_field_boundary = nodes.iter().any(|n| n.abs_offset == bit_idx);
                let is_byte_boundary = bit_idx % 8 == 0;

                if is_field_boundary {
                    spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
                } else if is_byte_boundary {
                    spans.push(Span::raw(" "));
                }
            }

            // Read the bit value.
            let (ch, available) = if bit_idx < readable {
                let bit = reader.read_bit().unwrap_or(false);
                (if bit { '1' } else { '0' }, true)
            } else {
                ('·', false)
            };

            let is_in_selected = selected_node.as_ref().is_some_and(|n| {
                bit_idx >= n.abs_offset && bit_idx < n.abs_offset + n.field.width()
            });

            let style = if bit_idx == state.cursor_bit {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else if !available {
                Style::default().fg(Color::DarkGray)
            } else if is_in_selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            spans.push(Span::styled(ch.to_string(), style));
        }

        // Padding past layout end to complete the byte.
        let padded = (layout_bits + 7) / 8 * 8;
        for bit_idx in layout_bits..padded {
            if bit_idx > 0 && bit_idx % 8 == 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled("·", Style::default().fg(Color::DarkGray)));
        }
    }

    let text = vec![Line::from(spans)];
    let p = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" Bits "))
        .scroll((0, state.scroll_x));
    frame.render_widget(p, area);
}

/// Draws the details panel showing info about the selected field and bit across all registered formats.
fn draw_details(frame: &mut ratatui::Frame, state: &LayoutViewState, area: ratatui::layout::Rect) {
    let mut lines = Vec::new();

    if let Some(node) = state.selected_field_node() {
        let field = node.field;
        let current_fmt_id = state.field_format(&node.path);

        lines.push(Line::from(vec![
            Span::styled("  Field: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(field.name()),
            Span::raw(format!(
                " (Path: {}, Abs Offset: {}, Width: {}b, Range: [{}, {}))",
                node.path,
                node.abs_offset,
                field.width(),
                node.abs_offset,
                node.abs_offset + field.width()
            )),
        ]));

        // Bit details.
        let cursor = state.cursor_bit;
        let relative = cursor - node.abs_offset;
        let value = state.read_bit_value(cursor);
        let value_str = match value {
            Some(true) => "1",
            Some(false) => "0",
            None => "?",
        };
        lines.push(Line::from(vec![
            Span::styled("  Bit: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "absolute={cursor}  relative={relative}  value={value_str}"
            )),
        ]));

        lines.push(Line::from(""));

        // Representations in all registered formats:
        let parsed_value = crate::format::extract_value(field, &state.data);

        let ctx = FormatContext {
            data: &state.data,
            offset: node.abs_offset,
            width: field.width(),
            parsed_value,
            field_type: Some(field.field_type()),
        };

        let mut format_spans = vec![Span::styled(
            "  Formats: ",
            Style::default().add_modifier(Modifier::BOLD),
        )];

        for (f_idx, formatter) in state.format_registry.formatters().iter().enumerate() {
            let is_active = formatter.id() == *current_fmt_id;
            let val_str = formatter.format(&ctx);
            let label = format!("{}: {}", formatter.name(), val_str);

            if f_idx > 0 {
                format_spans.push(Span::raw(" | "));
            }

            if is_active {
                format_spans.push(Span::styled(
                    label,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                format_spans.push(Span::raw(label));
            }
        }
        lines.push(Line::from(format_spans));
    } else {
        lines.push(Line::from("  No field selected."));
    }

    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Details "));
    frame.render_widget(p, area);
}
