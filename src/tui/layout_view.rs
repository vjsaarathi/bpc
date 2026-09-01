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
    field_formats: HashMap<usize, FormatId>,
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

    /// Returns the active format ID for the field at index `field_idx`.
    pub fn field_format(&self, field_idx: usize) -> &FormatId {
        self.field_formats
            .get(&field_idx)
            .unwrap_or(&self.global_format)
    }

    /// Sets the format ID for a specific field by index.
    pub fn set_field_format(&mut self, field_idx: usize, format_id: FormatId) {
        self.field_formats.insert(field_idx, format_id);
    }

    /// Cycles the active format for the currently selected field.
    pub fn toggle_selected_field_format(&mut self) {
        if let Some(idx) = self.selected_field_index() {
            let current = self.field_format(idx).clone();
            if let Some(next) = self.format_registry.next_format_id(&current) {
                self.set_field_format(idx, next);
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

    /// Returns the index of the field containing the cursor.
    pub fn selected_field_index(&self) -> Option<usize> {
        self.layout.field_index_at_bit(self.cursor_bit)
    }

    /// Moves the cursor to the start of the next field.
    pub fn move_next_field(&mut self) {
        if let Some(idx) = self.selected_field_index() {
            if idx + 1 < self.layout.field_count() {
                self.cursor_bit = self.layout.field(idx + 1).unwrap().offset();
            }
        } else if !self.layout.is_empty() {
            self.cursor_bit = 0;
        }
    }

    /// Moves the cursor to the start of the previous field.
    pub fn move_prev_field(&mut self) {
        if let Some(idx) = self.selected_field_index() {
            if idx > 0 {
                self.cursor_bit = self.layout.field(idx - 1).unwrap().offset();
            }
        } else if !self.layout.is_empty() {
            let last = self.layout.fields().last().unwrap();
            self.cursor_bit = last.offset();
        }
    }

    /// Moves the cursor to the next bit within the current field.
    pub fn move_next_bit(&mut self) {
        if let Some(idx) = self.selected_field_index() {
            let field = self.layout.field(idx).unwrap();
            if self.cursor_bit + 1 < field.end() {
                self.cursor_bit += 1;
            }
        }
    }

    /// Moves the cursor to the previous bit within the current field.
    pub fn move_prev_bit(&mut self) {
        if let Some(idx) = self.selected_field_index() {
            let field = self.layout.field(idx).unwrap();
            if self.cursor_bit > field.offset() {
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

    /// Formats the field value using its configured format.
    pub fn format_field_value(&self, field_idx: usize) -> String {
        let field = match self.layout.field(field_idx) {
            Some(f) => f,
            None => return "(invalid field)".to_string(),
        };

        let format_id = self.field_format(field_idx);
        let formatter = match self.format_registry.get(format_id) {
            Some(f) => f,
            None => return format!("(unknown format {})", format_id.as_str()),
        };

        let numeric_value = if field.width() <= 64 && field.end() <= self.data.len() * 8 {
            let mut reader = BitReader::from_bytes(&self.data);
            if reader.skip(field.offset()).is_ok() {
                reader.read_bits(field.width() as u32).ok()
            } else {
                None
            }
        } else {
            None
        };

        let ctx = FormatContext {
            data: &self.data,
            offset: field.offset(),
            width: field.width(),
            numeric_value,
        };

        formatter.format(&ctx)
    }

    /// Adjusts scroll to keep the selected field visible.
    pub fn ensure_cursor_visible(&mut self, visible_width: u16) {
        let fields = self.layout.fields();
        if fields.is_empty() {
            self.scroll_x = 0;
            return;
        }

        let selected_idx = self.selected_field_index().unwrap_or(0);

        // Compute character offset of the selected field in the diagram.
        let mut pos = 3usize; // "  ┌" prefix
        for (i, f) in fields.iter().enumerate() {
            let val_str = self.format_field_value(i);
            let w = field_visual_width(f, &val_str);
            if i == selected_idx {
                let end = pos + w + 1;
                let vw = visible_width as usize;
                let sx = self.scroll_x as usize;

                if pos < sx + 1 {
                    self.scroll_x = pos.saturating_sub(1) as u16;
                } else if end > sx + vw {
                    self.scroll_x = end.saturating_sub(vw) as u16;
                }
                return;
            }
            pos += w + 1; // +1 for separator
        }
    }
}

/// Computes the visual character width for a field in the diagram.
fn field_visual_width(field: &LayoutField, val_str: &str) -> usize {
    let name_len = field.name().len();
    let width_label_len = format!("{}b", field.width()).len();
    let val_len = val_str.len();
    let min_content = name_len.max(width_label_len).max(val_len);
    (min_content + 2).max(4)
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
        Constraint::Length(7), // field diagram (border + 5 lines + border)
        Constraint::Length(3), // bit values (border + 1 line + border)
        Constraint::Min(4),   // details panel
    ])
    .split(area);

    draw_field_diagram(frame, state, chunks[0]);
    draw_bit_values(frame, state, chunks[1]);
    draw_details(frame, state, chunks[2]);
}

/// Draws the field diagram (boxes with names, values, widths, and active formats).
fn draw_field_diagram(frame: &mut ratatui::Frame, state: &LayoutViewState, area: ratatui::layout::Rect) {
    let layout = &state.layout;
    let selected_idx = state.selected_field_index();

    let field_values: Vec<String> = (0..layout.field_count())
        .map(|i| state.format_field_value(i))
        .collect();

    let widths: Vec<usize> = layout
        .fields()
        .iter()
        .zip(&field_values)
        .map(|(f, val)| field_visual_width(f, val))
        .collect();

    let field_count = layout.field_count();

    // Top border: ┌───┬───┬...┐
    let mut top = String::from("  ┌");
    for (i, &w) in widths.iter().enumerate() {
        for _ in 0..w {
            top.push('─');
        }
        top.push(if i < field_count - 1 { '┬' } else { '┐' });
    }

    // Name line: │ name │ name │...
    let mut name_spans = vec![Span::raw("  │")];
    for (i, (field, &w)) in layout.fields().iter().zip(&widths).enumerate() {
        let is_selected = Some(i) == selected_idx;
        let label = format!("{:^width$}", field.name(), width = w);
        let style = if is_selected {
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        name_spans.push(Span::styled(label, style));
        name_spans.push(Span::raw("│"));
    }

    // Value line: │ 0x05 │ 123  │...
    let mut value_spans = vec![Span::raw("  │")];
    for (i, (val, &w)) in field_values.iter().zip(&widths).enumerate() {
        let is_selected = Some(i) == selected_idx;
        let label = format!("{:^width$}", val, width = w);
        let style = if is_selected {
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        value_spans.push(Span::styled(label, style));
        value_spans.push(Span::raw("│"));
    }

    // Width & format indicator line: │ 3b [hex] │  5b [dec]  │...
    let mut width_spans = vec![Span::raw("  │")];
    for (i, (field, &w)) in layout.fields().iter().zip(&widths).enumerate() {
        let is_selected = Some(i) == selected_idx;
        let fmt_id = state.field_format(i);
        let label = format!("{:^width$}", format!("{}b [{}]", field.width(), fmt_id.as_str()), width = w);
        let style = if is_selected {
            Style::default().bg(Color::Yellow).fg(Color::Black)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        width_spans.push(Span::styled(label, style));
        width_spans.push(Span::raw("│"));
    }

    // Bottom border: └───┴───┴...┘
    let mut bottom = String::from("  └");
    for (i, &w) in widths.iter().enumerate() {
        for _ in 0..w {
            bottom.push('─');
        }
        bottom.push(if i < field_count - 1 { '┴' } else { '┘' });
    }

    let title_line = format!(
        " Layout [Global Format: {} (press 'F' to cycle all, 'f' to cycle field)] ",
        state.global_format.as_str()
    );

    let text = vec![
        Line::from(top),
        Line::from(name_spans),
        Line::from(value_spans),
        Line::from(width_spans),
        Line::from(bottom),
    ];

    let p = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(title_line))
        .scroll((0, state.scroll_x));
    frame.render_widget(p, area);
}

/// Draws the bit values line with field and byte boundary markers.
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
        let selected_idx = state.selected_field_index();

        for bit_idx in 0..layout_bits {
            // Separator before this bit.
            if bit_idx > 0 {
                let is_field_boundary = layout.fields().iter().any(|f| f.offset() == bit_idx);
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

            let style = if bit_idx == state.cursor_bit {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else if !available {
                Style::default().fg(Color::DarkGray)
            } else if selected_idx
                .and_then(|i| layout.field(i))
                .is_some_and(|f| f.contains(bit_idx))
            {
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
    let layout = &state.layout;
    let mut lines = Vec::new();

    if let Some(idx) = state.selected_field_index() {
        let field = layout.field(idx).unwrap();
        let current_fmt_id = state.field_format(idx);

        lines.push(Line::from(vec![
            Span::styled("  Field: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(field.name()),
            Span::raw(format!(
                " (Offset: {}, Width: {}b, Range: [{}, {}))",
                field.offset(),
                field.width(),
                field.offset(),
                field.end()
            )),
        ]));

        // Bit details.
        let cursor = state.cursor_bit;
        let relative = cursor - field.offset();
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
        let numeric_value = if field.width() <= 64 && field.end() <= state.data.len() * 8 {
            let mut reader = BitReader::from_bytes(&state.data);
            if reader.skip(field.offset()).is_ok() {
                reader.read_bits(field.width() as u32).ok()
            } else {
                None
            }
        } else {
            None
        };

        let ctx = FormatContext {
            data: &state.data,
            offset: field.offset(),
            width: field.width(),
            numeric_value,
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
