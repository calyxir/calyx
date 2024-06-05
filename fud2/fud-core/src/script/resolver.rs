use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc};

use super::{exec_scripts::RhaiResult, report::RhaiReport};

pub(super) struct Resolver {
    files: HashMap<PathBuf, rhai::AST>,
    modules: RefCell<HashMap<PathBuf, Rc<rhai::Module>>>,
}

impl Resolver {
    pub(super) fn new(files: HashMap<PathBuf, rhai::AST>) -> Self {
        Self {
            files,
            modules: RefCell::new(HashMap::default()),
        }
    }

    fn get(&self, path: &PathBuf) -> Option<Rc<rhai::Module>> {
        self.modules.borrow().get(path).map(Rc::clone)
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

        if let Some(module) = self.get(&path_buf) {
            Ok(module)
        } else {
            let new_module = rhai::Module::eval_ast_as_new(
                rhai::Scope::new(),
                &self.files[&path_buf],
                engine,
            )
            .map(Rc::new)
            .inspect_err(|e| e.report(&path_buf))?;

            self.modules
                .borrow_mut()
                .insert(path_buf, Rc::clone(&new_module));
            Ok(new_module)
        }
    }
}
