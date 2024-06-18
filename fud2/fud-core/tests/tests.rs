use fud_core::DriverBuilder;

#[test]
fn find_path_simple_graph_test() {
    let mut bld = DriverBuilder::new("fud2");
    let s1 = bld.state("s1", &[]);
    let s2 = bld.state("s2", &[]);
    let t1 = bld.op("t1", &[], s1, s2, |_, _, _| Ok(()));
    let driver = bld.build();
    assert_eq!(
        Some(vec![(t1, vec![s2])]),
        driver.find_path(&[s1], &[s2], &[])
    );
    assert_eq!(None, driver.find_path(&[s1], &[s1], &[]));
}

#[test]
fn find_path_multi_op_graph() {
    let mut bld = DriverBuilder::new("fud2");
    let s1 = bld.state("s1", &[]);
    let s2 = bld.state("s2", &[]);
    let s3 = bld.state("s3", &[]);
    let t1 = bld.op("t1", &[], s1, s3, |_, _, _| Ok(()));
    let _ = bld.op("t2", &[], s2, s3, |_, _, _| Ok(()));
    let driver = bld.build();
    assert_eq!(
        Some(vec![(t1, vec![s3])]),
        driver.find_path(&[s1], &[s3], &[])
    );
}

#[test]
fn find_path_multi_path_graph() {
    let mut bld = DriverBuilder::new("fud2");
    let s1 = bld.state("s1", &[]);
    let s2 = bld.state("s2", &[]);
    let s3 = bld.state("s3", &[]);
    let s4 = bld.state("s4", &[]);
    let s5 = bld.state("s5", &[]);
    let s6 = bld.state("s6", &[]);
    let s7 = bld.state("s7", &[]);
    let t1 = bld.op("t1", &[], s1, s3, |_, _, _| Ok(()));
    let t2 = bld.op("t2", &[], s2, s3, |_, _, _| Ok(()));
    let _ = bld.op("t3", &[], s3, s4, |_, _, _| Ok(()));
    let t4 = bld.op("t4", &[], s3, s5, |_, _, _| Ok(()));
    let t5 = bld.op("t5", &[], s3, s5, |_, _, _| Ok(()));
    let _ = bld.op("t6", &[], s6, s7, |_, _, _| Ok(()));
    let driver = bld.build();
    assert_eq!(
        Some(vec![(t1, vec![s3]), (t4, vec![s5])]),
        driver.find_path(&[s1], &[s5], &[])
    );
    assert_eq!(
        Some(vec![(t1, vec![s3]), (t5, vec![s5])]),
        driver.find_path(&[s1], &[s5], &[t5])
    );
    assert_eq!(None, driver.find_path(&[s6], &[s5], &[]));
    assert_eq!(None, driver.find_path(&[s1], &[s5], &[t2]));
}

#[test]
fn find_path_only_state_graph() {
    let mut bld = DriverBuilder::new("fud2");
    let s1 = bld.state("s1", &[]);
    let driver = bld.build();
    assert_eq!(None, driver.find_path(&[s1], &[s1], &[]));
}

#[test]
fn find_path_self_loop() {
    let mut bld = DriverBuilder::new("fud2");
    let s1 = bld.state("s1", &[]);
    let t1 = bld.op("t1", &[], s1, s1, |_, _, _| Ok(()));
    let driver = bld.build();
    assert_eq!(
        Some(vec![(t1, vec![s1])]),
        driver.find_path(&[s1], &[s1], &[t1])
    );
}

#[test]
fn find_path_cycle_graph() {
    let mut bld = DriverBuilder::new("fud2");
    let s1 = bld.state("s1", &[]);
    let s2 = bld.state("s2", &[]);
    let t1 = bld.op("t1", &[], s1, s2, |_, _, _| Ok(()));
    let t2 = bld.op("t2", &[], s2, s1, |_, _, _| Ok(()));
    let driver = bld.build();
    assert_eq!(
        Some(vec![(t1, vec![s2]), (t2, vec![s1])]),
        driver.find_path(&[s1], &[s1], &[])
    );
    assert_eq!(
        Some(vec![(t1, vec![s2])]),
        driver.find_path(&[s1], &[s2], &[])
    );
    assert_eq!(
        Some(vec![(t2, vec![s1])]),
        driver.find_path(&[s2], &[s1], &[])
    );
}

#[test]
fn find_path_nontrivial_cycle() {
    let mut bld = DriverBuilder::new("fud2");
    let s1 = bld.state("s1", &[]);
    let s2 = bld.state("s2", &[]);
    let s3 = bld.state("s3", &[]);
    let _t1 = bld.op("t1", &[], s2, s2, |_, _, _| Ok(()));
    let t2 = bld.op("t2", &[], s1, s2, |_, _, _| Ok(()));
    let t3 = bld.op("t3", &[], s2, s3, |_, _, _| Ok(()));
    let driver = bld.build();
    assert_eq!(
        Some(vec![(t2, vec![s2]), (t3, vec![s3])]),
        driver.find_path(&[s1], &[s3], &[])
    );
}
