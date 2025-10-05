//! Performance benchmarks for utility functions.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gnomics::utils::*;
use rand::SeedableRng;

fn bench_min(c: &mut Criterion) {
    c.bench_function("min", |b| {
        b.iter(|| {
            black_box(min(black_box(42), black_box(99)));
        });
    });
}

fn bench_max(c: &mut Criterion) {
    c.bench_function("max", |b| {
        b.iter(|| {
            black_box(max(black_box(42), black_box(99)));
        });
    });
}

fn bench_rand_uint(c: &mut Criterion) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);

    c.bench_function("rand_uint", |b| {
        b.iter(|| {
            black_box(rand_uint(black_box(0), black_box(1000), black_box(&mut rng)));
        });
    });
}

fn bench_shuffle(c: &mut Criterion) {
    let mut group = c.benchmark_group("shuffle");

    for size in [100, 1000, 10000].iter() {
        let mut arr: Vec<u32> = (0..*size).collect();
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| shuffle(black_box(&mut arr), black_box(size as usize), black_box(&mut rng)));
        });
    }
    group.finish();
}

fn bench_shuffle_indices(c: &mut Criterion) {
    let mut group = c.benchmark_group("shuffle_indices");

    for size in [100, 1000, 10000].iter() {
        let mut arr: Vec<usize> = (0..*size).collect();
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| shuffle_indices(black_box(&mut arr), black_box(&mut rng)));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_min,
    bench_max,
    bench_rand_uint,
    bench_shuffle,
    bench_shuffle_indices
);

criterion_main!(benches);
