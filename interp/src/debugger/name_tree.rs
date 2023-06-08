use serde::Serialize;

use crate::structures::names::GroupQualifiedInstanceName;
use owo_colors::OwoColorize;
use std::{collections::HashSet, fmt::Write, iter::once};

#[derive(Debug, Clone)]
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
            write!(out, "{}", self.name.prefix.as_id().blue()).unwrap();
        } else if let crate::structures::names::GroupName::None =
            &self.name.group
        {
            write!(
                out,
                "{}",
                self.name.prefix.last().unwrap().instance.green()
            )
            .unwrap();
        } else {
            write!(
                out,
                "{}",
                self.name.prefix.last().unwrap().component_id.name.blue()
            )
            .unwrap();
        }

        match &self.name.group {
            crate::structures::names::GroupName::Group(g) => {
                write!(out, "::{}", g.blue()).unwrap()
            }
            crate::structures::names::GroupName::Phantom(p) => {
                write!(out, "::<{}>", p.magenta()).unwrap()
            }
            crate::structures::names::GroupName::None => {}
        }

        writeln!(out).unwrap();

        for child in self.children.iter() {
            let child_out = child.format_tree::<false>(indent_level + 2);
            write!(out, "{}", child_out.magenta()).unwrap();
        }

        out
    }

    pub fn flatten(self) -> ActiveVec {
        if self.name.is_leaf() {
            once(self.name)
                .chain(self.children.into_iter().flat_map(Self::flatten))
                .collect()
        } else {
            self.children.into_iter().flat_map(Self::flatten).collect()
        }
    }

    #[inline]
    pub fn flat_set(self) -> ActiveSet {
        self.flatten().into()
    }
}

#[derive(Debug)]
pub struct ActiveVec(Vec<GroupQualifiedInstanceName>);

impl From<Vec<GroupQualifiedInstanceName>> for ActiveVec {
    fn from(v: Vec<GroupQualifiedInstanceName>) -> Self {
        Self(v)
    }
}

impl FromIterator<GroupQualifiedInstanceName> for ActiveVec {
    fn from_iter<T: IntoIterator<Item = GroupQualifiedInstanceName>>(
        iter: T,
    ) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl IntoIterator for ActiveVec {
    type Item = GroupQualifiedInstanceName;

    type IntoIter = std::vec::IntoIter<GroupQualifiedInstanceName>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct ActiveSet(HashSet<(u64, String)>);

impl From<ActiveVec> for ActiveSet {
    fn from(v: ActiveVec) -> Self {
        Self(
            v.0.into_iter()
                .filter_map(|x| {
                    x.pos_tag.and_then(|tag| (tag, x.format_name()).into())
                })
                .collect(),
        )
    }
}

impl ActiveSet {
    pub fn iter(&self) -> impl Iterator<Item = &(u64, String)> {
        self.0.iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = (u64, String)> {
        self.0.into_iter()
    }
}
