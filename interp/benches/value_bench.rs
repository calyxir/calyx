// use criterion::{black_box, criterion_group, criterion_main, Criterion};
// use ibig::ubig;
// use interp::values::Value;

// pub fn simple_constructor(c: &mut Criterion) {
//     c.bench_function("bool constructors", |b| b.iter(|| Value::bit_high()));
// }

// pub fn ubig_constructor(c: &mut Criterion) {
//     c.bench_function("ubig constructor", |b| {
//         b.iter(|| Value::from(ubig!(0784546895470695874360), 32))
//     });
// }

// pub fn u32_constructor(c: &mut Criterion) {
//     c.bench_function("u32 constructor", |b| {
//         b.iter(|| Value::from(65436543_u32, 32))
//     });
// }

// criterion_group!(
//     benches,
//     simple_constructor,
//     ubig_constructor,
//     u32_constructor
// );
// criterion_main!(benches);
