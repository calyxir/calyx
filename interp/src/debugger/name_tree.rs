use crate::structures::names::GroupQIN;
use std::collections::{HashMap, HashSet};

#[derive(Default, Debug)]
pub struct NameTreeRoot {
    roots: Vec<NameTree>,
}

impl NameTreeRoot {
    fn insert(&mut self, val: GroupQIN) {
        for root in self.roots.iter_mut() {
            if root.node.equal_prefix(&val) {
                self.roots.push(NameTree::new(val));
                return;
            } else if root.node.shared_prefix_of(&val) {
                return root.insert(val);
            }
        }
        self.roots.push(NameTree::new(val));
    }
}

struct NameTree {
    node: GroupQIN,
    children: Vec<NameTree>,
}

impl NameTree {
    fn new(val: GroupQIN) -> Self {
        Self {
            node: val,
            children: vec![],
        }
    }

    fn insert(&mut self, val: GroupQIN) {
        for child in self.children.iter_mut() {
            if child.node.shared_prefix_of(&val) {
                return child.insert(val);
            }
        }
        self.children.push(Self::new(val));
    }
}

impl std::fmt::Debug for NameTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NameTree")
            .field("node", &self.node.as_id())
            .field("children", &self.children)
            .finish()
    }
}

impl NameTreeRoot {
    pub fn new(active: HashSet<GroupQIN>) -> Self {
        let mut length_map: HashMap<usize, Vec<GroupQIN>> = HashMap::new();

        for group in active {
            length_map
                .entry(group.prefix_length())
                .or_default()
                .push(group)
        }

        // TODO (Griffin): Fix this for fully structural

        let mut output = Self::default();

        let mut ticker: usize = 1;

        while !length_map.is_empty() {
            if let Some(vs) = length_map.remove(&ticker) {
                for v in vs {
                    output.insert(v);
                }
            }
            ticker += 1
        }

        output
    }
}
