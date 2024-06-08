use crossterm::{cursor, style, terminal, ExecutableCommand, QueueableCommand};
use std::cmp::min;

/// Implements a scrollback buffer.
///
/// # Example
/// ```
/// use calyx_pass::scrollback_buffer::ScrollbackBuffer;
/// use std::io::Write;
///
/// fn example() -> std::io::Result<()> {
///     let mut stdout = std::io::stdout();
///     let mut scrollback_buffer = ScrollbackBuffer::new(&mut stdout)?;
///     writeln!(scrollback_buffer, "hi\nhi\nhi\n...");
///     scrollback_buffer.display()?;
///     scrollback_buffer.scroll_down();
///     scrollback_buffer.clear();
///     Ok(())
/// }
/// ```
pub struct ScrollbackBuffer<'a> {
    output: &'a mut dyn std::io::Write,
    lines: Vec<String>,
    io_line_completed: bool,
    io_col: usize,
    pos: usize,
    rows: usize,
    cols: usize,
}

impl<'a> ScrollbackBuffer<'a> {
    /// Constructs a new scrollback buffer for the static *TTY* output stream
    /// `output`.
    ///
    /// Requires: `output` is a TTY.
    pub fn new(output: &'a mut dyn std::io::Write) -> std::io::Result<Self> {
        let initial_size = terminal::size()?;
        Ok(Self {
            output,
            lines: vec![],
            io_line_completed: false,
            io_col: 0,
            pos: 0,
            rows: initial_size.1 as usize,
            cols: initial_size.0 as usize,
        })
    }

    /// The number of rows that the scrollback buffer will display.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// The number of columns that the scrollback buffer will display.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Resizes the dimensions of the scrollback buffer with the given `rows`
    /// and `cols` values.
    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.rows = rows;
        self.cols = cols;
    }

    /// Switches the screen of the output stream to the scrollback buffer.
    pub fn enter(&mut self) -> std::io::Result<()> {
        self.output.execute(cursor::SavePosition)?;
        self.output.execute(cursor::Hide)?;
        self.output.execute(terminal::EnterAlternateScreen)?;
        Ok(())
    }

    /// Restores the original screen on the output stream.
    pub fn exit(&mut self) -> std::io::Result<()> {
        self.output.execute(terminal::LeaveAlternateScreen)?;
        self.output.execute(cursor::RestorePosition)?;
        self.output.execute(cursor::Show)?;
        self.output.execute(cursor::MoveDown(1))?;
        Ok(())
    }

    /// Displays the current visible region of the buffer on the output stream.
    pub fn display(&mut self) -> std::io::Result<()> {
        self.output.queue(cursor::MoveTo(0, 0))?;
        self.output.queue(style::ResetColor)?;

        let fill = " ".repeat(self.cols);
        for i in 0..self.rows {
            self.output.queue(cursor::MoveTo(0, i as u16))?;
            write!(self.output, "{}", fill)?;
        }

        let rows_to_print = self.visible_row_count();

        for i in 0..rows_to_print {
            self.output.queue(cursor::MoveTo(0, i as u16))?;
            write!(self.output, "{}", self.lines[i + self.pos])?;
        }

        self.output.flush()?;

        Ok(())
    }

    /// Scrolls the display up by one row (and does nothing if there is no more
    /// to scroll).
    pub fn scroll_up(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    /// Scrolls the display down by one row (and does nothing if there is no
    /// more to scroll).
    pub fn scroll_down(&mut self) {
        if self.pos < self.max_allowed_position() {
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

impl<'a> std::io::Write for ScrollbackBuffer<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.lines.is_empty() {
            self.lines.push("".into());
        }
        // inv: self.lines is non-empty

        /// Adds a string `acc` to the scrollback buffer. If a line has just
        /// been completed (marked by setting the `buffer.line_completed`
        /// flag), `acc` will be added to a new line. Otherwise,
        /// `acc` will be appended to the end of the final line.
        fn push_acc(buffer: &mut ScrollbackBuffer, acc: &str) {
            if buffer.io_line_completed {
                buffer.lines.push(acc.to_string());
            } else {
                buffer.lines.last_mut().expect("invariant").push_str(acc);
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
