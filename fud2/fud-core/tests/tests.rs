use std::collections::BTreeSet;

use fud_core::{
    exec::plan::{EnumeratePlanner, FindPlan},
    DriverBuilder,
};
use rand::SeedableRng as _;

mod graph_gen;

#[cfg(feature = "egg_planner")]
use fud_core::exec::plan::EggPlanner;

#[cfg(feature = "egg_planner")]
const MULTI_PLANNERS: [&dyn FindPlan; 2] =
    [&EnumeratePlanner {}, &EggPlanner {}];

#[cfg(not(feature = "egg_planner"))]
const MULTI_PLANNERS: [&dyn FindPlan; 1] = [&EnumeratePlanner {}];

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
            path_finder.find_plan(
                &[s1],
                &[s2],
                &[],
                &driver.ops,
                &driver.states
            )
        );
        assert_eq!(
            None,
            path_finder.find_plan(
                &[s1],
                &[s1],
                &[],
                &driver.ops,
                &driver.states
            )
        );
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
            path_finder.find_plan(
                &[s1],
                &[s3],
                &[],
                &driver.ops,
                &driver.states
            )
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
            path_finder.find_plan(
                &[s1],
                &[s5],
                &[t4],
                &driver.ops,
                &driver.states
            )
        );
        assert_eq!(
            Some(vec![(t1, vec![s3]), (t5, vec![s5])]),
            path_finder.find_plan(
                &[s1],
                &[s5],
                &[t5],
                &driver.ops,
                &driver.states
            )
        );
        assert_eq!(
            None,
            path_finder.find_plan(
                &[s6],
                &[s5],
                &[],
                &driver.ops,
                &driver.states
            )
        );
        assert_eq!(
            None,
            path_finder.find_plan(
                &[s1],
                &[s5],
                &[t2],
                &driver.ops,
                &driver.states
            )
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
        assert_eq!(
            None,
            path_finder.find_plan(
                &[s1],
                &[s1],
                &[],
                &driver.ops,
                &driver.states
            )
        );
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
            path_finder.find_plan(
                &[s1],
                &[s1],
                &[t1],
                &driver.ops,
                &driver.states
            )
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
            path_finder.find_plan(
                &[s1],
                &[s2],
                &[],
                &driver.ops,
                &driver.states
            )
        );
        assert_eq!(
            Some(vec![(t2, vec![s1])]),
            path_finder.find_plan(
                &[s2],
                &[s1],
                &[],
                &driver.ops,
                &driver.states
            )
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
            path_finder.find_plan(
                &[s1],
                &[s3],
                &[],
                &driver.ops,
                &driver.states
            )
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
            path_finder.find_plan(
                &[s0],
                &[s1, s2],
                &[],
                &driver.ops,
                &driver.states
            )
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
            path_finder.find_plan(
                &[s1, s2],
                &[s0],
                &[],
                &driver.ops,
                &driver.states
            )
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
                .find_plan(&[s0], &[s4, s5], &[], &driver.ops, &driver.states)
                .map(BTreeSet::from_iter)
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
                .find_plan(&[s0, s2], &[s5], &[], &driver.ops, &driver.states)
                .map(BTreeSet::from_iter)
        );
    }
}

#[test]
fn correctness_fuzzing() {
    const LAYERS: u64 = 5;
    const STATES_PER_LAYER: u64 = 100;
    const OPS_PER_LAYER: u64 = 10;
    const MAX_IO_SIZE: u64 = 5;
    const MAX_REQUIRED_OPS: u64 = 3;
    const RANDOM_SEED: u64 = 0xDEADBEEF;
    const NUM_TESTS: u64 = 50;

    for planner in MULTI_PLANNERS {
        let rng = rand_chacha::ChaChaRng::seed_from_u64(RANDOM_SEED);
        let seeds = (0..NUM_TESTS).map(|_| rng.get_stream());
        for seed in seeds {
            let test = graph_gen::simple_random_graphs(
                LAYERS,
                STATES_PER_LAYER,
                OPS_PER_LAYER,
                MAX_IO_SIZE,
                MAX_REQUIRED_OPS,
                seed,
            );
            match test.eval(planner) {
                graph_gen::PlannerTestResult::FoundValidPlan
                | graph_gen::PlannerTestResult::NoPlanFound => (),
                graph_gen::PlannerTestResult::FoundInvalidPlan => panic!(
                    "Invalid plan generated with test parameters:
                        layers: {}
                        states_per_layer: {}
                        ops_per_layer: {}
                        max_io_size: {}
                        max_required_ops: {}
                        random_seed: {}",
                    LAYERS,
                    STATES_PER_LAYER,
                    OPS_PER_LAYER,
                    MAX_IO_SIZE,
                    MAX_REQUIRED_OPS,
                    seed
                ),
            }
        }
    }
}
