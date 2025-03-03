use crate::{
    pass_explorer::{PassApplicationStatus, PassExplorer},
    scrollback_buffer::ScrollbackBuffer,
};
use crossterm::{
    event::{self, Event, KeyCode},
    style::Stylize,
    terminal,
};
use std::cmp::{max, min};
use std::io::Write;

/// Quit the program.
const QUIT: char = 'q';

/// See [`PassExplorer::accept`].
const ACCEPT: char = 'a';

/// See [`PassExplorer::skip`].
const SKIP: char = 's';

/// See [`PassExplorer::undo`].
const UNDO: char = 'u';

/// Scroll forward [`JUMP`] lines.
const JUMP_FWD: char = 'f';

/// Scroll backward [`JUMP`] lines.
const JUMP_BCK: char = 'b';

/// See [`JUMP_FWD`] and [`JUMP_BCK`].
const JUMP: usize = 4;

/// A response to an input event in the main loop (for the TUI).
enum TUIAction {
    /// Continue the main loop.
    Continue,

    /// Exit the main loop.
    Quit,
}

/// Interactive environment for using [`PassExplorer`].
pub struct PassExplorerTUI<'a> {
    /// Pass explorer.
    pass_explorer: PassExplorer,

    /// An optional component to focus on.
    component: Option<String>,

    /// The scrollback buffer used for rendering.
    scrollback_buffer: ScrollbackBuffer<'a>,

    /// Whether the display should be refreshed, a fast operation. This flag
    /// requests that e.g. the scroll buffer appropriately reflect terminal
    /// size changes or scrolling. DO NOT SET DIRECTORY. Instead use
    /// [`PassExplorerTUI::request_refresh`].
    needs_refresh: bool,

    /// Whether the display should be redrawn, a slow operation. In
    /// particular, this flag requests that the pass explorer information be
    /// recomputed. DO NOT SET DIRECTORY. Instead use
    /// [`PassExplorerTUI::request_redraw`].
    needs_redraw: bool,
}

impl<'a> PassExplorerTUI<'a> {
    /// Constructs a pass explorer TUI for `pass_explorer` and optionally
    /// focusing on `component`.
    pub fn from(
        output: &'a mut dyn std::io::Write,
        pass_explorer: PassExplorer,
        component: Option<String>,
    ) -> std::io::Result<Self> {
        Ok(Self {
            pass_explorer,
            component,
            scrollback_buffer: ScrollbackBuffer::new(output)?,
            needs_redraw: true,
            needs_refresh: true,
        })
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        terminal::enable_raw_mode()?;
        self.scrollback_buffer.enter()?;

        loop {
            self.auto_redraw()?;
            self.auto_refresh()?;

            if let TUIAction::Quit = self.handle_input()? {
                break;
            }

            if self.pass_explorer.incoming_pass().is_none() {
                break;
            }
        }

        terminal::disable_raw_mode()?;
        self.scrollback_buffer.exit()?;

        Ok(())
    }

    fn request_refresh(&mut self) {
        self.needs_refresh = true;
    }

    fn request_redraw(&mut self) {
        self.needs_redraw = true;
    }

    fn auto_refresh(&mut self) -> std::io::Result<()> {
        if self.needs_refresh {
            self.refresh()?;
            self.needs_refresh = false;
        }
        Ok(())
    }

    fn auto_redraw(&mut self) -> std::io::Result<()> {
        if self.needs_redraw {
            self.redraw()?;
            self.needs_redraw = false;
        }
        Ok(())
    }

    fn refresh(&mut self) -> std::io::Result<()> {
        self.scrollback_buffer.display()?;
        Ok(())
    }

    #[allow(clippy::write_literal)]
    #[allow(clippy::needless_range_loop)]
    fn redraw(&mut self) -> std::io::Result<()> {
        self.scrollback_buffer.clear();
        writeln!(
            self.scrollback_buffer,
            "{}",
            "Calyx Pass Explorer".underlined()
        )?;
        writeln!(
        self.scrollback_buffer,
        "Usage:\n  1. Explore: {} {}, {} {}, {} {}, {} {}\n  2. Movement: {} {}, {} {}, up/down arrows, scroll",
        ACCEPT.to_string().green(),
        "accept".dark_green(),
        SKIP,
        "skip",
        QUIT.to_string().red(),
        "quit".dark_red(),
        UNDO.to_string().cyan(),
        "undo".dark_cyan(),
        JUMP_FWD.to_string().magenta(),
        "forward".dark_magenta(),
        JUMP_BCK.to_string().magenta(),
        "back".dark_magenta(),
    )?;

        let current_pass_application =
            self.pass_explorer.current_pass_application();
        if let Some(incoming_pos) = current_pass_application
            .iter()
            .position(|(_, status)| *status == PassApplicationStatus::Incoming)
        {
            write!(self.scrollback_buffer, "Passes: ")?;
            let start_index = max(0, (incoming_pos as isize) - 3) as usize;
            if start_index > 0 {
                write!(self.scrollback_buffer, "[{} more] ... ", start_index)?;
            }

            let length = min(5, current_pass_application.len() - start_index);
            for i in start_index..start_index + length {
                if i > start_index {
                    write!(self.scrollback_buffer, ", ")?;
                }
                let (name, status) = &current_pass_application[i];
                let name = name.clone();
                let colored_name = match status {
                    PassApplicationStatus::Applied => name.green().bold(),
                    PassApplicationStatus::Skipped => name.grey().dim(),
                    PassApplicationStatus::Incoming => {
                        format!("[INCOMING] {}", name).yellow().bold()
                    }
                    PassApplicationStatus::Future => name.magenta().bold(),
                };
                write!(self.scrollback_buffer, "{}", colored_name)?;
            }

            let remaining_count =
                current_pass_application.len() - start_index - length;
            if remaining_count > 0 {
                write!(
                    self.scrollback_buffer,
                    " ... [{} more]",
                    remaining_count
                )?;
            }

            writeln!(self.scrollback_buffer)?;
        }

        if let Some(review) =
            self.pass_explorer.review(self.component.clone())?
        {
            writeln!(
                self.scrollback_buffer,
                "{}",
                "─".repeat(self.scrollback_buffer.cols()).dim()
            )?;
            write!(self.scrollback_buffer, "{}", review)?;
        }

        Ok(())
    }

    fn handle_input(&mut self) -> std::io::Result<TUIAction> {
        if event::poll(std::time::Duration::from_secs(0))? {
            // The only way anything happens is from an event, so we want
            // to refresh on that.
            self.request_refresh();

            match event::read()? {
                Event::Key(event) => match event.code {
                    KeyCode::Char(c) => match c {
                        QUIT => {
                            return Ok(TUIAction::Quit);
                        }
                        ACCEPT => {
                            self.pass_explorer.accept()?;
                            self.request_redraw();
                        }
                        SKIP => {
                            self.pass_explorer.skip()?;
                            self.request_redraw();
                        }
                        UNDO => {
                            self.pass_explorer.undo()?;
                            self.request_redraw();
                        }
                        JUMP_FWD => {
                            for _ in 0..JUMP {
                                self.scrollback_buffer.scroll_down()
                            }
                        }
                        JUMP_BCK => {
                            for _ in 0..JUMP {
                                self.scrollback_buffer.scroll_up()
                            }
                        }
                        _ => (),
                    },
                    KeyCode::Up => self.scrollback_buffer.scroll_up(),
                    KeyCode::Down => self.scrollback_buffer.scroll_down(),
                    _ => (),
                },
                Event::Resize(cols, rows) => {
                    self.scrollback_buffer.resize(rows as usize, cols as usize);
                }
                _ => (),
            }
        }
        Ok(TUIAction::Continue)
    }
}
