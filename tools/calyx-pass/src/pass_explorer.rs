use crate::util::{self, capture_command_stdout};
use colored::Colorize;
use similar::{ChangeTag, TextDiff};
use std::{fs, path::PathBuf};
use tempdir::TempDir;

const FIRST_FILE_NAME: &str = "SOURCE.futil";

pub struct PassExplorer {
    work_dir: TempDir,
    calyx_exec: String,
    breakpoint: Option<String>,
    passes: Vec<String>,
    passes_applied: Vec<usize>,
    index: isize,
    inc_file_exists: bool
}

impl PassExplorer {
    /// Constructs a new pass explorer for exploring how a given pass alias
    /// `pass_alias` transforms an input file `input_file`.
    pub fn new(
        work_dir: TempDir, calyx_exec: String, breakpoint: Option<String>,
        pass_alias: String, input_file: PathBuf
    ) -> std::io::Result<Self> {
        // Parse the output of `calyx pass-help {pass_alias}` to determine the
        // passes executed as part of `pass_alias`.
        let passes: Vec<String> = util::capture_command_stdout(
            &calyx_exec,
            &["pass-help", &pass_alias],
            true
        )?
        .lines()
        .skip(1)
        .map(|line| {
            line.trim_start_matches(|c: char| !c.is_alphanumeric())
                .to_string()
        })
        .collect();

        let mut dest_path = PathBuf::from(work_dir.path());
        dest_path.push(FIRST_FILE_NAME);
        fs::copy(input_file, dest_path.clone())?;

        Ok(Self {
            work_dir,
            calyx_exec,
            breakpoint,
            passes,
            passes_applied: vec![],
            index: 0,
            inc_file_exists: false
        })
    }

    pub fn last_pass(&self) -> Option<String> {
        self.passes_applied
            .last()
            .map(|index| self.passes[*index].clone())
    }

    pub fn incoming_pass(&self) -> Option<String> {
        self.passes.get(self.index as usize).cloned()
    }

    pub fn review(
        &mut self, component: Option<String>
    ) -> std::io::Result<Option<String>> {
        self.ensure_inc_file_exists()?;
        let mut last_file_content = fs::read_to_string(self.last_file())
            .expect("Could not read the last file");

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
                ChangeTag::Equal => format!("{}", change)
            };
            output.push_str(&line);
        }
        Ok(Some(output))
    }

    pub fn accept(&mut self) -> std::io::Result<()> {
        if self.incoming_pass().is_some() {
            fs::remove_file(self.last_file())?;
            self.inc_file_exists = false;
            self.passes_applied.push(self.index as usize);
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
            last_file_path.push(FIRST_FILE_NAME);
        }
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
        if !self.inc_file_exists {
            if let Some(inc_file) = self.incoming_file() {
                capture_command_stdout(
                    &self.calyx_exec,
                    &[
                        "--output",
                        inc_file.to_str().unwrap(),
                        self.last_file().to_str().unwrap(),
                        "-p",
                        &self.incoming_pass().unwrap()
                    ],
                    true
                )?;
            }
            self.inc_file_exists = true;
        }

        Ok(())
    }

    fn filter_component_lines(
        &self, file_content: &str, component: &str
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
                } else {
                    result.push_str(line);
                    result.push('\n');
                }
            }
        }

        result
    }
}
