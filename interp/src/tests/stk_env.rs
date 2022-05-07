#[cfg(test)]
use crate::structures::stk_env::StackMap;
#[allow(unused)]
use std::collections::HashMap;

#[test]
fn smoosher_get_empty() {
    let smoosher = StackMap::<i32, i32>::new();
    assert_eq!(None, smoosher.get(&4));
}

#[test]
fn smoosher_get_set() {
    let mut smoosher = StackMap::new();
    smoosher.set("hey", 2);
    assert_eq!(*smoosher.get(&"hey").unwrap(), 2);
}

#[test]
fn smoosher_get_set_2_scopes() {
    let mut smoosher = StackMap::new();
    smoosher.set("hey", 2);
    smoosher.set("alma", 18);
    assert_eq!(*smoosher.get(&"hey").unwrap(), 2);
    smoosher.new_scope();
    smoosher.set("hey", 3);
    //test a binding shadowed from the top scope
    assert_eq!(*smoosher.get(&"hey").unwrap(), 3);
    //test a binding found not on top scope
    assert_eq!(*smoosher.get(&"alma").unwrap(), 18);
}

#[test]
fn smoosher_smoosh_basic() {
    let mut smoosher = StackMap::new();
    smoosher.set("hey", 2);
    smoosher.set("alma", 18);
    smoosher.new_scope();
    smoosher.set("hey", 3);
    smoosher.set("bruh", 3);
    let smoosher = smoosher.smoosh(1);
    //test bindings have been maintained
    assert_eq!(*smoosher.get(&"bruh").unwrap(), 3);
    assert_eq!(*smoosher.get(&"alma").unwrap(), 18);
    //test the right "hey" was written
    assert_eq!(*smoosher.get(&"hey").unwrap(), 3);
}
#[test]
fn smoosher_smoosh_many_lvls() {
    let mut smoosher = StackMap::new();
    smoosher.set("hey", 2);
    smoosher.set("alma", 18);
    smoosher.new_scope();
    smoosher.set("hey", 3);
    smoosher.set("bruh", 3);
    smoosher.new_scope();
    smoosher.set("hey", 7);
    smoosher.new_scope();
    smoosher.set("hey", 8);
    smoosher.new_scope();
    smoosher.set("hey", 9);
    let smoosher = smoosher.smoosh(4);
    //test bindings have been maintained
    assert_eq!(*smoosher.get(&"bruh").unwrap(), 3);
    assert_eq!(*smoosher.get(&"alma").unwrap(), 18);
    //test the right "hey" was written
    assert_eq!(*smoosher.get(&"hey").unwrap(), 9);
}

#[test]
fn smoosher_merge_basic() {
    let mut smoosher = StackMap::new();
    smoosher.set("alma", 18);
    smoosher.set("jonathan", 14);
    smoosher.set("jenny", 2);
    //the below fork adds a new scope to [smoosher]
    let mut smoosher2 = smoosher.fork();
    smoosher2.set("alma", 19);
    smoosher.set("jonathan", 15);
    let smoosher_merged = StackMap::merge(smoosher, smoosher2);
    assert_eq!(*smoosher_merged.get(&"alma").unwrap(), 19);
    assert_eq!(*smoosher_merged.get(&"jonathan").unwrap(), 15);
}

//tests that we can merge different branch length. should fail now
#[test]
fn smoosher_merge_complex() {
    let mut smoosher = StackMap::new();
    smoosher.set("alma", 18);
    smoosher.set("jonathan", 14);
    smoosher.set("jenny", 2);
    //the below fork adds a new scope to [smoosher]
    let mut smoosher2 = smoosher.fork();
    smoosher2.set("alma", 19);
    //add another 2 scopes to smoosher, see if that can be merged
    smoosher.set("jonathan", 15);
    smoosher.new_scope();
    smoosher.set("jenny", 3);
    let smoosher_merged = StackMap::merge(smoosher, smoosher2);
    assert_eq!(*smoosher_merged.get(&"alma").unwrap(), 19);
    assert_eq!(*smoosher_merged.get(&"jonathan").unwrap(), 15);
    assert_eq!(*smoosher_merged.get(&"jenny").unwrap(), 3);
}

#[test]
fn smoosher_list_b_vars() {
    let mut smoosher = StackMap::new();
    smoosher.set("alma", 18);
    smoosher.new_scope();
    smoosher.set("jonathan", 14);
    smoosher.new_scope();
    smoosher.set("joseph", 19);
    smoosher.set("ari", 12);
    //assert lbv 0 is joseph and ari
    //assert lbv1 is joseph, ari, jonathan
    let hs0 = StackMap::list_bound_vars(&smoosher, 0);
    let hs1 = StackMap::list_bound_vars(&smoosher, 1);
    assert!(hs0.contains(&"joseph"));
    assert!(hs0.contains(&"ari"));
    assert!(!hs0.contains(&"jonathan"));
    assert!(!hs0.contains(&"alma"));
    //now test from 1 level deep
    assert!(hs1.contains(&"joseph"));
    assert!(hs1.contains(&"ari"));
    assert!(hs1.contains(&"jonathan"));
    assert!(!hs1.contains(&"alma"));
}

#[test]
fn smoosher_to_hm() {
    let mut smoosher = StackMap::new();
    smoosher.set("alma", 18);
    smoosher.new_scope();
    smoosher.set("jonathan", 14);
    smoosher.new_scope();
    smoosher.set("joseph", 19);
    smoosher.new_scope();
    smoosher.set("joseph", 436);
    smoosher.set("ari", 12);
    let hm = smoosher.to_hm();
    //that type annotation seems a bit wack
    assert_eq!(hm.len(), 4);
    assert_eq!(**hm.get(&"alma").unwrap(), 18);
    assert_eq!(**hm.get(&"jonathan").unwrap(), 14);
    assert_eq!(**hm.get(&"joseph").unwrap(), 436);
    assert_eq!(**hm.get(&"ari").unwrap(), 12);
}

#[test]
fn smoosher_from() {
    let mut smoosher = StackMap::new();
    smoosher.set("alma", 18);
    smoosher.new_scope();
    smoosher.set("jonathan", 14);
    smoosher.new_scope();
    smoosher.set("joseph", 19);
    smoosher.new_scope();
    smoosher.set("joseph", 436);
    smoosher.set("ari", 12);
    let hm: HashMap<&&str, &i32> = HashMap::from(&smoosher);
    //that type annotation seems a bit wack
    assert_eq!(hm.len(), 4);
    assert_eq!(**hm.get(&"alma").unwrap(), 18);
    assert_eq!(**hm.get(&"jonathan").unwrap(), 14);
    assert_eq!(**hm.get(&"joseph").unwrap(), 436);
    assert_eq!(**hm.get(&"ari").unwrap(), 12);
}

#[test]
fn smoosher_diff_2() {
    let mut smoosher = StackMap::new();
    smoosher.set("alma", 18);
    smoosher.new_scope();
    smoosher.set("joseph", 19);
    smoosher.new_scope();
    smoosher.set("jonathan", 14);
    smoosher.new_scope();
    smoosher.set("jonathan", 15);
    smoosher.new_scope();
    smoosher.set("alma", 19);
    smoosher.set("joseph", 19);
    //there are 5 scopes, check diff 2 and see that the resulting hm
    //has alma, jonathan, but not joseph.
    let diff_2 = smoosher.diff(2);
    assert!(diff_2.contains_key(&"alma"));
    assert!(diff_2.contains_key(&"jonathan"));
    assert!(!diff_2.contains_key(&"joseph"));
}

#[test]
fn smoosher_diff_other() {
    let mut smoosher = StackMap::new();
    smoosher.set("alma", 18);
    smoosher.new_scope();
    smoosher.set("joseph", 19);
    smoosher.new_scope();
    smoosher.set("jonathan", 14);
    let mut smoosher2 = StackMap::new();
    smoosher2.set("jonathan", 15);
    smoosher2.new_scope();
    smoosher2.set("alma", 19);
    smoosher2.set("joseph", 19);
    let diff_2 = smoosher.diff_other(&smoosher2);
    assert!(diff_2.contains_key(&"alma"));
    assert_eq!(**diff_2.get(&"alma").unwrap(), 18);
    assert!(diff_2.contains_key(&"jonathan"));
    assert_eq!(**diff_2.get(&"jonathan").unwrap(), 14);
    assert!(!diff_2.contains_key(&"joseph"));
}

mod values_stk_env_test {
    #[allow(unused)]
    use crate::structures::stk_env::StackMap;
    #[allow(unused)]
    use crate::values::Value;

    #[test]
    fn smoosher_val_get_set() {
        let mut sm = StackMap::new();
        let val = Value::from(8, 4);
        sm.set("reg_out", val);
        assert_eq!(sm.get(&"reg_out").unwrap().as_u64(), 8);
    }

    #[test]
    fn smoosher_get_set_2_scopes() {
        let mut smoosher = StackMap::new();
        smoosher.set("hey", Value::from(2, 32));
        smoosher.set("alma", Value::from(18, 32));
        assert_eq!(smoosher.get(&"hey").unwrap().as_u64(), 2);
        smoosher.new_scope();
        smoosher.set("hey", Value::from(3, 32));
        //test a binding shadowed from the top scope
        assert_eq!(smoosher.get(&"hey").unwrap().as_u64(), 3);
        //test a binding found not on top scope
        assert_eq!(smoosher.get(&"alma").unwrap().as_u64(), 18);
    }

    #[test]
    fn value_eq_get_set() {
        let mut smoosher = StackMap::new();
        smoosher.set("hey", Value::from(2, 32));
        smoosher.set("alma", Value::from(18, 32));
        assert_eq!(*smoosher.get(&"hey").unwrap(), Value::from(2, 32));
        smoosher.new_scope();
        smoosher.set("hey", Value::from(3, 32));
        //test a binding shadowed from the top scope
        assert_eq!(*smoosher.get(&"hey").unwrap(), Value::from(3, 32));
        //test a binding found not on top scope
        assert_eq!(*smoosher.get(&"alma").unwrap(), Value::from(18, 32));
    }

    #[test]
    fn smoosher_smoosh_basic() {
        let mut smoosher = StackMap::new();
        smoosher.set("hey", Value::from(2, 32));
        smoosher.set("alma", Value::from(18, 32));
        smoosher.new_scope();
        smoosher.set("hey", Value::from(3, 32));
        smoosher.set("bruh", Value::from(3, 32));
        let smoosher = smoosher.smoosh(1);
        //test bindings have been maintained
        assert_eq!(smoosher.get(&"bruh").unwrap().as_u64(), 3);
        assert_eq!(smoosher.get(&"alma").unwrap().as_u64(), 18);
        //test the right "hey" was written
        assert_eq!(smoosher.get(&"hey").unwrap().as_u64(), 3);
    }
    #[test]
    fn smoosher_smoosh_many_lvls() {
        let mut smoosher = StackMap::new();
        smoosher.set("hey", Value::from(2, 32));
        smoosher.set("alma", Value::from(18, 32));
        smoosher.new_scope();
        smoosher.set("hey", Value::from(3, 32));
        smoosher.set("bruh", Value::from(3, 32));
        smoosher.new_scope();
        smoosher.set("hey", Value::from(7, 32));
        smoosher.new_scope();
        smoosher.set("hey", Value::from(8, 32));
        smoosher.new_scope();
        smoosher.set("hey", Value::from(9, 32));
        let smoosher = smoosher.smoosh(4);
        //test bindings have been maintained
        assert_eq!(smoosher.get(&"bruh").unwrap().as_u64(), 3);
        assert_eq!(smoosher.get(&"alma").unwrap().as_u64(), 18);
        //test the right "hey" was written
        assert_eq!(smoosher.get(&"hey").unwrap().as_u64(), 9);
    }

    #[test]
    fn smoosher_merge_basic() {
        let mut smoosher = StackMap::new();
        smoosher.set("alma", Value::from(18, 32));
        smoosher.set("jonathan", Value::from(14, 32));
        smoosher.set("jenny", Value::from(2, 32));
        //the below fork adds a new scope to [smoosher]
        let mut smoosher2 = smoosher.fork();
        smoosher2.set("alma", Value::from(19, 32));
        smoosher.set("jonathan", Value::from(15, 32));
        let smoosher_merged = StackMap::merge(smoosher, smoosher2);
        assert_eq!(smoosher_merged.get(&"alma").unwrap().as_u64(), 19);
        assert_eq!(smoosher_merged.get(&"jonathan").unwrap().as_u64(), 15);
    }

    //tests that we can merge different branch length. should fail now
    #[test]
    fn smoosher_merge_complex() {
        let mut smoosher = StackMap::new();
        smoosher.set("alma", Value::from(18, 32));
        smoosher.set("jonathan", Value::from(14, 32));
        smoosher.set("jenny", Value::from(2, 32));
        //the below fork adds a new scope to [smoosher]
        let mut smoosher2 = smoosher.fork();
        smoosher2.set("alma", Value::from(19, 32));
        //add another 2 scopes to smoosher, see if that can be merged
        smoosher.set("jonathan", Value::from(15, 32));
        smoosher.new_scope();
        smoosher.set("jenny", Value::from(3, 32));
        let smoosher_merged = StackMap::merge(smoosher, smoosher2);
        assert_eq!(smoosher_merged.get(&"alma").unwrap().as_u64(), 19);
        assert_eq!(smoosher_merged.get(&"jonathan").unwrap().as_u64(), 15);
        assert_eq!(smoosher_merged.get(&"jenny").unwrap().as_u64(), 3);
    }

    #[test]
    fn smoosher_list_b_vars() {
        let mut smoosher = StackMap::new();
        smoosher.set("alma", Value::from(18, 32));
        smoosher.new_scope();
        smoosher.set("jonathan", Value::from(14, 32));
        smoosher.new_scope();
        smoosher.set("joseph", Value::from(19, 32));
        smoosher.set("ari", Value::from(12, 32));
        //assert lbv 0 is joseph and ari
        //assert lbv1 is joseph, ari, jonathan
        let hs0 = StackMap::list_bound_vars(&smoosher, 0);
        let hs1 = StackMap::list_bound_vars(&smoosher, 1);
        assert!(hs0.contains(&"joseph"));
        assert!(hs0.contains(&"ari"));
        assert!(!hs0.contains(&"jonathan"));
        assert!(!hs0.contains(&"alma"));
        //now test from 1 level deep
        assert!(hs1.contains(&"joseph"));
        assert!(hs1.contains(&"ari"));
        assert!(hs1.contains(&"jonathan"));
        assert!(!hs1.contains(&"alma"));
    }

    #[test]
    fn smoosher_to_hm() {
        let mut smoosher = StackMap::new();
        smoosher.set("alma", Value::from(18, 32));
        smoosher.new_scope();
        smoosher.set("jonathan", Value::from(14, 32));
        smoosher.new_scope();
        smoosher.set("joseph", Value::from(19, 32));
        smoosher.new_scope();
        smoosher.set("joseph", Value::from(436, 32));
        smoosher.set("ari", Value::from(12, 32));
        let hm = smoosher.to_hm();
        //that type annotation seems a bit wack
        assert_eq!(hm.len(), 4);
        assert_eq!(hm.get(&"alma").unwrap().as_u64(), 18);
        assert_eq!(hm.get(&"jonathan").unwrap().as_u64(), 14);
        assert_eq!(hm.get(&"joseph").unwrap().as_u64(), 436);
        assert_eq!(hm.get(&"ari").unwrap().as_u64(), 12);
    }

    #[test]
    fn value_smoosher_hm_from() {
        use std::collections::HashMap;
        let mut smoosher = StackMap::new();
        smoosher.set("alma", Value::from(18, 32));
        smoosher.new_scope();
        smoosher.set("jonathan", Value::from(14, 32));
        smoosher.new_scope();
        smoosher.set("joseph", Value::from(19, 32));
        smoosher.new_scope();
        smoosher.set("joseph", Value::from(436, 32));
        smoosher.set("ari", Value::from(12, 32));
        let hm: HashMap<&&str, &Value> = HashMap::from(&smoosher);
        //that type annotation seems a bit wack
        assert_eq!(hm.len(), 4);
        assert_eq!(hm.get(&"alma").unwrap().as_u64(), 18);
        assert_eq!(hm.get(&"jonathan").unwrap().as_u64(), 14);
        assert_eq!(hm.get(&"joseph").unwrap().as_u64(), 436);
        assert_eq!(hm.get(&"ari").unwrap().as_u64(), 12);
    }

    #[test]
    fn smoosher_diff_2() {
        let mut smoosher = StackMap::new();
        smoosher.set("alma", Value::from(18, 32));
        smoosher.new_scope();
        smoosher.set("joseph", Value::from(19, 32));
        smoosher.new_scope();
        smoosher.set("jonathan", Value::from(14, 32));
        smoosher.new_scope();
        smoosher.set("jonathan", Value::from(15, 32));
        smoosher.new_scope();
        smoosher.set("alma", Value::from(19, 32));
        smoosher.set("joseph", Value::from(19, 32));
        //there are 5 scopes, check diff 2 and see that the resulting hm
        //has alma, jonathan, but not joseph.
        let diff_2 = smoosher.diff(2);
        assert!(diff_2.contains_key(&"alma"));
        assert!(diff_2.contains_key(&"jonathan"));
        assert!(!diff_2.contains_key(&"joseph"));
    }

    #[test]
    fn smoosher_diff_other() {
        let mut smoosher = StackMap::new();
        smoosher.set("alma", Value::from(18, 32));
        smoosher.new_scope();
        smoosher.set("joseph", Value::from(19, 32));
        smoosher.new_scope();
        smoosher.set("jonathan", Value::from(14, 32));
        let mut smoosher2 = StackMap::new();
        smoosher2.set("jonathan", Value::from(15, 32));
        smoosher2.new_scope();
        smoosher2.set("alma", Value::from(19, 32));
        smoosher2.set("joseph", Value::from(19, 32));
        let diff_2 = smoosher.diff_other(&smoosher2);
        assert!(diff_2.contains_key(&"alma"));
        assert_eq!(diff_2.get(&"alma").unwrap().as_u64(), 18);
        assert!(diff_2.contains_key(&"jonathan"));
        assert_eq!(diff_2.get(&"jonathan").unwrap().as_u64(), 14);
        assert!(!diff_2.contains_key(&"joseph"));
    }
}
