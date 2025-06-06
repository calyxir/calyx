use super::{
    ast::{ComponentDef, NamespaceDef},
    parser,
};
use crate::{LibrarySignatures, source_info::SourceInfoTable};
use calyx_utils::{CalyxResult, Error, WithPos};
use itertools::Itertools;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

/// String representing the basic compilation primitives that need to be present
/// to support compilation.
const COMPILE_LIB: &str = include_str!("../resources/compile.futil");

/// A Workspace represents all Calyx files transitively discovered while trying to compile a
/// top-level file.
///
/// # Example
/// When parsing a file `foo.futil`:
/// ```text
/// import "core.futil";
///
/// component main() -> () { ... }
/// ```
///
/// The workspace gets the absolute path for `core.futil` and adds `main` to the set of defined
/// components. `core.futil` is searched *both* relative to the current file and the library path.
/// Next `core.futil` is parsed:
/// ```
/// extern "core.sv" {
///     primitive std_add[width](left: width, right: width) -> (out: width);
/// }
/// ```
/// The workspace adds `std_add` to the currently defined primitives and looks for `core.sv` in a
/// relative path to this file. It *does not* look for `core.sv` on the library path.
///
/// Finally, since `core.futil` does not `import` any file, the parsing process is completed.
#[derive(Default)]
pub struct Workspace {
    /// List of component definitions that need to be compiled.
    pub components: Vec<ComponentDef>,
    /// List of component definitions that should be used as declarations and
    /// not compiled. This is used when the compiler is invoked with File
    /// compilation mode.
    pub declarations: Vec<ComponentDef>,
    /// Absolute path to extern definitions and primitives defined by them.
    pub lib: LibrarySignatures,
    /// Original import statements present in the top-level file.
    pub original_imports: Vec<String>,
    /// Optional opaque metadata attached to the top-level file.
    pub metadata: Option<String>,
    /// Optional source info table attached to the top-level file
    pub source_info_table: Option<SourceInfoTable>,
}

impl Workspace {
    /// Returns the absolute location to an imported file.
    ///
    /// An import path is first resolved as an absolute or
    /// relative(-to-`parent`) path, and if no file exists at either such
    /// extended path exists, it assumed to be under the library path
    /// `lib_paths`.
    fn canonicalize_import<S>(
        import: S,
        parent: &Path,
        lib_paths: &[PathBuf],
    ) -> CalyxResult<PathBuf>
    where
        S: AsRef<Path> + Clone + WithPos,
    {
        let absolute_import = import.as_ref();
        if absolute_import.is_absolute() && absolute_import.exists() {
            return Ok(import.as_ref().to_path_buf());
        }

        let relative_import = parent.join(&import);
        if relative_import.exists() {
            return Ok(relative_import);
        }

        let library_imports: Vec<_> = lib_paths
            .iter()
            .filter_map(|lib_path| {
                let library_import = lib_path.join(&import);
                library_import.exists().then_some(library_import)
            })
            .collect();

        match library_imports.len() {
            0 => {
                Err(Error::invalid_file(format!(
                    "Import path `{}` found neither as an absolute path, nor in the parent ({}), nor in library path ({})",
                    import.as_ref().to_string_lossy(),
                    parent.to_string_lossy(),
                    lib_paths.iter().map(|p| p.to_string_lossy()).format(", ")
                ))
                .with_pos(&import))
            }
            1 => Ok(library_imports.into_iter().next().unwrap()),
            _ => {
                Err(Error::misc(format!(
                    "Import path `{}` found in multiple library paths ({})",
                    import.as_ref().to_string_lossy(),
                    library_imports
                        .iter()
                        .map(|p| p.to_string_lossy())
                        .format(", ")
                ))
                .with_pos(&import))
            }
        }
    }

    // Get the absolute path to an extern. Extern can only exist on paths
    // relative to the parent.
    #[cfg(not(target_arch = "wasm32"))]
    fn canonicalize_extern<S>(
        extern_path: S,
        parent: &Path,
    ) -> CalyxResult<PathBuf>
    where
        S: AsRef<Path> + Clone + WithPos,
    {
        parent
            .join(extern_path.clone())
            .canonicalize()
            .map_err(|_| {
                Error::invalid_file(format!(
                    "Extern path `{}` not found in parent directory ({})",
                    extern_path.as_ref().to_string_lossy(),
                    parent.to_string_lossy(),
                ))
                .with_pos(&extern_path)
            })
    }

    /// Construct a new workspace using the `compile.futil` library which
    /// contains the core primitives needed for compilation.
    pub fn from_compile_lib() -> CalyxResult<Self> {
        let mut ns = NamespaceDef::construct_from_str(COMPILE_LIB)?;
        // No imports allowed
        assert!(
            ns.imports.is_empty(),
            "core library should not contain any imports"
        );
        // No metadata allowed
        assert!(
            ns.metadata.is_none(),
            "core library should not contain any metadata"
        );
        // Only inline externs are allowed
        assert!(
            ns.externs.len() == 1 && ns.externs[0].0.is_none(),
            "core library should only contain inline externs"
        );
        let (_, externs) = ns.externs.pop().unwrap();
        let mut lib = LibrarySignatures::default();
        for ext in externs {
            lib.add_inline_primitive(ext);
        }
        let ws = Workspace {
            components: ns.components,
            lib,
            ..Default::default()
        };
        Ok(ws)
    }

    /// Construct a new workspace from an input stream representing a Calyx
    /// program.
    pub fn construct(
        file: &Option<PathBuf>,
        lib_paths: &[PathBuf],
    ) -> CalyxResult<Self> {
        Self::construct_with_all_deps::<false>(
            file.iter().cloned().collect(),
            lib_paths,
        )
    }

    /// Construct the Workspace using the given [NamespaceDef] and ignore all
    /// imported dependencies.
    pub fn construct_shallow(
        file: &Option<PathBuf>,
        lib_paths: &[PathBuf],
    ) -> CalyxResult<Self> {
        Self::construct_with_all_deps::<true>(
            file.iter().cloned().collect(),
            lib_paths,
        )
    }

    fn get_parent(p: &Path) -> PathBuf {
        let maybe_parent = p.parent();
        match maybe_parent {
            None => PathBuf::from("."),
            Some(path) => {
                if path.to_string_lossy() == "" {
                    PathBuf::from(".")
                } else {
                    PathBuf::from(path)
                }
            }
        }
    }

    /// Merge the contents of a namespace into this workspace.
    /// `is_source` identifies this namespace as a source file.
    /// The output is a list of files that need to be parsed next and whether they are source files.
    pub fn merge_namespace(
        &mut self,
        ns: NamespaceDef,
        is_source: bool,
        parent: &Path,
        shallow: bool,
        lib_paths: &[PathBuf],
    ) -> CalyxResult<Vec<(PathBuf, bool)>> {
        // Canonicalize the extern paths and add them
        for (path, exts) in ns.externs {
            match path {
                Some(p) => {
                    #[cfg(not(target_arch = "wasm32"))]
                    let abs_path = Self::canonicalize_extern(p, parent)?;

                    // For the WebAssembly target, we avoid depending on the filesystem to
                    // canonicalize paths to imported files. (This canonicalization is not
                    // necessary because imports for the WebAssembly target work differently
                    // anyway.)
                    #[cfg(target_arch = "wasm32")]
                    let abs_path = p.into();

                    let p = self.lib.add_extern(abs_path, exts);
                    if is_source {
                        p.set_source();
                    }
                }
                None => {
                    for ext in exts {
                        let p = self.lib.add_inline_primitive(ext);
                        if is_source {
                            p.set_source();
                        }
                    }
                }
            }
        }

        // Add components defined by this namespace to either components or
        // declarations
        if !is_source && shallow {
            self.declarations.extend(&mut ns.components.into_iter());
        } else {
            self.components.extend(&mut ns.components.into_iter());
        }
        // Return the canonical location of import paths
        let deps = ns
            .imports
            .into_iter()
            .map(|p| {
                Self::canonicalize_import(p, parent, lib_paths)
                    .map(|s| (s, false))
            })
            .collect::<CalyxResult<_>>()?;

        Ok(deps)
    }

    /// Construct the Workspace using the given files and all their dependencies.
    /// If SHALLOW is true, then parse imported components as declarations and not added to the workspace components.
    /// If in doubt, set SHALLOW to false.
    pub fn construct_with_all_deps<const SHALLOW: bool>(
        mut files: Vec<PathBuf>,
        lib_paths: &[PathBuf],
    ) -> CalyxResult<Self> {
        // Construct initial namespace. If `files` is empty, then we're reading from the standard input.
        let first = files.pop();
        let ns = NamespaceDef::construct(&first)?;
        let parent_path = first
            .as_ref()
            .map(|p| Self::get_parent(p))
            .unwrap_or_else(|| PathBuf::from("."));

        // Set of current dependencies and whether they are considered source files.
        let mut dependencies: Vec<(PathBuf, bool)> =
            files.into_iter().map(|p| (p, true)).collect();
        // Set of imports that have already been parsed once.
        let mut already_imported: HashSet<PathBuf> = HashSet::new();

        let mut ws = Workspace::default();

        let abs_lib_paths: Vec<_> = lib_paths
            .iter()
            .map(|lib_path| {
                lib_path.canonicalize().map_err(|err| {
                    Error::invalid_file(format!(
                        "Failed to canonicalize library path `{}`: {}",
                        lib_path.to_string_lossy(),
                        err
                    ))
                })
            })
            .collect::<CalyxResult<_>>()?;

        // Add original imports to workspace
        ws.original_imports =
            ns.imports.iter().map(|imp| imp.to_string()).collect();

        // TODO (griffin): Probably not a great idea to clone the metadata
        // string but it works for now
        ws.metadata = ns.metadata.clone();
        ws.source_info_table = ns.source_info_table.clone();

        // Merge the initial namespace
        let parent_canonical = parent_path.canonicalize().map_err(|err| {
            Error::invalid_file(format!(
                "Failed to canonicalize parent path `{}`: {}",
                parent_path.to_string_lossy(),
                err
            ))
        })?;
        let mut deps = ws.merge_namespace(
            ns,
            true,
            &parent_canonical,
            false,
            &abs_lib_paths,
        )?;
        dependencies.append(&mut deps);

        while let Some((p, source)) = dependencies.pop() {
            if already_imported.contains(&p) {
                continue;
            }
            let ns = parser::CalyxParser::parse_file(&p)?;
            let parent = Self::get_parent(&p);

            let mut deps = ws.merge_namespace(
                ns,
                source,
                &parent,
                SHALLOW,
                &abs_lib_paths,
            )?;
            dependencies.append(&mut deps);

            already_imported.insert(p);
        }
        Ok(ws)
    }
}
