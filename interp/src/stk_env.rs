//a stack of environments, to be used like a version tree

//This version: linked list! from the rust book detailing how to do linked lists

use super::{primitives, primitives::Primitive, values::Value};
use calyx::ir;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryInto;
use std::mem;
use std::rc::Rc; //got this from the "Learning Rust..." LL book

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
    //problem: node owns its element (? is this a problem)
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
    head: HashMap<K, V>,           //mutable
    tail: Rc<List<HashMap<K, V>>>, //read-only
}

impl<K: Eq + std::hash::Hash, V> Smoosher<K, V> {
    fn new() -> Smoosher<K, V> {
        Smoosher {
            head: HashMap::new(),
            tail: Rc::new(List::new()),
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

    //doesn't seem possible rn. if you want to edit the binding of a key in a previous
    //scope... not sure how you could
    fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        todo!()
    }

    //Sets a new binding of [k] to [v] in the highest scope
    fn set(&mut self, k: K, v: V) {
        self.head.insert(k, v);
    }

    //Returns a new Smoosher and mutates [self]. The new Smoosher has a new scope
    //as [head] and all of [self] as [tail]. [Self] has a fresh scope pushed onto
    //it. Invariant this method enforces: you cannot mutate a scope that has children
    //forks.
    fn fork(&mut self) -> Self {
        //first save self's head, and replace it with a clean HM
        let old_head = mem::replace(&mut self.head, HashMap::new());
        //create tail to both Self and the new Smoosher
        let new_tail = Rc::new(self.tail.push(old_head));
        //update Self and create new Smoosher
        self.tail = Rc::clone(&new_tail); //will this die after the end of [fork]?
        Smoosher {
            head: HashMap::new(),
            tail: Rc::clone(&new_tail),
        }
    }

    //Pushes a new, empty scope onto [self]
    fn new_scope(&mut self) {
        let old_head = mem::replace(&mut self.head, HashMap::new());
        let new_tail = Rc::new(self.tail.push(old_head));
        self.tail = Rc::clone(&new_tail);
    }

    //Returns a pointer to the newest scope of this Smoosher
    fn top(&self) -> &HashMap<K, V> {
        &self.head
    }

    fn smoosh(&mut self, top_i: u64, bottom_i: u64) -> () {
        todo!()
    }

    /// Transposes the bindings from the topmost [levels] HMs onto the HM
    /// [levels] deep down from the top. Calling self.Smoosh(0) has no effect,
    /// calling self.Smoosh(1) will transpose the bindings from the topmost scope
    /// onto the scope directly below it. [Levels] is how many scopes to smoosh
    /// down. Calling [smoosh_lvl] with a [levels] greater than the number of
    /// scopes has undefined behavior.
    fn smoosh_lvl(&mut self, levels: u64) {
        let upd_stk = List::<HashMap<K, V>>::new();
        //if levels >= 1, add head + (levels-1) from the top
        //other HMs to update_stack
        if levels >= 1 {
            let top = mem::replace(&mut self.head, HashMap::new());
            upd_stk.push(top);
            //now mutably borrow the other nodes and put them on upd_stk
            let iter = self.tail.iter();
            let mut count = levels;
            for hm in iter {
                if count > 0 {}
                count -= 1;
            }
        }
        //give a var name to target level

        //then iterate thru the update_stack, borrow the HMs mutable (they're in RCs)
        //do a lot of mem::replaces to move all bindings into target level
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
