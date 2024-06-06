use console::Term;
use std::{cmp::min, io::Write};

/// Implements a scrollback buffer.
///
/// # Example
/// ```
/// fn example(output: &Term,) {
///     let scrollback_buffer = ScrollbackBuffer::new(output,);
///     writeln!(scrollback_buffer, "hi");
///     scrollback_buffer.display();
///     scrollback_buffer.scroll_down();
///     scrollback_buffer.clear();
/// }
/// ```
pub struct ScrollbackBuffer {
    output: Term,
    rows: usize,
    cols: usize,
    lines: Vec<String>,
    io_line_completed: bool,
    io_col: usize,
    pos: usize,
}

impl ScrollbackBuffer {
    /// Constructs a new scrollback buffer for the *TTY* output stream `output`.
    pub fn new(output: &Term) -> Self {
        let (rows, cols) = {
            let (rows, cols) = output.size();
            (rows as usize, cols as usize)
        };
        Self {
            output: output.clone(),
            rows,
            cols,
            lines: vec![],
            io_line_completed: false,
            io_col: 0,
            pos: 0,
        }
    }

    /// Displays the current visible region of the buffer on the output stream.
    pub fn display(&self) -> std::io::Result<()> {
        let fill = " ".repeat(self.cols);
        for i in 0..self.rows {
            self.output.move_cursor_to(0, i)?;
            write!(&self.output, "{}", fill)?;
        }

        let rows_to_print = self.visible_row_count();

        for i in 0..rows_to_print {
            self.output.move_cursor_to(0, i)?;
            write!(&self.output, "{}", self.lines[i + self.pos])?;
        }

        self.output.flush()?;

        Ok(())
    }

    /// Scrolls the display up by one row.
    pub fn scroll_up(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    /// Scrolls the display down by one row.
    pub fn scroll_down(&mut self) {
        if self.pos + 1 <= self.max_allowed_position() {
            self.pos += 1;
        }
    }

    /// Clears all lines in the buffer.
    pub fn clear(&mut self) {
        self.pos = 0;
        self.io_col = 0;
        self.io_line_completed = false;
        self.lines.clear();
    }

    /// The number of rows visible from the current scroll position.
    fn visible_row_count(&self) -> usize {
        min(self.rows, self.lines.len() - self.pos)
    }

    /// The furthest the scroll position is allowed to advance.
    fn max_allowed_position(&self) -> usize {
        self.lines.len() - self.visible_row_count()
    }
}

impl std::io::Write for ScrollbackBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.lines.is_empty() {
            self.lines.push("".into());
        }
        // inv: self.lines is non-empty

        /// Adds a string `acc` to the scrollback buffer. If a line has just
        /// been completed (marked by setting the `buffer.line_completed`
        /// flag), `acc` will be added to a new line. Otherwise,
        /// `acc` will be appended to the end of the final line.
        fn push_acc(buffer: &mut ScrollbackBuffer, acc: &String) {
            if buffer.io_line_completed {
                buffer.lines.push(acc.clone());
            } else {
                buffer.lines.last_mut().expect("invariant").push_str(&acc);
            }
        }

        let s = String::from_utf8(buf.to_vec()).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        let mut acc = String::new();
        for c in s.chars() {
            // TODO: parse ANSI escape codes to prevent splitting them
            if c == '\n' || self.io_col == self.cols {
                push_acc(self, &acc);
                acc.clear();
                self.io_line_completed = true;
                self.io_col = 0;
            } else {
                self.io_col += 1;
            }
            if c != '\n' {
                acc.push(c);
            }
        }
        if !acc.is_empty() {
            push_acc(self, &acc);
            self.io_line_completed = false;
            self.io_col = 0;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
