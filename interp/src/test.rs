#[cfg(test)]
#[cfg(test)]

mod stk_env_test {
    use crate::stk_env::Smoosher;

    #[test]
    fn smoosher_get_set() {
        let k = "hey";
        let v = 2;
        let mut smoosher = Smoosher::new();
        smoosher.set(k, v);
        assert_eq!(*smoosher.get(&k).unwrap(), 2);
    }

    #[test]
    fn smoosher_get_set_2_scopes() {
        let k = "hey";
        let v = 2;
        let mut smoosher = Smoosher::new();
        smoosher.set(k, v);
        smoosher.set("alma", 18);
        assert_eq!(*smoosher.get(&k).unwrap(), 2);
        let v1 = 3;
        smoosher.new_scope();
        smoosher.set(k, v1);
        //test a binding shadowed from the top scope
        assert_eq!(*smoosher.get(&k).unwrap(), 3);
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
    }

    #[test]
    fn smoosher_merge_basic() {
        let mut smoosher = Smoosher::new();
        smoosher.set("alma", 18);
        smoosher.set("jonathan", 14);
        smoosher.set("jenny", 2);
        let mut smoosher2 = smoosher.fork();
        smoosher2.set("alma", 19);
        smoosher.set("jonathan", 15);
        let smoosher_merged = Smoosher::merge(smoosher, smoosher2);
        assert_eq!(*smoosher_merged.get(&"alma").unwrap(), 19);
        assert_eq!(*smoosher_merged.get(&"jonathan").unwrap(), 15);
    }
}

// mod prim_test {
//     use crate::primitives::*;
//     use crate::values::*;
//     use calyx::ir;
//     #[test]
//     fn test_mem_d1_tlv() {
//         let mut mem_d1 = StdMemD1::new(32, 8, 3);
//         let val = Value::try_from_init(5, 32).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr = Value::try_from_init(2, 3).unwrap();
//         let input = (ir::Id::from("write_data"), val.clone());
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr);
//         let mut mem_out = mem_d1.execute_mut(&[input, write_en, addr0]);
//         match &mut mem_out[..] {
//             [read_data, done] => match (read_data, done) {
//                 (
//                     (_, OutputValue::LockedValue(rd)),
//                     (_, OutputValue::LockedValue(d)),
//                 ) => {
//                     assert_eq!(rd.get_count(), 1);
//                     assert_eq!(d.get_count(), 1);
//                     rd.dec_count();
//                     d.dec_count();
//                     assert_eq!(rd.unlockable(), true);
//                     assert_eq!(d.unlockable(), true);
//                     assert_eq!(
//                         rd.clone().unlock().as_u64(),
//                         val.clone().as_u64()
//                     );
//                     assert_eq!(d.clone().unlock().as_u64(), 1);
//                 }
//                 _ => {
//                     panic!("std_mem did not return 2 LockedValues")
//                 }
//             },
//             _ => panic!("Returned more than 2 outputs"),
//         }
//     }
//     #[test]
//     fn test_mem_d1_imval() {
//         let mut mem_d1 = StdMemD1::new(32, 8, 3);
//         let val = Value::try_from_init(5, 32).unwrap();
//         let enable = Value::try_from_init(0, 1).unwrap();
//         let addr = Value::try_from_init(2, 3).unwrap();
//         let input = (ir::Id::from("write_data"), val.clone());
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr);
//         let mut mem_out =
//             mem_d1.execute_mut(&[input, write_en, addr0]).into_iter();
//         let (read_data, done) =
//             (mem_out.next().unwrap(), mem_out.next().unwrap());
//         assert!(mem_out.next().is_none()); //make sure it's only of length 2
//         let rd = read_data.1.unwrap_imm();
//         let d = done.1.unwrap_imm();
//         assert_eq!(rd.as_u64(), 0); // assuming this b/c mem hasn't been initialized
//         assert_eq!(d.as_u64(), 0);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d1_panic_addr() {
//         // Access address larger than the size of memory
//         let mut mem_d1 = StdMemD1::new(32, 2, 1);
//         let val = Value::try_from_init(5, 32).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr = Value::try_from_init(4, 3).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr);
//         let mut _mem_out = mem_d1.execute_mut(&[input, write_en, addr0]);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d1_panic_input() {
//         // Input width larger than the memory capacity
//         let mut mem_d1 = StdMemD1::new(2, 2, 1);
//         let val = Value::try_from_init(10, 4).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr = Value::try_from_init(1, 1).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr);
//         let mut _mem_out = mem_d1.execute_mut(&[input, write_en, addr0]);
//     }
//     #[test]
//     fn test_mem_d2_tlv() {
//         let mut mem_d2 = StdMemD2::new(32, 8, 8, 3, 3);
//         let val = Value::try_from_init(5, 32).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(2, 3).unwrap();
//         let addr_1 = Value::try_from_init(0, 3).unwrap();
//         let input = (ir::Id::from("write_data"), val.clone());
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let mut mem_out = mem_d2.execute_mut(&[input, write_en, addr0, addr1]);
//         match &mut mem_out[..] {
//             [read_data, done] => match (read_data, done) {
//                 (
//                     (_, OutputValue::LockedValue(rd)),
//                     (_, OutputValue::LockedValue(d)),
//                 ) => {
//                     assert_eq!(rd.get_count(), 1);
//                     assert_eq!(d.get_count(), 1);
//                     rd.dec_count();
//                     d.dec_count();
//                     assert_eq!(rd.unlockable(), true);
//                     assert_eq!(d.unlockable(), true);
//                     assert_eq!(
//                         rd.clone().unlock().as_u64(),
//                         val.clone().as_u64()
//                     );
//                     assert_eq!(d.clone().unlock().as_u64(), 1);
//                 }
//                 _ => {
//                     panic!("std_mem did not return 2 LockedValues")
//                 }
//             },
//             _ => panic!("Returned more than 2 outputs"),
//         }
//     }
//     #[test]
//     fn test_mem_d2_imval() {
//         let mut mem_d2 = StdMemD2::new(32, 8, 8, 3, 3);
//         let val = Value::try_from_init(5, 32).unwrap();
//         let enable = Value::try_from_init(0, 1).unwrap();
//         let addr_0 = Value::try_from_init(2, 3).unwrap();
//         let addr_1 = Value::try_from_init(0, 3).unwrap();
//         let input = (ir::Id::from("write_data"), val.clone());
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let mut mem_out = mem_d2
//             .execute_mut(&[input, write_en, addr0, addr1])
//             .into_iter();
//         let (read_data, done) =
//             (mem_out.next().unwrap(), mem_out.next().unwrap());
//         assert!(mem_out.next().is_none()); //make sure it's only of length 2
//         let rd = read_data.1.unwrap_imm();
//         let d = done.1.unwrap_imm();
//         assert_eq!(rd.as_u64(), 0); // assuming this b/c mem hasn't been initialized
//         assert_eq!(d.as_u64(), 0);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d2_panic_addr0() {
//         // Access address larger than the size of memory
//         let mut mem_d2 = StdMemD2::new(32, 2, 1, 2, 1);
//         let val = Value::try_from_init(5, 32).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(4, 3).unwrap();
//         let addr_1 = Value::try_from_init(0, 1).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let mut _mem_out = mem_d2.execute_mut(&[input, write_en, addr0, addr1]);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d2_panic_addr1() {
//         // Access address larger than the size of memory
//         let mut mem_d2 = StdMemD2::new(32, 2, 1, 2, 1);
//         let val = Value::try_from_init(5, 32).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(0, 1).unwrap();
//         let addr_1 = Value::try_from_init(4, 3).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let mut _mem_out = mem_d2.execute_mut(&[input, write_en, addr0, addr1]);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d2_panic_input() {
//         // Input width larger than the memory capacity
//         let mut mem_d2 = StdMemD2::new(2, 2, 1, 2, 1);
//         let val = Value::try_from_init(10, 4).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(0, 1).unwrap();
//         let addr_1 = Value::try_from_init(1, 1).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let mut _mem_out = mem_d2.execute_mut(&[input, write_en, addr0, addr1]);
//     }
//     #[test]
//     fn test_mem_d3_tlv() {
//         let mut mem_d3 = StdMemD3::new(1, 2, 2, 2, 1, 1, 1);
//         let val = Value::try_from_init(1, 1).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap(); //so nothing will be written
//         let addr0 = Value::try_from_init(1, 1).unwrap();
//         let addr1 = (ir::Id::from("addr1"), addr0.clone());
//         let addr2 = (ir::Id::from("addr2"), addr0.clone());
//         let addr0 = (ir::Id::from("addr0"), addr0.clone());
//         let input = (ir::Id::from("write_data"), val.clone());
//         let write_en = (ir::Id::from("write_en"), enable.clone());
//         let mut mem_out = mem_d3
//             .execute_mut(&[input, write_en, addr0, addr1, addr2])
//             .into_iter();
//         let (read_data, done) =
//             (mem_out.next().unwrap(), mem_out.next().unwrap());
//         assert!(mem_out.next().is_none()); //make sure it's only of length 2
//         let mut rd = read_data.1.unwrap_tlv();
//         let mut d = done.1.unwrap_tlv();
//         assert_eq!(rd.get_count(), 1);
//         assert_eq!(d.get_count(), 1);
//         rd.dec_count();
//         d.dec_count();
//         assert_eq!(rd.unlockable(), true);
//         assert_eq!(d.unlockable(), true);
//         assert_eq!(rd.clone().unlock().as_u64(), val.clone().as_u64());
//         assert_eq!(d.clone().unlock().as_u64(), 1);
//     }
//     #[test]
//     fn test_mem_d3_imval() {
//         let mut mem_d3 = StdMemD3::new(1, 2, 2, 2, 1, 1, 1);
//         let val = Value::try_from_init(1, 1).unwrap();
//         let enable = Value::try_from_init(0, 1).unwrap(); //so nothing will be written
//         let addr0 = Value::try_from_init(1, 1).unwrap();
//         let addr1 = (ir::Id::from("addr1"), addr0.clone());
//         let addr2 = (ir::Id::from("addr2"), addr0.clone());
//         let addr0 = (ir::Id::from("addr0"), addr0.clone());
//         let input = (ir::Id::from("write_data"), val.clone());
//         let write_en = (ir::Id::from("write_en"), enable.clone());
//         let mut mem_out = mem_d3
//             .execute_mut(&[input, write_en, addr0, addr1, addr2])
//             .into_iter();
//         let (read_data, done) =
//             (mem_out.next().unwrap(), mem_out.next().unwrap());
//         assert!(mem_out.next().is_none()); //make sure it's only of length 2
//         let rd = read_data.1.unwrap_imm();
//         let d = done.1.unwrap_imm();
//         assert_eq!(rd.as_u64(), 0); // assuming this b/c mem hasn't been initialized
//         assert_eq!(d.as_u64(), 0);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d3_panic_addr0() {
//         // Access address larger than the size of memory
//         let mut mem_d3 = StdMemD3::new(1, 2, 2, 2, 1, 1, 1); //2 x 2 x 2, storing 1 bit in each slot
//         let val = Value::try_from_init(1, 1).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(0, 4).unwrap();
//         let addr_1 = Value::try_from_init(1, 1).unwrap();
//         let addr_2 = Value::try_from_init(1, 1).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let addr2 = (ir::Id::from("addr2"), addr_2);
//         let mut _mem_out =
//             mem_d3.execute_mut(&[input, write_en, addr0, addr1, addr2]);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d3_panic_addr1() {
//         // Access address larger than the size of memory
//         let mut mem_d3 = StdMemD3::new(1, 2, 2, 2, 1, 1, 1); //2 x 2 x 2, storing 1 bit in each slot
//         let val = Value::try_from_init(1, 1).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(0, 1).unwrap();
//         let addr_1 = Value::try_from_init(1, 4).unwrap();
//         let addr_2 = Value::try_from_init(1, 1).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let addr2 = (ir::Id::from("addr2"), addr_2);
//         let mut _mem_out =
//             mem_d3.execute_mut(&[input, write_en, addr0, addr1, addr2]);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d3_panic_addr2() {
//         // Access address larger than the size of memory
//         let mut mem_d3 = StdMemD3::new(1, 2, 2, 2, 1, 1, 1); //2 x 2 x 2, storing 1 bit in each slot
//         let val = Value::try_from_init(1, 1).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(0, 1).unwrap();
//         let addr_1 = Value::try_from_init(1, 1).unwrap();
//         let addr_2 = Value::try_from_init(1, 4).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let addr2 = (ir::Id::from("addr2"), addr_2);
//         let mut _mem_out =
//             mem_d3.execute_mut(&[input, write_en, addr0, addr1, addr2]);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d3_panic_input() {
//         // Input width larger than the memory capacity
//         let mut mem_d3 = StdMemD3::new(1, 2, 2, 2, 1, 1, 1);
//         let val = Value::try_from_init(10, 4).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(0, 1).unwrap();
//         let addr_1 = Value::try_from_init(1, 1).unwrap();
//         let addr_2 = Value::try_from_init(1, 1).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let addr2 = (ir::Id::from("addr2"), addr_2);
//         let mut _mem_out =
//             mem_d3.execute_mut(&[input, write_en, addr0, addr1, addr2]);
//     }
//     #[test]
//     fn test_mem_d4_tlv() {
//         let mut mem_d4 = StdMemD4::new(1, 2, 2, 2, 2, 1, 1, 1, 1);
//         let val = Value::try_from_init(1, 1).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap(); //so nothing will be written
//         let addr0 = Value::try_from_init(1, 1).unwrap();
//         let addr1 = (ir::Id::from("addr1"), addr0.clone());
//         let addr2 = (ir::Id::from("addr2"), addr0.clone());
//         let addr3 = (ir::Id::from("addr3"), addr0.clone());
//         let addr0 = (ir::Id::from("addr0"), addr0.clone());
//         let input = (ir::Id::from("write_data"), val.clone());
//         let write_en = (ir::Id::from("write_en"), enable.clone());
//         let mut mem_out = mem_d4
//             .execute_mut(&[input, write_en, addr0, addr1, addr2, addr3])
//             .into_iter();
//         let (read_data, done) =
//             (mem_out.next().unwrap(), mem_out.next().unwrap());
//         assert!(mem_out.next().is_none()); //make sure it's only of length 2
//         let mut rd = read_data.1.unwrap_tlv();
//         let mut d = done.1.unwrap_tlv();
//         assert_eq!(rd.get_count(), 1);
//         assert_eq!(d.get_count(), 1);
//         rd.dec_count();
//         d.dec_count();
//         assert_eq!(rd.unlockable(), true);
//         assert_eq!(d.unlockable(), true);
//         assert_eq!(rd.clone().unlock().as_u64(), val.clone().as_u64());
//         assert_eq!(d.clone().unlock().as_u64(), 1);
//     }
//     #[test]
//     fn test_mem_d4_imval() {
//         let mut mem_d4 = StdMemD4::new(32, 8, 8, 8, 8, 3, 3, 3, 3);
//         let val = Value::try_from_init(5, 32).unwrap();
//         let enable = Value::try_from_init(0, 1).unwrap();
//         let addr_0 = Value::try_from_init(2, 3).unwrap();
//         let addr_1 = Value::try_from_init(1, 3).unwrap();
//         let addr_2 = Value::try_from_init(5, 3).unwrap();
//         let addr_3 = Value::try_from_init(2, 3).unwrap();
//         let input = (ir::Id::from("write_data"), val.clone());
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let addr2 = (ir::Id::from("addr2"), addr_2);
//         let addr3 = (ir::Id::from("addr3"), addr_3);
//         let mut mem_out = mem_d4
//             .execute_mut(&[input, write_en, addr0, addr1, addr2, addr3])
//             .into_iter();
//         let (read_data, done) =
//             (mem_out.next().unwrap(), mem_out.next().unwrap());
//         assert!(mem_out.next().is_none()); //make sure it's only of length 2
//         let rd = read_data.1.unwrap_imm();
//         let d = done.1.unwrap_imm();
//         assert_eq!(rd.as_u64(), 0); // assuming this b/c mem hasn't been initialized
//         assert_eq!(d.as_u64(), 0);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d4_panic_addr0() {
//         // Access address larger than the size of memory
//         let mut mem_d4 = StdMemD4::new(32, 3, 2, 3, 2, 3, 2, 3, 2);
//         let val = Value::try_from_init(5, 32).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(4, 3).unwrap();
//         let addr_1 = Value::try_from_init(0, 2).unwrap();
//         let addr_2 = Value::try_from_init(1, 2).unwrap();
//         let addr_3 = Value::try_from_init(2, 2).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let addr2 = (ir::Id::from("addr2"), addr_2);
//         let addr3 = (ir::Id::from("addr3"), addr_3);
//         let mut _mem_out =
//             mem_d4.execute_mut(&[input, write_en, addr0, addr1, addr2, addr3]);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d4_panic_addr1() {
//         // Access address larger than the size of memory
//         let mut mem_d4 = StdMemD4::new(32, 3, 2, 3, 2, 3, 2, 3, 2);
//         let val = Value::try_from_init(5, 32).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(0, 2).unwrap();
//         let addr_1 = Value::try_from_init(4, 3).unwrap();
//         let addr_2 = Value::try_from_init(1, 2).unwrap();
//         let addr_3 = Value::try_from_init(2, 2).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let addr2 = (ir::Id::from("addr2"), addr_2);
//         let addr3 = (ir::Id::from("addr3"), addr_3);
//         let mut _mem_out =
//             mem_d4.execute_mut(&[input, write_en, addr0, addr1, addr2, addr3]);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d4_panic_addr2() {
//         // Access address larger than the size of memory
//         let mut mem_d4 = StdMemD4::new(32, 3, 2, 3, 2, 3, 2, 3, 2);
//         let val = Value::try_from_init(5, 32).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(0, 2).unwrap();
//         let addr_1 = Value::try_from_init(1, 2).unwrap();
//         let addr_2 = Value::try_from_init(4, 3).unwrap();
//         let addr_3 = Value::try_from_init(2, 2).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let addr2 = (ir::Id::from("addr2"), addr_2);
//         let addr3 = (ir::Id::from("addr3"), addr_3);
//         let mut _mem_out =
//             mem_d4.execute_mut(&[input, write_en, addr0, addr1, addr2, addr3]);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d4_panic_addr3() {
//         // Access address larger than the size of memory
//         let mut mem_d4 = StdMemD4::new(32, 3, 2, 3, 2, 3, 2, 3, 2);
//         let val = Value::try_from_init(5, 32).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(0, 2).unwrap();
//         let addr_1 = Value::try_from_init(1, 2).unwrap();
//         let addr_2 = Value::try_from_init(2, 2).unwrap();
//         let addr_3 = Value::try_from_init(4, 3).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let addr2 = (ir::Id::from("addr2"), addr_2);
//         let addr3 = (ir::Id::from("addr3"), addr_3);
//         let mut _mem_out =
//             mem_d4.execute_mut(&[input, write_en, addr0, addr1, addr2, addr3]);
//     }
//     #[test]
//     #[should_panic]
//     fn test_mem_d4_panic_input() {
//         // Input width larger than the memory capacity
//         let mut mem_d4 = StdMemD4::new(32, 3, 2, 3, 2, 3, 2, 3, 2);
//         let val = Value::try_from_init(10, 4).unwrap();
//         let enable = Value::try_from_init(1, 1).unwrap();
//         let addr_0 = Value::try_from_init(0, 2).unwrap();
//         let addr_1 = Value::try_from_init(1, 2).unwrap();
//         let addr_2 = Value::try_from_init(2, 2).unwrap();
//         let addr_3 = Value::try_from_init(3, 2).unwrap();
//         let input = (ir::Id::from("write_data"), val);
//         let write_en = (ir::Id::from("write_en"), enable);
//         let addr0 = (ir::Id::from("addr0"), addr_0);
//         let addr1 = (ir::Id::from("addr1"), addr_1);
//         let addr2 = (ir::Id::from("addr2"), addr_2);
//         let addr3 = (ir::Id::from("addr3"), addr_3);
//         let mut _mem_out =
//             mem_d4.execute_mut(&[input, write_en, addr0, addr1, addr2, addr3]);
//     }
//     #[test]
//     fn test_std_reg_tlv() {
//         let val = Value::try_from_init(16, 6).unwrap();
//         let mut reg1 = StdReg::new(6);
//         let input = ir::Id::from("in");
//         let input_tup = (input, val.clone());
//         let write_en = ir::Id::from("write_en");
//         let write_en_tup = (write_en, Value::try_from_init(1, 1).unwrap());
//         let output_vals = reg1.execute_mut(&[input_tup, write_en_tup]);
//         println!("output_vals: {:?}", output_vals);
//         let mut output_vals = output_vals.into_iter();
//         let (read_data, done) =
//             (output_vals.next().unwrap(), output_vals.next().unwrap());
//         assert!(output_vals.next().is_none()); //make sure it's only of length 2
//         let mut d = done.1.unwrap_tlv();
//         let mut rd = read_data.1.unwrap_tlv();
//         assert_eq!(rd.get_count(), 1);
//         assert_eq!(d.get_count(), 1);
//         rd.dec_count();
//         d.dec_count();
//         assert_eq!(rd.unlockable(), true);
//         assert_eq!(d.unlockable(), true);
//         assert_eq!(rd.clone().unlock().as_u64(), val.clone().as_u64());
//     }

//     #[test]
//     fn test_std_reg_imval() {
//         let val = Value::try_from_init(16, 6).unwrap();
//         let mut reg1 = StdReg::new(6);
//         let input = ir::Id::from("in");
//         let input_tup = (input, val.clone());
//         let write_en = ir::Id::from("write_en");
//         let write_en_tup = (write_en, Value::try_from_init(0, 1).unwrap());
//         let output_vals = reg1.execute_mut(&[input_tup, write_en_tup]);
//         println!("output_vals: {:?}", output_vals);
//         let mut output_vals = output_vals.into_iter();
//         let (read_data, done) =
//             (output_vals.next().unwrap(), output_vals.next().unwrap());
//         assert!(output_vals.next().is_none()); //make sure it's only of length 2
//         let d = done.1.unwrap_imm();
//         let rd = read_data.1.unwrap_imm();
//         assert_eq!(d.as_u64(), 0);
//         assert_eq!(rd.as_u64(), 0); // assuming this b/c reg1 hasn't been initialized
//     }
//     #[test]
//     #[should_panic]
//     fn reg_too_big() {
//         let mut reg1 = StdReg::new(5);
//         // now try loading in a value that is too big(??)
//         let val = Value::try_from_init(32, 6).unwrap();
//         let input = (ir::Id::from("in"), val);
//         let write_en = (
//             ir::Id::from("write_en"),
//             Value::try_from_init(1, 1).unwrap(),
//         );
//         let _output_vals = reg1.execute_mut(&[input, write_en]);
//     }
//     #[test]
//     fn test_std_const() {
//         let val_31 = Value::try_from_init(31, 5).unwrap();
//         let const_31 = StdConst::new(5, val_31);
//         assert_eq!(const_31.read_val().as_u64(), 31); //can rust check this equality?
//         assert_eq!(const_31.read_u64(), 31);
//     }
//     #[test]
//     #[should_panic]
//     fn test_std_const_panic() {
//         let val = Value::try_from_init(75, 7).unwrap();
//         let std_const = StdConst::new(5, val);
//     }
//     #[test]
//     fn test_std_lsh() {
//         // lsh with overflow
//         // [11111] (31) -> [11100] (28)
//         let left = Value::try_from_init(31, 5).unwrap();
//         let right = Value::try_from_init(2, 5).unwrap(); //lsh takes only values as parameters
//         let lsh = StdLsh::new(5);
//         let out = lsh.execute_bin(&left, &right).unwrap_imm();
//         println!("lsh of 31 by 2: {}", out);
//         assert_eq!(out.as_u64(), 28);
//         // lsh without overflow
//         // lsh [010000] (16) by 1 -> [100000] (32)
//         let left = Value::try_from_init(16, 6).unwrap();
//         let right = Value::try_from_init(1, 6).unwrap();
//         let lsh = StdLsh::new(6);
//         let out = lsh.execute_bin(&left, &right).unwrap_imm();
//         assert_eq!(out.as_u64(), 32);
//     }
//     #[test]
//     fn test_std_rsh() {
//         // Not sure how to catagorize this
//         // [1111] (15) -> [0011] (3)
//         let left = Value::try_from_init(15, 4).unwrap();
//         let right = Value::try_from_init(2, 4).unwrap();
//         let rsh = StdRsh::new(4);
//         let out = rsh.execute_bin(&left, &right).unwrap_imm();
//         assert_eq!(out.as_u64(), 3);
//         // Division by 2
//         // [1000] (8) -> [0100] ( 4)
//         let left = Value::try_from_init(8, 4).unwrap();
//         let right = Value::try_from_init(1, 4).unwrap();
//         let out = rsh.execute_bin(&left, &right).unwrap_imm();
//         assert_eq!(out.as_u64(), 4);
//     }
//     #[test]
//     fn test_std_add() {
//         // without overflow
//         // add [0011] (3) and [1010] (10) -> [1101] (13)
//         let add0 = Value::try_from_init(3, 4).unwrap();
//         let add1 = Value::try_from_init(10, 4).unwrap();
//         let add = StdAdd::new(4);
//         let res_add = add.execute_bin(&add0, &add1).unwrap_imm();
//         assert_eq!(res_add.as_u64(), 13);
//         // with overflow
//         // add [1010] (10) and [0110] (6) -> [0000] (0)
//         let add0 = Value::try_from_init(10, 4).unwrap();
//         let add1 = Value::try_from_init(6, 4).unwrap();
//         let res_add = add.execute_bin(&add0, &add1).unwrap_imm();
//         assert_eq!(res_add.as_u64(), 0);
//     }
//     #[test]
//     #[should_panic]
//     fn test_std_add_panic() {
//         let add0 = Value::try_from_init(81, 7).unwrap();
//         let add1 = Value::try_from_init(10, 4).unwrap();
//         let add = StdAdd::new(7);
//         let res_add = add.execute_bin(&add0, &add1);
//     }
//     #[test]
//     fn test_std_sub() {
//         // without overflow
//         // sub [0110] (6) from [1010] (10) -> [0100] (4)
//         let sub0 = Value::try_from_init(10, 4).unwrap();
//         let sub1 = Value::try_from_init(6, 4).unwrap();
//         let sub = StdSub::new(4);
//         let res_sub = sub.execute_bin(&sub0, &sub1).unwrap_imm();
//         assert_eq!(res_sub.as_u64(), 4);
//         // with overflow (would produce a negative #, depending on how program thinks abt this...)
//         // sub [1011] (11) from [1010] (10) ->  [1010] + [0101] = [1111] which is -1 in 2bc and 15 unsigned
//         // for some reason producing [0101] ? that's just 'right + 1
//         let sub1 = Value::try_from_init(11, 4).unwrap();
//         let res_sub = sub.execute_bin(&sub0, &sub1).unwrap_imm();
//         assert_eq!(res_sub.as_u64(), 15);
//         // sub [1111] (15) from [1000] (8) -> [1000] + [0001] which is [1001] -7 in 2c but 9 in unsigned
//         let sub0 = Value::try_from_init(8, 4).unwrap();
//         let sub1 = Value::try_from_init(15, 4).unwrap();
//         let res_sub = sub.execute_bin(&sub0, &sub1).unwrap_imm();
//         assert_eq!(res_sub.as_u64(), 9);
//     }
//     #[test]
//     #[should_panic]
//     fn test_std_sub_panic() {
//         let sub0 = Value::try_from_init(52, 6).unwrap();
//         let sub1 = Value::try_from_init(16, 5).unwrap();
//         let sub = StdAdd::new(5);
//         let res_sub = sub.execute_bin(&sub0, &sub1);
//     }
//     #[test]
//     fn test_std_slice() {
//         // 101 in binary is [1100101], take first 4 bits -> [0101] = 5
//         let to_slice = Value::try_from_init(101, 7).unwrap();
//         let std_slice = StdSlice::new(7, 4);
//         let res_slice = std_slice.execute_unary(&to_slice).unwrap_imm(); //note that once we implement execute_unary, have to change this
//         assert_eq!(res_slice.as_u64(), 5);
//         // Slice the entire bit
//         let to_slice = Value::try_from_init(548, 10).unwrap();
//         let std_slice = StdSlice::new(10, 10);
//         let res_slice = std_slice.execute_unary(&to_slice).unwrap_imm();
//         assert_eq!(res_slice.as_u64(), 548);
//     }
//     #[test]
//     #[should_panic]
//     fn test_std_slice_panic() {
//         let to_slice = Value::try_from_init(3, 2).unwrap();
//         let std_slice = StdSlice::new(7, 4);
//         let res_slice = std_slice.execute_unary(&to_slice);
//     }
//     #[test]
//     fn test_std_pad() {
//         // Add 2 zeroes, should keep the same value
//         let to_pad = Value::try_from_init(101, 7).unwrap();
//         let std_pad = StdPad::new(7, 9);
//         let res_pad = std_pad.execute_unary(&to_pad).unwrap_imm();
//         assert_eq!(res_pad.as_u64(), 101);
//         // hard to think of another test case but just to have 2:
//         let to_pad = Value::try_from_init(1, 7).unwrap();
//         let res_pad = std_pad.execute_unary(&to_pad).unwrap_imm();
//         assert_eq!(res_pad.as_u64(), 1);
//     }
//     #[test]
//     #[should_panic]
//     fn test_std_pad_panic() {
//         let to_pad = Value::try_from_init(21, 5).unwrap();
//         let std_pad = StdPad::new(3, 9);
//         let res_pad = std_pad.execute_unary(&to_pad);
//     }
//     /// Logical Operators
//     #[test]
//     fn test_std_not() {
//         // ![1010] (!10) -> [0101] (5)
//         let not0 = Value::try_from_init(10, 4).unwrap();
//         let std_not = StdNot::new(4);
//         let res_not = std_not.execute_unary(&not0).unwrap_imm();
//         assert_eq!(res_not.as_u64(), 5);
//         // ![0000] (!0) -> [1111] (15)
//         let not0 = Value::try_from_init(0, 4).unwrap();
//         let res_not = std_not.execute_unary(&not0).unwrap_imm();
//         assert_eq!(res_not.as_u64(), 15);
//     }

//     #[test]
//     #[should_panic]
//     fn test_std_not_panic() {
//         //input too short
//         let not0 = Value::try_from_init(0, 4).unwrap();
//         let std_not = StdNot::new(5);
//         let _res_not = std_not.execute_unary(&not0).unwrap_imm();
//     }

//     #[test]
//     fn test_std_and() {
//         //101: [1100101], 78: [1001110] & -> [1000100] which is 68
//         let and0 = Value::try_from_init(101, 7).unwrap();
//         let and1 = Value::try_from_init(78, 7).unwrap();
//         let std_and = StdAnd::new(7);
//         let res_and = std_and.execute_bin(&and0, &and1).unwrap_imm();
//         assert_eq!(res_and.as_u64(), 68);
//         //[1010] (10) & [0101] (5) is [0000]
//         let and0 = Value::try_from_init(10, 4).unwrap();
//         let and1 = Value::try_from_init(5, 4).unwrap();
//         let std_and = StdAnd::new(4);
//         let res_and = std_and.execute_bin(&and0, &and1).unwrap_imm();
//         assert_eq!(res_and.as_u64(), 0);
//     }

//     #[test]
//     #[should_panic]
//     fn test_std_and_panic() {
//         let and0 = Value::try_from_init(91, 7).unwrap();
//         let and1 = Value::try_from_init(43, 6).unwrap();
//         let std_and = StdAnd::new(7);
//         let _res_and = std_and.execute_bin(&and0, &and1).unwrap_imm();
//     }

//     #[test]
//     fn test_std_or() {
//         //[101] (5) or [011] (3) is [111] (7)
//         let or0 = Value::try_from_init(5, 3).unwrap();
//         let or1 = Value::try_from_init(3, 3).unwrap();
//         let std_or = StdOr::new(3);
//         let res_or = std_or.execute_bin(&or0, &or1).unwrap_imm();
//         assert_eq!(res_or.as_u64(), 7);
//         //anything or zero is itself
//         //[001] (1) or [000] (0) is [001] (1)
//         let or0 = Value::try_from_init(1, 3).unwrap();
//         let or1 = Value::try_from_init(0, 3).unwrap();
//         let res_or = std_or.execute_bin(&or0, &or1).unwrap_imm();
//         assert_eq!(res_or.as_u64(), or0.as_u64());
//     }

//     #[test]
//     #[should_panic]
//     fn test_std_or_panic() {
//         let or0 = Value::try_from_init(16, 5).unwrap();
//         let or1 = Value::try_from_init(78, 7).unwrap();
//         let std_or = StdOr::new(5);
//         let _res_or = std_or.execute_bin(&or0, &or1).unwrap_imm();
//     }
//     #[test]
//     fn test_std_xor() {
//         //[101] (5) XOR [011] (3) is [110] (6)
//         let xor0 = Value::try_from_init(5, 3).unwrap();
//         let xor1 = Value::try_from_init(3, 3).unwrap();
//         let std_xor = StdXor::new(3);
//         let res_xor = std_xor.execute_bin(&xor0, &xor1).unwrap_imm();
//         assert_eq!(res_xor.as_u64(), 6);
//         //anything xor itself is 0
//         assert_eq!(std_xor.execute_bin(&xor0, &xor0).unwrap_imm().as_u64(), 0);
//     }
//     #[test]
//     #[should_panic]
//     fn test_std_xor_panic() {
//         let xor0 = Value::try_from_init(56, 6).unwrap();
//         let xor1 = Value::try_from_init(92, 7).unwrap();
//         let std_xor = StdXor::new(6);
//         let _res_xor = std_xor.execute_bin(&xor0, &xor1).unwrap_imm();
//     }
//     /// Comparison Operators
//     // is there any point in testing this more than once?
//     // no weird overflow or anything. maybe test along with
//     // equals
//     #[test]
//     fn test_std_gt() {
//         let gt0 = Value::try_from_init(7, 16).unwrap();
//         let gt1 = Value::try_from_init(3, 16).unwrap();
//         let std_gt = StdGt::new(16);
//         let res_gt = std_gt.execute_bin(&gt0, &gt1).unwrap_imm();
//         assert_eq!(res_gt.as_u64(), 1);
//         //7 > 7 ? no!
//         assert_eq!(std_gt.execute_bin(&gt0, &gt0).unwrap_imm().as_u64(), 0);
//     }
//     #[test]
//     #[should_panic]
//     fn test_std_gt_panic() {
//         let gt0 = Value::try_from_init(9, 4).unwrap();
//         let gt1 = Value::try_from_init(3, 2).unwrap();
//         let std_gt = StdGt::new(3);
//         let res_gt = std_gt.execute_bin(&gt0, &gt1);
//     }
//     #[test]
//     fn test_std_lt() {
//         let lt0 = Value::try_from_init(7, 16).unwrap();
//         let lt1 = Value::try_from_init(3, 16).unwrap();
//         let std_lt = StdLt::new(16);
//         let res_lt = std_lt.execute_bin(&lt0, &lt1).unwrap_imm();
//         assert_eq!(res_lt.as_u64(), 0);
//         // 7 < 7 ? no!
//         assert_eq!(std_lt.execute_bin(&lt0, &lt0).unwrap_imm().as_u64(), 0);
//     }
//     #[test]
//     #[should_panic]
//     fn test_std_lt_panic() {
//         let lt0 = Value::try_from_init(58, 6).unwrap();
//         let lt1 = Value::try_from_init(12, 4).unwrap();
//         let std_lt = StdLt::new(5);
//         let res_lt = std_lt.execute_bin(&lt0, &lt1);
//     }
//     #[test]
//     fn test_std_eq() {
//         let eq0 = Value::try_from_init(4, 16).unwrap();
//         let eq1 = Value::try_from_init(4, 16).unwrap();
//         let std_eq = StdEq::new(16);
//         let res_eq = std_eq.execute_bin(&eq0, &eq1).unwrap_imm();
//         assert_eq!(res_eq.as_u64(), 1);
//         // 4 = 5 ? no!
//         assert_eq!(
//             std_eq
//                 .execute_bin(&eq0, &(Value::try_from_init(5, 16).unwrap()))
//                 .unwrap_imm()
//                 .as_u64(),
//             0
//         );
//     }
//     #[test]
//     #[should_panic]
//     fn test_std_eq_panic() {
//         let eq0 = Value::try_from_init(42, 6).unwrap();
//         let std_eq = StdEq::new(5);
//         let res_eq = std_eq.execute_bin(&eq0, &eq0);
//     }
//     #[test]
//     fn test_std_neq() {
//         let neq0 = Value::try_from_init(4, 16).unwrap();
//         let neq1 = Value::try_from_init(4, 16).unwrap();
//         let std_neq = StdNeq::new(16);
//         let res_neq = std_neq.execute_bin(&neq0, &neq1).unwrap_imm();
//         //4 != 4 ? no!
//         assert!(res_neq.as_u64() == 0);
//         // 4 != 5? yes!
//         assert_eq!(
//             std_neq
//                 .execute_bin(&neq0, &(Value::try_from_init(5, 16).unwrap()))
//                 .unwrap_imm()
//                 .as_u64(),
//             1
//         );
//     }
//     #[test]
//     #[should_panic]
//     fn test_std_neq_panic() {
//         let neq0 = Value::try_from_init(45, 6).unwrap();
//         let neq1 = Value::try_from_init(4, 3).unwrap();
//         let std_neq = StdNeq::new(5);
//         let res_neq = std_neq.execute_bin(&neq0, &neq1);
//     }

//     #[test]
//     fn test_std_ge() {
//         let ge0 = Value::try_from_init(35, 8).unwrap();
//         let ge1 = Value::try_from_init(165, 8).unwrap();
//         let std_ge = StdGe::new(8);
//         let res_ge = std_ge.execute_bin(&ge0, &ge1).unwrap_imm();
//         //35 >= 165 ? no!
//         assert_eq!(res_ge.as_u64(), 0);
//         // 35 >= 35 ? yes
//         assert_eq!(std_ge.execute_bin(&ge0, &ge0).unwrap_imm().as_u64(), 1);
//     }
//     #[test]
//     #[should_panic]
//     fn test_std_ge_panic() {
//         let ge0 = Value::try_from_init(40, 6).unwrap();
//         let ge1 = Value::try_from_init(75, 7).unwrap();
//         let std_ge = StdGe::new(6);
//         let res_ge = std_ge.execute_bin(&ge0, &ge1);
//     }
//     #[test]
//     fn test_std_le() {
//         let le0 = Value::try_from_init(12, 4).unwrap();
//         let le1 = Value::try_from_init(8, 4).unwrap();
//         let std_le = StdLe::new(4);
//         let res_le = std_le.execute_bin(&le0, &le1).unwrap_imm();
//         //12 <= 4 ? no!
//         assert_eq!(res_le.as_u64(), 0);
//         //12 <= 12? yes!
//         assert_eq!(std_le.execute_bin(&le0, &le0).unwrap_imm().as_u64(), 1);
//     }
//     #[test]
//     #[should_panic]
//     fn test_std_le_panic() {
//         let le0 = Value::try_from_init(93, 7).unwrap();
//         let le1 = Value::try_from_init(68, 7).unwrap();
//         let std_le = StdLe::new(6);
//         let res_le = std_le.execute_bin(&le0, &le1);
//     }
// }

#[cfg(test)]
mod val_test {
    use crate::values::Value;
    #[test]
    fn basic_print_test() {
        let v1 = Value::try_from_init(12, 5).unwrap();
        println!("12 with bit width 5: {}", v1);
        assert_eq!(v1.as_u64(), 12);
    }
    #[test]
    fn basic_print_test2() {
        let v1 = Value::try_from_init(33, 6).unwrap();
        println!("33 with bit width 6: {}", v1);
        assert_eq!(v1.as_u64(), 33);
    }
    #[test]
    fn too_few_bits() {
        let v_16_4 = Value::try_from_init(16, 4).unwrap();
        println!("16 with bit width 4: {}", v_16_4);
        assert_eq!(v_16_4.as_u64(), 0);
        let v_31_4 = Value::try_from_init(31, 4).unwrap();
        println!("31 with bit width 4: {}", v_31_4);
        let v_15_4 = Value::try_from_init(15, 4).unwrap();
        println!("15 with bit width 4: {}", v_15_4);
        assert_eq!(v_31_4.as_u64(), v_15_4.as_u64());
    }
    #[test]
    fn clear() {
        let v_15_4 = Value::try_from_init(15, 4).unwrap();
        let v_15_4 = v_15_4.clear();
        println!("15 with bit width 4 AFTER clear: {}", v_15_4);
        assert_eq!(v_15_4.as_u64(), 0);
    }
    #[test]
    fn ext() {
        let v_15_4 = Value::try_from_init(15, 4).unwrap();
        assert_eq!(v_15_4.as_u64(), v_15_4.ext(8).as_u64());
    }

    //is there even a point of sext, if bit_vec can't take negative numbers? Or can it?
}
