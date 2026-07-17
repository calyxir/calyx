use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(PartialEq, Eq, Hash, Clone, Deserialize, Debug)]
pub enum Adl {
    Calyx,
    Py,
    Dahlia,
}

#[derive(PartialEq, Eq, Hash, Clone, Deserialize)]
pub struct AdlInfo {
    pub adl: Adl,
    pub components: Vec<ComponentInfo>,
}

#[derive(PartialEq, Eq, Hash, Clone, Deserialize)]
pub struct ComponentInfo {
    // components may not have metadata attached.
    pub component: String,
    pub filename: Option<String>,
    pub linenum: Option<usize>,
    // association name
    pub varname: Option<String>,
    pub cells: Vec<PosInfo>,
    pub groups: Vec<PosInfo>,
}

#[derive(PartialEq, Eq, Hash, Clone, Deserialize)]
pub struct PosInfo {
    pub name: String,
    pub filename: String,
    pub linenum: usize,
    pub varname: String,
}

pub fn parse_adl_file(adl_filename: &str) -> AdlInfo {
    let adl_file = File::open(adl_filename);
    serde_json::from_reader(BufReader::new(adl_file.unwrap())).unwrap()
}
