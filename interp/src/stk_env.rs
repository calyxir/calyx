//a stack of environments, to be used like a version tree
//This version: linked list! from the rust book detailing how to do linked lists

use super::{primitives, primitives::Primitive, values::Value};
use calyx::ir;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryInto;
use std::mem;
use std::rc::Rc;

//Invariants:
//=> To Smoosh X levels down, no fork may still exist from any node amongst those
// X levels. So the stack must look like:
//  _
// |_|
//  |
//  _
// |_|
//  |
//  _
// |_|
//  |
//=> To merge, both branches must be X levels from their shared root
//  _     _
// |_|   |_|
//  |     |
//   \ _ /
//    |*|
//     |
//

// From "Learning Rust with Entirely Too Many Linked Lists" (2018), Chapter 4.5:
#[derive(Default, Debug)]
pub struct List<T> {
    head: Link<T>,
}

type Link<T> = Option<Rc<Node<T>>>;

//a way to unwrap and grab mutably?
//
#[derive(Debug)]
struct Node<T> {
    //problem: node owns its element (? is this a problem)
    elem: T,
    next: Link<T>,
}

impl<T> Clone for List<T> {
    fn clone(&self) -> Self {
        List {
            head: self.head.clone(),
        }
    }
}

impl<T> List<T> {
    pub fn is_empty(&self) -> bool {
        if let Some(node) = &self.head {
            false
        } else {
            true
        }
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

    ///List w/o its head.
    pub fn tail(&self) -> List<T> {
        List {
            head: self.head.as_ref().and_then(|node| node.next.clone()),
        }
    }

    ///Hacky method that consumes the current list, returning
    ///its head (if it exists), and a list with that head removed (or empty)
    pub fn split(self) -> (Option<T>, Self) {
        if self.is_empty() {
            return (None, self);
        } else {
            if let Ok(head) = Rc::try_unwrap(self.head.unwrap()) {
                let tail_list = List { head: head.next }; //better: not cloning the tail
                return (Some(head.elem), tail_list);
            } else {
                panic!("Cannot unwrap the head of this list. You probably tried Smooshing while a fork exists!")
            }
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
#[derive(Default, Debug)]
struct Smoosher<K: Eq + std::hash::Hash, V> {
    head: HashMap<K, V>,       //mutable
    tail: List<HashMap<K, V>>, //read-only
}

impl<K: Eq + std::hash::Hash, V> Smoosher<K, V> {
    ///Gets the highest-scoped binding containing [k], if it exists.
    ///else returns None.
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

    ///doesn't seem possible rn. if you want to edit the binding of a key in a previous
    ///scope... not sure how you could
    fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        todo!()
    }

    ///Sets a new binding of [k] to [v] in the highest scope
    fn set(&mut self, k: K, v: V) {
        self.head.insert(k, v);
    }

    ///Returns a new Smoosher and mutates [self]. The new Smoosher has a new scope
    ///as [head] and all of [self] as [tail]. [Self] has a fresh scope pushed onto
    ///it. Invariant this method enforces: you cannot mutate a scope that has children
    ///forks.
    fn fork(&mut self) -> Self {
        //first save self's head, and replace it with a clean HM
        let old_head = mem::replace(&mut self.head, HashMap::new()); //can replace with mem::take()
                                                                     //create tail to both Self and the new Smoosher
        let new_tail = self.tail.push(old_head);
        //update Self and create new Smoosher
        self.tail = new_tail.clone(); //will this die after the end of [fork]? no
        Smoosher {
            head: HashMap::new(),
            tail: new_tail,
        }
    }

    ///Pushes a new, empty scope onto [self]
    fn new_scope(&mut self) {
        let old_head = mem::replace(&mut self.head, HashMap::new());
        self.tail = self.tail.push(old_head);
    }

    //maybe not necessary VV

    ///Returns a pointer to the newest scope of this Smoosher. A Smoosher upon
    ///instantiation will always have 1 empty HM as its first scope, so this
    ///method will always return.
    fn top(&self) -> &HashMap<K, V> {
        &self.head
    }

    /// Returns a Smoosher
    fn smoosh_once(self) -> Self {
        //move head to a sep variable
        let wr_head = self.head;
        //now move the head of the tail into the head of the smoosher
        let interm_tail = self.tail;
        let (new_head, new_tail) = interm_tail.split();
        if let Some(mut new_head) = new_head {
            for (k, v) in wr_head {
                new_head.insert(k, v);
            }
            return Smoosher {
                head: new_head,
                tail: new_tail,
            };
        } else {
            panic!()
        }
    }

    /// Transposes the bindings from the topmost [levels] HMs onto the HM
    /// [levels] deep down from the top (topmost HM considered to have index 0).
    /// Calling self.Smoosh(0) has no effect,
    /// calling self.Smoosh(1) will transpose the bindings from the topmost scope
    /// onto the scope directly below it. [Levels] is how many scopes to smoosh
    /// down. Calling [smoosh_lvl] with a [levels] greater than the number of
    /// scopes has undefined behavior.
    /// Required: There exist no forks from any of the topmost [levels] scopes.
    fn smoosh(self, levels: u64) -> Self {
        let upd_stk = List::<HashMap<K, V>>::default();
        todo!()
    }

    ///works on a two branches w/ exactly one layer.
    ///so both Smooshers have the same head of their [tail]
    ///
    fn merge_once(self, other: Self) -> Self {
        todo!()
    }

    fn merge(self, other: Self) -> Self {
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
