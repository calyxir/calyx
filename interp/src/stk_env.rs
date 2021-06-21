//a stack of environments, to be used like a version tree

use super::{primitives, primitives::Primitive, values::Value};
use calyx::ir;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

//use push front and pop front and iterator is in right order then

struct Smoosher<K: Eq + std::hash::Hash, V> {
    pub ds: VecDeque<Rc<RefCell<HashMap<K, V>>>>,
    //note: piece of aux data? to keep track of fork index?
    //maybe a length to keep track of indecies in general
    //if two forks are related have to agree on same fork index

    //actually another note, maybe we don't need to keep this in the struct b/c VecDeques
    //have indecies, so fork index would just be length of current VecDeque - length of original?
    //maybe a more targeted question is who keeps track of clone(fork?) and original, and how?
}

//methods we will implement
// new, get, set, clone, top, bottom, smoosh, diff
impl<K: Eq + std::hash::Hash, V> Smoosher<K, V> {
    fn new(k: K, v: V) -> Smoosher<K, V> {
        let hm: HashMap<K, V> = HashMap::new();
        let rc_rc_hm: Rc<RefCell<HashMap<K, V>>> = Rc::new(RefCell::new(hm));
        let mut ds: VecDeque<Rc<RefCell<HashMap<K, V>>>> = VecDeque::new();
        ds.push_back(rc_rc_hm);
        Smoosher { ds }
    }

    //two notes:
    //make wrapper struct for read-only environment  (HashMap)
    //perhaps make internal DS vector to push all the borrows onto so they don't
    //get dropped...?
    //write_handle and read_handle internal DS so we can keep the ref alive
    //and return it

    ///get(k) returns an Option containing the most recent binding of k. As in, returns the value associated
    ///with k from the topmost HashMap that contains some key-value pair (k, v). If no HashMap exists with
    ///a key-value pair (k, v), returns None.
    fn get(&self, k: &K) -> Option<&V> {
        for hm in self.ds.iter() {
            if let Some(val) = &&hm.borrow().get(k) {
                return Some(val);
            }
        }
        None
    }

    ///forgot why we put this down
    fn get_mut(&self, k: &K) -> Option<&mut V> {
        for hm in self.ds.iter() {
            if let Some(mut val) = &&hm.borrow().get(k) {
                return Some(&mut *val);
            }
        }
        None
    }

    ///set(k, v) mutates the current Smoosher, inserting the key-value pair (k, v) to the topmost HashMap of
    ///the Smoosher. Overwrites the existing (k, v') pair if one exists in the topmost HashMap at the time
    ///of the set(k, v) call.
    fn set(&mut self, k: K, v: V) {
        //note vecdeque can never be empty b/c initialized w/ a new hashmap
        if let Some(front) = self.ds.front() {
            let front_ref = &mut front.borrow_mut();
            front_ref.insert(k, v);
        }
    }

    //note: if we change everything here to deal with Rc<RefCell...>, then clone
    //is simple we just new_scope and fork

    ///Returns a copy of the stk_env with a clean HashMap ontop (at front of internal VecDeque)
    fn fork(&self) -> Self {
        todo!()
    }

    ///Add a clean HashMap ontop of internal VecDeque
    fn new_scope(&mut self) {
        todo!()
    }

    ///Returns a reference to the frontmost HashMap
    fn top(&self) -> &HashMap<K, V> {
        let mp = self.ds.get(0).unwrap();
        &mp.borrow()
    }

    /// updates [bottom_i] to reflect all bindings contained in the HashMaps of indecies
    /// [bottom_i, top_i], with the higher-indecied HashMaps given precedence to
    /// their bindings, and then removes all HashMaps with index greater than [bottom_i],  
    /// note: vertical pushing down
    fn smoosh(&mut self, top_i: u64, bottom_i: u64) -> () {
        todo!()
    }

    ///merge: note: lateral (collects all forks that are parallel and merge them)
    fn merge(&mut self, other: &mut Self) -> Self {
        todo!()
    }

    ///Returns a set of all variables bound in any HashMap in the range
    ///[bottom_i, top_i]
    fn list_bound_vars(&self, top_i: u64, bottom_i: u64) -> HashSet<K> {
        todo!()
    }

    ///in order to set unmodified values to zero
    ///
    fn diff(&self, top_i: u64, bottom_i: u64) -> Vec<(K, V)> {
        todo!()
    }
}
