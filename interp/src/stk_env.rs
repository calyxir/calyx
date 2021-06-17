//a stack of environments, to be used like a version tree

use super::{primitives, primitives::Primitive, values::Value};
use calyx::ir;
use std::collections::{HashMap, VecDeque};

struct Smoosher<K, V> {
    pub ds: VecDeque<HashMap<K, V>>,
}

//methods we will implement
// new, get, set, clone, top, bottom, smoosh, sieve
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
    fn get() -> Option<V> {
        todo!()
    }

    ///set(k, v) mutates the current Smoosher, inserting the key-value pair (k, v) to the topmost HashMap of
    ///the Smoosher. Overwrites the existing (k, v') pair if one exists in the topmost HashMap at the time
    ///of the set(k, v) call.
    fn set() {
        todo!()
    }

    fn clone() -> () {
        todo!()
    }

    fn top() -> () {
        todo!()
    }

    fn smoosh() -> () {
        todo!()
    }

    fn sieve() -> () {
        todo!()
    }
}
