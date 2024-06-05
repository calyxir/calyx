use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    error::Error,
    fmt::Display,
    path::PathBuf,
    rc::Rc,
};

use rhai::EvalAltResult;

use super::{exec_scripts::RhaiResult, report::RhaiReport};

#[derive(Default)]
pub(super) struct Resolver {
    files: HashMap<PathBuf, rhai::AST>,
    modules: RefCell<HashMap<PathBuf, Rc<rhai::Module>>>,
    failed: RefCell<HashSet<PathBuf>>,
}

#[derive(Debug)]
enum ResolverError {
    Failed(String),
}

impl Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolverError::Failed(m) => write!(f, "Loading {m} failed."),
        }
    }
}

impl Error for ResolverError {}

impl Resolver {
    pub(super) fn new(files: HashMap<PathBuf, rhai::AST>) -> Self {
        Self {
            files,
            ..Default::default()
        }
    }

    fn get(&self, path: &PathBuf) -> Option<Rc<rhai::Module>> {
        self.modules.borrow().get(path).map(Rc::clone)
    }

    fn insert(&self, path: PathBuf, module: rhai::Module) -> Rc<rhai::Module> {
        let rc_mod = Rc::new(module);
        self.modules.borrow_mut().insert(path, Rc::clone(&rc_mod));
        rc_mod
    }

    fn did_fail(&self, path: &PathBuf) -> RhaiResult<()> {
        if self.failed.borrow().contains(path) {
            Err(Box::new(EvalAltResult::ErrorSystem(
                "Failed module loading".to_string(),
                Box::new(ResolverError::Failed(format!("{path:?}"))),
            )))
        } else {
            Ok(())
        }
    }

    fn add_failed(&self, path: PathBuf) {
        self.failed.borrow_mut().insert(path);
    }
}

impl rhai::ModuleResolver for Resolver {
    fn resolve(
        &self,
        engine: &rhai::Engine,
        _source: Option<&str>,
        path: &str,
        _pos: rhai::Position,
    ) -> RhaiResult<Rc<rhai::Module>> {
        let path_buf = PathBuf::from(path);

        // if this path has already failed, don't try loading it again
        self.did_fail(&path_buf)?;

        // return the module of a path if we have already loaded it
        if let Some(module) = self.get(&path_buf) {
            Ok(module)
        } else {
            // otherwise, make a new module, cache it, and return it
            let new_module = rhai::Module::eval_ast_as_new(
                rhai::Scope::new(),
                &self.files[&path_buf],
                engine,
            );

            match new_module {
                Ok(n) => Ok(self.insert(path_buf, n)),
                Err(e) => {
                    e.report(&path_buf);
                    self.add_failed(path_buf);
                    Err(e)
                }
            }
        }
    }
}
