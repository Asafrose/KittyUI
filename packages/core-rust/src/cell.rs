//! Cell buffer and double buffering for efficient terminal rendering.
//!
//! Provides a 2D grid of [`Cell`]s with double buffering. The renderer
//! writes to the back buffer, then diffs against the front buffer to
//! emit only the ANSI sequences needed to update the terminal.

use crate::ansi::{self, Style};

// ---------------------------------------------------------------------------
// Cell
// ---------------------------------------------------------------------------

/// A single terminal cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The character displayed in this cell.
    pub ch: char,
    /// Visual style (foreground, background, attributes).
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// CellBuffer
// ---------------------------------------------------------------------------

/// A 2D grid of cells sized to the terminal dimensions.
#[derive(Debug, Clone)]
pub struct CellBuffer {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl CellBuffer {
    /// Create a new buffer filled with default (blank space) cells.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); width * height],
        }
    }

    /// Buffer width in columns.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Buffer height in rows.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Get a reference to a cell at (row, col). Returns `None` if out of bounds.
    #[must_use]
    pub fn get(&self, row: usize, col: usize) -> Option<&Cell> {
        if row < self.height && col < self.width {
            Some(&self.cells[row * self.width + col])
        } else {
            None
        }
    }

    /// Get a mutable reference to a cell at (row, col). Returns `None` if out of bounds.
    pub fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut Cell> {
        if row < self.height && col < self.width {
            Some(&mut self.cells[row * self.width + col])
        } else {
            None
        }
    }

    /// Set a cell at (row, col). Returns `false` if out of bounds.
    pub fn set(&mut self, row: usize, col: usize, cell: Cell) -> bool {
        if row < self.height && col < self.width {
            self.cells[row * self.width + col] = cell;
            true
        } else {
            false
        }
    }

    /// Fill the entire buffer with the default cell.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::default();
        }
    }

    /// Resize the buffer. Existing content that fits is preserved;
    /// new cells are filled with the default. Content outside the new
    /// dimensions is discarded.
    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        if new_width == self.width && new_height == self.height {
            return;
        }

        let mut new_cells = vec![Cell::default(); new_width * new_height];

        let copy_rows = self.height.min(new_height);
        let copy_cols = self.width.min(new_width);

        for row in 0..copy_rows {
            for col in 0..copy_cols {
                new_cells[row * new_width + col] = self.cells[row * self.width + col].clone();
            }
        }

        self.width = new_width;
        self.height = new_height;
        self.cells = new_cells;
    }

    /// Write a string at (row, col) with the given style.
    /// Characters are placed in consecutive columns. Overflow is clipped.
    pub fn put_str(&mut self, row: usize, col: usize, s: &str, style: Style) {
        if row >= self.height {
            return;
        }
        for (i, ch) in s.chars().enumerate() {
            let c = col + i;
            if c >= self.width {
                break;
            }
            self.cells[row * self.width + c] = Cell { ch, style };
        }
    }
}

// ---------------------------------------------------------------------------
// Double buffer
// ---------------------------------------------------------------------------

/// Double-buffered terminal renderer.
///
/// The **back buffer** is where the application draws. After drawing,
/// call [`diff`](DoubleBuffer::diff) to get the ANSI sequences that
/// update the terminal from the current **front buffer** state to the
/// back buffer state, then call [`swap`](DoubleBuffer::swap) to make
/// the back buffer the new front.
#[derive(Debug)]
pub struct DoubleBuffer {
    front: CellBuffer,
    back: CellBuffer,
}

impl DoubleBuffer {
    /// Create a new double buffer with the given dimensions.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            front: CellBuffer::new(width, height),
            back: CellBuffer::new(width, height),
        }
    }

    /// Width of the buffers.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.back.width
    }

    /// Height of the buffers.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.back.height
    }

    /// Get a mutable reference to the back buffer for drawing.
    pub fn back_mut(&mut self) -> &mut CellBuffer {
        &mut self.back
    }

    /// Get a reference to the back buffer.
    #[must_use]
    pub fn back(&self) -> &CellBuffer {
        &self.back
    }

    /// Get a reference to the front buffer (what is currently displayed).
    #[must_use]
    pub fn front(&self) -> &CellBuffer {
        &self.front
    }

    /// Compare back and front buffers, returning the ANSI escape
    /// sequences needed to update the terminal.
    ///
    /// The diff minimises output by:
    /// - Skipping cells that haven't changed
    /// - Run-length encoding consecutive changed cells on the same row
    ///   that share a style
    /// - Only emitting cursor-move sequences when there are gaps
    #[must_use]
    pub fn diff(&self) -> Vec<u8> {
        let mut output = Vec::new();
        let mut last_style: Option<Style> = None;
        // Track cursor position to avoid redundant moves.
        // After writing a character at (row, col), the cursor is at (row, col+1).
        let mut cursor_row: Option<usize> = None;
        let mut cursor_col: usize = 0;

        for row in 0..self.back.height {
            for col in 0..self.back.width {
                let idx = row * self.back.width + col;
                let front_cell = &self.front.cells[idx];
                let back_cell = &self.back.cells[idx];

                if front_cell == back_cell {
                    continue;
                }

                // Emit cursor movement if not already positioned here.
                let need_move = cursor_row != Some(row) || cursor_col != col;
                if need_move {
                    // ANSI CUP is 1-based.
                    let r = u16::try_from(row + 1).unwrap_or(u16::MAX);
                    let c = u16::try_from(col + 1).unwrap_or(u16::MAX);
                    output.extend_from_slice(&ansi::cursor_to(r, c));
                }

                // Emit style change if needed.
                if last_style.as_ref() != Some(&back_cell.style) {
                    output.extend_from_slice(&back_cell.style.to_sgr());
                    last_style = Some(back_cell.style);
                }

                // Emit the character.
                let mut char_buf = [0u8; 4];
                let encoded = back_cell.ch.encode_utf8(&mut char_buf);
                output.extend_from_slice(encoded.as_bytes());

                cursor_row = Some(row);
                cursor_col = col + 1;
            }
        }

        // Reset style at the end if we emitted anything.
        if last_style.is_some() {
            output.extend_from_slice(&ansi::sgr_reset());
        }

        output
    }

    /// Swap front and back: copy back into front, then clear back.
    pub fn swap(&mut self) {
        self.front = self.back.clone();
        self.back.clear();
    }

    /// Swap without clearing the back buffer — useful when the
    /// application wants to keep drawing incrementally.
    pub fn swap_no_clear(&mut self) {
        self.front = self.back.clone();
    }

    /// Resize both buffers. Existing content is preserved where possible.
    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        self.front.resize(new_width, new_height);
        self.back.resize(new_width, new_height);
    }

    /// Generate ANSI sequences for a full redraw of the back buffer
    /// (ignoring the front buffer). Useful after terminal resize or
    /// when the screen is corrupted.
    #[must_use]
    pub fn full_render(&self) -> Vec<u8> {
        let mut output = Vec::new();
        let mut last_style: Option<Style> = None;

        for row in 0..self.back.height {
            let r = u16::try_from(row + 1).unwrap_or(u16::MAX);
            output.extend_from_slice(&ansi::cursor_to(r, 1));

            for col in 0..self.back.width {
                let cell = &self.back.cells[row * self.back.width + col];

                if last_style.as_ref() != Some(&cell.style) {
                    output.extend_from_slice(&cell.style.to_sgr());
                    last_style = Some(cell.style);
                }

                let mut char_buf = [0u8; 4];
                let encoded = cell.ch.encode_utf8(&mut char_buf);
                output.extend_from_slice(encoded.as_bytes());
            }
        }

        if last_style.is_some() {
            output.extend_from_slice(&ansi::sgr_reset());
        }

        output
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::{Color, UnderlineStyle};

    // -- Cell --

    #[test]
    fn default_cell_is_blank_space() {
        let cell = Cell::default();
        assert_eq!(cell.ch, ' ');
        assert_eq!(cell.style, Style::new());
    }

    #[test]
    fn cell_equality() {
        let a = Cell {
            ch: 'A',
            style: Style {
                bold: true,
                ..Style::new()
            },
        };
        let b = Cell {
            ch: 'A',
            style: Style {
                bold: true,
                ..Style::new()
            },
        };
        assert_eq!(a, b);
    }

    #[test]
    fn cell_inequality_different_char() {
        let a = Cell::default();
        let b = Cell {
            ch: 'X',
            ..Cell::default()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn cell_inequality_different_style() {
        let a = Cell::default();
        let b = Cell {
            style: Style {
                bold: true,
                ..Style::new()
            },
            ..Cell::default()
        };
        assert_ne!(a, b);
    }

    // -- CellBuffer --

    #[test]
    fn buffer_dimensions() {
        let buf = CellBuffer::new(80, 24);
        assert_eq!(buf.width(), 80);
        assert_eq!(buf.height(), 24);
    }

    #[test]
    fn buffer_initialized_with_defaults() {
        let buf = CellBuffer::new(10, 5);
        for row in 0..5 {
            for col in 0..10 {
                let cell = buf.get(row, col);
                assert!(cell.is_some());
                assert_eq!(cell.map(|c| c.ch), Some(' '));
            }
        }
    }

    #[test]
    fn buffer_get_out_of_bounds() {
        let buf = CellBuffer::new(10, 5);
        assert!(buf.get(5, 0).is_none());
        assert!(buf.get(0, 10).is_none());
        assert!(buf.get(100, 100).is_none());
    }

    #[test]
    fn buffer_set_and_get() {
        let mut buf = CellBuffer::new(10, 5);
        let cell = Cell {
            ch: 'X',
            style: Style {
                fg: Some(Color::Ansi(1)),
                ..Style::new()
            },
        };
        assert!(buf.set(2, 3, cell.clone()));
        assert_eq!(buf.get(2, 3), Some(&cell));
    }

    #[test]
    fn buffer_set_out_of_bounds() {
        let mut buf = CellBuffer::new(10, 5);
        assert!(!buf.set(10, 0, Cell::default()));
        assert!(!buf.set(0, 20, Cell::default()));
    }

    #[test]
    fn buffer_get_mut() {
        let mut buf = CellBuffer::new(10, 5);
        if let Some(cell) = buf.get_mut(1, 1) {
            cell.ch = 'Z';
            cell.style.bold = true;
        }
        let cell = buf.get(1, 1);
        assert_eq!(cell.map(|c| c.ch), Some('Z'));
        assert_eq!(cell.map(|c| c.style.bold), Some(true));
    }

    #[test]
    fn buffer_clear() {
        let mut buf = CellBuffer::new(5, 3);
        buf.set(
            0,
            0,
            Cell {
                ch: 'A',
                style: Style {
                    bold: true,
                    ..Style::new()
                },
            },
        );
        buf.clear();
        assert_eq!(buf.get(0, 0), Some(&Cell::default()));
    }

    #[test]
    fn buffer_put_str() {
        let mut buf = CellBuffer::new(10, 3);
        let style = Style {
            fg: Some(Color::Ansi(2)),
            ..Style::new()
        };
        buf.put_str(0, 2, "Hi", style);
        assert_eq!(buf.get(0, 2).map(|c| c.ch), Some('H'));
        assert_eq!(buf.get(0, 3).map(|c| c.ch), Some('i'));
        assert_eq!(buf.get(0, 2).map(|c| c.style), Some(style));
    }

    #[test]
    fn buffer_put_str_clips_overflow() {
        let mut buf = CellBuffer::new(5, 1);
        buf.put_str(0, 3, "Hello", Style::new());
        assert_eq!(buf.get(0, 3).map(|c| c.ch), Some('H'));
        assert_eq!(buf.get(0, 4).map(|c| c.ch), Some('e'));
        // "llo" is clipped
    }

    #[test]
    fn buffer_put_str_row_out_of_bounds() {
        let mut buf = CellBuffer::new(10, 3);
        buf.put_str(5, 0, "test", Style::new());
        // Should not panic, just silently skip
    }

    // -- Resize --

    #[test]
    fn resize_grow() {
        let mut buf = CellBuffer::new(5, 3);
        buf.set(
            0,
            0,
            Cell {
                ch: 'A',
                ..Cell::default()
            },
        );
        buf.set(
            2,
            4,
            Cell {
                ch: 'B',
                ..Cell::default()
            },
        );
        buf.resize(10, 6);
        assert_eq!(buf.width(), 10);
        assert_eq!(buf.height(), 6);
        assert_eq!(buf.get(0, 0).map(|c| c.ch), Some('A'));
        assert_eq!(buf.get(2, 4).map(|c| c.ch), Some('B'));
        // New cells are defaults
        assert_eq!(buf.get(5, 9), Some(&Cell::default()));
    }

    #[test]
    fn resize_shrink() {
        let mut buf = CellBuffer::new(10, 6);
        buf.set(
            0,
            0,
            Cell {
                ch: 'A',
                ..Cell::default()
            },
        );
        buf.set(
            5,
            9,
            Cell {
                ch: 'Z',
                ..Cell::default()
            },
        );
        buf.resize(5, 3);
        assert_eq!(buf.width(), 5);
        assert_eq!(buf.height(), 3);
        assert_eq!(buf.get(0, 0).map(|c| c.ch), Some('A'));
        // (5,9) is now out of bounds
        assert!(buf.get(5, 9).is_none());
    }

    #[test]
    fn resize_same_is_noop() {
        let mut buf = CellBuffer::new(10, 5);
        buf.set(
            2,
            3,
            Cell {
                ch: 'X',
                ..Cell::default()
            },
        );
        buf.resize(10, 5);
        assert_eq!(buf.get(2, 3).map(|c| c.ch), Some('X'));
    }

    // -- DoubleBuffer --

    #[test]
    fn double_buffer_dimensions() {
        let db = DoubleBuffer::new(80, 24);
        assert_eq!(db.width(), 80);
        assert_eq!(db.height(), 24);
    }

    #[test]
    fn diff_empty_buffers_produces_nothing() {
        let db = DoubleBuffer::new(10, 5);
        let diff = db.diff();
        assert!(diff.is_empty());
    }

    #[test]
    fn diff_detects_single_cell_change() {
        let mut db = DoubleBuffer::new(10, 5);
        db.back_mut().set(
            0,
            0,
            Cell {
                ch: 'A',
                ..Cell::default()
            },
        );
        let diff = db.diff();
        let diff_str = String::from_utf8_lossy(&diff);
        // Should contain a cursor move to (1,1) and the character 'A'
        assert!(diff_str.contains("A"));
        assert!(!diff.is_empty());
    }

    #[test]
    fn diff_skips_unchanged_cells() {
        let mut db = DoubleBuffer::new(10, 5);
        // Set same cell in both front and back
        let cell = Cell {
            ch: 'X',
            ..Cell::default()
        };
        db.back_mut().set(0, 0, cell.clone());
        // Swap to make front match back
        db.swap_no_clear();
        // Now set same value again in back
        db.back_mut().set(0, 0, cell);
        let diff = db.diff();
        assert!(diff.is_empty());
    }

    #[test]
    fn diff_emits_style_changes() {
        let mut db = DoubleBuffer::new(10, 1);
        let red_cell = Cell {
            ch: 'R',
            style: Style {
                fg: Some(Color::Ansi(1)),
                ..Style::new()
            },
        };
        db.back_mut().set(0, 0, red_cell);
        let diff = db.diff();
        let diff_str = String::from_utf8_lossy(&diff);
        // Should contain SGR for red foreground (31) and the char
        assert!(diff_str.contains(";31m"));
        assert!(diff_str.contains('R'));
    }

    #[test]
    fn diff_consecutive_same_style_no_redundant_sgr() {
        let mut db = DoubleBuffer::new(10, 1);
        let style = Style {
            bold: true,
            ..Style::new()
        };
        db.back_mut().set(0, 0, Cell { ch: 'A', style });
        db.back_mut().set(0, 1, Cell { ch: 'B', style });
        let diff = db.diff();
        let diff_str = String::from_utf8_lossy(&diff);
        // Should have one SGR for bold, then AB, not two SGRs
        // Count SGR sequences (excluding final reset)
        let sgr_count = diff_str.matches("\x1b[").count();
        // cursor_to + one style + trailing reset = 3
        assert_eq!(sgr_count, 3, "diff: {diff_str}");
    }

    #[test]
    fn diff_skips_cursor_move_for_consecutive_cells() {
        let mut db = DoubleBuffer::new(10, 1);
        db.back_mut().set(
            0,
            0,
            Cell {
                ch: 'A',
                ..Cell::default()
            },
        );
        db.back_mut().set(
            0,
            1,
            Cell {
                ch: 'B',
                ..Cell::default()
            },
        );
        let diff = db.diff();
        let diff_str = String::from_utf8_lossy(&diff);
        // Should only have one CUP (cursor_to), not two
        let cup_count = diff_str.matches('H').count();
        assert_eq!(cup_count, 1, "diff: {diff_str}");
    }

    #[test]
    fn diff_emits_cursor_move_for_gap() {
        let mut db = DoubleBuffer::new(10, 1);
        db.back_mut().set(
            0,
            0,
            Cell {
                ch: 'A',
                ..Cell::default()
            },
        );
        // Skip cell 1, change cell 5
        db.back_mut().set(
            0,
            5,
            Cell {
                ch: 'F',
                ..Cell::default()
            },
        );
        let diff = db.diff();
        let diff_str = String::from_utf8_lossy(&diff);
        // Should have two CUP sequences
        let cup_count = diff_str.matches('H').count();
        assert_eq!(cup_count, 2, "diff: {diff_str}");
    }

    #[test]
    fn diff_multiline() {
        let mut db = DoubleBuffer::new(5, 3);
        db.back_mut().set(
            0,
            0,
            Cell {
                ch: 'A',
                ..Cell::default()
            },
        );
        db.back_mut().set(
            2,
            4,
            Cell {
                ch: 'Z',
                ..Cell::default()
            },
        );
        let diff = db.diff();
        let diff_str = String::from_utf8_lossy(&diff);
        assert!(diff_str.contains('A'));
        assert!(diff_str.contains('Z'));
        // Two CUPs — one for each changed cell
        let cup_count = diff_str.matches('H').count();
        assert_eq!(cup_count, 2, "diff: {diff_str}");
    }

    #[test]
    fn swap_moves_back_to_front_and_clears_back() {
        let mut db = DoubleBuffer::new(5, 3);
        db.back_mut().set(
            0,
            0,
            Cell {
                ch: 'X',
                ..Cell::default()
            },
        );
        db.swap();
        // Front should now have 'X'
        assert_eq!(db.front().get(0, 0).map(|c| c.ch), Some('X'));
        // Back should be cleared
        assert_eq!(db.back().get(0, 0), Some(&Cell::default()));
    }

    #[test]
    fn swap_no_clear_preserves_back() {
        let mut db = DoubleBuffer::new(5, 3);
        let cell = Cell {
            ch: 'Y',
            ..Cell::default()
        };
        db.back_mut().set(0, 0, cell.clone());
        db.swap_no_clear();
        assert_eq!(db.front().get(0, 0).map(|c| c.ch), Some('Y'));
        assert_eq!(db.back().get(0, 0).map(|c| c.ch), Some('Y'));
    }

    #[test]
    fn diff_after_swap_is_empty() {
        let mut db = DoubleBuffer::new(5, 3);
        db.back_mut().set(
            0,
            0,
            Cell {
                ch: 'A',
                ..Cell::default()
            },
        );
        db.swap_no_clear();
        // Front and back are now identical
        let diff = db.diff();
        assert!(diff.is_empty());
    }

    #[test]
    fn double_buffer_resize() {
        let mut db = DoubleBuffer::new(10, 5);
        db.back_mut().set(
            0,
            0,
            Cell {
                ch: 'A',
                ..Cell::default()
            },
        );
        db.resize(20, 10);
        assert_eq!(db.width(), 20);
        assert_eq!(db.height(), 10);
        // Preserved content
        assert_eq!(db.back().get(0, 0).map(|c| c.ch), Some('A'));
        // New cells are default
        assert_eq!(db.back().get(9, 19), Some(&Cell::default()));
    }

    // -- Full render --

    #[test]
    fn full_render_outputs_all_cells() {
        let mut db = DoubleBuffer::new(3, 2);
        db.back_mut().put_str(0, 0, "ABC", Style::new());
        db.back_mut().put_str(1, 0, "DEF", Style::new());
        let rendered = db.full_render();
        let rendered_str = String::from_utf8_lossy(&rendered);
        assert!(rendered_str.contains("ABC"));
        assert!(rendered_str.contains("DEF"));
    }

    #[test]
    fn full_render_includes_styles() {
        let mut db = DoubleBuffer::new(3, 1);
        let style = Style {
            bold: true,
            fg: Some(Color::Rgb(255, 0, 0)),
            ..Style::new()
        };
        db.back_mut().put_str(0, 0, "Hi!", style);
        let rendered = db.full_render();
        let rendered_str = String::from_utf8_lossy(&rendered);
        assert!(rendered_str.contains(";1;"));
        assert!(rendered_str.contains("38;2;255;0;0"));
        assert!(rendered_str.contains("Hi!"));
    }

    // -- Style reuse from ansi.rs --

    #[test]
    fn cell_with_underline_style() {
        let cell = Cell {
            ch: 'U',
            style: Style {
                underline_style: Some(UnderlineStyle::Curly),
                underline_color: Some(Color::Rgb(255, 0, 0)),
                ..Style::new()
            },
        };
        let sgr = cell.style.to_sgr();
        let sgr_str = String::from_utf8_lossy(&sgr);
        assert!(sgr_str.contains("4:3"));
        assert!(sgr_str.contains("58;2;255;0;0"));
    }

    #[test]
    fn cell_with_overline() {
        let cell = Cell {
            ch: 'O',
            style: Style {
                overline: true,
                ..Style::new()
            },
        };
        let sgr = cell.style.to_sgr();
        let sgr_str = String::from_utf8_lossy(&sgr);
        assert!(sgr_str.contains(";53"));
    }

    // -- Benchmark-style test --

    #[test]
    fn diff_200x50_performance() {
        let mut db = DoubleBuffer::new(200, 50);

        // Fill back buffer with content
        let style = Style {
            fg: Some(Color::Rgb(200, 200, 200)),
            bg: Some(Color::Rgb(0, 0, 40)),
            ..Style::new()
        };
        for row in 0..50 {
            for col in 0..200 {
                db.back_mut().set(
                    row,
                    col,
                    Cell {
                        ch: if (row + col) % 2 == 0 { '#' } else { '.' },
                        style,
                    },
                );
            }
        }

        // Time the diff
        let start = std::time::Instant::now();
        let _diff = db.diff();
        let elapsed = start.elapsed();

        // Should be well under 1ms
        assert!(
            elapsed.as_millis() < 10,
            "diff took {elapsed:?}, expected <1ms (using 10ms tolerance for CI)"
        );
    }

    #[test]
    fn diff_partial_update_performance() {
        let mut db = DoubleBuffer::new(200, 50);

        // Fill both buffers with same content
        let style = Style::new();
        for row in 0..50 {
            for col in 0..200 {
                db.back_mut().set(row, col, Cell { ch: '.', style });
            }
        }
        db.swap_no_clear();

        // Change only 10% of cells in the back buffer
        for row in 0..50 {
            for col in (0..200).step_by(10) {
                db.back_mut().set(
                    row,
                    col,
                    Cell {
                        ch: '#',
                        style: Style {
                            bold: true,
                            ..Style::new()
                        },
                    },
                );
            }
        }

        let start = std::time::Instant::now();
        let diff = db.diff();
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 10,
            "partial diff took {elapsed:?}, expected <1ms"
        );
        // Diff should only contain the changed cells, not all 10000
        assert!(!diff.is_empty());
    }
}
