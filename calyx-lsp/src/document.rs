//! Represents a single Calyx file

use std::collections::HashMap;
use std::path::PathBuf;

use itertools::{multizip, Itertools};
use regex::Regex;
use resolve_path::PathResolveExt;
use tower_lsp::lsp_types as lspt;
use tree_sitter as ts;

use crate::convert::{Contains, Point, Range};
use crate::log;
use crate::ts_utils::ParentUntil;
use crate::{tree_sitter_calyx, Config};

pub struct Document {
    pub url: lspt::Url,
    text: String,
    tree: Option<ts::Tree>,
    parser: ts::Parser,
    /// Map the stores information about every component defined in this file.
    pub components: HashMap<String, ComponentInfo>,
}

/// Public information about a component
#[derive(Debug)]
pub struct ComponentSig {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

/// File-private information about each component
#[derive(Debug)]
pub struct ComponentInfo {
    /// the signature of this component
    pub signature: ComponentSig,
    /// map from cell names to component names
    pub cells: HashMap<String, String>,
    /// the names of groups in this component
    pub groups: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum Things<'a> {
    /// Identifier referring to a cell
    Cell(ts::Node<'a>, String),
    /// Identifier referring to a port
    SelfPort(ts::Node<'a>, String),
    /// Identifier refeferring to a component
    Component(String),
    /// Identifier referring to a group
    Group(ts::Node<'a>, String),
    /// Mainly a way to test jumping to other files. How does this work with LSP?
    Import(ts::Node<'a>, String),
}

/// Describes the section of a Calyx program we are currently editing.
#[derive(Debug)]
pub enum Context {
    Toplevel,
    Component,
    Cells,
    Group,
    Wires,
    Control,
}

/// Transform an iterator of `treesit::Node` to `Range`.
pub trait NodeRangesIter<'a>: Iterator<Item = ts::Node<'a>> + Sized {
    fn ranges(self) -> impl Iterator<Item = Range> {
        self.map(Range::from)
    }
}

impl Document {
    /// Create an empty document for `url`.
    pub fn new(url: lspt::Url) -> Self {
        let mut parser = ts::Parser::new();
        parser.set_language(unsafe { tree_sitter_calyx() }).unwrap();
        Self {
            url,
            text: String::new(),
            tree: None,
            parser,
            components: HashMap::default(),
        }
    }

    /// Create a new document with `text` for `url`.
    pub fn new_with_text(url: lspt::Url, text: &str) -> Self {
        let mut doc = Self::new(url);
        doc.parse_whole_text(text);
        doc
    }

    /// Update the document with a with entirely new text.
    pub fn parse_whole_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.tree = self.parser.parse(text, None);
        self.update_component_map();
        log::Debug::update(
            "tree",
            self.tree.as_ref().unwrap().root_node().to_sexp(),
        )
    }

    /// Returns the root `treesit` node.
    pub fn root_node(&self) -> Option<ts::Node> {
        self.tree.as_ref().map(|t| t.root_node())
    }

    /// Translate a `byte_offset` into a `Point`.
    pub fn byte_to_point(&self, byte_offset: usize) -> Option<Point> {
        if byte_offset == 0 {
            Some(Point::zero())
        } else if byte_offset < self.text.len() {
            let portion = &self.text[..byte_offset];
            let lines = portion.lines();
            let line_num = lines.clone().count();
            let res = lines.last().map(|l| Point::new(line_num - 1, l.len()));
            log::stdout!("{byte_offset} -> {res:?}");
            res
        } else {
            None
        }
    }

    /// Compile `pattern` into a treesit query, run the query,
    /// and return a map of capture names to captured nodes.
    pub fn captures<'a, 'node: 'a>(
        &'a self,
        node: ts::Node<'node>,
        pattern: &str,
    ) -> HashMap<String, Vec<ts::Node>> {
        // create the struct that manages query state
        let mut cursor = ts::QueryCursor::new();
        // create the query from the passed in pattern
        let lang = unsafe { tree_sitter_calyx() };
        let query = ts::Query::new(lang, pattern)
            .unwrap_or_else(|err| panic!("Invalid Query:\n{}", err.message));
        // grab the @ capture names so that we can map idxes back to names
        let capture_names = query.capture_names();

        // run the query and gather the results in a map from capture names
        // to the nodes they capture
        let mut map = HashMap::default();

        // initialize all the capture names so that it's always safe
        // to query the map for a name that shows up in a pattern
        for name in capture_names {
            map.insert(name.to_string(), vec![]);
        }

        for qmatch in cursor.matches(&query, node, self.text.as_bytes()) {
            for capture in qmatch.captures {
                map.entry(capture_names[capture.index as usize].to_string())
                    .and_modify(|e: &mut Vec<ts::Node>| {
                        e.extend(&[capture.node])
                    })
                    .or_insert(vec![capture.node]);
            }
        }
        map
    }

    /// Update the component map for this document.
    fn update_component_map(&mut self) {
        self.components = self
            .root_node()
            .into_iter()
            .flat_map(|root| {
                // capture relevant sections of every component
                // in this file
                let map = self.captures(
                    root,
                    r#"(component (ident) @comp
                         (signature (io_port_list) @inputs
                                    (io_port_list) @outputs)
                         (cells) @cells
                         (wires) @wires)"#,
                );

                // create an iterator over all the captured nodes.
                // we are guaranteed that there will be the same
                // number of each of these
                multizip((
                    map["comp"].iter(),
                    map["inputs"].iter(),
                    map["outputs"].iter(),
                    map["cells"].iter(),
                    map["wires"].iter(),
                ))
                .map(|(comp, inputs, outputs, cells, wires)| {
                    (
                        // the name of the component
                        self.node_text(comp).to_string(),
                        // construct the component info from captured nodes
                        ComponentInfo {
                            signature: ComponentSig {
                                inputs: self.captures(*inputs, "(ident) @id")["id"]
                                    .iter()
                                    .map(|n| self.node_text(n).to_string())
                                    .collect(),
                                outputs: self.captures(*outputs, "(ident) @id")["id"]
                                    .iter()
                                    .map(|n| self.node_text(n).to_string())
                                    .collect(),
                            },
                            cells: {
                                let cells = self.captures(
                                    *cells,
                                    "(cell_assignment (ident) @name (instantiation (ident) @cell))",
                                );
                                multizip((cells["name"].iter(), cells["cell"].iter()))
                                    .map(|(name, cell)| {
                                        (
                                            self.node_text(name).to_string(),
                                            self.node_text(cell).to_string(),
                                        )
                                    })
                                    .collect()
                            },
                            groups: self.captures(*wires, "(group (ident) @id)")["id"]
                                .iter()
                                .map(|n| self.node_text(n).to_string())
                                .collect(),
                        },
                    )
                })
                .collect_vec()
            })
            .collect();
    }

    /// Return an iterator over components or primitives
    pub fn components(&self) -> impl Iterator<Item = ts::Node> {
        self.root_node().into_iter().flat_map(|root| {
            self.captures(
                root,
                "(component (ident) @comp) (primitive (ident) @comp)",
            )["comp"]
                .clone()
        })
    }

    /// Find all the names of cells for the component that contains `node`
    pub fn enclosing_cells<'a>(
        &'a self,
        node: ts::Node<'a>,
    ) -> impl Iterator<Item = ts::Node<'a>> {
        node.parent_until(|n| n.kind() == "component")
            .into_iter()
            .flat_map(|comp_node| {
                // XXX: should be able to avoid this clone somehow
                self.captures(comp_node, "(cell_assignment (ident) @cell)")
                    ["cell"]
                    .clone()
            })
    }

    /// Find all group names for the component that contains `node`
    pub fn enclosing_groups<'a>(
        &'a self,
        node: ts::Node<'a>,
    ) -> impl Iterator<Item = ts::Node<'a>> {
        node.parent_until(|n| n.kind() == "component")
            .into_iter()
            .flat_map(|comp_node| {
                self.captures(comp_node, "(group (ident) @group)")["group"]
                    .clone()
            })
    }

    /// Find all input/output ports for the component that contains `node`
    pub fn enclosing_component_ports<'a>(
        &'a self,
        node: ts::Node<'a>,
    ) -> impl Iterator<Item = ts::Node<'a>> {
        node.parent_until(|n| n.kind() == "component")
            .into_iter()
            .flat_map(|comp_node| {
                self.captures(comp_node, "(io_port (ident) @port)")["port"]
                    .clone()
            })
    }

    /// Find the name of the component that contains `node`
    pub fn enclosing_component_name(&self, node: ts::Node) -> Option<String> {
        node.parent_until(|n| n.kind() == "component")
            .and_then(|comp_node| {
                self.captures(comp_node, "(component (ident) @name)")["name"]
                    .first()
                    .map(|n| self.node_text(n).to_string())
            })
    }

    /// Return the list of imported files
    pub fn raw_imports(&self) -> Vec<String> {
        self.tree
            .as_ref()
            .iter()
            .flat_map(|t| {
                self.captures(t.root_node(), "(import (string) @file)")["file"]
                    .clone()
            })
            // the nodes have quotes in them, so we have to remove them
            .map(|n| self.node_text(&n).to_string().replace('"', ""))
            .collect()
    }

    /// Resolve the imports into full paths
    pub fn resolved_imports<'a>(
        &'a self,
        config: &'a Config,
    ) -> impl Iterator<Item = PathBuf> + 'a {
        let lib_paths = &config.calyx_lsp.library_paths;
        let cur_dir = self
            .url
            .to_file_path()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        self.raw_imports()
            .into_iter()
            .cartesian_product(
                vec![cur_dir]
                    .into_iter()
                    .chain(lib_paths.iter().map(PathBuf::from)),
            )
            .map(|(base_path, lib_path)| {
                lib_path.join(base_path).resolve().into_owned()
            })
            .filter(|p| p.exists())
    }

    /// Return signatures for all components
    pub fn signatures(
        &self,
    ) -> impl Iterator<Item = (String, ComponentSig)> + '_ {
        self.components()
            .filter_map(|comp_node| {
                comp_node
                    .parent_until_names(&["component", "primitive"])
                    .map(|p| (p, self.node_text(&comp_node)))
            })
            .flat_map(move |(comp_node, name)| {
                let mut map = self.captures(
                    comp_node,
                    "(signature (io_port_list) @inputs (io_port_list) @outputs)",
                );
                multizip((
                    map.remove("inputs").unwrap().into_iter(),
                    map.remove("outputs").unwrap().into_iter(),
                ))
                .map(move |(inputs, outputs)| {
                    (
                        name.to_string(),
                        ComponentSig {
                            inputs: self.captures(inputs, "(io_port (ident) @id . (_))")["id"]
                                .iter()
                                .map(|n| self.node_text(n).to_string())
                                .collect(),
                            outputs: self.captures(outputs, "(io_port (ident) @id . (_))")["id"]
                                .iter()
                                .map(|n| self.node_text(n).to_string())
                                .collect(),
                        },
                    )
                })
            })
    }

    /// Find the treesit node at `point`
    pub fn node_at_point(&self, point: &Point) -> Option<ts::Node> {
        self.root_node().and_then(|root| {
            root.descendant_for_point_range(
                point.clone().into(),
                point.clone().into(),
            )
        })
    }

    /// Find the semantic thing that is under `point`
    pub fn thing_at_point(&self, point: Point) -> Option<Things> {
        self.node_at_point(&point).and_then(|node| {
            if node.parent().is_some_and(|p| p.kind() == "port") {
                // when our parent is a port and we have a next sibling
                // we are looking at a cell. if we don't have a next
                // sibling, we are looking at a port on our current component
                if node.next_sibling().is_some() {
                    Some(Things::Cell(node, self.node_text(&node).to_string()))
                } else if node.prev_sibling().is_none() {
                    Some(Things::SelfPort(
                        node,
                        self.node_text(&node).to_string(),
                    ))
                } else {
                    None
                }
            } else if node.parent().is_some_and(|p| p.kind() == "enable") {
                // if we are in an enable control statement, we are looking
                // at a group
                Some(Things::Group(node, self.node_text(&node).to_string()))
            } else if node.parent().is_some_and(|p| p.kind() == "hole") {
                // if we are looking at the first part of a hole, we are looking
                // at a group name
                if node.next_sibling().is_some() {
                    Some(Things::Group(node, self.node_text(&node).to_string()))
                } else {
                    None
                }
            } else if node.parent().is_some_and(|p| p.kind() == "port_with") {
                // inside a control `with` statement, we are looking at a group
                Some(Things::Group(node, self.node_text(&node).to_string()))
            } else if node.parent().is_some_and(|p| p.kind() == "instantiation")
            {
                // inside a cell instantiation, we are looking at a component
                Some(Things::Component(self.node_text(&node).to_string()))
            } else if node.parent().is_some_and(|p| p.kind() == "import") {
                // inside an import, we are ofc looking at an import
                Some(Things::Import(
                    node,
                    self.node_text(&node).to_string().replace('"', ""),
                ))
            } else {
                None
            }
        })
    }

    /// Find the context of the thing at point
    pub fn context_at_point(&self, point: &Point) -> Context {
        // to find the context at point, we can't just find the node and
        // infer context based on a relevant parent because incomplete
        // parse trees will sometimes place error nodes outside the
        // context we expect. for this reason, we find ranges for
        // relevant sections, and check if they contain the point
        self.node_at_point(point)
            .and_then(|n| {
                // if `n` is a component. we want to capture things
                // from `n`. otherwise, we find the parent component,
                // and capture things from there
                if n.kind() == "component" {
                    Some(n)
                } else {
                    n.parent_until_names(&["component"])
                }
            })
            .map(|comp| {
                let map = self.captures(
                    comp,
                    "(cells) @cells (wires (wires_inner (group) @group)) @wires (control) @control",
                );
                if map["cells"].contains(point) {
                    Context::Cells
                } else if map["group"].contains(point) {
                    Context::Group
                } else if map["wires"].contains(point) {
                    Context::Wires
                } else if map["control"].contains(point) {
                    Context::Control
                } else if Range::from(comp).contains(point) {
                    Context::Component
                } else {
                    Context::Toplevel
                }
            })
            .unwrap_or(Context::Toplevel)
    }

    /// Find the last complete word ending at `point`
    pub fn last_word_from_point(&self, point: &Point) -> Option<String> {
        let re = Regex::new(r"\b\w+\b").unwrap();
        self.text.lines().nth(point.row()).and_then(|cur_line| {
            let rev_line = cur_line[0..point.column()]
                .chars()
                .rev()
                .collect::<String>();
            re.find(&rev_line)
                .map(|m| m.as_str().chars().rev().collect::<String>())
        })
    }

    /// Return text string for `node`.
    pub fn node_text(&self, node: &ts::Node) -> &str {
        node.utf8_text(self.text.as_bytes()).unwrap()
    }
}
