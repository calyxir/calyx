#[cfg(test)]
use crate::stk_env::Smoosher;
#[allow(unused)]
use std::collections::HashMap;

#[test]
fn smoosher_get_empty() {
    let smoosher = Smoosher::<i32, i32>::new();
    assert_eq!(None, smoosher.get(&4));
}

#[test]
fn smoosher_get_set() {
    let mut smoosher = Smoosher::new();
    smoosher.set("hey", 2);
    assert_eq!(*smoosher.get(&"hey").unwrap(), 2);
}

#[test]
fn smoosher_get_set_2_scopes() {
    let mut smoosher = Smoosher::new();
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
    let mut smoosher = Smoosher::new();
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
    let mut smoosher = Smoosher::new();
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
    let mut smoosher = Smoosher::new();
    smoosher.set("alma", 18);
    smoosher.set("jonathan", 14);
    smoosher.set("jenny", 2);
    //the below fork adds a new scope to [smoosher]
    let mut smoosher2 = smoosher.fork();
    smoosher2.set("alma", 19);
    smoosher.set("jonathan", 15);
    let smoosher_merged = Smoosher::merge(smoosher, smoosher2);
    assert_eq!(*smoosher_merged.get(&"alma").unwrap(), 19);
    assert_eq!(*smoosher_merged.get(&"jonathan").unwrap(), 15);
}

//tests that we can merge different branch length. should fail now
#[test]
fn smoosher_merge_complex() {
    let mut smoosher = Smoosher::new();
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
    let smoosher_merged = Smoosher::merge(smoosher, smoosher2);
    assert_eq!(*smoosher_merged.get(&"alma").unwrap(), 19);
    assert_eq!(*smoosher_merged.get(&"jonathan").unwrap(), 15);
    assert_eq!(*smoosher_merged.get(&"jenny").unwrap(), 3);
}

#[test]
fn smoosher_list_b_vars() {
    let mut smoosher = Smoosher::new();
    smoosher.set("alma", 18);
    smoosher.new_scope();
    smoosher.set("jonathan", 14);
    smoosher.new_scope();
    smoosher.set("joseph", 19);
    smoosher.set("ari", 12);
    //assert lbv 0 is joseph and ari
    //assert lbv1 is joseph, ari, jonathan
    let hs0 = Smoosher::list_bound_vars(&smoosher, 0);
    let hs1 = Smoosher::list_bound_vars(&smoosher, 1);
    assert!(hs0.contains(&"joseph"));
    assert!(hs0.contains(&"ari"));
    assert_eq!(hs0.contains(&"jonathan"), false);
    assert_eq!(hs0.contains(&"alma"), false);
    //now test from 1 level deep
    assert!(hs1.contains(&"joseph"));
    assert!(hs1.contains(&"ari"));
    assert!(hs1.contains(&"jonathan"));
    assert_eq!(hs1.contains(&"alma"), false);
}

#[test]
fn smoosher_to_hm() {
    let mut smoosher = Smoosher::new();
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
    let mut smoosher = Smoosher::new();
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
    let mut smoosher = Smoosher::new();
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
    assert_eq!(diff_2.contains_key(&"joseph"), false);
}

#[test]
fn smoosher_diff_other() {
    let mut smoosher = Smoosher::new();
    smoosher.set("alma", 18);
    smoosher.new_scope();
    smoosher.set("joseph", 19);
    smoosher.new_scope();
    smoosher.set("jonathan", 14);
    let mut smoosher2 = Smoosher::new();
    smoosher2.set("jonathan", 15);
    smoosher2.new_scope();
    smoosher2.set("alma", 19);
    smoosher2.set("joseph", 19);
    let diff_2 = smoosher.diff_other(&smoosher2);
    assert!(diff_2.contains_key(&"alma"));
    assert_eq!(**diff_2.get(&"alma").unwrap(), 18);
    assert!(diff_2.contains_key(&"jonathan"));
    assert_eq!(**diff_2.get(&"jonathan").unwrap(), 14);
    assert_eq!(diff_2.contains_key(&"joseph"), false);
}

mod values_stk_env_test {
    #[allow(unused)]
    use crate::stk_env::Smoosher;
    #[allow(unused)]
    use crate::values::Value;

    #[test]
    fn smoosher_val_get_set() {
        let mut sm = Smoosher::new();
        let val = Value::try_from_init(8, 4).unwrap();
        sm.set("reg_out", val);
        assert_eq!(sm.get(&"reg_out").unwrap().as_u64(), 8);
    }

    #[test]
    fn smoosher_get_set_2_scopes() {
        let mut smoosher = Smoosher::new();
        smoosher.set("hey", Value::try_from_init(2, 32).unwrap());
        smoosher.set("alma", Value::try_from_init(18, 32).unwrap());
        assert_eq!(smoosher.get(&"hey").unwrap().as_u64(), 2);
        smoosher.new_scope();
        smoosher.set("hey", Value::try_from_init(3, 32).unwrap());
        //test a binding shadowed from the top scope
        assert_eq!(smoosher.get(&"hey").unwrap().as_u64(), 3);
        //test a binding found not on top scope
        assert_eq!(smoosher.get(&"alma").unwrap().as_u64(), 18);
    }

    #[test]
    fn value_eq_get_set() {
        let mut smoosher = Smoosher::new();
        smoosher.set("hey", Value::try_from_init(2, 32).unwrap());
        smoosher.set("alma", Value::try_from_init(18, 32).unwrap());
        assert_eq!(
            *smoosher.get(&"hey").unwrap(),
            Value::try_from_init(2, 32).unwrap()
        );
        smoosher.new_scope();
        smoosher.set("hey", Value::try_from_init(3, 32).unwrap());
        //test a binding shadowed from the top scope
        assert_eq!(
            *smoosher.get(&"hey").unwrap(),
            Value::try_from_init(3, 32).unwrap()
        );
        //test a binding found not on top scope
        assert_eq!(
            *smoosher.get(&"alma").unwrap(),
            Value::try_from_init(18, 32).unwrap()
        );
    }

    #[test]
    fn smoosher_smoosh_basic() {
        let mut smoosher = Smoosher::new();
        smoosher.set("hey", Value::try_from_init(2, 32).unwrap());
        smoosher.set("alma", Value::try_from_init(18, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("hey", Value::try_from_init(3, 32).unwrap());
        smoosher.set("bruh", Value::try_from_init(3, 32).unwrap());
        let smoosher = smoosher.smoosh(1);
        //test bindings have been maintained
        assert_eq!(smoosher.get(&"bruh").unwrap().as_u64(), 3);
        assert_eq!(smoosher.get(&"alma").unwrap().as_u64(), 18);
        //test the right "hey" was written
        assert_eq!(smoosher.get(&"hey").unwrap().as_u64(), 3);
    }
    #[test]
    fn smoosher_smoosh_many_lvls() {
        let mut smoosher = Smoosher::new();
        smoosher.set("hey", Value::try_from_init(2, 32).unwrap());
        smoosher.set("alma", Value::try_from_init(18, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("hey", Value::try_from_init(3, 32).unwrap());
        smoosher.set("bruh", Value::try_from_init(3, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("hey", Value::try_from_init(7, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("hey", Value::try_from_init(8, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("hey", Value::try_from_init(9, 32).unwrap());
        let smoosher = smoosher.smoosh(4);
        //test bindings have been maintained
        assert_eq!(smoosher.get(&"bruh").unwrap().as_u64(), 3);
        assert_eq!(smoosher.get(&"alma").unwrap().as_u64(), 18);
        //test the right "hey" was written
        assert_eq!(smoosher.get(&"hey").unwrap().as_u64(), 9);
    }

    #[test]
    fn smoosher_merge_basic() {
        let mut smoosher = Smoosher::new();
        smoosher.set("alma", Value::try_from_init(18, 32).unwrap());
        smoosher.set("jonathan", Value::try_from_init(14, 32).unwrap());
        smoosher.set("jenny", Value::try_from_init(2, 32).unwrap());
        //the below fork adds a new scope to [smoosher]
        let mut smoosher2 = smoosher.fork();
        smoosher2.set("alma", Value::try_from_init(19, 32).unwrap());
        smoosher.set("jonathan", Value::try_from_init(15, 32).unwrap());
        let smoosher_merged = Smoosher::merge(smoosher, smoosher2);
        assert_eq!(smoosher_merged.get(&"alma").unwrap().as_u64(), 19);
        assert_eq!(smoosher_merged.get(&"jonathan").unwrap().as_u64(), 15);
    }

    //tests that we can merge different branch length. should fail now
    #[test]
    fn smoosher_merge_complex() {
        let mut smoosher = Smoosher::new();
        smoosher.set("alma", Value::try_from_init(18, 32).unwrap());
        smoosher.set("jonathan", Value::try_from_init(14, 32).unwrap());
        smoosher.set("jenny", Value::try_from_init(2, 32).unwrap());
        //the below fork adds a new scope to [smoosher]
        let mut smoosher2 = smoosher.fork();
        smoosher2.set("alma", Value::try_from_init(19, 32).unwrap());
        //add another 2 scopes to smoosher, see if that can be merged
        smoosher.set("jonathan", Value::try_from_init(15, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("jenny", Value::try_from_init(3, 32).unwrap());
        let smoosher_merged = Smoosher::merge(smoosher, smoosher2);
        assert_eq!(smoosher_merged.get(&"alma").unwrap().as_u64(), 19);
        assert_eq!(smoosher_merged.get(&"jonathan").unwrap().as_u64(), 15);
        assert_eq!(smoosher_merged.get(&"jenny").unwrap().as_u64(), 3);
    }

    #[test]
    fn smoosher_list_b_vars() {
        let mut smoosher = Smoosher::new();
        smoosher.set("alma", Value::try_from_init(18, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("jonathan", Value::try_from_init(14, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("joseph", Value::try_from_init(19, 32).unwrap());
        smoosher.set("ari", Value::try_from_init(12, 32).unwrap());
        //assert lbv 0 is joseph and ari
        //assert lbv1 is joseph, ari, jonathan
        let hs0 = Smoosher::list_bound_vars(&smoosher, 0);
        let hs1 = Smoosher::list_bound_vars(&smoosher, 1);
        assert!(hs0.contains(&"joseph"));
        assert!(hs0.contains(&"ari"));
        assert_eq!(hs0.contains(&"jonathan"), false);
        assert_eq!(hs0.contains(&"alma"), false);
        //now test from 1 level deep
        assert!(hs1.contains(&"joseph"));
        assert!(hs1.contains(&"ari"));
        assert!(hs1.contains(&"jonathan"));
        assert_eq!(hs1.contains(&"alma"), false);
    }

    #[test]
    fn smoosher_to_hm() {
        let mut smoosher = Smoosher::new();
        smoosher.set("alma", Value::try_from_init(18, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("jonathan", Value::try_from_init(14, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("joseph", Value::try_from_init(19, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("joseph", Value::try_from_init(436, 32).unwrap());
        smoosher.set("ari", Value::try_from_init(12, 32).unwrap());
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
        let mut smoosher = Smoosher::new();
        smoosher.set("alma", Value::try_from_init(18, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("jonathan", Value::try_from_init(14, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("joseph", Value::try_from_init(19, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("joseph", Value::try_from_init(436, 32).unwrap());
        smoosher.set("ari", Value::try_from_init(12, 32).unwrap());
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
        let mut smoosher = Smoosher::new();
        smoosher.set("alma", Value::try_from_init(18, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("joseph", Value::try_from_init(19, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("jonathan", Value::try_from_init(14, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("jonathan", Value::try_from_init(15, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("alma", Value::try_from_init(19, 32).unwrap());
        smoosher.set("joseph", Value::try_from_init(19, 32).unwrap());
        //there are 5 scopes, check diff 2 and see that the resulting hm
        //has alma, jonathan, but not joseph.
        let diff_2 = smoosher.diff(2);
        assert!(diff_2.contains_key(&"alma"));
        assert!(diff_2.contains_key(&"jonathan"));
        assert_eq!(diff_2.contains_key(&"joseph"), false);
    }

    #[test]
    fn smoosher_diff_other() {
        let mut smoosher = Smoosher::new();
        smoosher.set("alma", Value::try_from_init(18, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("joseph", Value::try_from_init(19, 32).unwrap());
        smoosher.new_scope();
        smoosher.set("jonathan", Value::try_from_init(14, 32).unwrap());
        let mut smoosher2 = Smoosher::new();
        smoosher2.set("jonathan", Value::try_from_init(15, 32).unwrap());
        smoosher2.new_scope();
        smoosher2.set("alma", Value::try_from_init(19, 32).unwrap());
        smoosher2.set("joseph", Value::try_from_init(19, 32).unwrap());
        let diff_2 = smoosher.diff_other(&smoosher2);
        assert!(diff_2.contains_key(&"alma"));
        assert_eq!(diff_2.get(&"alma").unwrap().as_u64(), 18);
        assert!(diff_2.contains_key(&"jonathan"));
        assert_eq!(diff_2.get(&"jonathan").unwrap().as_u64(), 14);
        assert_eq!(diff_2.contains_key(&"joseph"), false);
    }
}
