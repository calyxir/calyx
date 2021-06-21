//a stack of environments, to be used like a version tree

use super::{primitives, primitives::Primitive, values::Value};
use calyx::ir;
use std::collections::{HashMap, HashSet, VecDeque};

struct Smoosher<K, V> {
    pub ds: VecDeque<HashMap<K, V>>,
    //note: piece of aux data? to keep track of fork index?
    //maybe a length to keep track of indecies in general
    //if two forks are related have to agree on same fork index

    //actually another note, maybe we don't need to keep this in the struct b/c VecDeques
    //have indecies, so fork index would just be length of current VecDeque - length of original?
    //maybe a more targeted question is who keeps track of clone(fork?) and original, and how?
}

//methods we will implement
// new, get, set, clone, top, bottom, smoosh, diff
impl<K, V> Smoosher<K, V> {
    fn new(k: K, v: V) -> Smoosher<K, V> {
        let hm: HashMap<K, V> = HashMap::new();
        let mut ds: VecDeque<HashMap<K, V>> = VecDeque::new();
        ds.push_back(hm);
        Smoosher { ds }
    }

    ///get(k) returns an Option containing the most recent binding of k. As in, returns the value associated
    ///with k from the topmost HashMap that contains some key-value pair (k, v). If no HashMap exists with
    ///a key-value pair (k, v), returns None.
    fn get(k: K) -> Option<&V> {
        todo!()
    }

    ///forgot why we put this down
    fn get_mut(k: K) -> Option<&mut V> {
        todo!()
    }

    ///set(k, v) mutates the current Smoosher, inserting the key-value pair (k, v) to the topmost HashMap of
    ///the Smoosher. Overwrites the existing (k, v') pair if one exists in the topmost HashMap at the time
    ///of the set(k, v) call.
    fn set(k: K, v: V) {
        todo!()
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
    fn top() -> () {
        todo!()
    }

    /// updates [bottom_i] to reflect all bindings contained in the HashMaps of indecies
    /// [bottom_i, top_i], with the higher-indecied HashMaps given precedence to
    /// their bindings, and then removes all HashMaps with index greater than [bottom_i],  
    /// note: vertical pushing down
    fn smoosh(&mut self, top_i: u64, bottom_i: u64) -> () {
        todo!()
    }

    ///merge: note: lateral (collects all forks that are parallel and merge them)
    fn merge(&mut self, &mut other: Self) -> Self {
        todo!()
    }

    ///Returns a set of all variables bound in any HashMap in the range
    ///[bottom_i, top_i]
    fn list_bound_vars(&self, top_i: u64, bottom_i: u64) -> HashSet<K> {
        todo!()
    }

    ///didn't write this down :/
    fn diff(&self, top_i: u64, bottom_i: u64) -> Vec<(K, V)> {
        todo!()
    }
}
