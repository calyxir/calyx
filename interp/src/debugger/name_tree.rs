use crate::structures::names::{
    ComponentQualifiedInstanceName, GroupQualifiedInstanceName,
};
use std::collections::{HashMap, HashSet};

pub struct ActiveTreeNode {
    pub node: GroupQualifiedInstanceName,
    pub children: Vec<ActiveTreeNode>,
}
impl ActiveTreeNode {
    pub fn new(node: GroupQualifiedInstanceName) -> Self {
        Self {
            node,
            children: vec![],
        }
    }
}
