use crate::{
    config::Config,
    error::{LocalError, LocalResult},
    plugin::{PluginCreate, PluginRef},
};
use libloading::{Library, Symbol};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use tempdir::TempDir;

#[derive(Default)]
pub struct Driver {
    plugins: HashMap<String, PluginRef>,
    loaded_libraries: Vec<Library>,
}

impl Driver {
    pub fn load(library_dirs: &[PathBuf]) -> LocalResult<Self> {
        let mut new_self = Self::default();
        for library_dir in library_dirs {
            library_dir
                .read_dir()
                .map_err(LocalError::from)
                .and_then(|library_paths| {
                    for library_path in library_paths {
                        let library_path =
                            library_path.map_err(LocalError::from)?.path();
                        if library_path
                            .extension()
                            .map(|e| e == "so" || e == "dylib")
                            .unwrap_or_default()
                        {
                            let library =
                                unsafe { Library::new(&library_path).unwrap() };
                            new_self.load_plugin(&library_path, library)?;
                        }
                    }
                    Ok(())
                })
                .map_err(LocalError::from)?;
        }
        Ok(new_self)
    }

    pub fn register<S: AsRef<str>>(&mut self, name: S, tb: PluginRef) {
        assert!(
            self.plugins.insert(name.as_ref().to_string(), tb).is_none(),
            "cannot re-register the same testbench name for a different testbench"
        );
    }

    fn load_plugin(
        &mut self,
        path: &Path,
        library: Library,
    ) -> LocalResult<()> {
        let create_plugin: Symbol<PluginCreate> =
            unsafe { library.get(b"_plugin_create") }.map_err(|_| {
                LocalError::other(format!(
                    "Plugin '{}' must `declare_plugin!`.",
                    extract_plugin_name(path)
                ))
            })?;
        let boxed_raw = unsafe { create_plugin() };
        let plugin = unsafe { Box::from_raw(boxed_raw) };
        self.register(plugin.name(), plugin);
        self.loaded_libraries.push(library);
        Ok(())
    }

    pub fn run<S: AsRef<str>, P: AsRef<Path>>(
        &self,
        name: S,
        path: P,
        input: String,
        tests: &[String],
    ) -> LocalResult<()> {
        if let Some(plugin) = self.plugins.get(name.as_ref()) {
            let work_dir =
                TempDir::new(".calyx-tb").map_err(LocalError::from)?;
            let mut config = Config::from(path, name)?;
            let input =
                copy_into(input, &work_dir).map_err(LocalError::from)?;
            let mut test_basenames = vec![];
            for test in tests {
                test_basenames.push(
                    copy_into(test, &work_dir).map_err(LocalError::from)?,
                );
            }
            plugin.setup(&mut config)?;
            config.doctor()?;
            plugin.run(input, &test_basenames, work_dir, &config)
        } else {
            Err(LocalError::Other(format!(
                "Unknown testbench '{}'",
                name.as_ref()
            )))
        }
    }
}

fn copy_into<S: AsRef<str>>(
    file: S,
    work_dir: &TempDir,
) -> std::io::Result<String> {
    let from_path = PathBuf::from(file.as_ref());
    let basename = from_path
        .file_name()
        .expect("path ended with ..")
        .to_str()
        .expect("invalid unicode")
        .to_string();
    let mut to_path = work_dir.path().to_path_buf();
    to_path.push(&basename);
    fs::copy(from_path, to_path)?;
    Ok(basename)
}

fn extract_plugin_name(path: &Path) -> &str {
    let stem = path
        .file_stem()
        .expect("invalid library path")
        .to_str()
        .expect("invalid unicode");
    stem.strip_prefix("lib").unwrap_or(stem)
}
