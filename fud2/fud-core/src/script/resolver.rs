use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    error::Error,
    fmt::Display,
    path::PathBuf,
    rc::Rc,
};

use itertools::Itertools;
use rhai::EvalAltResult;

use super::exec_scripts::RhaiResult;

#[derive(Default)]
pub(super) struct Resolver {
    files: Vec<(PathBuf, rhai::AST)>,
    modules: RefCell<HashMap<String, Rc<rhai::Module>>>,
    failed: RefCell<HashSet<String>>,
}

#[derive(Debug)]
enum ResolverError {
    Failed(String),
    Unknown(String),
}

impl Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolverError::Failed(m) => write!(f, "Loading {m} failed."),
            ResolverError::Unknown(m) => write!(f, "{m} was not found."),
        }
    }
}

impl Error for ResolverError {}

impl From<ResolverError> for Box<EvalAltResult> {
    fn from(value: ResolverError) -> Box<EvalAltResult> {
        Box::new(EvalAltResult::ErrorSystem(String::new(), Box::new(value)))
    }
}

impl Resolver {
    pub fn register_path(
        &mut self,
        path: PathBuf,
        ast: rhai::AST,
    ) -> rhai::AST {
        let functions = ast.clone_functions_only();
        self.files.push((path, ast));

        functions
    }

    pub fn register_data(
        &mut self,
        name: &'static str,
        ast: rhai::AST,
    ) -> rhai::AST {
        // TODO: normalize the name somehow
        self.register_path(PathBuf::from(name), ast)
    }

    pub fn paths(&self) -> Vec<PathBuf> {
        self.files
            .iter()
            .map(|(path, _)| path.clone())
            .sorted()
            .collect()
    }

    fn find_ast(&self, name: &str) -> Option<&rhai::AST> {
        self.files
            .iter()
            .find(|(path, _)| {
                // if name is directly equal to registered path
                // or name is equal to the path stem, then return
                // that ast
                Some(name) == path.to_str()
                    || Some(name) == path.file_stem().and_then(|os| os.to_str())
            })
            .map(|(_, ast)| ast)
    }

    fn resolve_filename(&self, name: &str) -> Option<&PathBuf> {
        self.files
            .iter()
            .find(|(path, _)| {
                Some(name) == path.to_str()
                    || Some(name) == path.file_stem().and_then(|os| os.to_str())
            })
            .map(|(path, _)| path)
    }

    fn normalize_name(&self, name: &str) -> String {
        PathBuf::from(name)
            .file_stem()
            .and_then(|x| x.to_str())
            .map(ToString::to_string)
            .unwrap()
    }

    fn get(&self, path: &str) -> Option<Rc<rhai::Module>> {
        let name = self.normalize_name(path);
        self.modules.borrow().get(&name).map(Rc::clone)
    }

    fn insert(&self, path: &str, module: rhai::Module) -> Rc<rhai::Module> {
        let rc_mod = Rc::new(module);
        let name = self.normalize_name(path);
        self.modules.borrow_mut().insert(name, Rc::clone(&rc_mod));
        rc_mod
    }

    fn did_fail(&self, path: &str) -> RhaiResult<()> {
        let name = self.normalize_name(path);
        if self.failed.borrow().contains(&name) {
            Err(Box::new(EvalAltResult::ErrorSystem(
                "".to_string(),
                Box::new(ResolverError::Failed(format!("{path:?}"))),
            )))
        } else {
            Ok(())
        }
    }

    fn add_failed(&self, path: &str) {
        let name = self.normalize_name(path);
        self.failed.borrow_mut().insert(name);
    }
}

impl rhai::ModuleResolver for Resolver {
    fn resolve(
        &self,
        engine: &rhai::Engine,
        _source: Option<&str>,
        name: &str,
        pos: rhai::Position,
    ) -> RhaiResult<Rc<rhai::Module>> {
        let path_buf = self
            .resolve_filename(name)
            .cloned()
            .unwrap_or(PathBuf::from(name));

        // if this path has already failed, don't try loading it again
        self.did_fail(name)?;

        // return the module of a path if we have already loaded it
        if let Some(module) = self.get(name) {
            Ok(module)
        } else {
            // otherwise, make a new module, cache it, and return it
            self.find_ast(name)
                .ok_or(ResolverError::Unknown(name.to_string()).into())
                .and_then(|ast| {
                    rhai::Module::eval_ast_as_new(
                        rhai::Scope::new(),
                        ast,
                        engine,
                    )
                })
                .map(|m| self.insert(name, m))
                .map_err(|e| {
                    Box::new(EvalAltResult::ErrorInModule(
                        path_buf.as_os_str().to_str().unwrap().to_string(),
                        e,
                        pos,
                    ))
                })
                .inspect_err(|_| self.add_failed(name))
        }
    }
}
