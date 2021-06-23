//a stack of environments, to be used like a version tree

//This version: linked list! from the rust book detailing how to do linked lists

use super::{primitives, primitives::Primitive, values::Value};
use calyx::ir;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryInto;
use std::rc::Rc;

//Invariant for use that makes implementing easier
//(this version can be used unsafely):
//After fork, a new_scope MUST be pushed. It's unsafe to continue modifying
//the root of the fork.

// From "Learning Rust with Entirely Too Many Linked Lists" (2018), Chapter 4.5:
pub struct List<T> {
    head: Link<T>,
}

type Link<T> = Option<Rc<Node<T>>>;

struct Node<T> {
    elem: T,
    next: Link<T>,
}

impl<T> List<T> {
    pub fn new() -> Self {
        List { head: None }
    }

    //renamed from "append" to "push"
    pub fn push(&self, elem: T) -> List<T> {
        List {
            head: Some(Rc::new(Node {
                elem,
                next: self.head.clone(), //hope this clone of an Option<Rc<U<T>>> is ok
            })),
        }
    }

    //List w/o its head. Don't need it for this DS, but including it for practice
    pub fn tail(&self) -> List<T> {
        List {
            head: self.head.as_ref().and_then(|node| node.next.clone()),
        }
    }

    pub fn head(&self) -> Option<&T> {
        self.head.as_ref().map(|node| &node.elem)
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            next: self.head.as_deref(),
        }
    }
}

pub struct Iter<'a, T> {
    next: Option<&'a Node<T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.map(|node| {
            self.next = node.next.as_deref();
            &node.elem
        })
    }
}
//

//When a new scope is added, head is added to the tail and becomes immutable
struct Smoosher<K: Eq + std::hash::Hash, V> {
    head: HashMap<K, V>,       //mutable
    tail: List<HashMap<K, V>>, //read-only
}

impl<K: Eq + std::hash::Hash, V> Smoosher<K, V> {
    fn new() -> Smoosher<K, V> {
        Smoosher {
            head: HashMap::new(),
            tail: List::new(),
        }
    }

    //Gets the highest-scoped binding containing [k], if it exists.
    //else returns None.
    fn get(&self, k: &K) -> Option<&V> {
        //first check if it's in the highest one
        if let Some(val) = self.head.get(k) {
            return Some(val);
        } else {
            let iter = self.tail.iter();
            for hm in iter {
                if let Some(val) = hm.get(k) {
                    return Some(val);
                }
            }
            //then check if it's anywhere in the other list
            return None;
        }
    }

    fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        todo!()
    }

    fn set(&mut self, k: K, v: V) {
        todo!()
    }

    fn fork(&self) -> Self {
        todo!()
    }

    fn new_scope(&mut self) {
        todo!()
    }

    fn top(&self) -> &Rc<RefCell<HashMap<K, V>>> {
        todo!()
    }

    fn smoosh(&mut self, top_i: u64, bottom_i: u64) -> () {
        todo!()
    }

    fn merge(&mut self, other: &mut Self) -> Self {
        todo!()
    }

    fn num_scopes(&self) -> u64 {
        todo!()
    }

    fn num_bindings(&self) -> u64 {
        todo!()
    }

    fn list_bound_vars(&self, top_i: u64, bottom_i: u64) -> HashSet<&K> {
        todo!()
    }

    fn diff(&self, top_i: u64, bottom_i: u64) -> Vec<(K, V)> {
        todo!()
    }
}
