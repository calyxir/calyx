use std::path::PathBuf;

use tower_lsp::lsp_types as lspt;
use tree_sitter as ts;

use crate::{
    Config,
    convert::Range,
    document::{Document, Things},
    query_result::QueryResult,
    ts_utils::ParentUntil,
};

#[derive(Clone, Debug)]
struct PortInfo {
    name: String,
    width: String,
    direction: PortDirection,
    attributes: Vec<String>,
}

#[derive(Clone, Debug)]
enum PortDirection {
    Input,
    Output,
}

impl PortDirection {
    fn label(&self) -> &'static str {
        match self {
            PortDirection::Input => "Input",
            PortDirection::Output => "Output",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SignatureInfo {
    kind: String,
    modifier: Option<String>,
    attributes: Option<String>,
    params: Vec<String>,
    inputs: Vec<PortInfo>,
    outputs: Vec<PortInfo>,
}

impl SignatureInfo {
    fn format_signature(&self, name: &str) -> String {
        let modifier = self
            .modifier
            .as_ref()
            .map(|modifier| format!("{} ", modifier))
            .unwrap_or_default();
        let attributes = self.attributes.as_deref().unwrap_or_default();
        let params = if self.params.is_empty() {
            String::new()
        } else {
            format!("[{}]", self.params.join(", "))
        };
        let suffix = if self.kind == "primitive" {
            ";".to_string()
        } else {
            " {}".to_string()
        };

        format!(
            "{}{} {}{}{}{} -> {}{}",
            modifier,
            self.kind,
            name,
            attributes,
            params,
            Self::format_port_list(&self.inputs),
            Self::format_port_list(&self.outputs),
            suffix,
        )
    }

    fn format_port_list(ports: &[PortInfo]) -> String {
        if ports.is_empty() {
            "()".to_string()
        } else {
            format!(
                "(\n{}\n)",
                ports
                    .iter()
                    .enumerate()
                    .map(|(index, port)| {
                        let comma =
                            if index + 1 == ports.len() { "" } else { "," };
                        format!("  {}{}", port.declaration(), comma)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    }

    fn find_port(&self, port_name: &str) -> Option<&PortInfo> {
        self.inputs
            .iter()
            .find(|port| port.name == port_name)
            .or_else(|| self.outputs.iter().find(|port| port.name == port_name))
    }

    fn kind_label(&self, name: &str) -> String {
        match self.kind.as_str() {
            "primitive" => format!("Primitive `{}`", name),
            "component" => format!("Component `{}`", name),
            other => format!("{} `{}`", other, name),
        }
    }

    fn component_markdown(&self, name: &str) -> String {
        format!(
            "### {}\n\n```calyx\n{}\n```\n",
            self.kind_label(name),
            self.format_signature(name),
        )
    }
}

impl PortInfo {
    fn declaration(&self) -> String {
        let attributes = if self.attributes.is_empty() {
            String::new()
        } else {
            format!("{} ", self.attributes.join(" "))
        };
        format!("{}{}: {}", attributes, self.name, self.width)
    }
}

#[derive(Clone, Debug)]
pub struct Hover {
    contents: lspt::HoverContents,
    range: lspt::Range,
}

impl Hover {
    fn new(value: String, range: lspt::Range) -> Self {
        Self {
            contents: lspt::HoverContents::Markup(lspt::MarkupContent {
                kind: lspt::MarkupKind::Markdown,
                value,
            }),
            range,
        }
    }

    fn calyx(title: String, value: String, range: lspt::Range) -> Self {
        Self {
            contents: lspt::HoverContents::Array(vec![
                lspt::MarkedString::String(format!("{}\n\n ", title)),
                lspt::MarkedString::LanguageString(lspt::LanguageString {
                    language: "calyx".to_string(),
                    value,
                }),
            ]),
            range,
        }
    }
}

impl From<Hover> for lspt::Hover {
    fn from(value: Hover) -> Self {
        lspt::Hover {
            contents: value.contents,
            range: Some(value.range),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum HoverQuery {
    Component {
        name: String,
        range: lspt::Range,
    },
    Port {
        component: String,
        port: String,
        range: lspt::Range,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum HoverRes {
    Found(Hover),
    Continue(Vec<PathBuf>, HoverQuery),
}

impl QueryResult for HoverRes {
    type Data = Hover;
    type Needle = HoverQuery;

    fn found(&self) -> Option<Self::Data> {
        match self {
            HoverRes::Found(data) => Some(data.clone()),
            HoverRes::Continue(..) => None,
        }
    }

    fn paths(&self) -> Vec<PathBuf> {
        match self {
            HoverRes::Found(_) => vec![],
            HoverRes::Continue(paths, _) => paths.clone(),
        }
    }

    fn resume(&self, config: &Config, doc: &Document) -> Option<Self> {
        match self {
            HoverRes::Found(_) => Some(self.clone()),
            HoverRes::Continue(_, HoverQuery::Component { name, range }) => doc
                .signature_info(name)
                .map(|signature| {
                    HoverRes::Found(Hover::new(
                        signature.component_markdown(name),
                        *range,
                    ))
                })
                .or_else(|| {
                    Some(HoverRes::Continue(
                        doc.resolved_imports(config).collect(),
                        HoverQuery::Component {
                            name: name.clone(),
                            range: *range,
                        },
                    ))
                }),
            HoverRes::Continue(
                _,
                HoverQuery::Port {
                    component,
                    port,
                    range,
                },
            ) => doc
                .signature_info(component)
                .and_then(|sig| {
                    sig.find_port(port)
                        .map(|port| HoverRes::Found(port_hover(port, *range)))
                })
                .or_else(|| {
                    Some(HoverRes::Continue(
                        doc.resolved_imports(config).collect(),
                        HoverQuery::Port {
                            component: component.clone(),
                            port: port.clone(),
                            range: *range,
                        },
                    ))
                }),
        }
    }
}

pub trait HoverProvider {
    fn hover(
        &self,
        point: crate::convert::Point,
        config: &Config,
    ) -> Option<HoverRes>;
    fn signature_info(&self, component_name: &str) -> Option<SignatureInfo>;
}

impl HoverProvider for Document {
    fn hover(
        &self,
        point: crate::convert::Point,
        config: &Config,
    ) -> Option<HoverRes> {
        self.thing_at_point(point).and_then(|thing| match thing {
            Things::Cell(node, name) => {
                let cell_type = self.cell_component_name(node, &name)?;
                let range = Range::from(node).into();
                let args = self.cell_instantiation_args(node, &name);
                Some(HoverRes::Found(Hover::new(
                    cell_markdown(&name, &cell_type, args.as_deref()),
                    range,
                )))
            }
            Things::SelfPort(node, name) => {
                let component_name = self.enclosing_component_name(node)?;
                Some(self.port_query(
                    config,
                    component_name,
                    name,
                    Range::from(node).into(),
                ))
            }
            Things::CellPort(node, cell_name, port_name) => {
                let cell_type = self.cell_component_name(node, &cell_name)?;
                Some(self.port_query(
                    config,
                    cell_type,
                    port_name,
                    Range::from(node).into(),
                ))
            }
            Things::Component(node, name) => Some(self.component_query(
                config,
                name,
                Range::from(node).into(),
            )),
            Things::Group(node, name) => Some(HoverRes::Found(Hover::new(
                format!("### Group `{}`", name),
                Range::from(node).into(),
            ))),
            Things::Import(node, name) => self
                .resolved_import_path(config, &name)
                .map(|path| {
                    HoverRes::Found(Hover::new(
                        format!(
                            "### Import\nResolved to: `{}`",
                            path.display()
                        ),
                        Range::from(node).into(),
                    ))
                })
                .or_else(|| {
                    Some(HoverRes::Found(Hover::new(
                        format!("### Import\nPath: `{}`", name),
                        Range::from(node).into(),
                    )))
                }),
        })
    }

    fn signature_info(&self, component_name: &str) -> Option<SignatureInfo> {
        self.components()
            .find(|component| self.node_text(component) == component_name)
            .and_then(|component| {
                component.parent_until_names(&["component", "primitive"])
            })
            .and_then(|component| {
                let kind = component.kind().to_string();
                let mut map = self.captures(
                    component,
                    "(signature (io_port_list) @inputs (io_port_list) @outputs)",
                );
                let inputs = map.remove("inputs")?.into_iter().next()?;
                let outputs = map.remove("outputs")?.into_iter().next()?;
                Some(SignatureInfo {
                    kind,
                    modifier: self.direct_child_text(component, "comb_or_static"),
                    attributes: self.direct_child_text(component, "attributes"),
                    params: self
                        .captures(component, "(params (ident) @param)")["param"]
                        .iter()
                        .map(|param| self.node_text(param).to_string())
                        .collect(),
                    inputs: self.port_info(inputs, PortDirection::Input),
                    outputs: self.port_info(outputs, PortDirection::Output),
                })
            })
    }
}

impl Document {
    fn direct_child_text(&self, node: ts::Node, kind: &str) -> Option<String> {
        let mut cursor = node.walk();
        let child = node
            .named_children(&mut cursor)
            .find(|child| child.kind() == kind);
        child.map(|child| self.node_text(&child).to_string())
    }

    fn port_info(
        &self,
        node: ts::Node,
        direction: PortDirection,
    ) -> Vec<PortInfo> {
        self.captures(node, "(io_port) @port")["port"]
            .iter()
            .filter_map(|port| {
                let map = self.captures(
                    *port,
                    "(io_port (ident) @name . (_) @width)",
                );
                let name = map["name"].first()?;
                let width = map["width"].first()?;
                Some(PortInfo {
                    name: self.node_text(name).to_string(),
                    width: self.node_text(width).to_string(),
                    direction: direction.clone(),
                    attributes: self.captures(*port, "(at_attribute) @attribute")
                        ["attribute"]
                        .iter()
                        .map(|attribute| self.node_text(attribute).to_string())
                        .collect(),
                })
            })
            .collect()
    }

    fn cell_component_name(
        &self,
        node: ts::Node,
        cell_name: &str,
    ) -> Option<String> {
        self.enclosing_component_name(node)
            .and_then(|component_name| {
                self.components.get(&component_name).and_then(|component| {
                    component.cells.get(cell_name).cloned()
                })
            })
    }

    fn resolved_import_path(
        &self,
        config: &Config,
        import_name: &str,
    ) -> Option<PathBuf> {
        self.resolved_imports(config).find(|path| {
            path.file_name().is_some_and(|name| name == import_name)
        })
    }

    fn cell_instantiation_args(
        &self,
        node: ts::Node,
        cell_name: &str,
    ) -> Option<Vec<String>> {
        let component = node.parent_until(|n| n.kind() == "component")?;
        let names = self.captures(component, "(cell_assignment (ident) @name)")
            ["name"]
            .clone();
        let name_node = names
            .into_iter()
            .find(|name| self.node_text(name) == cell_name)?;
        let assignment = name_node.parent()?;
        Some(
            self.captures(
                assignment,
                "(instantiation (arg_list (number) @arg))",
            )["arg"]
                .iter()
                .map(|arg| self.node_text(arg).to_string())
                .collect(),
        )
    }

    fn component_query(
        &self,
        config: &Config,
        name: String,
        range: lspt::Range,
    ) -> HoverRes {
        self.signature_info(&name)
            .map(|signature| {
                HoverRes::Found(Hover::new(
                    signature.component_markdown(&name),
                    range,
                ))
            })
            .unwrap_or_else(|| {
                HoverRes::Continue(
                    self.resolved_imports(config).collect(),
                    HoverQuery::Component { name, range },
                )
            })
    }

    fn port_query(
        &self,
        config: &Config,
        component: String,
        port: String,
        range: lspt::Range,
    ) -> HoverRes {
        self.signature_info(&component)
            .and_then(|sig| {
                sig.find_port(&port)
                    .map(|port| HoverRes::Found(port_hover(port, range)))
            })
            .unwrap_or_else(|| {
                HoverRes::Continue(
                    self.resolved_imports(config).collect(),
                    HoverQuery::Port {
                        component,
                        port,
                        range,
                    },
                )
            })
    }
}

fn cell_markdown(
    cell_name: &str,
    cell_type: &str,
    args: Option<&[String]>,
) -> String {
    let instantiated_type = args
        .map(|args| format!("{}({})", cell_type, args.join(", ")))
        .unwrap_or_else(|| cell_type.to_string());
    format!(
        "### Cell `{}`\n\nType: `{}`\n",
        cell_name, instantiated_type
    )
}

fn port_hover(port: &PortInfo, range: lspt::Range) -> Hover {
    Hover::calyx(
        format!("### {} `{}`", port.direction.label(), port.name),
        port.declaration().trim().to_string(),
        range,
    )
}
