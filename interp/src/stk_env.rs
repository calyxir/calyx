use super::{primitives, primitives::Primitive, values::Value};
use calyx::ir;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryInto;
use std::mem;
use std::rc::Rc;

// From "Learning Rust with Entirely Too Many Linked Lists" (2018), Chapter 4.5:
#[derive(Debug)]
pub struct List<T> {
    head: Link<T>,
}

type Link<T> = Option<Rc<Node<T>>>;

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

    /// Tests if the nodes at the head of [self] and [other] are equal;
    /// that is if the Rc points to the same location. Neither
    /// # Panics
    /// Panics if [self] or [other] are empty, though this should not occur
    /// when using Smooshers.
    /// # Example
    /// ```
    /// use interp::stk_env::List;
    /// let l1 = List::new().push(1).push(2);
    /// let l2 = l1.push(3);
    /// let l3 = l1.push(4);
    /// if let (Some(_), l2) = List::split(l2) {
    ///     if let (Some(_), l3) = List::split(l3) {
    ///         assert!(List::same_head(&l2, &l3));
    ///     }
    /// } else {
    ///    panic!("split gave None")
    /// }
    /// ```

    pub fn same_head(&self, other: &Self) -> bool {
        let self_head = self.head.as_ref().unwrap();
        let other_head = other.head.as_ref().unwrap();
        Rc::as_ptr(self_head) == Rc::as_ptr(other_head)
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
//=> To merge, both branches must share a common fork point
//  _
// |_|
//  |
//  _     _
// |_|   |_|
//  |     |
//   \ _ /
//    |*|
//     |
//
#[derive(Debug)]
pub struct Smoosher<K: Eq + std::hash::Hash, V: Eq> {
    head: HashMap<K, V>,       //mutable
    tail: List<HashMap<K, V>>, //read-only
}

impl<K: Eq + std::hash::Hash, V: Eq> Smoosher<K, V> {
    pub fn new() -> Smoosher<K, V> {
        Smoosher {
            head: HashMap::new(),
            tail: List::new(),
        }
    }

    /// If [self] and [other] share a fork point, returns a pair (depthA, depthB)
    /// of the depth which the fork point can be found in [self] and [other], respectively.
    /// NOTE: should be private, only public for testing!
    pub fn shared_fork_point(&self, other: &Self) -> Option<(u64, u64)> {
        //check head
        if std::ptr::eq(&self.head, &other.head) {
            return Some((0, 0));
        } else {
            //check tail
            //these start at 1 b/c head was 0
            let mut a_depth = 1;
            let mut b_depth = 1;
            //iterate and check all other nodes
            for nd in self.tail.iter() {
                b_depth = 1;
                for nd_other in other.tail.iter() {
                    if std::ptr::eq(nd, nd_other) {
                        return Some((a_depth, b_depth));
                    }
                    b_depth += 1;
                }
                a_depth += 1;
            }
            return None; //do not share a fork point
        }
    }
    //hacky?
    pub fn drop(self) {
        //yea
    }
    /// Returns an Option of a pointer to the highest-scoped binding to [k],
    /// if it exists. Els None.
    pub fn get(&self, k: &K) -> Option<&V> {
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
    pub fn set(&mut self, k: K, v: V) {
        self.head.insert(k, v);
    }

    /// Returns a new Smoosher and mutates [self]. The new Smoosher has a new scope
    /// as [head] and all of (pre-mutation) [self] as [tail]. [Self] has a fresh scope pushed onto
    /// it. Invariant this method enforces: you cannot mutate a scope that has children
    /// forks (you can only mutate the fresh scope applied atop it)
    pub fn fork(&mut self) -> Self {
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
    pub fn new_scope(&mut self) {
        let old_head = mem::replace(&mut self.head, HashMap::new());
        self.tail = self.tail.push(old_head);
    }

    //maybe not necessary VV

    /// Returns a pointer to the newest scope of this Smoosher. A Smoosher upon
    /// instantiation will always have 1 empty HM as its first scope, so this
    /// method will always meaningfully return.
    pub fn top(&self) -> &HashMap<K, V> {
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
    pub fn smoosh_once(self) -> Self {
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
    pub fn smoosh(self, levels: u64) -> Self {
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

    /// Consumes two Smooshers, which must share a fork point (else fn panics).
    /// Merges all topmost scopes above the shared fork point, smooshing the resulting
    /// node onto the fork point. Returns the resulting Smoosher. Example:
    /// Smoosher 1: (A, B, C, F, G) Smoosher 2: (D, E, F, G)          
    /// [A]
    ///  |
    /// [B]        [D]
    ///  |          |
    /// [C]        [E]
    ///  |          |
    ///   \        /
    ///    \      /
    ///      [F]
    ///       |
    ///      [G]
    /// *merge(Smoosher_1, Smoosher_2)
    /// Returns:
    ///      [ABCDE merged and smooshed onto F]
    ///       |
    ///      [F]
    /// * IMPORTANT INVARIANT: The intersection of the bindings of the two branches
    ///  MUST be the empty set! Otherwise, no guarantee for which binding will be included
    /// in the merge.
    pub fn merge(self, other: Self) -> Self {
        //find shared fork point; if doesn't exist, panic
        let mut a = self;
        let mut b = other;
        if let Some((depth_a, depth_b)) = Smoosher::shared_fork_point(&a, &b) {
            //smoosh [self] and [other] to right before that point
            a = a.smoosh(depth_a - 1);
            b = b.smoosh(depth_b - 1);

            //now do merge

            //get both heads of smooshed a and b
            let mut a_head = a.head;
            //merge a_head and b_head.
            for (k, v) in b.head {
                a_head.insert(k, v);
            }
            //now drop b
            std::mem::drop(b.tail); //b.head already consumed above
                                    //create a'
            let (a_new_head, a_new_tail) = a.tail.split();
            if let Some(mut a_new_head) = a_new_head {
                a = Smoosher {
                    head: a_new_head,
                    tail: a_new_tail,
                };
            } else {
                panic!("trying to merge, but [self] is empty")
            }
            //push_scope this new merged node onto A'
            a.push_scope(a_head);
            a.smoosh_once()
        } else {
            panic!("tried to merge Smooshers with no common fork point")
        }
    }

    /// Returns a HashSet of references to keys bounded in the
    /// top [levels] levels of the Smoosher.
    /// [self.list_bound_vars(0)] returns all keys in the topmost scope
    /// [self.list.bound_vars(1)] returns all keys in the top two scopes
    /// Undefined behavior if levels >= (# of scopes)
    pub fn list_bound_vars(&self, levels: u64) -> HashSet<&K> {
        let mut tr_hs = HashSet::new();
        //levels is at least 0, so add everything from head
        for (k, _) in self.head.iter() {
            tr_hs.insert(k);
        }
        //now iterate and only add if levels > 0
        let mut levels = levels as i32;
        for hm in self.tail.iter() {
            if levels > 0 {
                for (k, _) in hm.iter() {
                    tr_hs.insert(k);
                }
            }
            levels -= 1;
        }
        tr_hs
    }

    /// Returns a HM of all (&K, &V) (bindings of references) found in the top
    /// [levels] levels of the Smoosher that differ from the bindings found
    /// in the [levels]-th level of the Smoosher.
    /// Example: Say our Smoosher looked as follows:
    /// (lvl 3) [(a, 1), (b, 2)] -> [(a, 3)] -> [(c, 4)] -> [(d, 15)] (lvl 0)
    /// the calling diff(3) on this Smoosher would result in a HM that looks
    /// as follows:
    /// [(a, 3), (c, 4), (d, 15)]
    /// Requires: 0 < [levels] < # of scopes in this smoosher
    /// Undefined behavior if [levels] >= # of scopes in this Smoosher
    pub fn diff(&self, levels: u64) -> HashMap<&K, &V> {
        if levels == 0 {
            panic!("cannot compute diff(0)");
        }
        //iterate from top (0) to (levels - 1) and add all bindings
        //continue iterating from (levels - 1) to bottom, check if binding
        //is in tr HM. If so, remove it.
        let mut tr = HashMap::new();
        //first add from head
        for (k, v) in HashMap::iter(&self.head) {
            tr.insert(k, v);
        }
        //now worry about tail. while 1 <= ind <= levels - 1 insert (preserve scope),
        //and while ind >= levels, get, check, remove.
        let mut ind = 1;
        for nd in List::iter(&self.tail) {
            for (k, v) in HashMap::iter(nd) {
                if ind <= (levels - 1) {
                    //add, but only if the binding isn't yet in the HM (preserve scope)
                    if None == tr.get(k) {
                        tr.insert(k, v);
                    }
                } else {
                    //do the check and remove
                    if let Some(&v_prime) = tr.get(k) {
                        if v_prime == v {
                            tr.remove(k); //bc it's not a new binding
                        }
                    }
                }
                ind += 1;
            }
        }
        tr
    }

    /// Returns a HM of all (&K, &V) (bindings of references) found in [self].
    /// A use case would be when you want a HM representing a snapshot of the
    /// current state of the environment, which is easily iterable.
    pub fn to_hm(&self) -> HashMap<&K, &V> {
        //just add all bindings
        let mut tr = HashMap::new();
        //first add from head
        for (k, v) in HashMap::iter(&self.head) {
            tr.insert(k, v);
        }
        //then from tail
        for nd in List::iter(&self.tail) {
            for (k, v) in HashMap::iter(nd) {
                //add, but only if the binding isn't yet in the HM (preserve scope)
                if None == tr.get(k) {
                    tr.insert(k, v);
                }
            }
        }
        tr
    }

    /// Returns a HM of all (&K, &V) (bindings of references) found in [self] and absent from
    /// [other]. For example, if:
    /// [self] : [(a, 1), (b, 2)], and
    /// [other] : [(a, 1), (b, 3)], then
    /// [self.smoosher_diff(other)] produces the HM [(b, 3)].
    pub fn diff_other(&self, other: &Self) -> HashMap<&K, &V> {
        let mut self_hm = Smoosher::to_hm(&self);
        let other_hm = Smoosher::to_hm(&other);
        for (&k, &v) in other_hm.iter() {
            if let Some(&v_self) = self_hm.get(k) {
                if v_self == v {
                    self_hm.remove(k);
                }
            }
        }
        self_hm
    }
}
