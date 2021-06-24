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
#[derive(Debug)]
pub struct List<T> {
    head: Link<T>,
}

type Link<T> = Option<Rc<Node<T>>>;

//a way to unwrap and grab mutably?
//
#[derive(Debug)]
struct Node<T> {
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
    pub fn new() -> Self {
        List { head: None }
    }

    pub fn is_empty(&self) -> bool {
        if let Some(node) = &self.head {
            false
        } else {
            true
        }
    }

    /// Returns a list identical to [self], with [elem] pushed onto the front
    pub fn push(&self, elem: T) -> List<T> {
        List {
            head: Some(Rc::new(Node {
                elem,
                next: self.head.clone(),
            })),
        }
    }

    /// Returns a list identical to [self], with its head pointing to the second elem of [self]
    pub fn tail(&self) -> List<T> {
        List {
            head: self.head.as_ref().and_then(|node| node.next.clone()),
        }
    }

    /// Consumes the current list, returning an Option of the
    /// element contained in its [head] and its [tail].
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

    /// Returns an Option of a pointer to the head of this list
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
#[derive(Debug)]
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
    /// Returns an Option of a pointer to the highest-scoped binding to [k],
    /// if it exists. Els None.
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

    /// Sets a new binding of [k] to [v] in the highest scope
    fn set(&mut self, k: K, v: V) {
        self.head.insert(k, v);
    }

    /// Returns a new Smoosher and mutates [self]. The new Smoosher has a new scope
    /// as [head] and all of (pre-mutation) [self] as [tail]. [Self] has a fresh scope pushed onto
    /// it. Invariant this method enforces: you cannot mutate a scope that has children
    /// forks.
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

    /// Returns a pointer to the newest scope of this Smoosher. A Smoosher upon
    /// instantiation will always have 1 empty HM as its first scope, so this
    /// method will always meaningfully return.
    fn top(&self) -> &HashMap<K, V> {
        &self.head
    }

    /// Returns a Smoosher with all bindings in the topmost scope transposed
    /// onto the second-newest scope, with the topmost scope then discarded.
    /// Invariant: [self] must have at least 2 scopes, and neither scope may
    /// be the root of any fork:
    ///* [A]   [B]
    ///   |     |
    ///    \   /
    ///     [C]
    /// Cannot smoosh A into B -- must first MERGE A and B, then smoosh the resulting
    /// node into C
    ///* [A]   [B]
    ///   |     |
    ///    \   /
    ///     [C]
    ///      |
    ///     [D]
    /// Cannot smoosh C into D (not even possible with this method)
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

    /// Applies all bindings found in the top [levels] scopes to the [levels]th
    /// scope. [smoosh(0)] has no effect, while [smoosh(1)] is equivalent to
    /// [smoosh_once].
    /// Invariant: None of the top [levels] scopes may be the root of any fork
    /// Required: [levels] >= # of scopes in the Smoosher
    fn smoosh(self, levels: u64) -> Self {
        let mut tr = self;
        for n in 0..levels {
            tr = tr.smoosh_once();
        }
        tr
    }

    /// For internal use only
    /// Set [new] as the topmost scope of [self]
    fn push_scope(&mut self, new: HashMap<K, V>) {
        let old_head = mem::replace(&mut self.head, new);
        self.tail = self.tail.push(old_head);
    }

    /// Consumes two Smooshers, which must have the same # of scopes above
    /// their shared fork point (no check is performed on this).
    /// Merges their topmost scope, smooshing that
    /// resulting scope onto the new head of [self]. Returns a tupule of the
    /// new self, and [other] with its head removed. Best described by example:
    /// Smoosher 1: (A, B, E) Smoosher 2: (B, D, E)          
    /// [A]        [B]
    ///  |          |
    /// [C]        [D]
    ///  |          |
    ///   \        /
    ///    \      /
    ///      [E]
    /// *merge_once(A, B)
    /// Smoosher 1: (AmB onto C, E), Smoosher 2: (D, E)
    /// [Smoosh (AmB) onto C]        [D]
    ///  |                            |
    ///   \                          /
    ///    \                        /
    ///                 [E]    
    /// * IMPORTANT INVARIANT: The intersection of the bindings of the two topmost
    /// scopes MUST be the empty set!   
    fn merge_once(self, other: Self) -> (Self, Self) {
        //get both heads of [self] and [other]
        let mut a_head = self.head;
        let b_head = other.head;
        let (a_new_head, a_new_tail) = self.tail.split();
        let (b_new_head, b_new_tail) = other.tail.split();
        //create A' and B' from the tails we got above
        let mut a = Smoosher::new();
        let mut b = Smoosher::new();
        if let Some(mut a_new_head) = a_new_head {
            a = Smoosher {
                head: a_new_head,
                tail: a_new_tail,
            };
        } else {
            panic!("trying to merge, but [self] is empty")
        }
        if let Some(b_new_head) = b_new_head {
            b = Smoosher {
                head: b_new_head,
                tail: b_new_tail,
            };
        } else {
            panic!("trying to merge, but [other] is empty")
        }
        //merge a_head and b_head.
        //here is why it's important they don't have overlapping writes
        for (k, v) in b_head {
            a_head.insert(k, v);
        }
        //push_scope this new merged node onto A'
        a.push_scope(a_head);
        //smoosh the new scope down one
        //return A' and B'
        (a.smoosh_once(), b)
    }

    //hard to do because we don't know until when to merge... I guess
    //that constant pointer stuff that compares pointers to see if they
    //share a node?
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
