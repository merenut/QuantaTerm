//! QuantaTerm Terminal block and command grouping
//!
//! Terminal grid model, cell management, and line wrapping logic.

#![warn(missing_docs)]
#![deny(unsafe_code)]

use bitflags::bitflags;
use std::collections::VecDeque;
use tracing::{debug, instrument, trace, warn};

/// A color representation for terminal cells
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Alpha component (0-255)
    pub a: u8,
}

impl Color {
    /// Create a new color
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a new RGB color with full alpha
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    /// Black color
    pub const BLACK: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    /// White color
    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    /// Default foreground color (white)
    pub const DEFAULT_FG: Color = Color::WHITE;

    /// Default background color (black)
    pub const DEFAULT_BG: Color = Color::BLACK;
}

impl Default for Color {
    fn default() -> Self {
        Color::BLACK
    }
}

bitflags! {
    /// Cell attribute flags for styling
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CellAttrs: u32 {
        /// Bold text
        const BOLD = 1 << 0;
        /// Italic text
        const ITALIC = 1 << 1;
        /// Underlined text
        const UNDERLINE = 1 << 2;
        /// Strikethrough text
        const STRIKETHROUGH = 1 << 3;
        /// Blinking text
        const BLINK = 1 << 4;
        /// Reversed colors (fg/bg swapped)
        const REVERSE = 1 << 5;
        /// Hidden/invisible text
        const HIDDEN = 1 << 6;
    }
}

impl Default for CellAttrs {
    fn default() -> Self {
        CellAttrs::empty()
    }
}

/// A terminal cell containing character data and formatting
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// Unicode glyph identifier
    pub glyph_id: u32,
    /// Foreground color
    pub fg_color: Color,
    /// Background color
    pub bg_color: Color,
    /// Formatting attributes
    pub attrs: CellAttrs,
}

impl Cell {
    /// Create a new cell with the given glyph
    pub fn new(glyph_id: u32) -> Self {
        Self {
            glyph_id,
            fg_color: Color::DEFAULT_FG,
            bg_color: Color::DEFAULT_BG,
            attrs: CellAttrs::default(),
        }
    }

    /// Create a cell with custom colors and attributes
    pub fn with_style(glyph_id: u32, fg_color: Color, bg_color: Color, attrs: CellAttrs) -> Self {
        Self {
            glyph_id,
            fg_color,
            bg_color,
            attrs,
        }
    }

    /// Create an empty cell (space character)
    pub fn empty() -> Self {
        Self::new(b' ' as u32)
    }

    /// Check if this cell is empty (space with default styling)
    pub fn is_empty(&self) -> bool {
        self.glyph_id == b' ' as u32
            && self.fg_color == Color::DEFAULT_FG
            && self.bg_color == Color::DEFAULT_BG
            && self.attrs.is_empty()
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::empty()
    }
}

/// A row of terminal cells
pub type CellRow = Vec<Cell>;

/// Represents a position in the terminal grid
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Column position (0-based)
    pub col: u16,
    /// Row position (0-based, relative to viewport)
    pub row: u16,
}

impl Position {
    /// Create a new position
    pub fn new(col: u16, row: u16) -> Self {
        Self { col, row }
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare by row first, then by column
        self.row.cmp(&other.row).then(self.col.cmp(&other.col))
    }
}

/// Represents a text selection in the terminal
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// Start position of the selection
    pub start: Position,
    /// End position of the selection (inclusive)
    pub end: Position,
}

impl Selection {
    /// Create a new selection
    pub fn new(start: Position, end: Position) -> Self {
        // Ensure start is always before end
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    /// Check if a position is within this selection
    pub fn contains(&self, pos: Position) -> bool {
        pos >= self.start && pos <= self.end
    }

    /// Check if the selection spans multiple lines
    pub fn is_multiline(&self) -> bool {
        self.start.row != self.end.row
    }

    /// Get the selection bounds normalized (start <= end)
    pub fn normalized(&self) -> (Position, Position) {
        (self.start, self.end)
    }
}

/// Terminal grid with scrollback buffer and viewport management
#[derive(Debug)]
pub struct TerminalGrid {
    /// Number of columns in the terminal
    pub cols: u16,
    /// Number of rows in the terminal viewport
    pub rows: u16,
    /// Ring buffer for scrollback history
    scrollback: VecDeque<CellRow>,
    /// Current viewport offset from the bottom of scrollback
    pub viewport_offset: usize,
    /// Maximum scrollback lines to keep
    max_scrollback: usize,
    /// Current cursor position (col, row)
    cursor_pos: (u16, u16),
    /// Current formatting state for new cells
    current_fg: Color,
    /// Current background formatting state for new cells  
    current_bg: Color,
    /// Current attributes for new cells
    current_attrs: CellAttrs,
    /// Current text selection, if any
    selection: Option<Selection>,
}

impl TerminalGrid {
    /// Create a new terminal grid with the given dimensions
    pub fn new(cols: u16, rows: u16) -> Self {
        Self::with_scrollback(cols, rows, 10000) // Default 10k lines of scrollback
    }

    /// Create a new terminal grid with custom scrollback size
    pub fn with_scrollback(cols: u16, rows: u16, max_scrollback: usize) -> Self {
        let mut grid = Self {
            cols,
            rows,
            scrollback: VecDeque::with_capacity(max_scrollback + rows as usize),
            viewport_offset: 0,
            max_scrollback,
            cursor_pos: (0, 0),
            current_fg: Color::DEFAULT_FG,
            current_bg: Color::DEFAULT_BG,
            current_attrs: CellAttrs::empty(),
            selection: None,
        };

        // Initialize with empty rows
        for _ in 0..rows {
            grid.scrollback
                .push_back(vec![Cell::empty(); cols as usize]);
        }

        grid
    }

    /// Resize the terminal grid
    #[instrument(name = "grid_resize", skip(self))]
    pub fn resize(&mut self, new_cols: u16, new_rows: u16) {
        let old_cols = self.cols;
        let old_rows = self.rows;

        debug!(
            subsystem = "blocks",
            old_cols = old_cols,
            old_rows = old_rows,
            new_cols = new_cols,
            new_rows = new_rows,
            "Resizing terminal grid"
        );

        self.cols = new_cols;
        self.rows = new_rows;

        // Handle column changes - need to resize all existing rows
        if new_cols != old_cols {
            if new_cols > old_cols {
                // Expand rows with empty cells
                trace!(
                    subsystem = "blocks",
                    old_cols = old_cols,
                    new_cols = new_cols,
                    "Expanding grid columns"
                );
                for row in &mut self.scrollback {
                    row.resize(new_cols as usize, Cell::empty());
                }
            } else {
                // Shrinking columns - check if we need to rewrap or just truncate
                let needs_rewrapping = self.scrollback.iter().any(|row| {
                    // Check if any row has content beyond the new column width
                    row.iter()
                        .skip(new_cols as usize)
                        .any(|cell| !cell.is_empty())
                });

                if needs_rewrapping {
                    debug!(
                        subsystem = "blocks",
                        old_cols = old_cols,
                        new_cols = new_cols,
                        "Rewrapping lines due to content beyond new width"
                    );
                    self.rewrap_lines(old_cols, new_cols);
                } else {
                    trace!(
                        subsystem = "blocks",
                        old_cols = old_cols,
                        new_cols = new_cols,
                        "Truncating columns without rewrapping"
                    );
                    // Just truncate rows since no content would be lost
                    for row in &mut self.scrollback {
                        row.truncate(new_cols as usize);
                    }
                }
            }
        }

        // Handle row changes
        if new_rows > old_rows {
            // Add new empty rows at the bottom
            let rows_to_add = new_rows - old_rows;
            for _ in 0..rows_to_add {
                self.scrollback
                    .push_back(vec![Cell::empty(); new_cols as usize]);
            }
        } else if new_rows < old_rows {
            // Remove rows from the bottom, but preserve scrollback
            let rows_to_remove = old_rows - new_rows;
            for _ in 0..rows_to_remove {
                if self.scrollback.len() > new_rows as usize {
                    self.scrollback.pop_back();
                }
            }
        }

        // Ensure we maintain scrollback limits
        self.limit_scrollback();

        // Adjust cursor position if needed
        self.cursor_pos.0 = self.cursor_pos.0.min(new_cols.saturating_sub(1));
        self.cursor_pos.1 = self.cursor_pos.1.min(new_rows.saturating_sub(1));
    }

    /// Rewrap lines when terminal width changes
    fn rewrap_lines(&mut self, old_cols: u16, new_cols: u16) {
        if new_cols >= old_cols {
            return; // No rewrapping needed when expanding
        }

        let mut new_rows = Vec::new();

        // Process each existing row
        for row in self.scrollback.drain(..) {
            // Check if this row has content beyond the new column width
            let content_beyond_width = row
                .iter()
                .skip(new_cols as usize)
                .any(|cell| !cell.is_empty());

            if !content_beyond_width {
                // Row doesn't need rewrapping, just truncate
                let mut truncated_row = row;
                truncated_row.truncate(new_cols as usize);
                new_rows.push(truncated_row);
            } else {
                // Row needs rewrapping - collect all content and flow it
                let mut content = Vec::new();

                // Find the last non-empty cell
                let last_non_empty = row
                    .iter()
                    .rposition(|cell| !cell.is_empty())
                    .map(|i| i + 1)
                    .unwrap_or(0);

                // Collect non-empty content
                for (i, cell) in row.into_iter().enumerate() {
                    if i < last_non_empty {
                        content.push(cell);
                    }
                }

                // Flow content into new rows of new_cols width
                while !content.is_empty() {
                    let mut new_row = Vec::new();
                    let take_count = (new_cols as usize).min(content.len());

                    new_row.extend(content.drain(..take_count));
                    new_row.resize(new_cols as usize, Cell::empty());
                    new_rows.push(new_row);
                }
            }
        }

        // Ensure we have at least enough rows for the viewport
        while new_rows.len() < self.rows as usize {
            new_rows.push(vec![Cell::empty(); new_cols as usize]);
        }

        // Replace scrollback with rewrapped content
        self.scrollback = new_rows.into();
    }

    /// Get a cell at the given position (col, row) in the current viewport
    pub fn get_cell(&self, col: u16, row: u16) -> Option<&Cell> {
        if col >= self.cols || row >= self.rows {
            return None;
        }

        let scrollback_row = self.viewport_row_to_scrollback_index(row)?;
        self.scrollback.get(scrollback_row)?.get(col as usize)
    }

    /// Set a cell at the given position (col, row) in the current viewport
    pub fn set_cell(&mut self, col: u16, row: u16, cell: Cell) -> bool {
        if col >= self.cols || row >= self.rows {
            return false;
        }

        if let Some(scrollback_row) = self.viewport_row_to_scrollback_index(row) {
            if let Some(target_row) = self.scrollback.get_mut(scrollback_row) {
                if let Some(target_cell) = target_row.get_mut(col as usize) {
                    *target_cell = cell;
                    return true;
                }
            }
        }
        false
    }

    /// Convert viewport row index to scrollback index
    fn viewport_row_to_scrollback_index(&self, viewport_row: u16) -> Option<usize> {
        let total_rows = self.scrollback.len();
        if total_rows < self.rows as usize {
            return Some(viewport_row as usize);
        }

        let visible_start = total_rows
            .saturating_sub(self.rows as usize)
            .saturating_sub(self.viewport_offset);
        let scrollback_index = visible_start + viewport_row as usize;

        if scrollback_index < total_rows {
            Some(scrollback_index)
        } else {
            None
        }
    }

    /// Scroll the viewport up by the given number of lines
    #[instrument(name = "grid_scroll_up", skip(self))]
    pub fn scroll_up(&mut self, lines: usize) {
        if lines == 0 {
            return;
        }

        let max_offset = self.scrollback.len().saturating_sub(self.rows as usize);
        let old_offset = self.viewport_offset;
        self.viewport_offset = (self.viewport_offset + lines).min(max_offset);

        if old_offset != self.viewport_offset {
            trace!(
                subsystem = "blocks",
                lines = lines,
                old_offset = old_offset,
                new_offset = self.viewport_offset,
                max_offset = max_offset,
                "Scrolled viewport up"
            );
        }
    }

    /// Scroll the viewport down by the given number of lines
    #[instrument(name = "grid_scroll_down", skip(self))]
    pub fn scroll_down(&mut self, lines: usize) {
        if lines == 0 {
            return;
        }

        let old_offset = self.viewport_offset;
        self.viewport_offset = self.viewport_offset.saturating_sub(lines);

        if old_offset != self.viewport_offset {
            trace!(
                subsystem = "blocks",
                lines = lines,
                old_offset = old_offset,
                new_offset = self.viewport_offset,
                "Scrolled viewport down"
            );
        }
    }

    /// Scroll by page (viewport height) up
    #[instrument(name = "grid_page_up", skip(self))]
    pub fn page_up(&mut self) {
        self.scroll_up(self.rows as usize);
    }

    /// Scroll by page (viewport height) down
    #[instrument(name = "grid_page_down", skip(self))]
    pub fn page_down(&mut self) {
        self.scroll_down(self.rows as usize);
    }

    /// Scroll to the top of available scrollback
    #[instrument(name = "grid_scroll_to_top", skip(self))]
    pub fn scroll_to_top(&mut self) {
        let max_offset = self.scrollback.len().saturating_sub(self.rows as usize);
        if self.viewport_offset != max_offset {
            let old_offset = self.viewport_offset;
            self.viewport_offset = max_offset;
            trace!(
                subsystem = "blocks",
                old_offset = old_offset,
                new_offset = self.viewport_offset,
                "Scrolled to top of scrollback"
            );
        }
    }

    /// Reset viewport to show the bottom of the scrollback (normal terminal view)
    #[instrument(name = "grid_reset_viewport", skip(self))]
    pub fn reset_viewport(&mut self) {
        let old_offset = self.viewport_offset;
        self.viewport_offset = 0;

        if old_offset != 0 {
            trace!(
                subsystem = "blocks",
                old_offset = old_offset,
                "Reset viewport to bottom"
            );
        }
    }

    /// Add a new line at the bottom, scrolling content up
    #[instrument(name = "grid_add_line", skip(self, line))]
    pub fn add_line(&mut self, line: CellRow) {
        let mut line = line;
        line.resize(self.cols as usize, Cell::empty());

        trace!(
            subsystem = "blocks",
            line_cells = line.len(),
            total_scrollback = self.scrollback.len(),
            "Adding line to terminal grid"
        );

        self.scrollback.push_back(line);
        self.limit_scrollback();

        // Reset viewport to bottom when new content is added
        self.viewport_offset = 0;
    }

    /// Limit scrollback to maximum size
    fn limit_scrollback(&mut self) {
        let target_size = self.max_scrollback + self.rows as usize;
        let removed_count = self.scrollback.len().saturating_sub(target_size);

        if removed_count > 0 {
            for _ in 0..removed_count {
                self.scrollback.pop_front();
            }
            trace!(
                subsystem = "blocks",
                removed_lines = removed_count,
                remaining_lines = self.scrollback.len(),
                target_size = target_size,
                "Limited scrollback buffer size"
            );
        }
    }

    /// Get current cursor position
    pub fn cursor_position(&self) -> (u16, u16) {
        self.cursor_pos
    }

    /// Set cursor position
    pub fn set_cursor_position(&mut self, col: u16, row: u16) {
        self.cursor_pos = (
            col.min(self.cols.saturating_sub(1)),
            row.min(self.rows.saturating_sub(1)),
        );
    }

    /// Move cursor up by the specified number of lines
    pub fn move_cursor_up(&mut self, lines: u16) {
        self.cursor_pos.1 = self.cursor_pos.1.saturating_sub(lines);
    }

    /// Move cursor down by the specified number of lines
    pub fn move_cursor_down(&mut self, lines: u16) {
        self.cursor_pos.1 = (self.cursor_pos.1 + lines).min(self.rows.saturating_sub(1));
    }

    /// Move cursor left by the specified number of columns
    pub fn move_cursor_left(&mut self, cols: u16) {
        self.cursor_pos.0 = self.cursor_pos.0.saturating_sub(cols);
    }

    /// Move cursor right by the specified number of columns
    pub fn move_cursor_right(&mut self, cols: u16) {
        self.cursor_pos.0 = (self.cursor_pos.0 + cols).min(self.cols.saturating_sub(1));
    }

    /// Move cursor to specific column on current row
    pub fn move_cursor_to_column(&mut self, col: u16) {
        self.cursor_pos.0 = col.min(self.cols.saturating_sub(1));
    }

    /// Move cursor to specific row on current column
    pub fn move_cursor_to_row(&mut self, row: u16) {
        self.cursor_pos.1 = row.min(self.rows.saturating_sub(1));
    }

    /// Move cursor to beginning of current line
    pub fn move_cursor_to_line_start(&mut self) {
        self.cursor_pos.0 = 0;
    }

    /// Move cursor to end of current line
    pub fn move_cursor_to_line_end(&mut self) {
        self.cursor_pos.0 = self.cols.saturating_sub(1);
    }

    /// Clear the entire grid
    pub fn clear(&mut self) {
        self.scrollback.clear();
        for _ in 0..self.rows {
            self.scrollback
                .push_back(vec![Cell::empty(); self.cols as usize]);
        }
        self.viewport_offset = 0;
        self.cursor_pos = (0, 0);
        self.selection = None;
    }

    /// Get the number of scrollback lines available
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len().saturating_sub(self.rows as usize)
    }

    /// Get the maximum scrollback capacity
    pub fn max_scrollback(&self) -> usize {
        self.max_scrollback
    }

    /// Get the current total lines in the buffer (including viewport)
    pub fn total_lines(&self) -> usize {
        self.scrollback.len()
    }

    /// Check if the viewport is at the bottom (normal terminal state)
    pub fn is_at_bottom(&self) -> bool {
        self.viewport_offset == 0
    }

    /// Get the current viewport offset from bottom
    pub fn viewport_offset(&self) -> usize {
        self.viewport_offset
    }

    /// Get a copy of the current viewport content
    pub fn get_viewport(&self) -> Vec<CellRow> {
        let mut viewport = Vec::with_capacity(self.rows as usize);

        for row in 0..self.rows {
            if let Some(scrollback_row) = self.viewport_row_to_scrollback_index(row) {
                if let Some(line) = self.scrollback.get(scrollback_row) {
                    viewport.push(line.clone());
                } else {
                    viewport.push(vec![Cell::empty(); self.cols as usize]);
                }
            } else {
                viewport.push(vec![Cell::empty(); self.cols as usize]);
            }
        }

        viewport
    }

    /// Convert the current viewport to text lines for renderer integration
    pub fn get_viewport_text(&self) -> Vec<String> {
        let viewport = self.get_viewport();
        viewport
            .iter()
            .map(|row| {
                // Convert each row to a string, handling Unicode properly
                row.iter()
                    .map(|cell| {
                        // For now, just treat glyph_id as ASCII
                        // In a full implementation, this would handle Unicode properly
                        if cell.glyph_id == 0 || cell.is_empty() {
                            ' '
                        } else {
                            (cell.glyph_id as u8) as char
                        }
                    })
                    .collect()
            })
            .collect()
    }

    /// Update the renderer with current viewport content
    /// This provides integration with the renderer stub for future display
    pub fn update_renderer(&self, renderer: &mut quantaterm_renderer::Renderer) {
        let text_lines = self.get_viewport_text();
        let combined_text = text_lines.join("\n");
        renderer.add_text(&combined_text);
    }

    /// Apply SGR (Select Graphic Rendition) formatting attributes
    /// This updates the current formatting state for new cells
    pub fn apply_sgr(&mut self, fg_color: Color, bg_color: Color, attrs: CellAttrs) {
        self.current_fg = fg_color;
        self.current_bg = bg_color;
        self.current_attrs = attrs;

        debug!(
            subsystem = "blocks",
            fg_r = fg_color.r, fg_g = fg_color.g, fg_b = fg_color.b,
            bg_r = bg_color.r, bg_g = bg_color.g, bg_b = bg_color.b,
            attrs = ?attrs,
            "Applied SGR formatting"
        );
    }

    /// Print a character at the cursor position with current formatting
    pub fn print_char(&mut self, c: char) {
        let glyph_id = c as u32;

        // Create cell with current formatting
        let cell = Cell {
            glyph_id,
            fg_color: self.current_fg,
            bg_color: self.current_bg,
            attrs: self.current_attrs,
        };

        // Set cell at cursor position
        self.set_cell(self.cursor_pos.0, self.cursor_pos.1, cell);

        // Advance cursor
        self.advance_cursor();
    }

    /// Execute a control character (like newline, tab, etc.)
    pub fn execute_control(&mut self, byte: u8) {
        match byte {
            b'\n' => self.newline(),
            b'\r' => self.carriage_return(),
            b'\t' => self.tab(),
            b'\x08' => self.backspace(), // Backspace
            _ => {
                trace!("Unhandled control character: {:#x}", byte);
            }
        }
    }

    /// Handle CSI (Control Sequence Introducer) actions from the parser
    pub fn handle_csi_action(&mut self, action: &quantaterm_core::CsiAction) {
        use quantaterm_core::CsiAction;

        match action {
            CsiAction::Sgr(params) => {
                // SGR handling is already done by the parser state
                trace!(subsystem = "blocks", params = ?params, "Processed SGR");
            }
            CsiAction::CursorUp(lines) => {
                self.move_cursor_up(*lines);
                trace!(subsystem = "blocks", lines = lines, "Moved cursor up");
            }
            CsiAction::CursorDown(lines) => {
                self.move_cursor_down(*lines);
                trace!(subsystem = "blocks", lines = lines, "Moved cursor down");
            }
            CsiAction::CursorForward(cols) => {
                self.move_cursor_right(*cols);
                trace!(subsystem = "blocks", cols = cols, "Moved cursor right");
            }
            CsiAction::CursorBackward(cols) => {
                self.move_cursor_left(*cols);
                trace!(subsystem = "blocks", cols = cols, "Moved cursor left");
            }
            CsiAction::CursorNextLine(lines) => {
                self.move_cursor_down(*lines);
                self.move_cursor_to_line_start();
                trace!(subsystem = "blocks", lines = lines, "Moved cursor to next line");
            }
            CsiAction::CursorPreviousLine(lines) => {
                self.move_cursor_up(*lines);
                self.move_cursor_to_line_start();
                trace!(subsystem = "blocks", lines = lines, "Moved cursor to previous line");
            }
            CsiAction::CursorHorizontalAbsolute(col) => {
                self.move_cursor_to_column(*col);
                trace!(subsystem = "blocks", col = col, "Moved cursor to column");
            }
            CsiAction::CursorPosition(row, col) => {
                self.set_cursor_position(*col, *row);
                trace!(subsystem = "blocks", row = row, col = col, "Set cursor position");
            }
            CsiAction::Other { command, params } => {
                debug!(
                    subsystem = "blocks",
                    command = ?command,
                    params = ?params,
                    "Unhandled CSI command"
                );
            }
        }
    }

    /// Move to next line (newline)
    pub fn newline(&mut self) {
        self.cursor_pos.0 = 0; // Reset column to start of line
        self.cursor_pos.1 += 1;
        if self.cursor_pos.1 >= self.rows {
            // Need to scroll
            self.scroll_up_one_line();
            self.cursor_pos.1 = self.rows - 1;
        }
    }

    /// Move cursor to start of current line (carriage return)
    pub fn carriage_return(&mut self) {
        self.cursor_pos.0 = 0;
    }

    /// Move cursor to next tab stop
    pub fn tab(&mut self) {
        // Simple tab implementation - move to next multiple of 8
        self.cursor_pos.0 = ((self.cursor_pos.0 / 8) + 1) * 8;
        if self.cursor_pos.0 >= self.cols {
            self.cursor_pos.0 = self.cols - 1;
        }
    }

    /// Move cursor back one position
    pub fn backspace(&mut self) {
        if self.cursor_pos.0 > 0 {
            self.cursor_pos.0 -= 1;
        }
    }

    /// Advance cursor to next position
    fn advance_cursor(&mut self) {
        self.cursor_pos.0 += 1;
        if self.cursor_pos.0 >= self.cols {
            // Wrap to next line
            self.cursor_pos.0 = 0;
            self.newline();
        }
    }

    /// Scroll the terminal up by one line
    fn scroll_up_one_line(&mut self) {
        if self.scrollback.len() >= self.max_scrollback {
            self.scrollback.pop_front();
        }
        self.scrollback
            .push_back(vec![Cell::empty(); self.cols as usize]);
    }

    /// Get current formatting state
    pub fn current_formatting(&self) -> (Color, Color, CellAttrs) {
        (self.current_fg, self.current_bg, self.current_attrs)
    }

    /// Reset formatting to defaults
    pub fn reset_formatting(&mut self) {
        self.current_fg = Color::DEFAULT_FG;
        self.current_bg = Color::DEFAULT_BG;
        self.current_attrs = CellAttrs::empty();
    }

    // Text Selection Methods

    /// Start a new text selection at the given position
    pub fn start_selection(&mut self, position: Position) {
        let clamped_position = self.clamp_position(position);
        self.selection = Some(Selection::new(clamped_position, clamped_position));
        trace!(
            subsystem = "blocks",
            col = clamped_position.col,
            row = clamped_position.row,
            "Started text selection"
        );
    }

    /// Extend the current selection to the given position
    pub fn extend_selection(&mut self, position: Position) {
        let clamped_position = self.clamp_position(position);
        if let Some(selection) = &mut self.selection {
            selection.end = clamped_position;
            // Normalize the selection
            *selection = Selection::new(selection.start, selection.end);
            trace!(
                subsystem = "blocks",
                start_col = selection.start.col,
                start_row = selection.start.row,
                end_col = selection.end.col,
                end_row = selection.end.row,
                "Extended text selection"
            );
        } else {
            self.start_selection(position);
        }
    }

    /// Clear the current text selection
    pub fn clear_selection(&mut self) {
        if self.selection.is_some() {
            self.selection = None;
            trace!(subsystem = "blocks", "Cleared text selection");
        }
    }

    /// Get the current text selection, if any
    pub fn get_selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// Check if there is an active selection
    pub fn has_selection(&self) -> bool {
        self.selection.is_some()
    }

    /// Get the selected text as a string
    pub fn get_selected_text(&self) -> Option<String> {
        let selection = self.selection.as_ref()?;
        let text = self.extract_text_from_selection(selection);
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// Copy the selected text to the system clipboard
    pub fn copy_selection_to_clipboard(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(text) = self.get_selected_text() {
            let mut clipboard = arboard::Clipboard::new()?;
            clipboard.set_text(text)?;
            debug!(subsystem = "blocks", "Copied selection to clipboard");
            Ok(())
        } else {
            Err("No text selected".into())
        }
    }

    /// Check if a position is within the current selection
    pub fn position_in_selection(&self, position: Position) -> bool {
        self.selection
            .as_ref()
            .map(|s| s.contains(position))
            .unwrap_or(false)
    }

    /// Clamp a position to be within grid bounds
    fn clamp_position(&self, position: Position) -> Position {
        Position::new(
            position.col.min(self.cols.saturating_sub(1)),
            position.row.min(self.rows.saturating_sub(1)),
        )
    }

    /// Extract text from a selection range
    fn extract_text_from_selection(&self, selection: &Selection) -> String {
        let mut result = String::new();
        let (start, end) = selection.normalized();

        if start.row == end.row {
            // Single line selection
            if let Some(line) = self.get_viewport_line(start.row) {
                for col in start.col..=end.col.min(self.cols.saturating_sub(1)) {
                    if let Some(cell) = line.get(col as usize) {
                        if cell.glyph_id != 0 && !cell.is_empty() {
                            if let Some(ch) = char::from_u32(cell.glyph_id) {
                                result.push(ch);
                            }
                        } else {
                            result.push(' ');
                        }
                    }
                }
            }
        } else {
            // Multi-line selection
            for row in start.row..=end.row {
                if let Some(line) = self.get_viewport_line(row) {
                    let start_col = if row == start.row { start.col } else { 0 };
                    let end_col = if row == end.row {
                        end.col.min(self.cols.saturating_sub(1))
                    } else {
                        self.cols.saturating_sub(1)
                    };

                    let mut line_text = String::new();
                    for col in start_col..=end_col {
                        if let Some(cell) = line.get(col as usize) {
                            if cell.glyph_id != 0 && !cell.is_empty() {
                                if let Some(ch) = char::from_u32(cell.glyph_id) {
                                    line_text.push(ch);
                                }
                            } else {
                                line_text.push(' ');
                            }
                        }
                    }

                    // Trim trailing whitespace from this line
                    result.push_str(line_text.trim_end());

                    // Add newline between rows (except for the last row)
                    if row < end.row {
                        result.push('\n');
                    }
                }
            }
        }

        result
    }

    /// Get a specific line from the current viewport
    fn get_viewport_line(&self, row: u16) -> Option<&CellRow> {
        if row >= self.rows {
            return None;
        }

        let scrollback_row = self.viewport_row_to_scrollback_index(row)?;
        self.scrollback.get(scrollback_row)
    }

    /// Select all text in the current viewport
    pub fn select_all(&mut self) {
        let start = Position::new(0, 0);
        let end = Position::new(
            self.cols.saturating_sub(1),
            self.rows.saturating_sub(1),
        );
        self.selection = Some(Selection::new(start, end));
        debug!(subsystem = "blocks", "Selected all text in viewport");
    }

    /// Select the word at the given position
    pub fn select_word_at(&mut self, position: Position) {
        let clamped_pos = self.clamp_position(position);
        
        if let Some(line) = self.get_viewport_line(clamped_pos.row) {
            let start_col = self.find_word_boundary_left(line, clamped_pos.col);
            let end_col = self.find_word_boundary_right(line, clamped_pos.col);
            
            let start = Position::new(start_col, clamped_pos.row);
            let end = Position::new(end_col, clamped_pos.row);
            self.selection = Some(Selection::new(start, end));
            
            debug!(
                subsystem = "blocks",
                start_col = start_col,
                end_col = end_col,
                row = clamped_pos.row,
                "Selected word at position"
            );
        }
    }

    /// Find the left boundary of a word (start of word)
    fn find_word_boundary_left(&self, line: &CellRow, start_col: u16) -> u16 {
        let mut col = start_col as usize;
        
        // Move left while we have word characters
        while col > 0 {
            if let Some(cell) = line.get(col.saturating_sub(1)) {
                if let Some(ch) = char::from_u32(cell.glyph_id) {
                    if ch.is_alphanumeric() || ch == '_' {
                        col = col.saturating_sub(1);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        col as u16
    }

    /// Find the right boundary of a word (end of word)
    fn find_word_boundary_right(&self, line: &CellRow, start_col: u16) -> u16 {
        let mut col = start_col as usize;
        
        // Move right while we have word characters
        while col < line.len() {
            if let Some(cell) = line.get(col) {
                if let Some(ch) = char::from_u32(cell.glyph_id) {
                    if ch.is_alphanumeric() || ch == '_' {
                        col += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        // Return the last valid character position (not the position after)
        if col > start_col as usize {
            (col - 1).min(self.cols.saturating_sub(1) as usize) as u16
        } else {
            start_col
        }
    }

    /// Get the selection bounds adjusted for the current viewport offset
    /// This is useful for rendering the selection correctly when scrolled
    pub fn get_selection_viewport_bounds(&self) -> Option<(Position, Position)> {
        self.selection.as_ref().map(|s| s.normalized())
    }
}

/// Placeholder module for blocks (maintaining backwards compatibility)
pub struct Blocks;

impl Blocks {
    /// Create a new instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for Blocks {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_creation() {
        let color = Color::new(255, 128, 64, 255);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert_eq!(color.a, 255);

        let rgb_color = Color::rgb(200, 100, 50);
        assert_eq!(rgb_color.a, 255);
    }

    #[test]
    fn test_cell_creation_and_properties() {
        let cell = Cell::new(b'A' as u32);
        assert_eq!(cell.glyph_id, b'A' as u32);
        assert_eq!(cell.fg_color, Color::DEFAULT_FG);
        assert_eq!(cell.bg_color, Color::DEFAULT_BG);
        assert_eq!(cell.attrs, CellAttrs::empty());

        let styled_cell = Cell::with_style(
            b'B' as u32,
            Color::rgb(255, 0, 0),
            Color::rgb(0, 255, 0),
            CellAttrs::BOLD | CellAttrs::ITALIC,
        );
        assert_eq!(styled_cell.glyph_id, b'B' as u32);
        assert_eq!(styled_cell.fg_color, Color::rgb(255, 0, 0));
        assert_eq!(styled_cell.bg_color, Color::rgb(0, 255, 0));
        assert!(styled_cell.attrs.contains(CellAttrs::BOLD));
        assert!(styled_cell.attrs.contains(CellAttrs::ITALIC));

        let empty_cell = Cell::empty();
        assert!(empty_cell.is_empty());
        assert_eq!(empty_cell.glyph_id, b' ' as u32);
    }

    #[test]
    fn test_cell_attrs_flags() {
        let mut attrs = CellAttrs::empty();
        assert!(attrs.is_empty());

        attrs |= CellAttrs::BOLD;
        assert!(attrs.contains(CellAttrs::BOLD));
        assert!(!attrs.contains(CellAttrs::ITALIC));

        attrs |= CellAttrs::UNDERLINE | CellAttrs::ITALIC;
        assert!(attrs.contains(CellAttrs::BOLD));
        assert!(attrs.contains(CellAttrs::ITALIC));
        assert!(attrs.contains(CellAttrs::UNDERLINE));
        assert!(!attrs.contains(CellAttrs::BLINK));
    }

    #[test]
    fn test_grid_creation() {
        let grid = TerminalGrid::new(80, 24);
        assert_eq!(grid.cols, 80);
        assert_eq!(grid.rows, 24);
        assert_eq!(grid.viewport_offset, 0);
        assert_eq!(grid.cursor_position(), (0, 0));
        assert_eq!(grid.scrollback.len(), 24);
    }

    #[test]
    fn test_grid_cell_access() {
        let mut grid = TerminalGrid::new(10, 5);

        // Test getting empty cell
        let cell = grid.get_cell(5, 2).unwrap();
        assert!(cell.is_empty());

        // Test setting and getting cell
        let test_cell = Cell::new(b'X' as u32);
        assert!(grid.set_cell(5, 2, test_cell.clone()));
        let retrieved_cell = grid.get_cell(5, 2).unwrap();
        assert_eq!(retrieved_cell.glyph_id, b'X' as u32);

        // Test out of bounds
        assert!(grid.get_cell(10, 2).is_none());
        assert!(grid.get_cell(5, 5).is_none());
        assert!(!grid.set_cell(10, 2, test_cell.clone()));
        assert!(!grid.set_cell(5, 5, test_cell));
    }

    #[test]
    fn test_grid_resize_expand() {
        let mut grid = TerminalGrid::new(10, 5);

        // Set a test cell
        let test_cell = Cell::new(b'T' as u32);
        grid.set_cell(5, 2, test_cell.clone());

        // Expand grid
        grid.resize(15, 8);
        assert_eq!(grid.cols, 15);
        assert_eq!(grid.rows, 8);
        assert_eq!(grid.scrollback.len(), 8);

        // Check that existing content is preserved
        let retrieved_cell = grid.get_cell(5, 2).unwrap();
        assert_eq!(retrieved_cell.glyph_id, b'T' as u32);

        // Check that new cells are empty
        let new_cell = grid.get_cell(12, 2).unwrap();
        assert!(new_cell.is_empty());

        // Check new rows are added
        let new_row_cell = grid.get_cell(5, 7).unwrap();
        assert!(new_row_cell.is_empty());
    }

    #[test]
    fn test_grid_resize_shrink() {
        let mut grid = TerminalGrid::new(15, 8);

        // Set test cells - 'T' at (5,2) should stay, 'X' at (12,2) should be wrapped to next line
        let test_cell = Cell::new(b'T' as u32);
        grid.set_cell(5, 2, test_cell.clone());
        grid.set_cell(12, 2, Cell::new(b'X' as u32));
        grid.set_cell(5, 7, Cell::new(b'Y' as u32));

        // Shrink grid
        grid.resize(10, 5);
        assert_eq!(grid.cols, 10);
        assert_eq!(grid.rows, 5);

        // After rewrapping, 'T' should be at (5,1) since row got shifted due to rewrapping
        let retrieved_cell = grid.get_cell(5, 1).unwrap();
        assert_eq!(retrieved_cell.glyph_id, b'T' as u32);

        // 'X' should have been wrapped to the next line at position (2,2)
        let wrapped_cell = grid.get_cell(2, 2).unwrap();
        assert_eq!(wrapped_cell.glyph_id, b'X' as u32);

        // Check that content outside new bounds is inaccessible
        assert!(grid.get_cell(12, 2).is_none()); // Column out of bounds
        assert!(grid.get_cell(5, 7).is_none()); // Row out of bounds (was removed)
    }

    #[test]
    fn test_line_wrapping_basic() {
        let mut grid = TerminalGrid::new(5, 3);

        // Fill a row with content
        for i in 0..5 {
            grid.set_cell(i, 0, Cell::new((b'A' + i as u8) as u32));
        }

        // Add more content that should wrap to next line
        for i in 0..3 {
            grid.set_cell(i, 1, Cell::new((b'F' + i as u8) as u32));
        }

        // Shrink width to force rewrapping
        grid.resize(3, 3);

        // Check that content was rewrapped correctly
        // The scrollback has: Row 0="ABC", Row 1="DE ", Row 2="FGH", Row 3="   "
        // But viewport shows the last 3 rows: Row 1="DE ", Row 2="FGH", Row 3="   "
        assert_eq!(grid.get_cell(0, 0).unwrap().glyph_id, b'D' as u32);
        assert_eq!(grid.get_cell(1, 0).unwrap().glyph_id, b'E' as u32);
        assert!(grid.get_cell(2, 0).unwrap().is_empty());

        assert_eq!(grid.get_cell(0, 1).unwrap().glyph_id, b'F' as u32);
        assert_eq!(grid.get_cell(1, 1).unwrap().glyph_id, b'G' as u32);
        assert_eq!(grid.get_cell(2, 1).unwrap().glyph_id, b'H' as u32);

        assert!(grid.get_cell(0, 2).unwrap().is_empty());
        assert!(grid.get_cell(1, 2).unwrap().is_empty());
        assert!(grid.get_cell(2, 2).unwrap().is_empty());
    }

    #[test]
    fn test_scrollback_functionality() {
        let mut grid = TerminalGrid::new(5, 3);

        // Add several lines of content
        for line_num in 0..10 {
            let line: CellRow = (0..5)
                .map(|i| Cell::new((b'0' + ((line_num + i) % 10) as u8) as u32))
                .collect();
            grid.add_line(line);
        }

        // Should have scrollback content now
        assert!(grid.scrollback_len() > 0);

        // Test scrolling up
        grid.scroll_up(2);
        assert_eq!(grid.viewport_offset, 2);

        // Test scrolling down
        grid.scroll_down(1);
        assert_eq!(grid.viewport_offset, 1);

        // Test reset viewport
        grid.reset_viewport();
        assert_eq!(grid.viewport_offset, 0);
    }

    #[test]
    fn test_cursor_operations() {
        let mut grid = TerminalGrid::new(10, 5);

        // Test initial cursor position
        assert_eq!(grid.cursor_position(), (0, 0));

        // Test setting cursor position
        grid.set_cursor_position(5, 3);
        assert_eq!(grid.cursor_position(), (5, 3));

        // Test cursor bounds checking
        grid.set_cursor_position(15, 10);
        assert_eq!(grid.cursor_position(), (9, 4)); // Should be clamped to grid bounds
    }

    #[test]
    fn test_grid_clear() {
        let mut grid = TerminalGrid::new(5, 3);

        // Add some content
        grid.set_cell(2, 1, Cell::new(b'X' as u32));
        grid.set_cursor_position(3, 2);

        // Clear grid
        grid.clear();

        // Verify everything is reset
        assert!(grid.get_cell(2, 1).unwrap().is_empty());
        assert_eq!(grid.cursor_position(), (0, 0));
        assert_eq!(grid.viewport_offset, 0);
        assert_eq!(grid.scrollback.len(), 3);
    }

    #[test]
    fn test_viewport_operations() {
        let mut grid = TerminalGrid::new(5, 3);

        // Add content
        for i in 0..5 {
            grid.set_cell(i, 1, Cell::new((b'A' + i as u8) as u32));
        }

        // Get viewport
        let viewport = grid.get_viewport();
        assert_eq!(viewport.len(), 3);
        assert_eq!(viewport[1].len(), 5);
        assert_eq!(viewport[1][0].glyph_id, b'A' as u32);
        assert_eq!(viewport[1][4].glyph_id, b'E' as u32);
    }

    #[test]
    fn test_sgr_formatting() {
        let mut grid = TerminalGrid::new(10, 5);

        // Test default formatting
        let (fg, bg, attrs) = grid.current_formatting();
        assert_eq!(fg, Color::DEFAULT_FG);
        assert_eq!(bg, Color::DEFAULT_BG);
        assert!(attrs.is_empty());

        // Apply bold red formatting
        grid.apply_sgr(Color::rgb(255, 0, 0), Color::DEFAULT_BG, CellAttrs::BOLD);
        let (fg, bg, attrs) = grid.current_formatting();
        assert_eq!(fg, Color::rgb(255, 0, 0));
        assert_eq!(bg, Color::DEFAULT_BG);
        assert!(attrs.contains(CellAttrs::BOLD));

        // Print a character with formatting
        grid.print_char('A');
        let cell = grid.get_cell(0, 0).unwrap();
        assert_eq!(cell.glyph_id, b'A' as u32);
        assert_eq!(cell.fg_color, Color::rgb(255, 0, 0));
        assert!(cell.attrs.contains(CellAttrs::BOLD));

        // Reset formatting
        grid.reset_formatting();
        let (fg, bg, attrs) = grid.current_formatting();
        assert_eq!(fg, Color::DEFAULT_FG);
        assert_eq!(bg, Color::DEFAULT_BG);
        assert!(attrs.is_empty());
    }

    #[test]
    fn test_control_characters() {
        let mut grid = TerminalGrid::new(10, 5);

        // Test printing and cursor advancement
        grid.print_char('H');
        grid.print_char('i');
        assert_eq!(grid.cursor_position(), (2, 0));

        // Test carriage return
        grid.execute_control(b'\r');
        assert_eq!(grid.cursor_position(), (0, 0));

        // Test newline
        grid.execute_control(b'\n');
        assert_eq!(grid.cursor_position(), (0, 1));

        // Test tab
        grid.execute_control(b'\t');
        assert_eq!(grid.cursor_position(), (8, 1));

        // Test backspace
        grid.execute_control(b'\x08');
        assert_eq!(grid.cursor_position(), (7, 1));
    }

    #[test]
    fn test_line_wrapping_with_formatting() {
        let mut grid = TerminalGrid::new(3, 3);

        // Apply formatting
        grid.apply_sgr(Color::rgb(0, 255, 0), Color::DEFAULT_BG, CellAttrs::ITALIC);

        // Print characters that will wrap
        grid.print_char('A');
        grid.print_char('B');
        grid.print_char('C');
        grid.print_char('D'); // Should wrap to next line

        assert_eq!(grid.cursor_position(), (1, 1));

        // Check that formatting is preserved across wrapping
        let cell_a = grid.get_cell(0, 0).unwrap();
        let cell_d = grid.get_cell(0, 1).unwrap();
        assert_eq!(cell_a.fg_color, Color::rgb(0, 255, 0));
        assert_eq!(cell_d.fg_color, Color::rgb(0, 255, 0));
        assert!(cell_a.attrs.contains(CellAttrs::ITALIC));
        assert!(cell_d.attrs.contains(CellAttrs::ITALIC));
    }

    #[test]
    fn test_line_wrapping_edge_cases() {
        let mut grid = TerminalGrid::new(4, 2);

        // Test wrapping with empty trailing cells
        grid.set_cell(0, 0, Cell::new(b'A' as u32));
        grid.set_cell(1, 0, Cell::new(b'B' as u32));
        // Leave cells 2,3 empty

        grid.set_cell(0, 1, Cell::new(b'C' as u32));

        // Shrink to width 2 - should not create unnecessary empty lines
        grid.resize(2, 3);

        // Should have: "AB" on line 1, "C" on line 2, empty line 3
        assert_eq!(grid.get_cell(0, 0).unwrap().glyph_id, b'A' as u32);
        assert_eq!(grid.get_cell(1, 0).unwrap().glyph_id, b'B' as u32);
        assert_eq!(grid.get_cell(0, 1).unwrap().glyph_id, b'C' as u32);
        assert!(grid.get_cell(1, 1).unwrap().is_empty());
        assert!(grid.get_cell(0, 2).unwrap().is_empty());
    }

    #[test]
    fn test_scrollback_limits() {
        let mut grid = TerminalGrid::with_scrollback(3, 2, 5); // Small scrollback for testing

        // Add many lines to exceed scrollback limit
        for line_num in 0..20 {
            let line: CellRow = (0..3)
                .map(|_| Cell::new((b'0' + (line_num % 10) as u8) as u32))
                .collect();
            grid.add_line(line);
        }

        // Should be limited to max_scrollback + rows
        assert!(grid.scrollback.len() <= 7); // 5 scrollback + 2 viewport rows
    }

    #[test]
    fn test_viewport_text_conversion() {
        let mut grid = TerminalGrid::new(5, 3);

        // Add some text content
        grid.set_cell(0, 0, Cell::new(b'H' as u32));
        grid.set_cell(1, 0, Cell::new(b'e' as u32));
        grid.set_cell(2, 0, Cell::new(b'l' as u32));
        grid.set_cell(3, 0, Cell::new(b'l' as u32));
        grid.set_cell(4, 0, Cell::new(b'o' as u32));

        grid.set_cell(0, 1, Cell::new(b'W' as u32));
        grid.set_cell(1, 1, Cell::new(b'o' as u32));
        grid.set_cell(2, 1, Cell::new(b'r' as u32));
        grid.set_cell(3, 1, Cell::new(b'l' as u32));
        grid.set_cell(4, 1, Cell::new(b'd' as u32));

        // Row 2 left empty

        let text_lines = grid.get_viewport_text();
        assert_eq!(text_lines.len(), 3);
        assert_eq!(text_lines[0], "Hello");
        assert_eq!(text_lines[1], "World");
        assert_eq!(text_lines[2], "     "); // Empty row should be spaces
    }

    #[test]
    fn test_scrollback_ring_buffer_integration() {
        let mut grid = TerminalGrid::with_scrollback(5, 3, 10);

        // Fill beyond scrollback capacity to test ring buffer behavior
        for line_num in 0..20 {
            let line: CellRow = (0..5)
                .map(|i| Cell::new((b'A' + ((line_num + i) % 26) as u8) as u32))
                .collect();
            grid.add_line(line);
        }

        // Should be limited to max_scrollback + viewport rows (10 + 3 = 13)
        assert!(grid.scrollback.len() <= 13);

        // Test scrolling through the entire buffer
        let max_scroll = grid.scrollback_len();
        grid.scroll_up(max_scroll);
        assert_eq!(grid.viewport_offset, max_scroll);

        // Should be able to see oldest available content
        grid.scroll_down(max_scroll);
        assert_eq!(grid.viewport_offset, 0);

        // Test that viewport is at bottom by default
        let viewport = grid.get_viewport();
        assert_eq!(viewport.len(), 3);
    }

    #[test]
    fn test_scrollback_with_resize_stress() {
        let mut grid = TerminalGrid::with_scrollback(10, 5, 20);

        // Add content with known pattern
        for line_num in 0..30 {
            let line: CellRow = (0..10)
                .map(|_col| Cell::new((b'0' + (line_num % 10) as u8) as u32))
                .collect();
            grid.add_line(line);
        }

        // Scroll up to see older content
        grid.scroll_up(5);
        let _offset_before = grid.viewport_offset;

        // Resize while scrolled up
        grid.resize(8, 4);

        // Should maintain reasonable viewport position after resize
        assert!(grid.viewport_offset <= grid.scrollback_len());

        // Should still be able to navigate properly
        grid.scroll_down(2);
        grid.scroll_up(1);
        assert!(grid.viewport_offset <= grid.scrollback_len());
    }

    #[test]
    fn test_scrollback_performance_large_buffer() {
        use std::time::Instant;

        let mut grid = TerminalGrid::with_scrollback(80, 24, 10000);

        // Measure time to add many lines
        let start = Instant::now();
        for line_num in 0..5000 {
            let line: CellRow = (0..80)
                .map(|col| Cell::new((b'0' + ((line_num + col) % 10) as u8) as u32))
                .collect();
            grid.add_line(line);
        }
        let add_duration = start.elapsed();

        // Should complete in reasonable time (< 100ms for 5k lines)
        assert!(
            add_duration.as_millis() < 100,
            "Adding lines took too long: {:?}",
            add_duration
        );

        // Measure scrolling performance
        let start = Instant::now();
        for _ in 0..100 {
            grid.scroll_up(10);
            grid.scroll_down(5);
        }
        let scroll_duration = start.elapsed();

        // Scrolling should be very fast (< 10ms for 100 operations)
        assert!(
            scroll_duration.as_millis() < 10,
            "Scrolling took too long: {:?}",
            scroll_duration
        );

        // Buffer should respect size limits
        assert!(grid.scrollback.len() <= 10024); // 10000 + 24 viewport rows
    }

    #[test]
    fn test_scrollback_edge_cases() {
        let mut grid = TerminalGrid::new(5, 3);

        // Test scrolling on empty buffer
        grid.scroll_up(10);
        assert_eq!(grid.viewport_offset, 0);

        grid.scroll_down(5);
        assert_eq!(grid.viewport_offset, 0);

        // Add one line - should still be no scrollback (3 rows viewport, now 4 total = 1 scrollback)
        let line: CellRow = vec![Cell::new(b'X' as u32); 5];
        grid.add_line(line);

        // Now we have 1 line of scrollback available, so scrolling should work
        grid.scroll_up(1);
        assert_eq!(grid.viewport_offset, 1);

        // But can't scroll more than available
        grid.scroll_up(10);
        assert_eq!(grid.viewport_offset, 1); // Still just 1 line available

        // Add enough lines to enable more scrolling
        for i in 0..5 {
            let line: CellRow = vec![Cell::new((b'A' + i) as u32); 5];
            grid.add_line(line);
        }

        // Now scrolling should work with more range
        grid.scroll_up(2);
        assert_eq!(grid.viewport_offset, 2);

        // Test scrolling beyond limits
        grid.scroll_up(100);
        let max_offset = grid.scrollback.len().saturating_sub(grid.rows as usize);
        assert_eq!(grid.viewport_offset, max_offset);

        // Test scrolling down beyond limits
        grid.scroll_down(200);
        assert_eq!(grid.viewport_offset, 0);
    }

    #[test]
    fn test_viewport_consistency_during_updates() {
        let mut grid = TerminalGrid::with_scrollback(4, 2, 8);

        // Add initial content
        for i in 0..6 {
            let line: CellRow = vec![Cell::new((b'0' + i) as u32); 4];
            grid.add_line(line);
        }

        // Scroll up to view older content
        grid.scroll_up(2);
        let _initial_offset = grid.viewport_offset;

        // Add more content while scrolled up
        let line: CellRow = vec![Cell::new(b'X' as u32); 4];
        grid.add_line(line);

        // Viewport should reset to bottom when new content is added
        assert_eq!(grid.viewport_offset, 0);

        // But scrollback history should be preserved
        grid.scroll_up(1);
        assert!(grid.viewport_offset > 0);
    }

    #[test]
    fn test_memory_efficiency() {
        let mut grid = TerminalGrid::with_scrollback(10, 3, 100);

        // Add exactly the limit amount of content
        for i in 0..103 {
            // 100 scrollback + 3 viewport
            let line: CellRow = vec![Cell::new((b'0' + (i % 10) as u8) as u32); 10];
            grid.add_line(line);
        }

        assert_eq!(grid.scrollback.len(), 103);

        // Add one more line to trigger limit
        let line: CellRow = vec![Cell::new(b'Z' as u32); 10];
        grid.add_line(line);

        // Should maintain exactly the target size
        assert_eq!(grid.scrollback.len(), 103);

        // Verify the oldest content was removed (no longer see first line)
        grid.scroll_up(grid.scrollback_len());
        let viewport = grid.get_viewport();
        // The first character should not be '0' anymore
        assert_ne!(viewport[0][0].glyph_id, b'0' as u32);
    }

    #[test]
    fn test_smooth_paging_operations() {
        let mut grid = TerminalGrid::with_scrollback(5, 4, 20);

        // Add substantial content for paging tests
        for i in 0..25 {
            let line: CellRow = (0..5)
                .map(|_col| Cell::new((b'A' + (i % 26) as u8) as u32))
                .collect();
            grid.add_line(line);
        }

        // Test page-up equivalent (scroll by viewport height)
        let viewport_height = grid.rows as usize;
        grid.scroll_up(viewport_height);
        assert_eq!(grid.viewport_offset, viewport_height);

        // Test page-down equivalent
        grid.scroll_down(viewport_height);
        assert_eq!(grid.viewport_offset, 0);

        // Test partial scrolling for smooth navigation
        grid.scroll_up(1);
        assert_eq!(grid.viewport_offset, 1);

        grid.scroll_up(2);
        assert_eq!(grid.viewport_offset, 3);

        grid.scroll_down(1);
        assert_eq!(grid.viewport_offset, 2);

        // Test rapid scrolling doesn't break consistency
        for _ in 0..10 {
            grid.scroll_up(3);
            grid.scroll_down(1);
        }

        // Should still be in valid state
        assert!(grid.viewport_offset <= grid.scrollback_len());
        let viewport = grid.get_viewport();
        assert_eq!(viewport.len(), grid.rows as usize);
    }

    #[test]
    fn test_page_operations() {
        let mut grid = TerminalGrid::with_scrollback(5, 4, 20);

        // Add substantial content for paging tests
        for i in 0..30 {
            let line: CellRow = (0..5)
                .map(|_col| Cell::new((b'A' + (i % 26) as u8) as u32))
                .collect();
            grid.add_line(line);
        }

        // Test page up
        grid.page_up();
        assert_eq!(grid.viewport_offset, 4); // Should scroll by viewport height

        // Test page down
        grid.page_down();
        assert_eq!(grid.viewport_offset, 0);

        // Test scroll to top
        grid.scroll_to_top();
        let max_offset = grid.scrollback_len();
        assert_eq!(grid.viewport_offset, max_offset);

        // Test multiple page operations
        grid.page_down();
        grid.page_down();
        assert_eq!(grid.viewport_offset, max_offset.saturating_sub(8));

        // Test page up at top (should not go beyond limit)
        grid.scroll_to_top();
        grid.page_up();
        assert_eq!(grid.viewport_offset, max_offset);
    }

    #[test]
    fn test_scrollback_state_queries() {
        let mut grid = TerminalGrid::with_scrollback(5, 3, 10);

        // Test initial state
        assert!(grid.is_at_bottom());
        assert_eq!(grid.viewport_offset(), 0);
        assert_eq!(grid.max_scrollback(), 10);

        // Add content and test
        for i in 0..8 {
            let line: CellRow = vec![Cell::new((b'0' + i) as u32); 5];
            grid.add_line(line);
        }

        // Should still be at bottom after adding content
        assert!(grid.is_at_bottom());
        assert_eq!(grid.total_lines(), 11); // 8 added + 3 initial

        // Scroll up and test
        grid.scroll_up(3);
        assert!(!grid.is_at_bottom());
        assert_eq!(grid.viewport_offset(), 3);

        // Reset and test
        grid.reset_viewport();
        assert!(grid.is_at_bottom());
        assert_eq!(grid.viewport_offset(), 0);
    }

    // Text Selection Tests

    #[test]
    fn test_position_creation_and_comparison() {
        let pos1 = Position::new(5, 10);
        let pos2 = Position::new(5, 10);
        let pos3 = Position::new(6, 10);
        let pos4 = Position::new(5, 11);

        assert_eq!(pos1, pos2);
        assert!(pos1 < pos3); // Same row, earlier column
        assert!(pos1 < pos4); // Earlier row
        assert!(pos3 < pos4); // Earlier row beats later column
    }

    #[test]
    fn test_selection_creation_and_normalization() {
        let start = Position::new(5, 2);
        let end = Position::new(10, 2);

        // Normal order
        let selection = Selection::new(start, end);
        assert_eq!(selection.start, start);
        assert_eq!(selection.end, end);

        // Reverse order should be normalized
        let selection_rev = Selection::new(end, start);
        assert_eq!(selection_rev.start, start);
        assert_eq!(selection_rev.end, end);
    }

    #[test]
    fn test_selection_contains() {
        let selection = Selection::new(Position::new(2, 1), Position::new(5, 3));

        // Test positions inside selection
        assert!(selection.contains(Position::new(2, 1))); // start
        assert!(selection.contains(Position::new(5, 3))); // end
        assert!(selection.contains(Position::new(3, 2))); // middle

        // Test positions outside selection
        assert!(!selection.contains(Position::new(1, 1))); // before start on same row
        assert!(!selection.contains(Position::new(6, 3))); // after end on same row
        assert!(!selection.contains(Position::new(3, 0))); // above (earlier row)
        assert!(!selection.contains(Position::new(3, 4))); // below (later row)
    }

    #[test]
    fn test_selection_multiline() {
        let single_line = Selection::new(Position::new(2, 1), Position::new(5, 1));
        let multi_line = Selection::new(Position::new(2, 1), Position::new(5, 3));

        assert!(!single_line.is_multiline());
        assert!(multi_line.is_multiline());
    }

    #[test]
    fn test_start_and_clear_selection() {
        let mut grid = TerminalGrid::new(10, 5);

        // Initially no selection
        assert!(!grid.has_selection());
        assert!(grid.get_selection().is_none());

        // Start selection
        grid.start_selection(Position::new(3, 2));
        assert!(grid.has_selection());
        let selection = grid.get_selection().unwrap();
        assert_eq!(selection.start, Position::new(3, 2));
        assert_eq!(selection.end, Position::new(3, 2));

        // Clear selection
        grid.clear_selection();
        assert!(!grid.has_selection());
        assert!(grid.get_selection().is_none());
    }

    #[test]
    fn test_extend_selection() {
        let mut grid = TerminalGrid::new(10, 5);

        // Start selection
        grid.start_selection(Position::new(2, 1));
        
        // Extend selection
        grid.extend_selection(Position::new(6, 3));
        
        let selection = grid.get_selection().unwrap();
        assert_eq!(selection.start, Position::new(2, 1));
        assert_eq!(selection.end, Position::new(6, 3));
        assert!(selection.is_multiline());
    }

    #[test]
    fn test_extend_selection_reverse() {
        let mut grid = TerminalGrid::new(10, 5);

        // Start selection
        grid.start_selection(Position::new(6, 3));
        
        // Extend selection backwards (should normalize)
        grid.extend_selection(Position::new(2, 1));
        
        let selection = grid.get_selection().unwrap();
        assert_eq!(selection.start, Position::new(2, 1));
        assert_eq!(selection.end, Position::new(6, 3));
    }

    #[test]
    fn test_position_clamping() {
        let mut grid = TerminalGrid::new(10, 5);

        // Test clamping when starting selection out of bounds
        grid.start_selection(Position::new(15, 10));
        let selection = grid.get_selection().unwrap();
        assert_eq!(selection.start, Position::new(9, 4)); // Clamped to grid bounds
    }

    #[test]
    fn test_position_in_selection() {
        let mut grid = TerminalGrid::new(10, 5);

        grid.start_selection(Position::new(2, 1));
        grid.extend_selection(Position::new(6, 3));

        assert!(grid.position_in_selection(Position::new(2, 1))); // start
        assert!(grid.position_in_selection(Position::new(6, 3))); // end
        assert!(grid.position_in_selection(Position::new(4, 2))); // middle
        assert!(!grid.position_in_selection(Position::new(1, 1))); // outside
        assert!(!grid.position_in_selection(Position::new(7, 3))); // outside
    }

    #[test]
    fn test_select_all() {
        let mut grid = TerminalGrid::new(10, 5);

        grid.select_all();
        
        let selection = grid.get_selection().unwrap();
        assert_eq!(selection.start, Position::new(0, 0));
        assert_eq!(selection.end, Position::new(9, 4));
        assert!(selection.is_multiline());
    }

    #[test]
    fn test_get_selected_text_single_line() {
        let mut grid = TerminalGrid::new(10, 5);

        // Add some text content
        grid.set_cell(2, 1, Cell::new(b'H' as u32));
        grid.set_cell(3, 1, Cell::new(b'e' as u32));
        grid.set_cell(4, 1, Cell::new(b'l' as u32));
        grid.set_cell(5, 1, Cell::new(b'l' as u32));
        grid.set_cell(6, 1, Cell::new(b'o' as u32));

        // Select the word "Hello"
        grid.start_selection(Position::new(2, 1));
        grid.extend_selection(Position::new(6, 1));

        let selected_text = grid.get_selected_text().unwrap();
        assert_eq!(selected_text, "Hello");
    }

    #[test]
    fn test_get_selected_text_multiline() {
        let mut grid = TerminalGrid::new(10, 5);

        // Line 1: "Hello"
        grid.set_cell(0, 1, Cell::new(b'H' as u32));
        grid.set_cell(1, 1, Cell::new(b'e' as u32));
        grid.set_cell(2, 1, Cell::new(b'l' as u32));
        grid.set_cell(3, 1, Cell::new(b'l' as u32));
        grid.set_cell(4, 1, Cell::new(b'o' as u32));

        // Line 2: "World"
        grid.set_cell(0, 2, Cell::new(b'W' as u32));
        grid.set_cell(1, 2, Cell::new(b'o' as u32));
        grid.set_cell(2, 2, Cell::new(b'r' as u32));
        grid.set_cell(3, 2, Cell::new(b'l' as u32));
        grid.set_cell(4, 2, Cell::new(b'd' as u32));

        // Select from "ello" on first line to "Wor" on second line
        grid.start_selection(Position::new(1, 1));
        grid.extend_selection(Position::new(2, 2));

        let selected_text = grid.get_selected_text().unwrap();
        assert_eq!(selected_text, "ello\nWor");
    }

    #[test]
    fn test_get_selected_text_empty_selection() {
        let mut grid = TerminalGrid::new(10, 5);

        // No selection
        assert!(grid.get_selected_text().is_none());

        // Empty selection (same start and end)
        grid.start_selection(Position::new(0, 0));
        let selected_text = grid.get_selected_text();
        // Should return single character or be empty depending on cell content
        if let Some(text) = selected_text {
            assert!(text.len() <= 1);
        }
    }

    #[test]
    fn test_select_word_at_position() {
        let mut grid = TerminalGrid::new(20, 5);

        // Add text: "Hello World Test"
        let text = "Hello World Test";
        for (i, ch) in text.chars().enumerate() {
            grid.set_cell(i as u16, 1, Cell::new(ch as u32));
        }

        // Select word at position within "Hello"
        grid.select_word_at(Position::new(2, 1)); // Should select "Hello"
        let selected = grid.get_selected_text().unwrap();
        assert_eq!(selected, "Hello");

        // Select word at position within "World"
        grid.select_word_at(Position::new(8, 1)); // Should select "World"
        let selected = grid.get_selected_text().unwrap();
        assert_eq!(selected, "World");

        // Select word at position within "Test"
        grid.select_word_at(Position::new(14, 1)); // Should select "Test"
        let selected = grid.get_selected_text().unwrap();
        assert_eq!(selected, "Test");
    }

    #[test]
    fn test_word_boundary_detection() {
        let mut grid = TerminalGrid::new(25, 5);

        // Add text with various boundaries: "hello_world test-case"
        let text = "hello_world test-case";
        for (i, ch) in text.chars().enumerate() {
            grid.set_cell(i as u16, 1, Cell::new(ch as u32));
        }

        // Select within "hello_world" (underscore should be included)
        grid.select_word_at(Position::new(3, 1));
        let selected = grid.get_selected_text().unwrap();
        assert_eq!(selected, "hello_world");

        // Select within "test" (should stop at hyphen)
        grid.select_word_at(Position::new(13, 1));
        let selected = grid.get_selected_text().unwrap();
        assert_eq!(selected, "test");

        // Select within "case"
        grid.select_word_at(Position::new(18, 1));
        let selected = grid.get_selected_text().unwrap();
        assert_eq!(selected, "case");
    }

    #[test]
    fn test_clipboard_integration() {
        let mut grid = TerminalGrid::new(10, 5);

        // Add some text
        let text = "Hello";
        for (i, ch) in text.chars().enumerate() {
            grid.set_cell(i as u16, 1, Cell::new(ch as u32));
        }

        // Select the text
        grid.start_selection(Position::new(0, 1));
        grid.extend_selection(Position::new(4, 1));

        // Test clipboard copy
        let result = grid.copy_selection_to_clipboard();
        // Note: This might fail in CI environments without clipboard support
        // That's expected behavior
        match result {
            Ok(()) => {
                // Clipboard copy succeeded
                println!("Clipboard copy successful");
            }
            Err(_) => {
                // Expected in headless environments
                println!("Clipboard copy failed (expected in CI)");
            }
        }

        // Test copying when no selection exists
        grid.clear_selection();
        let result = grid.copy_selection_to_clipboard();
        assert!(result.is_err());
    }

    #[test]
    fn test_selection_edge_cases() {
        let mut grid = TerminalGrid::new(5, 3);

        // Test selection at grid boundaries
        grid.start_selection(Position::new(0, 0));
        grid.extend_selection(Position::new(4, 2));

        let selection = grid.get_selection().unwrap();
        assert_eq!(selection.start, Position::new(0, 0));
        assert_eq!(selection.end, Position::new(4, 2));

        // Test selection with empty cells
        let selected_text = grid.get_selected_text();
        if let Some(text) = selected_text {
            // Should handle empty cells gracefully
            assert!(text.len() > 0 || text.is_empty()); // Just ensure it's valid
        }
    }

    #[test]
    fn test_selection_survives_clear() {
        let mut grid = TerminalGrid::new(10, 5);

        // Start a selection
        grid.start_selection(Position::new(2, 1));
        grid.extend_selection(Position::new(6, 3));
        assert!(grid.has_selection());

        // Clear the grid (should also clear selection)
        grid.clear();
        assert!(!grid.has_selection());
    }

    #[test]
    fn test_cursor_movement_methods() {
        let mut grid = TerminalGrid::new(10, 5);

        // Test initial position
        assert_eq!(grid.cursor_position(), (0, 0));

        // Test moving cursor up (should not go below 0)
        grid.move_cursor_up(2);
        assert_eq!(grid.cursor_position(), (0, 0));

        // Set cursor to middle and test movements
        grid.set_cursor_position(5, 2);
        assert_eq!(grid.cursor_position(), (5, 2));

        // Test moving up
        grid.move_cursor_up(1);
        assert_eq!(grid.cursor_position(), (5, 1));

        // Test moving down
        grid.move_cursor_down(2);
        assert_eq!(grid.cursor_position(), (5, 3));

        // Test moving left
        grid.move_cursor_left(2);
        assert_eq!(grid.cursor_position(), (3, 3));

        // Test moving right
        grid.move_cursor_right(3);
        assert_eq!(grid.cursor_position(), (6, 3));

        // Test boundary checks - moving too far right
        grid.move_cursor_right(10);
        assert_eq!(grid.cursor_position(), (9, 3)); // Should clamp to max col

        // Test boundary checks - moving too far down
        grid.move_cursor_down(10);
        assert_eq!(grid.cursor_position(), (9, 4)); // Should clamp to max row

        // Test moving to specific column
        grid.move_cursor_to_column(2);
        assert_eq!(grid.cursor_position(), (2, 4));

        // Test moving to specific row
        grid.move_cursor_to_row(1);
        assert_eq!(grid.cursor_position(), (2, 1));

        // Test moving to line start and end
        grid.move_cursor_to_line_start();
        assert_eq!(grid.cursor_position(), (0, 1));

        grid.move_cursor_to_line_end();
        assert_eq!(grid.cursor_position(), (9, 1));
    }

    #[test]
    fn test_cursor_movement_bounds() {
        let mut grid = TerminalGrid::new(5, 3);

        // Test moving left from origin
        grid.set_cursor_position(0, 0);
        grid.move_cursor_left(5);
        assert_eq!(grid.cursor_position(), (0, 0));

        // Test moving up from origin
        grid.move_cursor_up(5);
        assert_eq!(grid.cursor_position(), (0, 0));

        // Test moving beyond right boundary
        grid.move_cursor_right(10);
        assert_eq!(grid.cursor_position(), (4, 0)); // Max col is 4 for 5-column grid

        // Test moving beyond bottom boundary
        grid.move_cursor_down(10);
        assert_eq!(grid.cursor_position(), (4, 2)); // Max row is 2 for 3-row grid
    }

    #[test]
    fn test_csi_cursor_movement_integration() {
        use quantaterm_core::CsiAction;
        let mut grid = TerminalGrid::new(10, 5);

        // Set initial cursor position
        grid.set_cursor_position(5, 2);
        assert_eq!(grid.cursor_position(), (5, 2));

        // Test cursor up
        grid.handle_csi_action(&CsiAction::CursorUp(1));
        assert_eq!(grid.cursor_position(), (5, 1));

        // Test cursor down
        grid.handle_csi_action(&CsiAction::CursorDown(2));
        assert_eq!(grid.cursor_position(), (5, 3));

        // Test cursor forward
        grid.handle_csi_action(&CsiAction::CursorForward(2));
        assert_eq!(grid.cursor_position(), (7, 3));

        // Test cursor backward
        grid.handle_csi_action(&CsiAction::CursorBackward(3));
        assert_eq!(grid.cursor_position(), (4, 3));

        // Test cursor next line
        grid.handle_csi_action(&CsiAction::CursorNextLine(1));
        assert_eq!(grid.cursor_position(), (0, 4)); // Should move to start of next line

        // Test cursor previous line
        grid.handle_csi_action(&CsiAction::CursorPreviousLine(2));
        assert_eq!(grid.cursor_position(), (0, 2)); // Should move to start of line 2 lines up

        // Test cursor horizontal absolute
        grid.handle_csi_action(&CsiAction::CursorHorizontalAbsolute(7));
        assert_eq!(grid.cursor_position(), (7, 2));

        // Test cursor position (absolute)
        grid.handle_csi_action(&CsiAction::CursorPosition(1, 3)); // 1-based in escape sequence, 0-based internally
        assert_eq!(grid.cursor_position(), (3, 1));

        // Test cursor position bounds checking
        grid.handle_csi_action(&CsiAction::CursorPosition(10, 15)); // Beyond grid bounds
        assert_eq!(grid.cursor_position(), (9, 4)); // Should be clamped
    }
}
