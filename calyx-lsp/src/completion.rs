use std::path::PathBuf;

use itertools::{multizip, Itertools};
use tower_lsp::lsp_types as lspt;

use crate::{
    convert::Point,
    document::{Context, Document},
    query_result::QueryResult,
    Config,
};

#[derive(Clone, Debug)]
pub struct CompletionItem {
    label: String,
    detail: String,
    snippet: Option<String>,
}

impl CompletionItem {
    fn simple<L, D>(label: L, detail: D) -> Self
    where
        L: ToString,
        D: ToString,
    {
        CompletionItem {
            label: label.to_string(),
            detail: detail.to_string(),
            snippet: None,
        }
    }

    fn snippet<L, D, S>(label: L, detail: D, snippet: S) -> Self
    where
        L: ToString,
        D: ToString,
        S: ToString,
    {
        Self {
            label: label.to_string(),
            detail: detail.to_string(),
            snippet: Some(snippet.to_string()),
        }
    }
}

impl From<CompletionItem> for lspt::CompletionItem {
    fn from(value: CompletionItem) -> Self {
        lspt::CompletionItem {
            label: value.label,
            detail: Some(value.detail),
            insert_text: value.snippet,
            insert_text_format: Some(lspt::InsertTextFormat::SNIPPET),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug)]
pub enum CompletionRes {
    Found(Vec<CompletionItem>),
    ContinuePort(Vec<PathBuf>, String),
    ContinueComponent(Vec<PathBuf>, Vec<CompletionItem>),
}

impl QueryResult for CompletionRes {
    type Data = Vec<CompletionItem>;
    type Needle = String;

    fn found(&self) -> Option<Self::Data> {
        match self {
            CompletionRes::Found(data) => Some(data.clone()),
            CompletionRes::ContinuePort(..)
            | CompletionRes::ContinueComponent(..) => None,
        }
    }

    fn paths(&self) -> Vec<PathBuf> {
        match self {
            CompletionRes::Found(_) => vec![],
            CompletionRes::ContinuePort(paths, _) => paths.clone(),
            CompletionRes::ContinueComponent(paths, _) => paths.clone(),
        }
    }

    fn resume(&self, config: &Config, doc: &Document) -> Option<Self> {
        match self {
            CompletionRes::Found(_) => Some(self.clone()),
            CompletionRes::ContinuePort(_, name) => doc
                .signatures()
                .find(|(n, _)| name == n)
                .map(|(_, sig)| {
                    Self::Found(
                        sig.inputs
                            .iter()
                            .map(|inp| (inp, "input"))
                            .chain(
                                sig.outputs.iter().map(|out| (out, "output")),
                            )
                            .map(|(name, descr)| {
                                CompletionItem::simple(name, descr)
                            })
                            .collect(),
                    )
                })
                .or_else(|| {
                    Some(Self::ContinuePort(
                        doc.resolved_imports(config).collect(),
                        name.to_string(),
                    ))
                }),
            CompletionRes::ContinueComponent(paths, compls) => {
                let mut imports =
                    doc.resolved_imports(config).collect::<Vec<_>>();
                let paths = &paths[1..];
                let comps_here = doc
                    .root_node()
                    .map(|root| {
                        let prims = doc.captures(
                            root,
                            "(primitive (ident) @name (params (ident) @param))",
                        );
                        let comps =
                            doc.captures(root, "(component (ident) @name)");
                        multizip((prims["name"].iter(), prims["param"].iter()))
                            .map(|(n, p)| (doc.node_text(n), doc.node_text(p)))
                            .group_by(|(n, _)| n.to_string())
                            .into_iter()
                            .map(|(n, p)| {
                                CompletionItem::snippet(
                                    n.to_string(),
                                    "primitive",
                                    format!(
                                        "{n}({});",
                                        p.enumerate()
                                            .map(|(i, (_, y))| format!(
                                                "${{{}:{y}}}",
                                                i + 1
                                            ))
                                            .join(", ")
                                    ),
                                )
                            })
                            .chain(compls.clone())
                            .chain(
                                comps["name"]
                                    .iter()
                                    .map(|n| doc.node_text(n))
                                    .map(|n| {
                                        CompletionItem::snippet(
                                            n,
                                            "component",
                                            format!("{n}();"),
                                        )
                                    }),
                            )
                            .collect()
                    })
                    .unwrap_or_default();
                Some(if paths.is_empty() && imports.is_empty() {
                    Self::Found(comps_here)
                } else {
                    imports.extend_from_slice(paths);
                    Self::ContinueComponent(imports, comps_here)
                })
            }
        }
    }
}

pub trait CompletionProvider {
    fn complete(
        &self,
        trigger_char: Option<&str>,
        point: &Point,
        config: &Config,
    ) -> Option<Vec<CompletionRes>>;
}

impl CompletionProvider for Document {
    fn complete(
        &self,
        trigger_char: Option<&str>,
        point: &Point,
        config: &Config,
    ) -> Option<Vec<CompletionRes>> {
        self.last_word_from_point(point).and_then(|word| {
            self.node_at_point(point).and_then(|node| {
                match (self.context_at_point(point), trigger_char) {
                    (Context::Toplevel, _) => {
                        Some(vec![CompletionRes::Found(vec![CompletionItem::snippet(
                            "component",
                            "block",
                            "component $1($2) -> ($3) {\n  cells {}\n  wires {}\n  control {}\n}",
                        )])])
                    }
                    (Context::Component, _) => None,
                    (Context::Cells, _) => Some(vec![
                        CompletionRes::Found(
                            self.components
                                .keys()
                                .map(|k| CompletionItem::snippet(k, "component", format!("{k}();")))
                                .collect(),
                        ),
                        CompletionRes::ContinueComponent(
                            self.resolved_imports(config).collect(),
                            vec![],
                        ),
                    ]),
                    (Context::Group, Some(".")) | (Context::Wires, Some(".")) => self
                        .enclosing_component_name(node)
                        .and_then(|comp_name| self.components.get(&comp_name))
                        .and_then(|ci| ci.cells.get(&word))
                        .and_then(|cell_name| {
                            self.components
                                .get(cell_name)
                                .map(|ci| {
                                    vec![CompletionRes::Found(
                                        ci.signature.inputs
                                            .iter()
                                            .map(|i| CompletionItem::simple(i, "input"))
                                            .chain(
                                                ci.signature.outputs
                                                    .iter()
                                                    .map(|o| CompletionItem::simple(o, "output")),
                                            )
                                            .collect(),
                                    )]
                                })
                                .or_else(|| {
                                    Some(vec![CompletionRes::ContinuePort(
                                        self.resolved_imports(config).collect(),
                                        cell_name.to_string(),
                                    )])
                                })
                        }),
                    (Context::Group, _) => self
                        .enclosing_component_name(node)
                        .and_then(|comp_name| self.components.get(&comp_name))
                        .map(|ci| {
                            vec![CompletionRes::Found(
                                ci.cells
                                    .keys()
                                    .map(|g| CompletionItem::simple(g, "cell"))
                                    .chain(ci.groups.iter().map(|g| {
                                        CompletionItem::snippet(g, "hole", format!("{g}[$1]"))
                                    }))
                                    .collect(),
                            )]
                        }),
                    (Context::Wires, _) => self
                        .enclosing_component_name(node)
                        .and_then(|comp_name| self.components.get(&comp_name))
                        .map(|ci| {
                            vec![CompletionRes::Found(
                                ci.cells
                                    .keys()
                                    .map(|g| CompletionItem::simple(g, "cell"))
                                    .collect(),
                            )]
                        }),

                    (Context::Control, _) => self
                        .enclosing_component_name(node)
                        .and_then(|comp_name| self.components.get(&comp_name))
                        .map(|ci| {
                            vec![CompletionRes::Found(
                                ci.groups
                                    .iter()
                                    .map(|g| CompletionItem::simple(g, "group"))
                                    .collect(),
                            )]
                        }),
                }
            })
        })
    }
}
