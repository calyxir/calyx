use crate::util::{self, capture_command_stdout};
use crossterm::style::Stylize;
use similar::{ChangeTag, TextDiff};
use std::{collections::HashSet, fs, path::PathBuf};
use tempdir::TempDir;

/// The initial file name to copy the input file to.
const SOURCE_FILE_NAME: &str = "SOURCE.futil";

/// The status of a pass in exploration.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum PassApplicationStatus {
    /// The pass has been applied.
    Applied,

    /// The pass has been skipped.
    Skipped,

    /// The pass is staged to be applied.
    Incoming,

    /// The pass will be staged for application later.
    Future,
}

pub struct Breakpoint {
    pass: String,
    skip: HashSet<String>,
}

impl Breakpoint {
    pub fn from(pass: String, skip: Vec<String>) -> Self {
        Self {
            pass,
            skip: HashSet::from_iter(skip),
        }
    }
}

/// A pass explorer can be used to visualize arbitrary pass transformations on
/// an original input file.
pub struct PassExplorer {
    /// The working directory where pass application files are stored.
    work_dir: TempDir,

    /// A path to the calyx executable.
    calyx_exec: String,

    /// A pass to breakpoint at.
    breakpoint: Option<Breakpoint>,

    /// The passes to explore.
    passes: Vec<String>,

    /// The subset of the indices of passes that have been applied.
    passes_applied: Vec<usize>,

    /// The index of the incomign pass.
    index: isize,

    /// Stores whether a given pass application file exists.
    file_exists: HashSet<PathBuf>,
}

impl PassExplorer {
    /// Constructs a new pass explorer for exploring how a given pass alias
    /// `pass_alias` transforms an input file `input_file`.
    pub fn new(
        work_dir: TempDir,
        calyx_exec: String,
        breakpoint: Option<Breakpoint>,
        pass_alias: String,
        input_file: PathBuf,
    ) -> std::io::Result<Self> {
        // Parse the output of `calyx pass-help {pass_alias}` to determine the
        // passes executed as part of `pass_alias`.
        let passes: Vec<String> = util::capture_command_stdout(
            &calyx_exec,
            &["pass-help", &pass_alias],
            true,
        )?
        .lines()
        .skip(1)
        .map(|line| {
            line.trim_start_matches(|c: char| !c.is_alphanumeric())
                .to_string()
        })
        .collect();

        assert!(!passes.is_empty(), "Unknown pass alias '{}'", pass_alias);

        let mut dest_path = PathBuf::from(work_dir.path());
        dest_path.push(SOURCE_FILE_NAME);
        fs::copy(input_file, dest_path.clone())?;

        let mut new_self = Self {
            work_dir,
            calyx_exec,
            breakpoint,
            passes,
            passes_applied: vec![],
            index: 0,
            file_exists: HashSet::new(),
        };

        if new_self.breakpoint.is_some() {
            let breakpoint = std::mem::take(&mut new_self.breakpoint).unwrap();
            assert!(new_self.passes.contains(&breakpoint.pass));
            while new_self.incoming_pass().expect("There is at least one pass by our prior assertion and we also must encounter the breakpoint").ne(&breakpoint.pass) {
                if !breakpoint.skip.contains(&new_self.incoming_pass().unwrap()) {
                new_self.ensure_inc_file_exists()?;
                new_self.accept()?;
                } else {
                    new_self.skip()?;
                }
            }
            new_self.breakpoint = Some(breakpoint);
        }

        Ok(new_self)
    }

    /// The pass most recently applied, if one exists.
    pub fn last_pass(&self) -> Option<String> {
        self.passes_applied
            .last()
            .map(|index| self.passes[*index].clone())
    }

    /// The pass staged to be applied, if one exists.
    pub fn incoming_pass(&self) -> Option<String> {
        self.passes.get(self.index as usize).cloned()
    }

    /// An association of each pass with its current exploration status.
    pub fn current_pass_application(
        &self,
    ) -> Vec<(String, PassApplicationStatus)> {
        let mut result = vec![];
        let mut walk_applied = 0;
        for i in 0..self.passes.len() {
            let pass = self.passes[i].clone();
            if walk_applied < self.passes_applied.len()
                && i == self.passes_applied[walk_applied]
            {
                result.push((pass, PassApplicationStatus::Applied));
                walk_applied += 1;
            } else if i < (self.index as usize) {
                result.push((pass, PassApplicationStatus::Skipped));
            } else if i == (self.index as usize) {
                result.push((pass, PassApplicationStatus::Incoming));
            } else {
                result.push((pass, PassApplicationStatus::Future));
            }
        }
        result
    }

    /// Produces a printable diff showing how the
    /// [`PassExplorer::incoming_pass`] will transform the current file state.
    pub fn review(
        &mut self,
        component: Option<String>,
    ) -> std::io::Result<Option<String>> {
        self.ensure_inc_file_exists()?;
        let mut last_file_content = fs::read_to_string(self.last_file())?;

        let mut incoming_file_content = fs::read_to_string(
        self.incoming_file()
                .as_ref()
                .expect("If there is another pass, there should be a file with the results of that pass")
            )
            .expect("Could not read the incoming file");

        if let Some(component) = component {
            last_file_content =
                self.filter_component_lines(&last_file_content, &component);
            incoming_file_content =
                self.filter_component_lines(&incoming_file_content, &component);
        }

        let diff =
            TextDiff::from_lines(&last_file_content, &incoming_file_content);
        let mut output = String::new();
        for change in diff.iter_all_changes() {
            let line = match change.tag() {
                ChangeTag::Delete => {
                    format!("{}{}", "- ".red(), change.to_string().red())
                }
                ChangeTag::Insert => {
                    format!("{}{}", "+ ".green(), change.to_string().green())
                }
                ChangeTag::Equal => format!("{}", change),
            };
            output.push_str(&line);
        }
        Ok(Some(output))
    }

    /// Applies the incoming pass.
    pub fn accept(&mut self) -> std::io::Result<()> {
        self.advance(true)
    }

    /// Skips the incoming pass.
    pub fn skip(&mut self) -> std::io::Result<()> {
        self.advance(false)
    }

    /// Undos the last acceptance or skip.
    pub fn undo(&mut self) -> std::io::Result<()> {
        if !self.passes_applied.is_empty() {
            assert!(self.file_exists.remove(&self.last_file()));
            let last_pass_index = self
                .passes_applied
                .pop()
                .expect("pop should succeed on non-empty array");
            self.index = last_pass_index as isize;
        } else if self.index > 0 {
            self.index -= 1;
        }
        Ok(())
    }

    /// Advances to the next pass (if one exists). If `apply`, the incoming pass
    /// changes will be made. Otherwise, it will be skipped.
    fn advance(&mut self, apply: bool) -> std::io::Result<()> {
        if self.incoming_pass().is_some() {
            if apply {
                self.passes_applied.push(self.index as usize);
            }
            self.index += 1;
        }
        Ok(())
    }

    /// A path to the file that has been recently transformed (or not
    /// transformed at all).
    fn last_file(&self) -> PathBuf {
        let mut last_file_path = PathBuf::from(self.work_dir.path());
        if let Some(last_pass) = self.last_pass() {
            last_file_path.push(last_pass);
            last_file_path.set_extension("futil");
        } else {
            last_file_path.push(SOURCE_FILE_NAME);
        }
        last_file_path
    }

    fn source_file(&self) -> PathBuf {
        let mut last_file_path = PathBuf::from(self.work_dir.path());
        last_file_path.push(SOURCE_FILE_NAME);
        last_file_path
    }

    /// A path to the file transformed by the incoming pass.
    fn incoming_file(&self) -> Option<PathBuf> {
        let mut inc_file_path = PathBuf::from(self.work_dir.path());
        inc_file_path.push(self.incoming_pass()?);
        inc_file_path.set_extension("futil");
        Some(inc_file_path)
    }

    /// Produces the incoming file if it does not exist already.
    fn ensure_inc_file_exists(&mut self) -> std::io::Result<()> {
        if let Some(inc_file) = self.incoming_file() {
            if !self.file_exists.contains(&inc_file) {
                // We reapply passes to the source file because calyx IR is not deserializable
                let source_file = self.source_file();
                let mut args = vec![
                    "-o",
                    inc_file.to_str().unwrap(),
                    source_file.to_str().unwrap(),
                ];
                for applied_index in &self.passes_applied {
                    args.push("-p");
                    args.push(&self.passes[*applied_index]);
                }
                let inc_pass = self.incoming_pass().unwrap();
                args.push("-p");
                args.push(&inc_pass);
                capture_command_stdout(&self.calyx_exec, &args, true)?;
                self.file_exists.insert(inc_file.clone());
            }
        }

        Ok(())
    }

    /// Extracts a component named `component` from a syntactically-correct and
    /// complete calyx program represented in `file_content`.
    fn filter_component_lines(
        &self,
        file_content: &str,
        component: &str,
    ) -> String {
        let mut result = String::new();
        let mut in_component = false;
        let mut brace_count = 0;

        for line in file_content.lines() {
            if line.contains(&format!("component {}(", component)) {
                in_component = true;
            }

            if in_component {
                brace_count += line.chars().filter(|&c| c == '{').count();
                brace_count -= line.chars().filter(|&c| c == '}').count();

                if brace_count == 0 {
                    in_component = false;
                }
                result.push_str(line);
                result.push('\n');
            }
        }

        result
    }
}
