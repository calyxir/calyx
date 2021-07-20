#[allow(unused)]
use crate::port_bindings;
#[allow(unused)]
use crate::primitives::{combinational as comb, stateful as stfl, Primitive};
#[allow(unused)]
use crate::values::{OutputValue, ReadableValue, TickableValue, Value};
#[allow(unused)]
use calyx::ir;

// #[test]
// fn test_std_mult_pipe() {
//     let mut std_mult = stfl::StdMultPipe::from_constants(16);
//     port_bindings![binds;
//         left -> (3, 16),
//         right -> (5, 16),
//         go -> (1, 1)
//     ];
//     let mut output_vals =
//         std_mult.validate_and_execute(&binds, Some(&Value::bit_low()));
//     assert_eq!(output_vals.len(), 2); //should be a done val
//     match &mut output_vals[..] {
//         [out, done] => match (out, done) {
//             (
//                 (_, OutputValue::LockedValue(prod)),
//                 (_, OutputValue::PulseValue(d)),
//             ) => {
//                 assert_eq!(prod.get_count(), 1);
//                 assert_eq!(d.get_val().as_u64(), 0);
//                 prod.dec_count();
//                 d.tick();
//                 assert!(prod.unlockable());
//                 assert_eq!(
//                     prod.clone().unlock().as_u64(), //the product should be 15
//                     15
//                 );
//                 //check done value goes to zero
//                 assert_eq!(d.get_val().as_u64(), 1);
//                 let d = d.clone().do_tick();
//                 assert!(matches!(d, OutputValue::ImmediateValue(_)));
//                 if let OutputValue::ImmediateValue(iv) = d {
//                     assert_eq!(iv.as_u64(), 0);
//                 }
//             }
//             _ => {
//                 panic!("std_mult_pipe did not return the expected output types")
//             }
//         },
//         _ => panic!("std_mult_pipe returned more than 2 outputs"),
//     }
//     //now commit updates, and see if changing inputs with a low go give a vec that still has 15
//     std_mult.commit_updates();
//     port_bindings![binds;
//         left -> (7, 16),
//         right -> (5, 16),
//         go -> (0, 1)
//     ];
//     let mut diff_inputs =
//         std_mult.validate_and_execute(&binds, Some(&Value::bit_low()));
//     match &mut diff_inputs[..] {
//         [out] => {
//             match out {
//                 (_, OutputValue::ImmediateValue(val)) => {
//                     assert_eq!(val.as_u64(), 15);
//                 }
//                 _ => {
//                     panic!("std_mult_pipe didn't return an IV when [done] is low")
//                 }
//             }
//         }
//         _ => panic!(
//             "std_mult_pipe returned more than 1 output after executing with low done"
//         ),
//     }
// }

// #[test]
// fn test_std_div_pipe() {
//     let mut std_div = stfl::StdDivPipe::from_constants(16);
//     port_bindings![binds;
//         left -> (25, 16), //25/3 = 8 r. 1
//         right -> (3, 16),
//         go -> (1, 1)
//     ];
//     let mut output_vals =
//         std_div.validate_and_execute(&binds, Some(&Value::bit_low()));
//     assert_eq!(output_vals.len(), 3); //should be a quotient, remainder, and done val
//     match &mut output_vals[..] {
//         [out_quotient, out_remainder, done] => {
//             match (out_quotient, out_remainder, done) {
//                 (
//                     (_, OutputValue::LockedValue(q)),
//                     (_, OutputValue::LockedValue(r)),
//                     (_, OutputValue::PulseValue(d)),
//                 ) => {
//                     assert_eq!(q.get_count(), 1);
//                     assert_eq!(r.get_count(), 1);
//                     assert_eq!(d.get_val().as_u64(), 0);
//                     q.dec_count();
//                     r.dec_count();
//                     d.tick();
//                     assert!(q.unlockable());
//                     assert_eq!(
//                         q.clone().unlock().as_u64(), //the product should be 15
//                         8
//                     );
//                     assert!(r.unlockable());
//                     assert_eq!(
//                         r.clone().unlock().as_u64(), //the product should be 15
//                         1
//                     );
//                     //check done value goes to zero
//                     assert_eq!(d.get_val().as_u64(), 1);
//                     let d = d.clone().do_tick();
//                     assert!(matches!(d, OutputValue::ImmediateValue(_)));
//                     if let OutputValue::ImmediateValue(iv) = d {
//                         assert_eq!(iv.as_u64(), 0);
//                     }
//                 }
//                 _ => {
//                     panic!(
//                         "std_div_pipe did not return the expected output types"
//                     )
//                 }
//             }
//         }
//         _ => panic!("std_div_pipe did not return 3 outputs"),
//     }
//     //now commit updates, and see if changing inputs with a low go give a vec that still has 8, 1
//     std_div.commit_updates();
//     port_bindings![binds;
//         left -> (7, 16),
//         right -> (5, 16),
//         go -> (0, 1)
//     ];
//     let mut diff_inputs =
//         std_div.validate_and_execute(&binds, Some(&Value::bit_low()));
//     match &mut diff_inputs[..] {
//         [out_quotient, out_remainder] => match (out_quotient, out_remainder) {
//             (
//                 (_, OutputValue::ImmediateValue(q)),
//                 (_, OutputValue::ImmediateValue(r)),
//             ) => {
//                 assert_eq!(q.as_u64(), 8);
//                 assert_eq!(r.as_u64(), 1);
//             }
//             _ => {
//                 panic!("std_div_pipe didn't return an IV when [done] is low")
//             }
//         },
//         _ => panic!(
//             "std_div_pipe returned not 2 outputs after executing with low done"
//         ),
//     }
// }

// #[test]
// fn test_mem_d1_tlv() {
//     let mut mem_d1 = stfl::StdMemD1::from_constants(32, 8, 3);
//     port_bindings![binds;
//         write_data -> (5, 32),
//         write_en -> (1, 1),
//         addr0 -> (2, 3)
//     ];
//     let mut mem_out =
//         mem_d1.validate_and_execute(&binds, Some(&Value::bit_low()));
//     match &mut mem_out[..] {
//         [read_data, done] => match (read_data, done) {
//             (
//                 (_, OutputValue::LockedValue(rd)),
//                 (_, OutputValue::PulseValue(d)),
//             ) => {
//                 assert_eq!(rd.get_count(), 1);
//                 assert_eq!(d.get_val().as_u64(), 0);
//                 rd.dec_count();
//                 d.tick();
//                 assert!(rd.unlockable());
//                 assert_eq!(
//                     rd.clone().unlock().as_u64(),
//                     write_data.clone().as_u64()
//                 );
//                 assert_eq!(d.get_val().as_u64(), 1);
//                 let d = d.clone().do_tick();
//                 assert!(matches!(d, OutputValue::ImmediateValue(_)));
//                 if let OutputValue::ImmediateValue(iv) = d {
//                     assert_eq!(iv.as_u64(), 0);
//                 }
//             }
//             _ => {
//                 panic!("std_mem did not return the expected output types")
//             }
//         },
//         _ => panic!("Returned more than 2 outputs"),
//     }
// }
// #[test]
// fn test_mem_d1_imval() {
//     let mut mem_d1 = stfl::StdMemD1::from_constants(32, 8, 3);
//     port_bindings![binds;
//         write_data -> (5, 32),
//         write_en -> (0, 1),
//         addr0 -> (2, 3)
//     ];
//     let mut mem_out = mem_d1
//         .validate_and_execute(&binds, (&Value::bit_low()).into())
//         .into_iter();
//     if let (read_data, None) = (mem_out.next().unwrap(), mem_out.next()) {
//         let rd = read_data.1.unwrap_imm();
//         assert_eq!(rd.as_u64(), 0); // assuming this b/c mem hasn't been initialized
//     } else {
//         panic!()
//     }
// }
// #[test]
// #[should_panic]
// fn test_mem_d1_panic_addr() {
//     // Access address larger than the size of memory
//     let mut mem_d1 = stfl::StdMemD1::from_constants(32, 2, 1);
//     port_bindings![binds;
//         write_data -> (5, 32),
//         write_en -> (1, 1),
//         addr0 -> (4, 3)
//     ];
//     mem_d1.validate_and_execute(&binds, (&Value::bit_low()).into());
// }
// #[test]
// #[should_panic]
// fn test_mem_d1_panic_input() {
//     // Input width larger than the memory capacity
//     let mut mem_d1 = stfl::StdMemD1::from_constants(2, 2, 1);
//     port_bindings![binds;
//         write_data -> (10, 4),
//         write_en -> (1, 1),
//         addr0 -> (1, 1)
//     ];
//     let mut _mem_out =
//         mem_d1.validate_and_execute(&binds, (&Value::bit_low()).into());
// }
// #[test]
// fn test_mem_d2_tlv() {
//     let mut mem_d2 = stfl::StdMemD2::from_constants(32, 8, 8, 3, 3);
//     port_bindings![binds;
//         write_data -> (5, 32),
//         write_en -> (1, 1),
//         addr0 -> (2, 3),
//         addr1 -> (0 ,3)
//     ];
//     let mut mem_out =
//         mem_d2.validate_and_execute(&binds, Some(&Value::bit_low()));
//     match &mut mem_out[..] {
//         [read_data, done] => match (read_data, done) {
//             (
//                 (_, OutputValue::LockedValue(rd)),
//                 (_, OutputValue::PulseValue(d)),
//             ) => {
//                 assert_eq!(rd.get_count(), 1);
//                 assert_eq!(d.get_val().as_u64(), 0);
//                 rd.dec_count();
//                 d.tick();
//                 assert!(rd.unlockable());
//                 assert_eq!(d.get_val().as_u64(), 1);
//                 assert_eq!(
//                     rd.clone().unlock().as_u64(),
//                     write_data.clone().as_u64()
//                 );
//                 let d = d.clone().do_tick();
//                 assert!(matches!(d, OutputValue::ImmediateValue(_)));
//                 if let OutputValue::ImmediateValue(iv) = d {
//                     assert_eq!(iv.as_u64(), 0);
//                 }
//             }
//             _ => {
//                 panic!("std_mem did not return a lockedval and a pulseval")
//             }
//         },
//         _ => panic!("Returned more than 2 outputs"),
//     }
// }
// #[test]
// fn test_mem_d2_imval() {
//     let mut mem_d2 = stfl::StdMemD2::from_constants(32, 8, 8, 3, 3);
//     port_bindings![binds;
//         write_data -> (5, 32),
//         write_en -> (0, 1),
//         addr0 -> (2, 3),
//         addr1 -> (0 ,3)
//     ];
//     let mut mem_out = mem_d2
//         .validate_and_execute(&binds, Some(&Value::bit_low()))
//         .into_iter();
//     if let (read_data, None) = (mem_out.next().unwrap(), mem_out.next()) {
//         let rd = read_data.1.unwrap_imm();
//         assert_eq!(rd.as_u64(), 0); // assuming this b/c mem hasn't been initialized
//     } else {
//         panic!()
//     }
// }
// #[test]
// #[should_panic]
// fn test_mem_d2_panic_addr0() {
//     // Access address larger than the size of memory
//     let mut mem_d2 = stfl::StdMemD2::from_constants(32, 2, 1, 2, 1);
//     port_bindings![binds;
//         write_data -> (5, 32),
//         write_en -> (1, 1),
//         addr0 -> (4, 3),
//         addr1 -> (0 ,3)
//     ];
//     mem_d2.validate_and_execute(&binds, Some(&Value::bit_low()));
// }
// #[test]
// #[should_panic]
// fn test_mem_d2_panic_addr1() {
//     // Access address larger than the size of memory
//     let mut mem_d2 = stfl::StdMemD2::from_constants(32, 2, 1, 2, 1);
//     port_bindings![binds;
//         write_data -> (5, 32),
//         write_en -> (1, 1),
//         addr0 -> (4, 3),
//         addr1 -> (0 ,3)
//     ];
//     mem_d2.validate_and_execute(&binds, Some(&Value::bit_low()));
// }

// #[test]
// #[should_panic]
// fn test_mem_d2_panic_input() {
//     // Input width larger than the memory capacity
//     let mut mem_d2 = stfl::StdMemD2::from_constants(2, 2, 1, 2, 1);
//     port_bindings![binds;
//         write_data -> (10, 4),
//         write_en -> (1, 1),
//         addr0 -> (0, 1),
//         addr1 -> (1, 1)
//     ];
//     mem_d2.validate_and_execute(&binds, Some(&Value::bit_low()));
// }
// #[test]
// fn test_mem_d3_tlv() {
//     let mut mem_d3 = stfl::StdMemD3::from_constants(1, 2, 2, 2, 1, 1, 1);
//     port_bindings![binds;
//         write_data -> (1, 1),
//         write_en -> (1, 1),
//         addr0 -> (1, 1),
//         addr1 -> (1, 1),
//         addr2 -> (1, 1)
//     ];
//     let mut mem_out = mem_d3
//         .validate_and_execute(&binds, Some(&Value::bit_low()))
//         .into_iter();
//     let (read_data, done) = (mem_out.next().unwrap(), mem_out.next().unwrap());
//     assert!(mem_out.next().is_none()); //make sure it's only of length 2
//     let mut rd = read_data.1.unwrap_tlv();
//     if let OutputValue::PulseValue(mut d) = done.1 {
//         assert_eq!(rd.get_count(), 1);
//         assert_eq!(d.get_val().as_u64(), 0);
//         rd.dec_count();
//         d.tick();
//         assert!(rd.unlockable());
//         assert_eq!(d.get_val().as_u64(), 1);

//         assert_eq!(rd.unlock().as_u64(), write_data.as_u64());
//         let d = d.do_tick();
//         assert!(matches!(d, OutputValue::ImmediateValue(_)));
//         if let OutputValue::ImmediateValue(iv) = d {
//             assert_eq!(iv.as_u64(), 0);
//         }
//     } else {
//         panic!()
//     }
// }
// #[test]
// fn test_mem_d3_imval() {
//     let mut mem_d3 = stfl::StdMemD3::from_constants(1, 2, 2, 2, 1, 1, 1);
//     port_bindings![binds;
//         write_data -> (1, 1),
//         write_en -> (0, 1),
//         addr0 -> (1, 1),
//         addr1 -> (1, 1),
//         addr2 -> (1, 1)
//     ];
//     let mut mem_out = mem_d3
//         .validate_and_execute(&binds, Some(&Value::bit_low()))
//         .into_iter();
//     if let (read_data, None) = (mem_out.next().unwrap(), mem_out.next()) {
//         let rd = read_data.1.unwrap_imm();
//         assert_eq!(rd.as_u64(), 0); // assuming this b/c mem hasn't been initialized
//     } else {
//         panic!()
//     }
// }
// #[test]
// #[should_panic]
// fn test_mem_d3_panic_addr0() {
//     // Access address larger than the size of memory
//     let mut mem_d3 = stfl::StdMemD3::from_constants(1, 2, 2, 2, 1, 1, 1); //2 x 2 x 2, storing 1 bit in each slot
//     port_bindings![binds;
//         write_data -> (1, 1),
//         write_en -> (1, 1),
//         addr0 -> (0, 4),
//         addr1 -> (1, 1),
//         addr2 -> (1, 1)
//     ];
//     mem_d3.validate_and_execute(&binds, Some(&Value::bit_low()));
// }
// #[test]
// #[should_panic]
// fn test_mem_d3_panic_addr1() {
//     // Access address larger than the size of memory
//     let mut mem_d3 = stfl::StdMemD3::from_constants(1, 2, 2, 2, 1, 1, 1); //2 x 2 x 2, storing 1 bit in each slot
//     port_bindings![binds;
//         write_data -> (1, 1),
//         write_en -> (1, 1),
//         addr0 -> (0, 1),
//         addr1 -> (1, 4),
//         addr2 -> (1, 1)
//     ];
//     mem_d3.validate_and_execute(&binds, Some(&Value::bit_low()));
// }
// #[test]
// #[should_panic]
// fn test_mem_d3_panic_addr2() {
//     // Access address larger than the size of memory
//     let mut mem_d3 = stfl::StdMemD3::from_constants(1, 2, 2, 2, 1, 1, 1); //2 x 2 x 2, storing 1 bit in each slot
//     port_bindings![binds;
//         write_data -> (1, 1),
//         write_en -> (1, 1),
//         addr0 -> (0, 1),
//         addr1 -> (1, 1),
//         addr2 -> (1, 4)
//     ];
//     mem_d3.validate_and_execute(&binds, Some(&Value::bit_low()));
// }
// #[test]
// #[should_panic]
// fn test_mem_d3_panic_input() {
//     // Input width larger than the memory capacity
//     let mut mem_d3 = stfl::StdMemD3::from_constants(1, 2, 2, 2, 1, 1, 1);
//     port_bindings![binds;
//         write_data -> (10, 4),
//         write_en -> (1, 1),
//         addr0 -> (0, 1),
//         addr1 -> (1, 1),
//         addr2 -> (1, 1)
//     ];
//     mem_d3.validate_and_execute(&binds, Some(&Value::bit_low()));
// }
// #[test]
// fn test_mem_d4_tlv() {
//     let mut mem_d4 = stfl::StdMemD4::from_constants(1, 2, 2, 2, 2, 1, 1, 1, 1);
//     port_bindings![binds;
//         write_data -> (1, 1),
//         write_en -> (1, 1),
//         addr0 -> (1, 1),
//         addr1 -> (1, 1),
//         addr2 -> (1, 1),
//         addr3 -> (1, 1)
//     ];
//     let mut mem_out = mem_d4
//         .validate_and_execute(&binds, Some(&Value::bit_low()))
//         .into_iter();
//     let (read_data, done) = (mem_out.next().unwrap(), mem_out.next().unwrap());
//     assert!(mem_out.next().is_none()); //make sure it's only of length 2
//     let mut rd = read_data.1.unwrap_tlv();
//     if let OutputValue::PulseValue(mut d) = done.1 {
//         assert_eq!(rd.get_count(), 1);
//         assert_eq!(d.get_val().as_u64(), 0);
//         rd.dec_count();
//         d.tick();
//         assert!(rd.unlockable());
//         assert_eq!(d.get_val().as_u64(), 1);

//         assert_eq!(rd.unlock().as_u64(), write_data.as_u64());
//         let d = d.do_tick();
//         assert!(matches!(d, OutputValue::ImmediateValue(_)));
//         if let OutputValue::ImmediateValue(iv) = d {
//             assert_eq!(iv.as_u64(), 0);
//         }
//     } else {
//         panic!()
//     }
// }
// #[test]
// fn test_mem_d4_imval() {
//     let mut mem_d4 = stfl::StdMemD4::from_constants(32, 8, 8, 8, 8, 3, 3, 3, 3);
//     port_bindings![binds;
//         write_data -> (5, 32),
//         write_en -> (0, 1),
//         addr0 -> (2, 3),
//         addr1 -> (1, 3),
//         addr2 -> (5, 3),
//         addr3 -> (2, 3)
//     ];
//     let mut mem_out = mem_d4
//         .validate_and_execute(&binds, Some(&Value::bit_low()))
//         .into_iter();
//     if let (read_data, None) = (mem_out.next().unwrap(), mem_out.next()) {
//         let rd = read_data.1.unwrap_imm();
//         assert_eq!(rd.as_u64(), 0); // assuming this b/c mem hasn't been initialized
//     }
// }
// #[test]
// #[should_panic]
// fn test_mem_d4_panic_addr0() {
//     // Access address larger than the size of memory
//     let mut mem_d4 = stfl::StdMemD4::from_constants(32, 3, 2, 3, 2, 3, 2, 3, 2);
//     port_bindings![binds;
//         write_data -> (5, 32),
//         write_en -> (1, 1),
//         addr0 -> (4, 3),
//         addr1 -> (0, 2),
//         addr2 -> (1, 2),
//         addr3 -> (2, 2)
//     ];
//     mem_d4.validate_and_execute(&binds, Some(&Value::bit_low()));
// }
// #[test]
// #[should_panic]
// fn test_mem_d4_panic_addr1() {
//     // Access address larger than the size of memory
//     let mut mem_d4 = stfl::StdMemD4::from_constants(32, 3, 2, 3, 2, 3, 2, 3, 2);
//     port_bindings![binds;
//         write_data -> (5, 32),
//         write_en -> (1, 1),
//         addr0 -> (0, 2),
//         addr1 -> (4, 3),
//         addr2 -> (1, 2),
//         addr3 -> (2, 2)
//     ];
//     mem_d4.validate_and_execute(&binds, Some(&Value::bit_low()));
// }
// #[test]
// #[should_panic]
// fn test_mem_d4_panic_addr2() {
//     // Access address larger than the size of memory
//     let mut mem_d4 = stfl::StdMemD4::from_constants(32, 3, 2, 3, 2, 3, 2, 3, 2);
//     port_bindings![binds;
//         write_data -> (5, 32),
//         write_en -> (1, 1),
//         addr0 -> (0, 2),
//         addr1 -> (1, 2),
//         addr2 -> (4, 3),
//         addr3 -> (2, 2)
//     ];
//     mem_d4.validate_and_execute(&binds, Some(&Value::bit_low()));
// }
// #[test]
// #[should_panic]
// fn test_mem_d4_panic_addr3() {
//     // Access address larger than the size of memory
//     let mut mem_d4 = stfl::StdMemD4::from_constants(32, 3, 2, 3, 2, 3, 2, 3, 2);
//     port_bindings![binds;
//         write_data -> (5, 32),
//         write_en -> (1, 1),
//         addr0 -> (0, 2),
//         addr1 -> (1, 2),
//         addr2 -> (2, 2),
//         addr3 -> (4, 3)
//     ];
//     mem_d4.validate_and_execute(&binds, Some(&Value::bit_low()));
// }
// #[test]
// #[should_panic]
// fn test_mem_d4_panic_input() {
//     // Input width larger than the memory capacity
//     let mut mem_d4 = stfl::StdMemD4::from_constants(32, 3, 2, 3, 2, 3, 2, 3, 2);
//     port_bindings![binds;
//         write_enable -> (1, 1),
//         write_data -> (10, 4),
//         addr0 -> (0, 2),
//         addr1 -> (1, 2),
//         addr2 -> (2, 2),
//         addr3 -> (3, 2)
//     ];
//     mem_d4.validate_and_execute(&binds, Some(&Value::bit_low()));
// }
// #[test]
// fn test_std_reg_tlv() {
//     let mut reg1 = stfl::StdReg::from_constants(6);
//     port_bindings![binds;
//         r#in -> (16, 6),
//         write_en -> (1, 1)
//     ];
//     let output_vals =
//         reg1.validate_and_execute(&binds, Some(&Value::bit_low()));
//     println!("output_vals: {:?}", output_vals);
//     let mut output_vals = output_vals.into_iter();
//     let (read_data, done) =
//         (output_vals.next().unwrap(), output_vals.next().unwrap());
//     assert!(output_vals.next().is_none()); //make sure it's only of length 2

//     if let OutputValue::PulseValue(mut d) = done.1 {
//         let mut rd = read_data.1.unwrap_tlv();
//         assert_eq!(rd.get_count(), 1);
//         assert_eq!(d.get_val().as_u64(), 0);
//         rd.dec_count();
//         d.tick();
//         assert!(rd.unlockable());
//         assert_eq!(d.get_val().as_u64(), 1);
//         assert_eq!(rd.unlock().as_u64(), r#in.as_u64());
//         let d = d.do_tick();
//         assert!(matches!(d, OutputValue::ImmediateValue(_)));
//         if let OutputValue::ImmediateValue(iv) = d {
//             assert_eq!(iv.as_u64(), 0);
//         }
//     } else {
//         panic!()
//     }
// }

#[test]
fn test_std_reg_imval() {
    let mut reg1 = stfl::StdReg::from_constants(6);
    //see that unitialized register, executed w/ write_en low,
    //returns 0 and 0
    port_bindings![binds;
        r#in -> (16, 6),
        write_en -> (0, 1)
    ];
    let output_vals = reg1.validate_and_execute(&binds);
    assert_eq!(0, output_vals.len()); //output_vals should be empty from execute
    let output_vals = reg1.do_tick();
    println!("output_vals: {:?}", output_vals);
    let mut output_vals = output_vals.into_iter();
    //should be a 0 and a 0 ([out] and [done])
    let (out, done_val) =
        (output_vals.next().unwrap(), output_vals.next().unwrap());
    let rd = out.1.unwrap_imm();
    assert_eq!(rd.as_u64(), 0);
    let d = done_val.1.unwrap_imm();
    assert_eq!(d.as_u64(), 0);
    //now have write_en high and see output from do_tick() is 16, 1
    port_bindings![binds;
        r#in -> (16, 6),
        write_en -> (1, 1)
    ];
    let output_vals = reg1.validate_and_execute(&binds);
    assert_eq!(0, output_vals.len()); //output_vals should be empty from execute
    let output_vals = reg1.do_tick();
    println!("output_vals: {:?}", output_vals);
    let mut output_vals = output_vals.into_iter();
    //should be a 16 and a 1 ([out] and [done])
    let (out, done_val) =
        (output_vals.next().unwrap(), output_vals.next().unwrap());
    let rd = out.1.unwrap_imm();
    assert_eq!(rd.as_u64(), 16);
    let d = done_val.1.unwrap_imm();
    assert_eq!(d.as_u64(), 1);
    //now try to overwrite but w/ write_en low, and see 16 and 0 is returned
    port_bindings![binds;
        r#in -> (16, 6),
        write_en -> (0, 1)
    ];
    let output_vals = reg1.validate_and_execute(&binds);
    assert_eq!(0, output_vals.len()); //output_vals should be empty from execute
    let output_vals = reg1.do_tick();
    println!("output_vals: {:?}", output_vals);
    let mut output_vals = output_vals.into_iter();
    //should be a 16 and a 1 ([out] and [done])
    let (out, done_val) =
        (output_vals.next().unwrap(), output_vals.next().unwrap());
    let rd = out.1.unwrap_imm();
    assert_eq!(rd.as_u64(), 16);
    let d = done_val.1.unwrap_imm();
    assert_eq!(d.as_u64(), 0);
}
// #[test]
// #[should_panic]
// fn reg_too_big() {
//     let mut reg1 = stfl::StdReg::from_constants(5);
//     // now try loading in a value that is too big(??)
//     port_bindings![binds;
//         r#in -> (32, 6),
//         write_en -> (1, 1)
//     ];
//     reg1.validate_and_execute(&binds, Some(&Value::bit_low()));
// }

// /* #[test]
// fn test_std_const() {
// let val_31 = Value::try_from_init(31, 5).unwrap();
// let const_31 = comb::StdConst::from_constants(5, val_31);
// assert_eq!(const_31.read_val().as_u64(), 31); //can rust check this equality?
// assert_eq!(const_31.read_u64(), 31);
// }
// #[test]
// #[should_panic]
// fn test_std_const_panic() {
// let val = Value::try_from_init(75, 7).unwrap();
// comb::StdConst::from_constants(5, val);
// } */
// #[test]
// fn test_std_lsh() {
//     // lsh with overflow
//     // [11111] (31) -> [11100] (28)
//     let mut lsh = comb::StdLsh::from_constants(5);
//     port_bindings![binds;
//         left -> (31, 5),
//         right -> (2, 5)
//     ];
//     let out = lsh
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     println!("lsh of 31 by 2: {}", out);
//     assert_eq!(out.as_u64(), 28);

//     // lsh without overflow
//     // lsh [010000] (16) by 1 -> [100000] (32)
//     let mut lsh = comb::StdLsh::from_constants(6);
//     port_bindings![binds;
//         left -> (16, 6),
//         right -> (1, 6)
//     ];
//     let out = lsh
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(out.as_u64(), 32);
// }

// #[test]
// fn test_std_lsh_above64() {
//     // lsh with overflow
//     let mut lsh = comb::StdLsh::from_constants(275);
//     port_bindings![binds;
//         left -> (31, 275),
//         right -> (275, 275)
//     ];
//     let out = lsh
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(out.as_u64(), 0);

//     // lsh without overflow
//     // lsh [010000] (16) by 1 -> [100000] (32)
//     let mut lsh = comb::StdLsh::from_constants(381);
//     port_bindings![binds;
//         left -> (16, 381),
//         right -> (1, 381)
//     ];
//     let out = lsh
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(out.as_u64(), 32);
// }

// #[test]
// fn test_std_rsh() {
//     // Not sure how to catagorize this
//     // [1111] (15) -> [0011] (3)
//     let mut rsh = comb::StdRsh::from_constants(4);
//     port_bindings![binds;
//         left -> (15, 4),
//         right -> (2, 4)
//     ];
//     let out = rsh
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(out.as_u64(), 3);
//     // Division by 2
//     // [1000] (8) -> [0100] ( 4)
//     port_bindings![binds;
//         left -> (8, 4),
//         right -> (1, 4)
//     ];
//     let out = rsh
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(out.as_u64(), 4);
// }

// #[test]
// fn test_std_rsh_above64() {
//     let mut rsh = comb::StdRsh::from_constants(275);
//     port_bindings![binds;
//         left -> (8, 275),
//         right -> (4, 275)
//     ];
//     let out = rsh
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(out.as_u64(), 0);
//     let mut rsh = comb::StdRsh::from_constants(381);
//     port_bindings![binds;
//         left -> (40, 381),
//         right -> (3, 381)
//     ];
//     let out = rsh
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(out.as_u64(), 5);
// }

// #[test]
// fn test_std_add() {
//     // without overflow
//     // add [0011] (3) and [1010] (10) -> [1101] (13)
//     let mut add = comb::StdAdd::from_constants(4);
//     port_bindings![binds;
//         left -> (3, 4),
//         right -> (10, 4)
//     ];
//     let res_add = add
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_add.as_u64(), 13);
//     // with overflow
//     // add [1010] (10) and [0110] (6) -> [0000] (0)
//     port_bindings![binds;
//         left -> (10, 4),
//         right -> (6, 4)
//     ];
//     let res_add = add
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_add.as_u64(), 0);
// }

// #[test]
// fn test_std_add_above64() {
//     // without overflow
//     let mut add = comb::StdAdd::from_constants(165);
//     port_bindings![binds;
//         left -> (17, 165),
//         right -> (35, 165)
//     ];
//     let res_add = add
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_add.as_u64(), 52);
// }

// #[test]
// #[should_panic]
// fn test_std_add_panic() {
//     let mut add = comb::StdAdd::from_constants(7);
//     port_bindings![binds;
//         left -> (81, 7),
//         right -> (10, 4)
//     ];
//     add.validate_and_execute(&binds, None);
// }
// #[test]
// fn test_std_sub() {
//     // without overflow
//     // sub [0110] (6) from [1010] (10) -> [0100] (4)
//     let mut sub = comb::StdSub::from_constants(4);
//     port_bindings![binds;
//         left -> (10, 4),
//         right -> (6, 4)
//     ];
//     let res_sub = sub
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_sub.as_u64(), 4);
//     // with overflow (would produce a negative #, depending on how program thinks abt this...)
//     // sub [1011] (11) from [1010] (10) ->  [1010] + [0101] = [1111] which is -1 in 2bc and 15 unsigned
//     // for some reason producing [0101] ? that's just 'right + 1
//     port_bindings![binds;
//         left -> (10, 4),
//         right -> (11, 4)
//     ];
//     let res_sub = sub
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_sub.as_u64(), 15);
//     // sub [1111] (15) from [1000] (8) -> [1000] + [0001] which is [1001] -7 in 2c but 9 in unsigned

//     port_bindings![binds;
//         left -> (8, 4),
//         right -> (15, 4)
//     ];
//     let res_sub = sub
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_sub.as_u64(), 9);
// }

// #[test]
// fn test_std_sub_above64() {
//     // without overflow
//     let mut sub = comb::StdSub::from_constants(1605);
//     port_bindings![binds;
//         left -> (57, 1605),
//         right -> (35, 1605)
//     ];
//     let res_sub = sub
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_sub.as_u64(), 22);
// }

// #[test]
// #[should_panic]
// fn test_std_sub_panic() {
//     let mut sub = comb::StdAdd::from_constants(5);
//     port_bindings![binds;
//         left -> (52, 6),
//         right -> (16, 5)
//     ];
//     sub.validate_and_execute(&binds, None);
// }
// #[test]
// fn test_std_slice() {
//     // 101 in binary is [1100101], take first 4 bits -> [0101] = 5
//     let to_slice = Value::from(101, 7).unwrap();
//     let mut std_slice = comb::StdSlice::from_constants(7, 4);
//     let res_slice = std_slice
//         .validate_and_execute(&[("in".into(), &to_slice)], None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm(); //note that once we implement execute_unary, have to change this
//     assert_eq!(res_slice.as_u64(), 5);
//     // Slice the entire bit
//     let to_slice = Value::from(548, 10).unwrap();
//     let mut std_slice = comb::StdSlice::from_constants(10, 10);
//     let res_slice = std_slice
//         .validate_and_execute(&[("in".into(), &to_slice)], None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_slice.as_u64(), 548);
// }
// #[test]
// #[should_panic]
// fn test_std_slice_panic() {
//     let to_slice = Value::from(3, 2).unwrap();
//     let mut std_slice = comb::StdSlice::from_constants(7, 4);
//     std_slice.validate_and_execute(&[("in".into(), &to_slice)], None);
// }
// #[test]
// fn test_std_pad() {
//     // Add 2 zeroes, should keep the same value
//     let to_pad = Value::from(101, 7).unwrap();
//     let mut std_pad = comb::StdPad::from_constants(7, 9);
//     let res_pad = std_pad
//         .validate_and_execute(&[("in".into(), &to_pad)], None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_pad.as_u64(), 101);
//     // hard to think of another test case but just to have 2:
//     let to_pad = Value::from(1, 7).unwrap();
//     let res_pad = std_pad
//         .validate_and_execute(&[("in".into(), &to_pad)], None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_pad.as_u64(), 1);
// }
// #[test]
// #[should_panic]
// fn test_std_pad_panic() {
//     let to_pad = Value::from(21, 5).unwrap();
//     let mut std_pad = comb::StdPad::from_constants(3, 9);
//     std_pad.validate_and_execute(&[("in".into(), &to_pad)], None);
// }
// /// Logical Operators
// #[test]
// fn test_std_not() {
//     // ![1010] (!10) -> [0101] (5)
//     let not0 = Value::from(10, 4).unwrap();
//     let mut std_not = comb::StdNot::from_constants(4);
//     let res_not = std_not
//         .validate_and_execute(&[("in".into(), &not0)], None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_not.as_u64(), 5);
//     // ![0000] (!0) -> [1111] (15)
//     let not0 = Value::from(0, 4).unwrap();
//     let res_not = std_not
//         .validate_and_execute(&[("in".into(), &not0)], None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_not.as_u64(), 15);
// }

// #[test]
// #[should_panic]
// fn test_std_not_panic() {
//     //input too short
//     let not0 = Value::from(0, 4).unwrap();
//     let mut std_not = comb::StdNot::from_constants(5);
//     std_not
//         .validate_and_execute(&[("in".into(), &not0)], None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
// }

// #[test]
// fn test_std_and() {
//     //101: [1100101], 78: [1001110] & -> [1000100] which is 68
//     let mut std_and = comb::StdAnd::from_constants(7);
//     port_bindings![binds;
//         left -> (101, 7),
//         right -> (78, 7)
//     ];
//     let res_and = std_and
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_and.as_u64(), 68);
//     //[1010] (10) & [0101] (5) is [0000]

//     let mut std_and = comb::StdAnd::from_constants(4);
//     port_bindings![binds;
//         left -> (10, 4),
//         right -> (5, 4)
//     ];
//     let res_and = std_and
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_and.as_u64(), 0);
// }

// #[test]
// #[should_panic]
// fn test_std_and_panic() {
//     let mut std_and = comb::StdAnd::from_constants(7);
//     port_bindings![binds;
//         left -> (91, 7),
//         right -> (43, 6)
//     ];
//     std_and
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
// }

// #[test]
// fn test_std_or() {
//     //[101] (5) or [011] (3) is [111] (7)
//     let mut std_or = comb::StdOr::from_constants(3);
//     port_bindings![binds;
//         left -> (5, 3),
//         right -> (3, 3)
//     ];
//     let res_or = std_or
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_or.as_u64(), 7);
//     //anything or zero is itself
//     //[001] (1) or [000] (0) is [001] (1)
//     port_bindings![binds;
//         left -> (1, 3),
//         right -> (0, 3)
//     ];
//     let res_or = std_or
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_or.as_u64(), left.as_u64());
// }

// #[test]
// #[should_panic]
// fn test_std_or_panic() {
//     let mut std_or = comb::StdOr::from_constants(5);
//     port_bindings![binds;
//         left -> (16, 5),
//         right -> (78, 7)
//     ];
//     std_or
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
// }
// #[test]
// fn test_std_xor() {
//     //[101] (5) XOR [011] (3) is [110] (6)
//     let mut std_xor = comb::StdXor::from_constants(3);
//     port_bindings![binds;
//         left -> (5, 3),
//         right -> (3, 3)
//     ];
//     let res_xor = std_xor
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_xor.as_u64(), 6);
//     //anything xor itself is 0
//     port_bindings![binds;
//         left -> (5, 3),
//         right -> (5, 3)
//     ];
//     assert_eq!(
//         std_xor
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         0
//     );
// }
// #[test]
// #[should_panic]
// fn test_std_xor_panic() {
//     let mut std_xor = comb::StdXor::from_constants(6);
//     port_bindings![binds;
//         left -> (56, 6),
//         right -> (92, 7)
//     ];
//     std_xor
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
// }
// /// Comparison Operators
// // is there any point in testing this more than once?
// // no weird overflow or anything. maybe test along with
// // equals
// #[test]
// fn test_std_gt() {
//     let mut std_gt = comb::StdGt::from_constants(16);
//     port_bindings![binds;
//         left -> (7 ,16),
//         right -> (3, 16)
//     ];
//     let res_gt = std_gt
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_gt.as_u64(), 1);
//     //7 > 7 ? no!
//     port_bindings![binds;
//         left -> (7, 16),
//         right -> (7, 16)
//     ];
//     assert_eq!(
//         std_gt
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         0
//     );
// }

// #[test]
// fn test_std_gt_above64() {
//     let mut std_gt = comb::StdGt::from_constants(716);
//     port_bindings![binds;
//         left -> (18446744073709551615_u64, 716),
//         right -> (14333, 716)
//     ];
//     let res_gt = std_gt
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_gt.as_u64(), 1);
//     //7 > 7 ? no!
//     let mut std_gt = comb::StdGt::from_constants(423);
//     port_bindings![binds;
//         left -> (7, 423),
//         right -> (7, 423)
//     ];
//     assert_eq!(
//         std_gt
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         0
//     );
// }
// #[test]
// #[should_panic]
// fn test_std_gt_panic() {
//     let mut std_gt = comb::StdGt::from_constants(3);
//     port_bindings![binds;
//         left -> (9, 4),
//         right -> (3, 2)
//     ];
//     std_gt.validate_and_execute(&binds, None);
// }
// #[test]
// fn test_std_lt() {
//     let mut std_lt = comb::StdLt::from_constants(16);
//     port_bindings![binds;
//         left -> (7, 16),
//         right -> (3, 16)
//     ];
//     let res_lt = std_lt
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_lt.as_u64(), 0);
//     // 7 < 7 ? no!
//     port_bindings![binds;
//         left -> (7, 16),
//         right -> (7, 16)
//     ];
//     assert_eq!(
//         std_lt
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         0
//     );
// }

// #[test]
// fn test_std_lt_above64() {
//     //7298791842 < 17298791842
//     let mut std_lt = comb::StdLt::from_constants(2706);
//     port_bindings![binds;
//         left -> (72987918, 2706),
//         right -> (18446744073709551615_u64, 2706)
//     ];
//     let res_lt = std_lt
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_lt.as_u64(), 1);
//     //3_000_000 < 3_000_000 ? no!
//     let mut std_lt = comb::StdLt::from_constants(2423);
//     port_bindings![binds;
//         left -> (3_000_000, 2423),
//         right -> (3_000_000, 2423)
//     ];
//     assert_eq!(
//         std_lt
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         0
//     );
// }

// #[test]
// #[should_panic]
// fn test_std_lt_panic() {
//     let mut std_lt = comb::StdLt::from_constants(5);
//     port_bindings![binds;
//         left -> (58, 6),
//         right -> (12, 4)
//     ];
//     std_lt.validate_and_execute(&binds, None);
// }
// #[test]
// fn test_std_eq() {
//     let mut std_eq = comb::StdEq::from_constants(16);
//     port_bindings![binds;
//         left -> (4, 16),
//         right -> (4, 16)
//     ];
//     let res_eq = std_eq
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_eq.as_u64(), 1);
//     // 4 = 5 ? no!
//     port_bindings![binds;
//         left -> (4, 16),
//         right -> (5, 16)
//     ];
//     assert_eq!(
//         std_eq
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         0
//     );
// }

// #[test]
// fn test_std_eq_above64() {
//     let mut std_eq = comb::StdEq::from_constants(716);
//     port_bindings![binds;
//         left -> (18446744073709551615_u64, 716),
//         right -> (18446744073709551615_u64, 716)
//     ];
//     let res_eq = std_eq
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_eq.as_u64(), 1);
//     // 123456 =12377456 ? no!
//     let mut std_eq = comb::StdEq::from_constants(421113);
//     port_bindings![binds;
//         left -> (123456, 421113),
//         right -> (12377456, 421113)
//     ];
//     assert_eq!(
//         std_eq
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         0
//     );
// }

// #[test]
// #[should_panic]
// fn test_std_eq_panic() {
//     let mut std_eq = comb::StdEq::from_constants(5);
//     port_bindings![binds;
//         left -> (42, 6),
//         right -> (42, 6)
//     ];
//     std_eq.validate_and_execute(&binds, None);
// }
// #[test]
// fn test_std_neq() {
//     let mut std_neq = comb::StdNeq::from_constants(16);
//     port_bindings![binds;
//         left -> (4, 16),
//         right -> (4, 16)
//     ];
//     let res_neq = std_neq
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     //4 != 4 ? no!
//     assert!(res_neq.as_u64() == 0);
//     // 4 != 5? yes!
//     port_bindings![binds;
//         left -> (4, 16),
//         right -> (5, 16)
//     ];
//     assert_eq!(
//         std_neq
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         1
//     );
// }

// #[test]
// fn test_std_neq_above64() {
//     let mut std_neq = comb::StdNeq::from_constants(4321);
//     port_bindings![binds;
//         left -> (18446744073709551615_u64, 4321),
//         right -> (18446744073709551615_u64, 4321)
//     ];
//     let res_neq = std_neq
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     //max != max ? no!
//     assert!(res_neq.as_u64() == 0);
//     port_bindings![binds;
//     left -> (18446744073709551615_u64, 4321),
//     right -> (18446744073709500000_u64, 4321)
//     ];
//     assert_eq!(
//         std_neq
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         1
//     );
// }

// #[test]
// #[should_panic]
// fn test_std_neq_panic() {
//     let mut std_neq = comb::StdNeq::from_constants(5);
//     port_bindings![binds;
//         left -> (45, 6),
//         right -> (4, 3)
//     ];
//     std_neq.validate_and_execute(&binds, None);
// }

// #[test]
// fn test_std_ge() {
//     let mut std_ge = comb::StdGe::from_constants(8);
//     port_bindings![binds;
//         left -> (35, 8),
//         right -> (165, 8)
//     ];
//     let res_ge = std_ge
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     //35 >= 165 ? no!
//     assert_eq!(res_ge.as_u64(), 0);
//     // 35 >= 35 ? yes
//     port_bindings![binds;
//         left -> (35, 8),
//         right -> (35, 8)
//     ];
//     assert_eq!(
//         std_ge
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         1
//     );
// }

// #[test]
// fn test_std_ge_above64() {
//     let mut std_ge = comb::StdGe::from_constants(716);
//     port_bindings![binds;
//         left -> (18446744073709551615_u64, 716),
//         right -> (14333, 716)
//     ];
//     let res_ge = std_ge
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_ge.as_u64(), 1);
//     // 35 >= 35 ? yes
//     let mut std_ge = comb::StdGe::from_constants(423);
//     port_bindings![binds;
//         left -> (35, 423),
//         right -> (35, 423)
//     ];
//     assert_eq!(
//         std_ge
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         1
//     );
// }

// #[test]
// #[should_panic]
// fn test_std_ge_panic() {
//     let mut std_ge = comb::StdGe::from_constants(6);
//     port_bindings![binds;
//         left -> (40, 6),
//         right -> (75, 7)
//     ];
//     std_ge.validate_and_execute(&binds, None);
// }
// #[test]
// fn test_std_le() {
//     let mut std_le = comb::StdLe::from_constants(4);
//     port_bindings![binds;
//         left -> (12, 4),
//         right -> (8, 4)
//     ];
//     let res_le = std_le
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     //12 <= 4 ? no!
//     assert_eq!(res_le.as_u64(), 0);
//     //12 <= 12? yes!
//     port_bindings![binds;
//         left -> (12, 4),
//         right -> (12, 4)
//     ];
//     assert_eq!(
//         std_le
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         1
//     );
// }

// #[test]
// fn test_std_le_above64() {
//     //72987918 <= 9729879
//     let mut std_le = comb::StdLe::from_constants(2706);
//     port_bindings![binds;
//         left -> (72_987_918, 2706),
//         right -> (93_729_879, 2706)
//     ];
//     let res_le = std_le
//         .validate_and_execute(&binds, None)
//         .into_iter()
//         .next()
//         .map(|(_, v)| v)
//         .unwrap()
//         .unwrap_imm();
//     assert_eq!(res_le.as_u64(), 1);
//     //3_000_000 <= 3_000_000 ? yes!
//     let mut std_le = comb::StdLe::from_constants(2423);
//     port_bindings![binds;
//         left -> (3_000_000, 2423),
//         right -> (3_000_000, 2423)
//     ];
//     assert_eq!(
//         std_le
//             .validate_and_execute(&binds, None)
//             .into_iter()
//             .next()
//             .map(|(_, v)| v)
//             .unwrap()
//             .unwrap_imm()
//             .as_u64(),
//         1
//     );
// }

// #[test]
// #[should_panic]
// fn test_std_le_panic() {
//     let mut std_le = comb::StdLe::from_constants(6);
//     port_bindings![binds;
//         left -> (93, 7),
//         right -> (68, 7)
//     ];
//     std_le.validate_and_execute(&binds, None);
// }
