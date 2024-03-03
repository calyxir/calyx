use std::path::PathBuf;

use tower_lsp::lsp_types as lspt;
use tree_sitter as ts;

use crate::{
    convert::Range,
    document::{Document, Things},
    query_result::QueryResult,
    Config,
};

#[derive(Clone, Debug)]
pub enum DefRes {
    /// We have found the location that we are looking for
    Found(lspt::Location),
    /// Continue searching with these paths, looking for String
    Continue(Vec<PathBuf>, String),
}

impl QueryResult for DefRes {
    type Data = lspt::Location;
    type Needle = String;

    fn found(&self) -> Option<Self::Data> {
        match self {
            DefRes::Found(loc) => Some(loc.clone()),
            DefRes::Continue(_, _) => None,
        }
    }

    fn paths(&self) -> Vec<PathBuf> {
        match self {
            DefRes::Found(_) => vec![],
            DefRes::Continue(paths, _) => paths.clone(),
        }
    }

    fn resume(&self, config: &Config, doc: &Document) -> Option<Self> {
        match self {
            DefRes::Found(_) => Some(self.clone()),
            DefRes::Continue(_, name) => {
                doc.find_component(config, name.to_string())
            }
        }
    }
}

pub trait DefinitionProvider {
    fn find_thing(
        &self,
        config: &Config,
        url: lspt::Url,
        thing: Things,
    ) -> Option<DefRes> {
        match thing {
            Things::Cell(node, name) => self.find_cell(url, node, name),
            Things::SelfPort(node, name) => {
                self.find_self_port(url, node, name)
            }
            Things::Group(node, name) => self.find_group(url, node, name),
            Things::Import(_node, name) => self.find_import(config, url, name),
            Things::Component(name) => self.find_component(config, name),
        }
    }

    fn find_cell(
        &self,
        url: lspt::Url,
        node: ts::Node,
        name: String,
    ) -> Option<DefRes>;
    fn find_self_port(
        &self,
        url: lspt::Url,
        node: ts::Node,
        name: String,
    ) -> Option<DefRes>;
    fn find_group(
        &self,
        url: lspt::Url,
        node: ts::Node,
        name: String,
    ) -> Option<DefRes>;
    fn find_import(
        &self,
        config: &Config,
        url: lspt::Url,
        name: String,
    ) -> Option<DefRes>;
    fn find_component(&self, config: &Config, name: String) -> Option<DefRes>;
}

impl DefinitionProvider for Document {
    fn find_cell(
        &self,
        url: lspt::Url,
        node: ts::Node,
        name: String,
    ) -> Option<DefRes> {
        self.enclosing_cells(node)
            .find(|n| self.node_text(n) == name)
            .map(|node| {
                DefRes::Found(lspt::Location::new(
                    url,
                    Range::from(node).into(),
                ))
            })
    }

    fn find_self_port(
        &self,
        url: lspt::Url,
        node: ts::Node,
        name: String,
    ) -> Option<DefRes> {
        self.enclosing_component_ports(node)
            .find(|n| self.node_text(n) == name)
            .map(|n| {
                DefRes::Found(lspt::Location::new(
                    url.clone(),
                    Range::from(n).into(),
                ))
            })
    }

    fn find_group(
        &self,
        url: lspt::Url,
        node: ts::Node,
        name: String,
    ) -> Option<DefRes> {
        self.enclosing_groups(node)
            .find(|g| self.node_text(g) == name)
            .map(|node| {
                DefRes::Found(lspt::Location::new(
                    url.clone(),
                    Range::from(node).into(),
                ))
            })
    }

    fn find_import(
        &self,
        _config: &Config,
        _url: lspt::Url,
        _name: String,
    ) -> Option<DefRes> {
        None
        // self.resolved_imports(config)
        // resolve_imports(
        //     url.to_file_path().unwrap().parent().unwrap().to_path_buf(),
        //     &config.calyx_lsp.library_paths,
        //     &[name],
        // )
        // .next()
        // .map(|path| {
        //     QueryResult::Found(lspt::Location::new(
        //         lspt::Url::parse(&format!("file://{}", path.display())).unwrap(),
        //         Range::zero().into(),
        //     ))
        // })
    }

    fn find_component(&self, config: &Config, name: String) -> Option<DefRes> {
        self.components()
            .find(|n| self.node_text(n) == name)
            .map(|n| {
                DefRes::Found(lspt::Location::new(
                    self.url.clone(),
                    Range::from(n).into(),
                ))
            })
            .or_else(|| {
                Some(DefRes::Continue(
                    self.resolved_imports(config).collect(),
                    name,
                ))
            })
    }
}
