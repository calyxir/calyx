use super::{
    ast::{ComponentDef, NamespaceDef},
    parser,
};
use crate::Primitive;
use calyx_utils::{CalyxResult, Error};
use linked_hash_map::LinkedHashMap;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

/// String representing the basic compilation primitives that need to be present
/// to support compilation.
const COMPILE_LIB: &str = include_str!("../../primitives/compile.futil");

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
    pub externs: LinkedHashMap<Option<PathBuf>, Vec<Primitive>>,
    /// Original import statements present in the top-level file.
    pub original_imports: Vec<String>,
    /// Optional opaque metadata attached to the top-level file
    pub metadata: Option<String>,
}

impl Workspace {
    /// Returns the absolute location to an imported file.
    /// Imports can refer to files either in the library path or in the parent
    /// folder.
    fn canonicalize_import<S>(
        import: S,
        parent: &Path,
        lib_path: &Path,
    ) -> CalyxResult<PathBuf>
    where
        S: AsRef<Path> + Clone,
    {
        let parent_path = parent.join(import.clone());
        if parent_path.exists() {
            return Ok(parent_path);
        }
        let lib = lib_path.join(import.clone());
        if lib.exists() {
            return Ok(lib);
        }

        Err(Error::invalid_file(
            format!("Import path `{}` found neither in the parent ({}) nor library path ({})",
            import.as_ref().to_string_lossy(),
            parent.to_string_lossy(),
            lib_path.to_string_lossy()
        )))
    }

    // Get the absolute path to an extern. Extern can only exist on paths
    // relative to the parent.
    fn canonicalize_extern<S>(
        extern_path: S,
        parent: &Path,
    ) -> CalyxResult<PathBuf>
    where
        S: AsRef<Path> + Clone,
    {
        let parent_path = parent.join(extern_path.clone());
        if parent_path.exists() {
            return Ok(parent_path);
        }
        Err(Error::invalid_file(format!(
            "Extern path `{}` not found in parent directory ({})",
            extern_path.as_ref().to_string_lossy(),
            parent.to_string_lossy(),
        )))
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
        let ws = Workspace {
            components: ns.components,
            externs: LinkedHashMap::from_iter(Some((None, externs))),
            ..Default::default()
        };
        Ok(ws)
    }

    /// Construct a new workspace from an input stream representing a Calyx
    /// program.
    pub fn construct(
        file: &Option<PathBuf>,
        lib_path: &Path,
    ) -> CalyxResult<Self> {
        Self::construct_with_all_deps::<false>(file, lib_path)
    }

    /// Construct the Workspace using the given [NamespaceDef] and ignore all
    /// imported dependencies.
    pub fn construct_shallow(
        file: &Option<PathBuf>,
        lib_path: &Path,
    ) -> CalyxResult<Self> {
        Self::construct_with_all_deps::<true>(file, lib_path)
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

    /// Construct the Workspace by transitively parsing all `import`ed Calyx
    /// files.
    /// If SHALLOW is true, then parse imported cmoponents as declarations
    fn construct_with_all_deps<const SHALLOW: bool>(
        file: &Option<PathBuf>,
        lib_path: &Path,
    ) -> CalyxResult<Self> {
        // Construct initial namespace.
        let namespace = NamespaceDef::construct(file)?;
        let parent_path = file
            .as_ref()
            .map(|p| Self::get_parent(p))
            .unwrap_or_else(|| PathBuf::from("."));

        // Set of current dependencies
        let mut dependencies: Vec<PathBuf> = Vec::new();
        // Set of imports that have already been parsed once.
        let mut already_imported: HashSet<PathBuf> = HashSet::new();

        let mut workspace = Workspace::default();
        let abs_lib_path = lib_path.canonicalize().map_err(|err| {
            Error::invalid_file(format!(
                "Failed to canonicalize library path `{}`: {}",
                lib_path.to_string_lossy(),
                err
            ))
        })?;

        // Add original imports to workspace
        workspace.original_imports = namespace.imports.clone();

        // TODO (griffin): Probably not a great idea to clone the metadata
        // string but it works for now
        workspace.metadata = namespace.metadata.clone();

        // Function to merge contents of a namespace into the workspace and
        // return the dependencies that need to be parsed next.
        let mut merge_into_ws = |ns: NamespaceDef,
                                 parent: &Path,
                                 shallow: bool|
         -> CalyxResult<Vec<PathBuf>> {
            // Canonicalize the extern paths and add them
            for (path, mut exts) in ns.externs {
                match path {
                    Some(p) => {
                        let abs_path = Self::canonicalize_extern(p, parent)?;
                        workspace
                            .externs
                            .entry(Some(abs_path))
                            .or_default()
                            .append(&mut exts);
                    }
                    None => {
                        workspace
                            .externs
                            .entry(None)
                            .or_default()
                            .append(&mut exts);
                    }
                }
            }

            // Add components defined by this namespace to either components or
            // declarations
            if shallow {
                workspace
                    .declarations
                    .extend(&mut ns.components.into_iter());
            } else {
                workspace.components.extend(&mut ns.components.into_iter());
            }

            // Return the canonical location of import paths
            let deps = ns
                .imports
                .into_iter()
                .map(|p| Self::canonicalize_import(p, parent, &abs_lib_path))
                .collect::<CalyxResult<_>>()?;

            Ok(deps)
        };

        // Merge the initial namespace
        let parent_canonical = parent_path.canonicalize().map_err(|err| {
            Error::invalid_file(format!(
                "Failed to canonicalize parent path `{}`: {}",
                parent_path.to_string_lossy(),
                err
            ))
        })?;
        let mut deps = merge_into_ws(namespace, &parent_canonical, false)?;
        dependencies.append(&mut deps);

        while let Some(p) = dependencies.pop() {
            if already_imported.contains(&p) {
                continue;
            }
            let ns = parser::CalyxParser::parse_file(&p)?;
            let parent = Self::get_parent(&p);

            let mut deps = merge_into_ws(ns, &parent, SHALLOW)?;
            dependencies.append(&mut deps);

            already_imported.insert(p);
        }
        Ok(workspace)
    }
}
