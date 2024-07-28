use std::collections::BTreeSet;

use fud_core::{
    exec::plan::{EggPlanner, EnumeratePlanner, FindPlan},
    DriverBuilder,
};

mod gen_graph;

const MULTI_PLANNERS: [&dyn FindPlan; 2] =
    [&EnumeratePlanner {}, &EggPlanner {}];

#[test]
fn find_plan_simple_graph_test() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {:?}", path_finder);
        let mut bld = DriverBuilder::new("fud2");
        let s1 = bld.state("s1", &[]);
        let s2 = bld.state("s2", &[]);
        let t1 = bld.op("t1", &[], s1, s2, |_, _, _| Ok(()));
        let driver = bld.build();
        assert_eq!(
            Some(vec![(t1, vec![s2])]),
            path_finder.find_plan(&[s1], &[s2], &[], &driver.ops)
        );
        assert_eq!(None, path_finder.find_plan(&[s1], &[s1], &[], &driver.ops));
    }
}

#[test]
fn find_plan_multi_op_graph() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {:?}", path_finder);
        let mut bld = DriverBuilder::new("fud2");
        let s1 = bld.state("s1", &[]);
        let s2 = bld.state("s2", &[]);
        let s3 = bld.state("s3", &[]);
        let t1 = bld.op("t1", &[], s1, s3, |_, _, _| Ok(()));
        let _ = bld.op("t2", &[], s2, s3, |_, _, _| Ok(()));
        let driver = bld.build();
        assert_eq!(
            Some(vec![(t1, vec![s3])]),
            path_finder.find_plan(&[s1], &[s3], &[], &driver.ops)
        );
    }
}

#[test]
fn find_plan_multi_path_graph() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {:?}", path_finder);
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
            path_finder.find_plan(&[s1], &[s5], &[t4], &driver.ops)
        );
        assert_eq!(
            Some(vec![(t1, vec![s3]), (t5, vec![s5])]),
            path_finder.find_plan(&[s1], &[s5], &[t5], &driver.ops)
        );
        assert_eq!(None, path_finder.find_plan(&[s6], &[s5], &[], &driver.ops));
        assert_eq!(
            None,
            path_finder.find_plan(&[s1], &[s5], &[t2], &driver.ops)
        );
    }
}

#[test]
fn find_plan_only_state_graph() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {:?}", path_finder);
        let mut bld = DriverBuilder::new("fud2");
        let s1 = bld.state("s1", &[]);
        let driver = bld.build();
        assert_eq!(None, path_finder.find_plan(&[s1], &[s1], &[], &driver.ops));
    }
}

#[test]
fn find_plan_self_loop() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {:?}", path_finder);
        let mut bld = DriverBuilder::new("fud2");
        let s1 = bld.state("s1", &[]);
        let t1 = bld.op("t1", &[], s1, s1, |_, _, _| Ok(()));
        let driver = bld.build();
        assert_eq!(
            Some(vec![(t1, vec![s1])]),
            path_finder.find_plan(&[s1], &[s1], &[t1], &driver.ops)
        );
    }
}

#[test]
fn find_plan_cycle_graph() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {:?}", path_finder);
        let mut bld = DriverBuilder::new("fud2");
        let s1 = bld.state("s1", &[]);
        let s2 = bld.state("s2", &[]);
        let t1 = bld.op("t1", &[], s1, s2, |_, _, _| Ok(()));
        let t2 = bld.op("t2", &[], s2, s1, |_, _, _| Ok(()));
        let driver = bld.build();
        assert_eq!(
            Some(vec![(t1, vec![s2])]),
            path_finder.find_plan(&[s1], &[s2], &[], &driver.ops)
        );
        assert_eq!(
            Some(vec![(t2, vec![s1])]),
            path_finder.find_plan(&[s2], &[s1], &[], &driver.ops)
        );
    }
}

#[test]
fn find_plan_nontrivial_cycle() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {:?}", path_finder);
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
            path_finder.find_plan(&[s1], &[s3], &[], &driver.ops)
        );
    }
}

#[test]
fn op_creating_two_states() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {:?}", path_finder);
        let mut bld = DriverBuilder::new("fud2");
        let s0 = bld.state("s0", &[]);
        let s1 = bld.state("s1", &[]);
        let s2 = bld.state("s2", &[]);
        let build_fn: fud_core::run::EmitBuildFn = |_, _, _| Ok(());
        let t0 = bld.add_op("t0", &[], &[s0], &[s1, s2], build_fn);
        let driver = bld.build();
        assert_eq!(
            Some(vec![(t0, vec![s1, s2])]),
            path_finder.find_plan(&[s0], &[s1, s2], &[], &driver.ops)
        );
    }
}

#[test]
fn op_compressing_two_states() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {:?}", path_finder);
        let mut bld = DriverBuilder::new("fud2");
        let s0 = bld.state("s0", &[]);
        let s1 = bld.state("s1", &[]);
        let s2 = bld.state("s2", &[]);
        let build_fn: fud_core::run::EmitBuildFn = |_, _, _| Ok(());
        let t0 = bld.add_op("t0", &[], &[s1, s2], &[s0], build_fn);
        let driver = bld.build();
        assert_eq!(
            Some(vec![(t0, vec![s0])]),
            path_finder.find_plan(&[s1, s2], &[s0], &[], &driver.ops)
        );
    }
}

#[test]
fn op_creating_two_states_not_initial_and_final() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {:?}", path_finder);
        let mut bld = DriverBuilder::new("fud2");
        let s0 = bld.state("s0", &[]);
        let s1 = bld.state("s1", &[]);
        let s2 = bld.state("s2", &[]);
        let s3 = bld.state("s3", &[]);
        let s4 = bld.state("s4", &[]);
        let s5 = bld.state("s5", &[]);
        let build_fn: fud_core::run::EmitBuildFn = |_, _, _| Ok(());
        let t0 = bld.add_op("t0", &[], &[s0], &[s1], build_fn);
        let t1 = bld.add_op("t1", &[], &[s1], &[s2, s3], build_fn);
        let t2 = bld.add_op("t2", &[], &[s2], &[s4], build_fn);
        let t3 = bld.add_op("t3", &[], &[s3], &[s5], build_fn);
        let driver = bld.build();
        assert_eq!(
            Some(BTreeSet::from_iter(vec![
                (t0, vec![s1]),
                (t1, vec![s2, s3]),
                (t2, vec![s4]),
                (t3, vec![s5])
            ])),
            path_finder
                .find_plan(&[s0], &[s4, s5], &[], &driver.ops)
                .map(|v| BTreeSet::from_iter(v))
        );
    }
}

#[test]
fn op_compressing_two_states_not_initial_and_final() {
    for path_finder in MULTI_PLANNERS {
        println!("testing planner: {:?}", path_finder);
        let mut bld = DriverBuilder::new("fud2");
        let s0 = bld.state("s0", &[]);
        let s1 = bld.state("s1", &[]);
        let s2 = bld.state("s2", &[]);
        let s3 = bld.state("s3", &[]);
        let s4 = bld.state("s4", &[]);
        let s5 = bld.state("s5", &[]);
        let build_fn: fud_core::run::EmitBuildFn = |_, _, _| Ok(());
        let t0 = bld.add_op("t0", &[], &[s0], &[s1], build_fn);
        let t1 = bld.add_op("t1", &[], &[s2], &[s3], build_fn);
        let t2 = bld.add_op("t2", &[], &[s1, s3], &[s4], build_fn);
        let t3 = bld.add_op("t3", &[], &[s4], &[s5], build_fn);
        let driver = bld.build();
        assert_eq!(
            Some(BTreeSet::from_iter(vec![
                (t0, vec![s1]),
                (t1, vec![s3]),
                (t2, vec![s4]),
                (t3, vec![s5]),
            ])),
            path_finder
                .find_plan(&[s0, s2], &[s5], &[], &driver.ops)
                .map(|v| BTreeSet::from_iter(v))
        );
    }
}
