//! Performance benchmarks for BitField operations.
//!
//! These benchmarks measure critical operations and compare against C++ targets:
//! - set_bit target: <3ns
//! - get_bit target: <2ns
//! - num_set (1024 bits) target: <60ns
//! - bitfield_copy_words (1024 bits) target: <60ns
//! - Logical operations (AND, OR, XOR)
//! - PartialEq comparison (critical for change tracking)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gnomics::{bitfield_copy_words, BitField};
use rand::SeedableRng;

// =============================================================================
// Single Bit Operations
// =============================================================================

fn bench_set_bit(c: &mut Criterion) {
    let mut ba = BitField::new(10000);

    c.bench_function("set_bit", |b| {
        let mut i = 0;
        b.iter(|| {
            ba.set_bit(black_box(i % 10000));
            i += 1;
        });
    });
}

fn bench_get_bit(c: &mut Criterion) {
    let mut ba = BitField::new(10000);
    ba.set_all();

    c.bench_function("get_bit", |b| {
        let mut i = 0;
        b.iter(|| {
            let _ = ba.get_bit(black_box(i % 10000));
            i += 1;
        });
    });
}

fn bench_clear_bit(c: &mut Criterion) {
    let mut ba = BitField::new(10000);
    ba.set_all();

    c.bench_function("clear_bit", |b| {
        let mut i = 0;
        b.iter(|| {
            ba.clear_bit(black_box(i % 10000));
            i += 1;
        });
    });
}

fn bench_toggle_bit(c: &mut Criterion) {
    let mut ba = BitField::new(10000);

    c.bench_function("toggle_bit", |b| {
        let mut i = 0;
        b.iter(|| {
            ba.toggle_bit(black_box(i % 10000));
            i += 1;
        });
    });
}

// =============================================================================
// Counting Operations
// =============================================================================

fn bench_num_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("num_set");

    for size in [32, 128, 1024, 4096, 16384].iter() {
        let mut ba = BitField::new(*size);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        ba.random_set_pct(&mut rng, 0.2);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(ba.num_set()));
        });
    }
    group.finish();
}

fn bench_num_similar(c: &mut Criterion) {
    let mut group = c.benchmark_group("num_similar");

    for size in [128, 1024, 4096].iter() {
        let mut ba1 = BitField::new(*size);
        let mut ba2 = BitField::new(*size);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        ba1.random_set_pct(&mut rng, 0.2);
        ba2.random_set_pct(&mut rng, 0.2);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(ba1.num_similar(&ba2)));
        });
    }
    group.finish();
}

// =============================================================================
// Bulk Operations
// =============================================================================

fn bench_set_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("set_all");

    for size in [128, 1024, 4096].iter() {
        let mut ba = BitField::new(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| ba.set_all());
        });
    }
    group.finish();
}

fn bench_clear_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("clear_all");

    for size in [128, 1024, 4096].iter() {
        let mut ba = BitField::new(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| ba.clear_all());
        });
    }
    group.finish();
}

// =============================================================================
// Vector Operations
// =============================================================================

fn bench_get_acts(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_acts");

    for size in [128, 1024, 4096].iter() {
        let mut ba = BitField::new(*size);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        ba.random_set_pct(&mut rng, 0.2);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(ba.get_acts()));
        });
    }
    group.finish();
}

fn bench_set_acts(c: &mut Criterion) {
    let mut group = c.benchmark_group("set_acts");

    for size in [128, 1024, 4096].iter() {
        let mut ba = BitField::new(*size);
        let indices: Vec<usize> = (0..*size / 5).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| ba.set_acts(black_box(&indices)));
        });
    }
    group.finish();
}

// =============================================================================
// Logical Operations
// =============================================================================

fn bench_bitwise_and(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitwise_and");

    for size in [128, 1024, 4096].iter() {
        let mut ba1 = BitField::new(*size);
        let mut ba2 = BitField::new(*size);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        ba1.random_set_pct(&mut rng, 0.2);
        ba2.random_set_pct(&mut rng, 0.2);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(&ba1 & &ba2));
        });
    }
    group.finish();
}

fn bench_bitwise_or(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitwise_or");

    for size in [128, 1024, 4096].iter() {
        let mut ba1 = BitField::new(*size);
        let mut ba2 = BitField::new(*size);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        ba1.random_set_pct(&mut rng, 0.2);
        ba2.random_set_pct(&mut rng, 0.2);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(&ba1 | &ba2));
        });
    }
    group.finish();
}

fn bench_bitwise_xor(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitwise_xor");

    for size in [128, 1024, 4096].iter() {
        let mut ba1 = BitField::new(*size);
        let mut ba2 = BitField::new(*size);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        ba1.random_set_pct(&mut rng, 0.2);
        ba2.random_set_pct(&mut rng, 0.2);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(&ba1 ^ &ba2));
        });
    }
    group.finish();
}

fn bench_bitwise_not(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitwise_not");

    for size in [128, 1024, 4096].iter() {
        let mut ba = BitField::new(*size);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        ba.random_set_pct(&mut rng, 0.2);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(!&ba));
        });
    }
    group.finish();
}

// =============================================================================
// Comparison Operations (CRITICAL for change tracking in Phase 2)
// =============================================================================

fn bench_equality_same(c: &mut Criterion) {
    let mut group = c.benchmark_group("equality_same");

    for size in [128, 1024, 4096, 16384].iter() {
        let mut ba1 = BitField::new(*size);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        ba1.random_set_pct(&mut rng, 0.2);
        let ba2 = ba1.clone();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(ba1 == ba2));
        });
    }
    group.finish();
}

fn bench_equality_different(c: &mut Criterion) {
    let mut group = c.benchmark_group("equality_different");

    for size in [128, 1024, 4096, 16384].iter() {
        let mut ba1 = BitField::new(*size);
        let mut ba2 = BitField::new(*size);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        ba1.random_set_pct(&mut rng, 0.2);
        ba2.random_set_pct(&mut rng, 0.2);
        ba2.toggle_bit(0); // Make them different

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(ba1 == ba2));
        });
    }
    group.finish();
}

// =============================================================================
// Word-Level Copy (CRITICAL for Phase 2 lazy copying)
// =============================================================================

fn bench_bitfield_copy_words(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitfield_copy_words");

    for size in [128, 1024, 4096].iter() {
        let mut src = BitField::new(*size);
        let mut dst = BitField::new(*size * 2);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        src.random_set_pct(&mut rng, 0.2);

        let num_words = src.num_words();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                bitfield_copy_words(black_box(&mut dst), black_box(&src), 0, 0, num_words);
            });
        });
    }
    group.finish();
}

// =============================================================================
// Random Operations
// =============================================================================

fn bench_random_set_num(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_set_num");

    for size in [128, 1024, 4096].iter() {
        let mut ba = BitField::new(*size);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let num = size / 5;

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| ba.random_set_num(black_box(&mut rng), black_box(num)));
        });
    }
    group.finish();
}

fn bench_random_shuffle(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_shuffle");

    for size in [128, 1024, 4096].iter() {
        let mut ba = BitField::new(*size);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        ba.random_set_pct(&mut rng, 0.2);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let mut rng_local = rand::rngs::StdRng::seed_from_u64(0);
            b.iter(|| ba.random_shuffle(black_box(&mut rng_local)));
        });
    }
    group.finish();
}

// =============================================================================
// Find Operations
// =============================================================================

fn bench_find_next_set_bit(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_next_set_bit");

    for size in [128, 1024, 4096].iter() {
        let mut ba = BitField::new(*size);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        ba.random_set_pct(&mut rng, 0.2);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(ba.find_next_set_bit(0)));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_set_bit,
    bench_get_bit,
    bench_clear_bit,
    bench_toggle_bit,
    bench_num_set,
    bench_num_similar,
    bench_set_all,
    bench_clear_all,
    bench_get_acts,
    bench_set_acts,
    bench_bitwise_and,
    bench_bitwise_or,
    bench_bitwise_xor,
    bench_bitwise_not,
    bench_equality_same,
    bench_equality_different,
    bench_bitfield_copy_words,
    bench_random_set_num,
    bench_random_shuffle,
    bench_find_next_set_bit
);

criterion_main!(benches);
