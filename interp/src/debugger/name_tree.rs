use crate::structures::names::GroupQualifiedInstanceName;
use std::fmt::Write;

#[derive(Debug)]
pub struct ActiveTreeNode {
    name: GroupQualifiedInstanceName,
    children: Vec<ActiveTreeNode>,
}
impl ActiveTreeNode {
    pub fn new(node: GroupQualifiedInstanceName) -> Self {
        Self {
            name: node,
            children: vec![],
        }
    }

    pub fn insert(&mut self, node: ActiveTreeNode) {
        self.children.push(node);
    }

    pub fn format_tree<const TOP: bool>(&self, indent_level: usize) -> String {
        let mut out = String::new();
        write!(out, "{}", " ".repeat(indent_level)).unwrap();

        if TOP {
            write!(out, "{}", self.name.prefix.as_id()).unwrap();
        } else {
            write!(
                out,
                "{}",
                self.name.prefix.last().unwrap().component_id.name
            )
            .unwrap();
        }

        match &self.name.group {
            crate::structures::names::GroupName::Group(g) => {
                write!(out, "::{}", g).unwrap()
            }
            crate::structures::names::GroupName::Phantom(p) => {
                write!(out, "::<{}>", p).unwrap()
            }
            crate::structures::names::GroupName::None => {}
        }

        writeln!(out).unwrap();

        for child in self.children.iter() {
            let child_out = child.format_tree::<false>(indent_level + 2);
            write!(out, "{}", child_out).unwrap();
        }

        out
    }
}
