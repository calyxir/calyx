use crate::errors::Error;
use crate::lang::ast;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Inputs {
    component: String,
    params: Vec<u64>,
    inputs: HashMap<String, Option<i64>>,
}

impl Inputs {
    pub fn from_file(file: PathBuf) -> Result<Self, Error> {
        let reader = File::open(file);
        match reader {
            Ok(r) => match serde_json::from_reader(r) {
                Ok(input) => Ok(input),
                Err(_e) => Err(Error::InvalidInputJSON),
            },
            Err(_e) => Err(Error::InvalidFile),
        }
    }

    pub fn component(&self) -> ast::Id {
        ast::Id::from(self.component.clone())
    }

    pub fn inputs(&self) -> HashMap<ast::Id, Option<i64>> {
        let mut inputs = HashMap::new();
        for (id, value) in self.inputs.iter() {
            inputs.insert(ast::Id::from(id.clone()), *value);
        }
        inputs
    }

    pub fn params(&self) -> Vec<u64> {
        self.params.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Outputs {
    outputs: HashMap<String, Option<i64>>,
}

impl Outputs {
    pub fn from(outputs: &HashMap<ast::Id, Option<i64>>) -> Self {
        let mut new_out = HashMap::new();
        for (id, value) in outputs.iter() {
            new_out.insert(id.to_string(), *value);
        }
        Outputs { outputs: new_out }
    }
}
