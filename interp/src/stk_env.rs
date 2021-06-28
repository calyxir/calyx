//use super::{primitives, primitives::Primitive, values::Value};
//use calyx::ir;
//use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
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

/// The underlying, functional linked list used by the Smoosher.
/// Taken from "Learning Rust with Entirely Too Many Linked Lists" (2018), Chapter 4.5
/// Added the [same_head], [is_empty], and [split] functions.
impl<T> List<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        List { head: None }
    }

    /// Tests if the nodes at the head of [self] and [other] are equal;
    /// that is if the Rc points to the same location.
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
        if Option::is_none(&self.head) || Option::is_none(&other.head) {
            panic!("cannot compare empty lists using [same_head]");
        }
        let self_head = self.head.as_ref().unwrap();
        let other_head = other.head.as_ref().unwrap();
        Rc::as_ptr(self_head) == Rc::as_ptr(other_head)
    }

    /// Tests if the head of [self] is [None], or [Some(nd)]
    ///
    /// # Example
    /// ```
    /// use interp::stk_env::List;
    /// let l1 = List::new();
    /// assert!(l1.is_empty());
    /// let l1 = l1.push(3);
    /// assert_eq!(l1.is_empty(), false);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
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

    /// Consumes [self], returning a tupule of ([self.head : Option<T>], [self.tail : List<T>]),
    /// where [tail] is all elements in [self] that are not [head]. If [self] is empty,
    /// [split] will return ([None], [self])
    ///
    /// # Panics
    /// Because [split] consumes [self], [split] panics if multiple lists exist that
    /// share (references to the same) elements in their **tail**.
    /// # Example
    /// Good:
    /// ```
    /// use interp::stk_env::List;
    /// let l1 = List::new().push(4).push(3);
    /// if let (Some(hd), l1) = l1.split(){
    ///     assert_eq!(hd, 3);
    /// } else {
    ///     panic!("could not split l1")
    /// }
    /// ```
    /// Shared tail panic:
    /// ```should_panic
    /// use interp::stk_env::List;
    /// let l1 = List::new().push(4).push(3);
    /// let l2 = l1.push(2);
    /// //This will panic, because while l1 exists, l2's tail cannot be split:
    /// if let (Some(hd), l2) = l2.split() {
    ///     let tup = l2.split();
    /// }
    /// l1.push(5);
    /// ```
    pub fn split(self) -> (Option<T>, Self) {
        if self.is_empty() {
            (None, self)
        } else if let Ok(head) = Rc::try_unwrap(self.head.unwrap()) {
            let tail_list = List { head: head.next }; //better: not cloning the tail
            (Some(head.elem), tail_list)
        } else {
            panic!("Cannot unwrap the head of this list. You probably tried Smooshing while a fork exists!")
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

///A Stack of HashMaps that supports scoping
///
///Invariants:
///=> To Smoosh X levels down, no fork may still exist from any node amongst those
/// X levels. So the stack must look like:
/// ```text
///  _
/// |_|
///  |
///  _
/// |_|
///  |
///  _
/// |_|
///  |
/// ```
///=> To merge, both branches must share a common fork point
/// ```text
///  _
/// |_|
///  |
///  _     _
/// |_|   |_|
///  |     |
///   \ _ /
///    |*|
///     |
/// ```
#[derive(Debug)]
pub struct Smoosher<K: Eq + std::hash::Hash, V: Eq> {
    head: HashMap<K, V>,       //mutable
    tail: List<HashMap<K, V>>, //read-only
}

impl<K: Eq + std::hash::Hash, V: Eq> Into<HashMap<&K, &V>> for Smoosher<K, V> {
    fn into(self) -> HashMap<&'static K, &'static V> {
        todo!()
    }
}

impl<K: Eq + std::hash::Hash, V: Eq> Smoosher<K, V> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Smoosher<K, V> {
        Smoosher {
            head: HashMap::new(),
            tail: List::new(),
        }
    }

    /// If [self] and [other] share a fork point, returns a pair (depth_a, depth_b)
    /// of the depth which the fork point can be found in [self] and [other], respectively.
    /// NOTE: should be private, only public for testing!
    pub fn shared_fork_point(&self, other: &Self) -> Option<(u64, u64)> {
        //check head
        if std::ptr::eq(&self.head, &other.head) {
            Some((0, 0))
        } else {
            //check tail
            //these start at 1 b/c head was 0
            let mut a_depth = 1;
            //iterate and check all other nodes
            for nd in self.tail.iter() {
                if let Some(b_dep) =
                    other.tail.iter().position(|el| std::ptr::eq(nd, el))
                {
                    return Some((a_depth, (b_dep + 1).try_into().unwrap()));
                }
                // for nd_other in other.tail.iter() {
                //     if std::ptr::eq(nd, nd_other) {
                //         return Some((a_depth, b_depth));
                //     }
                //     b_depth += 1;
                // }
                a_depth += 1;
            }
            None //do not share a fork point
        }
    }
    // NOTE: shold be private, only public for testing!
    pub fn drop(self) {
        //yea
    }

    /// Returns an Option of a pointer to the highest-scoped binding to [k],
    /// if it exists. Else None.
    ///
    /// # Example
    /// ```
    /// use interp::stk_env::Smoosher;
    /// let mut smoosher = Smoosher::new();
    /// smoosher.set("hi!", 1);
    /// assert_eq!(*smoosher.get(&"hi!").unwrap(), 1);
    /// assert_eq!(smoosher.get(&"hey"), None);
    /// ```
    pub fn get(&self, k: &K) -> Option<&V> {
        //first check if it's in the highest one
        if let Some(val) = self.head.get(k) {
            Some(val)
        } else {
            let iter = self.tail.iter();
            for hm in iter {
                if let Some(val) = hm.get(k) {
                    return Some(val);
                }
            }
            //then check if it's anywhere in the other list
            None
        }
    }

    /// ```text
    /// Sets a new binding of [k] to [v] in the highest scope.
    /// Guarantees that all following calls to [get(&k)] will return [&v],
    /// granted no new binding is set with [set]
    /// ```
    /// # Example
    /// Using one scope:
    /// ```rust
    /// use interp::stk_env::Smoosher;
    /// let mut smoosher = Smoosher::new();
    /// smoosher.set("hi!", 1);
    /// assert_eq!(*smoosher.get(&"hi!").unwrap(), 1);
    /// smoosher.set("hi!", 2);
    /// assert_eq!(*smoosher.get(&"hi!").unwrap(), 2);
    /// ```
    /// More than one scope:
    /// ```rust
    /// use interp::stk_env::Smoosher;
    /// let mut smoosher = Smoosher::new();
    /// smoosher.set("hi!", 1);
    /// assert_eq!(*smoosher.get(&"hi!").unwrap(), 1);
    /// smoosher.new_scope(); //scopes themselves do not affect gets, only new sets do.
    /// assert_eq!(*smoosher.get(&"hi!").unwrap(), 1);
    /// smoosher.set("hi!", 2);
    /// assert_eq!(*smoosher.get(&"hi!").unwrap(), 2);
    /// ```
    pub fn set(&mut self, k: K, v: V) {
        self.head.insert(k, v);
    }

    /// ```text
    /// Returns a new Smoosher and mutates [self]. The new Smoosher has a new scope
    /// as [head] and all of (pre-mutation) [self] as [tail]. [Self] has a fresh scope pushed onto
    /// it. Invariant this method enforces: you cannot mutate a scope that has children
    /// forks (you can only mutate the fresh scope applied atop it)
    /// ```
    /// # Examples
    /// ## Pictorial Example
    /// ```text
    /// [A]
    /// ```
    /// let B = A.fork();
    /// ```text
    /// [B] [A'] <- Both B and A' point to empty nodes spawned off A
    ///  |   |
    ///   \ /
    ///    |
    ///   [A]
    /// ```
    /// ## Code Example
    /// ```rust
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set("hi!", 1);
    /// let mut b = a.fork();
    /// a.set("hey", 2);
    /// b.set("hey", 3);
    /// assert_eq!(*a.get(&"hi!").unwrap(), *b.get(&"hi!").unwrap());
    /// assert_eq!(*a.get(&"hey").unwrap(), 2);
    /// assert_eq!(*b.get(&"hey").unwrap(), 3);
    /// ```
    pub fn fork(&mut self) -> Self {
        //first save self's head, and replace it with a clean HM
        let old_head = mem::take(&mut self.head); //can replace with mem::take()
                                                  //create tail to both Self and the new Smoosher
        let new_tail = self.tail.push(old_head);
        //update Self and create new Smoosher
        self.tail = new_tail.clone(); //will this die after the end of [fork]? no
        Smoosher {
            head: HashMap::new(),
            tail: new_tail,
        }
    }

    ///```text
    ///Pushes a new, empty scope onto [self]. Doing so has no effect on the
    ///bindings in [self], until a new [set] is called
    ///```
    /// # Example
    /// ```rust
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set("hi!", 1);
    /// a.new_scope();
    /// a.set("hi!", 2);
    /// assert_eq!(*a.get(&"hi!").unwrap(), 2);
    /// ```
    pub fn new_scope(&mut self) {
        let old_head = mem::take(&mut self.head);
        self.tail = self.tail.push(old_head);
    }

    /// ```text
    /// Returns a pointer to the newest scope of this Smoosher. A Smoosher upon
    /// instantiation will always have 1 empty HM as its first scope, so this
    /// method will always meaningfully return.
    /// ```
    /// # Example
    /// ```
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set("hi!", 1);
    /// a.set("bye", 0);
    /// let hm = a.top();
    /// assert_eq!(hm.len(), 2);
    /// assert_eq!(*hm.get(&"hi!").unwrap(), 1);
    /// assert_eq!(*hm.get(&"bye").unwrap(), 0);
    /// ```
    pub fn top(&self) -> &HashMap<K, V> {
        &self.head
    }

    /// ```text
    /// Consumes [self] and returns a Smoosher with all bindings in the topmost scope transposed
    /// onto the second-newest scope, with the topmost scope then discarded, and
    /// the second-newest scope now the topmost scope. Has no visible effect on
    /// methods like [get], as identical keys in different scopes are still shadowed,
    /// with only the newest key's binding visible.
    /// Invariant: [self] must have at least 2 scopes, and neither scope may
    /// be the root of any fork:
    ///* [A]   [B]
    ///   |     |
    ///    \   /
    ///     [C]
    /// Cannot smoosh A into C -- calling [merge(A, B)] will result in one node
    /// containing all bindings in A, B, and the bindings in C not found in A or B, though.
    ///* [A]   [B]
    ///   |     |
    ///    \   /
    ///     [C]
    ///      |
    ///     [D]
    /// Cannot smoosh C into D (not even possible with this method)
    /// ```
    /// # Panics
    /// ```text
    /// Panics if [self] has less than two scopes, or if any of the scopes are
    /// the roots of any existing forks
    ///```
    /// # Examples
    /// ## Pictorial Example
    ///  [A]   
    ///   |     => [smoosh_once(A, C)] => [A U C\A]
    ///  [C]
    /// ## Code Example
    ///```
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set("hi!", 1);
    /// a.set("bye", 0);
    /// a.new_scope();
    /// a.set("hi!", 2);
    /// assert_eq!(*a.get(&"hi!").unwrap(), 2);
    /// assert_eq!(*a.get(&"bye").unwrap(), 0);
    /// let a = a.smoosh_once();
    /// assert_eq!(*a.get(&"hi!").unwrap(), 2);
    /// assert_eq!(*a.get(&"bye").unwrap(), 0);
    ///```
    ///the following should panic:
    ///```should_panic
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set("hi!", 1);
    /// a.set("bye", 0);
    /// let a = a.smoosh_once(); //not enough scopes to smoosh
    ///```
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
            Smoosher {
                head: new_head,
                tail: new_tail,
            }
        } else {
            panic!("Could not smoosh, because [self] has less than two scopes")
        }
    }

    /// ```text
    /// Consumes [self] and returns a Smoosher in which
    /// all bindings found in the top [levels] scopes of [self] to the [levels]th
    /// scope are merged. [smoosh(0)] has no effect, while [smoosh(1)] is equivalent to
    /// [smoosh_once].
    /// ```
    /// # Guarantees
    ///  If [self] has **n** scopes, then
    /// [self.get(K)] == [self.smoosh(n).get(K)] for all K bound in [self]
    ///
    ///  # Invariant
    /// ```text
    /// None of the top [levels] scopes may be the root of any fork
    /// ```
    /// # Panics
    /// ```text
    /// Panics if [levels] >= # of scopes in the Smoosher, or any of the top
    /// [levels] scopes are the roots of any existing forks.
    /// ```
    /// # Examples
    /// ## Pictorial Examples
    /// ```text
    ///  [A]  
    ///   |
    ///  [B]
    ///   |     => [A.smoosh(2)] => [A U B\A U C\(A U B\A)]
    ///  [C]
    /// ```
    /// ## Code Examples
    ///```
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set("hi!", 1);
    /// a.new_scope();
    /// a.set("bye", 0);
    /// a.new_scope();
    /// a.set("hi!", 2);
    /// let a = a.smoosh(2);
    /// assert_eq!(*a.get(&"hi!").unwrap(), 2);
    /// assert_eq!(*a.get(&"bye").unwrap(), 0);
    ///```
    ///the following should panic:
    ///```should_panic
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set("hi!", 1);
    /// let b = a.fork();
    /// a.set("bye", 0);
    /// let a = a.smoosh(1); //cannot smoosh; a fork point exists in a's tail
    /// b.get(&"hi!");
    ///```
    pub fn smoosh(self, levels: u64) -> Self {
        let mut tr = self;
        for _n in 0..levels {
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

    /// ```text
    /// Consumes two Smooshers, which must share a fork point.
    /// Merges all topmost scopes above the shared fork point, smooshing the resulting
    /// node onto the fork point. Returns the resulting Smoosher.
    /// ```
    /// ## IMPORTANT INVARIANT
    /// ```text
    /// The intersection of the bindings of the two branches
    /// MUST be the empty set! Otherwise, no guarantee for which binding will be included
    /// in the merge.
    /// ```
    /// # Panics
    /// ```text
    /// Panics if [self] and [other] do not share a common fork point, or if either is [empty]
    /// ```
    /// # Examples
    /// ## Pictorial Example
    /// ```text
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
    ///      [G]
    /// ```
    /// ## Code Example
    /// good:
    /// ```
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set("hello", 15);
    /// a.set("hi!", 1);
    /// a.new_scope();
    /// a.set("bye", 0);
    /// let mut b = a.fork();
    /// a.set("hi!", 3);
    /// b.set("bye", 2);
    /// let c = Smoosher::merge(a, b);
    /// assert_eq!(*c.get(&"hi!").unwrap(), 3);
    /// assert_eq!(*c.get(&"bye").unwrap(), 2);
    /// assert_eq!(*c.get(&"hello").unwrap(), 15);
    /// ```
    /// will panic:
    /// ```should_panic
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set("hello", 15);
    /// let mut b = Smoosher::new();
    /// b.set("hey", 13);
    /// let c = Smoosher::merge(a, b); //there's no common fork point
    /// ```
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
            if let Some(a_new_head) = a_new_head {
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

    /// ```text
    /// Returns a HashSet of references to keys bounded in the
    /// top [levels] levels of the Smoosher.
    /// [self.list_bound_vars(0)] returns all keys in the topmost scope
    /// [self.list.bound_vars(1)] returns all keys in the top two scopes
    /// Undefined behavior if levels >= (# of scopes), or [self] is empty
    /// ```
    /// # Example
    /// ```
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set("hello", 15);
    /// a.set("hi!", 1);
    /// a.new_scope();
    /// a.set("bye", 0);
    /// a.set("hello", 0);
    /// let b = a.list_bound_vars(1);
    /// assert_eq!(b.len(), 3);
    /// assert!(b.contains(&"hello"));
    /// assert!(b.contains(&"hi!"));
    /// assert!(b.contains(&"bye"));
    /// ```

    pub fn list_bound_vars(&self, levels: u64) -> HashSet<&K> {
        let mut tr_hs = HashSet::new();
        //levels is at least 0, so add everything from head
        for (k, _) in self.head.iter() {
            tr_hs.insert(k);
        }
        //now iterate and only add if levels > 0

        for hm in self.tail.iter().take(levels.try_into().unwrap()) {
            for (k, _) in hm.iter() {
                tr_hs.insert(k);
            }
        }
        tr_hs
    }

    /// ```text
    /// Returns a HashMap of all (&K, &V) (bindings of references) found in the top
    /// [levels] levels of the Smoosher that differ from the bindings found
    /// in the [levels]-th level of the Smoosher and below.
    /// ```
    ///  # Requires
    ///  0 < [levels] < # of scopes in this smoosher
    /// # Examples
    /// ## Pictoral Example
    /// ```text
    /// Say our Smoosher A looked as follows:
    /// (lvl 3) [(a, 1), (b, 2)] -> [(a, 3)] -> [(c, 4)] -> [(d, 15)] (lvl 0)
    /// A.diff(3) gives the following HashMap:
    /// [(a, 3), (c, 4), (d, 15)]
    /// ```
    /// ## Code Example
    /// ```
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set("hello", 15);
    /// a.set("hi!", 1);
    /// a.new_scope();
    /// a.set("bye", 0);
    /// a.set("hello", 0);
    /// let diff1 = a.diff(1); //the diff between the topmost scope and all below
    /// assert_eq!(diff1.len(), 2);
    /// assert_eq!(**diff1.get(&"hello").unwrap(), 0);
    /// assert_eq!(**diff1.get(&"bye").unwrap(), 0);
    /// ```
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
                    if tr.get(k).is_none() {
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

    /// ```text
    /// Returns a HM of all (&K, &V) (bindings of references) found in [self].
    /// A use case would be when you want a HM representing a snapshot of the
    /// current state of the environment, which is easily iterable.
    /// ```
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

    /// ```text
    /// Returns a HM of all (&K, &V) (bindings of references) found in [self] and absent from
    /// [other].
    /// ```
    /// # Examples
    /// ## Text Example
    /// ```text
    /// [self] : [(a, 1), (b, 2)], and
    /// [other] : [(a, 1), (b, 3)], then
    /// [self.smoosher_diff(other)] produces the HM [(b, 2)].
    /// ```
    /// ## Code Example
    /// ```
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set("a", 1);
    /// a.new_scope();
    /// a.set("b", 2);
    /// let mut b = Smoosher::new();
    /// b.set("a", 1);
    /// b.set("b", 3);
    /// let diff_a_b = a.diff_other(&b);
    /// assert_eq!(diff_a_b.len(), 1);
    /// assert_eq!(**diff_a_b.get(&"b").unwrap(), 2);
    /// ```

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

    /// ```text
    /// Returns true if a binding of [k] exists in the topmost scope.
    /// False if [k] is not binded in the topmost scope (regardless of whether
    /// or not [k] exists in the entire Smoosher).
    /// ```iss
    /// # Example
    /// ```
    /// use interp::stk_env::Smoosher;
    /// let mut a = Smoosher::new();
    /// a.set(1, 2);
    /// a.new_scope();
    /// a.set(2, 3);
    /// assert_eq!(a.binded_in_new(&1), false);
    /// assert!(a.binded_in_new(&2));
    /// ```
    pub fn binded_in_new(&self, k: &K) -> bool {
        self.head.contains_key(k)
    }
}
