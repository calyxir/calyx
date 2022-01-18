use crate::structures::names::{
    ComponentQualifiedInstanceName, GroupQualifiedInstanceName,
};

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
}
