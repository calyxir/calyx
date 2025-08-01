use calyx_frontend as frontend;
use calyx_ir as ir;
use calyx_opt::passes;
use calyx_opt::traversal::Visitor;
use criterion::{
    BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main,
};
use std::path::{Path, PathBuf};

fn cell_share_bench(c: &mut Criterion) {
    let mut gemm_group = c.benchmark_group("gemm");
    for name in &["gemm2", "gemm3", "gemm4", "gemm6", "gemm8"] {
        gemm_group.bench_with_input(
            BenchmarkId::from_parameter(name),
            name,
            |b, &name| {
                b.iter_batched(
                    || {
                        let name =
                            format!("benches/component-sharing/{name}.futil");
                        let bench = Path::new(&name);
                        let lib = [PathBuf::from(".")];

                        let ws = frontend::Workspace::construct(
                            &Some(bench.into()),
                            &lib,
                        )
                        .unwrap();

                        let mut rep = ir::from_ast::ast_to_ir(
                            ws,
                            ir::from_ast::AstConversionConfig::default(),
                        )
                        .unwrap();

                        passes::SimplifyWithControl::do_pass_default(&mut rep)
                            .unwrap();
                        rep
                    },
                    |mut rep: ir::Context| {
                        passes::CellShare::do_pass_default(&mut rep).unwrap();
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }
    gemm_group.finish();
}

criterion_group! {
    name = cell_share;
    config = Criterion::default().sample_size(20);
    targets = cell_share_bench
}
criterion_main!(cell_share);
