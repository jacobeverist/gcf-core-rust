//! Comprehensive comparison benchmarks: Custom BitArray vs bitvec-based BitArrayBitvec.
//!
//! This benchmark suite validates whether migrating to bitvec maintains acceptable
//! performance for critical Phase 2 operations.
//!
//! # Critical Performance Targets (from RUST_CONVERSION_PLAN.md)
//!
//! Must maintain:
//! - set_bit: <3ns
//! - get_bit: <2ns
//! - num_set (1024 bits): <60ns
//! - bitarray_copy_words (1024 bits): <120ns (relaxed from 60ns)
//! - PartialEq (1024 bits): <100ns (critical for change tracking)
//!
//! Acceptable regression: <10% for critical operations, <20% for others

use criterion::{black_box, criterion_group, criterion_main, BenchmarkGroup, Criterion};
use gnomics::{bitarray_copy_words, bitarray_copy_words_bitvec, BitArray, BitArrayBitvec};
use rand::SeedableRng;

// Standard test configuration
const SIZE_STANDARD: usize = 1024; // Typical SDR size
const ACTIVATION_PCT: f64 = 0.1; // 10% active (typical for Gnomics)
const SEED: u64 = 42;

// =============================================================================
// Single Bit Operations (Hot Paths)
// =============================================================================

fn bench_set_bit(c: &mut Criterion) {
    let mut group = c.benchmark_group("set_bit");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        let mut i = 0;
        b.iter(|| {
            ba.set_bit(black_box(i % SIZE_STANDARD));
            i += 1;
        });
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        let mut i = 0;
        b.iter(|| {
            ba.set_bit(black_box(i % SIZE_STANDARD));
            i += 1;
        });
    });

    group.finish();
}

fn bench_get_bit(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_bit");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        ba.set_all();
        let mut i = 0;
        b.iter(|| {
            let _ = ba.get_bit(black_box(i % SIZE_STANDARD));
            i += 1;
        });
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        ba.set_all();
        let mut i = 0;
        b.iter(|| {
            let _ = ba.get_bit(black_box(i % SIZE_STANDARD));
            i += 1;
        });
    });

    group.finish();
}

fn bench_clear_bit(c: &mut Criterion) {
    let mut group = c.benchmark_group("clear_bit");

    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        ba.set_all();
        let mut i = 0;
        b.iter(|| {
            ba.clear_bit(black_box(i % SIZE_STANDARD));
            i += 1;
        });
    });

    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        ba.set_all();
        let mut i = 0;
        b.iter(|| {
            ba.clear_bit(black_box(i % SIZE_STANDARD));
            i += 1;
        });
    });

    group.finish();
}

fn bench_toggle_bit(c: &mut Criterion) {
    let mut group = c.benchmark_group("toggle_bit");

    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        let mut i = 0;
        b.iter(|| {
            ba.toggle_bit(black_box(i % SIZE_STANDARD));
            i += 1;
        });
    });

    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        let mut i = 0;
        b.iter(|| {
            ba.toggle_bit(black_box(i % SIZE_STANDARD));
            i += 1;
        });
    });

    group.finish();
}

// =============================================================================
// Counting Operations
// =============================================================================

fn bench_num_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("num_set");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(ba.num_set()));
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(ba.num_set()));
    });

    group.finish();
}

fn bench_num_similar(c: &mut Criterion) {
    let mut group = c.benchmark_group("num_similar");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba1 = BitArray::new(SIZE_STANDARD);
        let mut ba2 = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba1.random_set_pct(&mut rng, ACTIVATION_PCT);
        ba2.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(ba1.num_similar(&ba2)));
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba1 = BitArrayBitvec::new(SIZE_STANDARD);
        let mut ba2 = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba1.random_set_pct(&mut rng, ACTIVATION_PCT);
        ba2.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(ba1.num_similar(&ba2)));
    });

    group.finish();
}

// =============================================================================
// Bulk Operations
// =============================================================================

fn bench_set_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("set_all");

    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        b.iter(|| ba.set_all());
    });

    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        b.iter(|| ba.set_all());
    });

    group.finish();
}

fn bench_clear_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("clear_all");

    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        b.iter(|| ba.clear_all());
    });

    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        b.iter(|| ba.clear_all());
    });

    group.finish();
}

// =============================================================================
// Vector Operations (CRITICAL - Used Extensively)
// =============================================================================

fn bench_set_acts(c: &mut Criterion) {
    let mut group = c.benchmark_group("set_acts");

    let num_active = (SIZE_STANDARD as f64 * ACTIVATION_PCT) as usize;
    let indices: Vec<usize> = (0..num_active).collect();

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        b.iter(|| ba.set_acts(black_box(&indices)));
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        b.iter(|| ba.set_acts(black_box(&indices)));
    });

    group.finish();
}

fn bench_get_acts(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_acts");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(ba.get_acts()));
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(ba.get_acts()));
    });

    group.finish();
}

// =============================================================================
// Logical Operations
// =============================================================================

fn bench_bitwise_and(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitwise_and");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba1 = BitArray::new(SIZE_STANDARD);
        let mut ba2 = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba1.random_set_pct(&mut rng, ACTIVATION_PCT);
        ba2.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(&ba1 & &ba2));
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba1 = BitArrayBitvec::new(SIZE_STANDARD);
        let mut ba2 = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba1.random_set_pct(&mut rng, ACTIVATION_PCT);
        ba2.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(&ba1 & &ba2));
    });

    group.finish();
}

fn bench_bitwise_or(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitwise_or");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba1 = BitArray::new(SIZE_STANDARD);
        let mut ba2 = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba1.random_set_pct(&mut rng, ACTIVATION_PCT);
        ba2.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(&ba1 | &ba2));
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba1 = BitArrayBitvec::new(SIZE_STANDARD);
        let mut ba2 = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba1.random_set_pct(&mut rng, ACTIVATION_PCT);
        ba2.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(&ba1 | &ba2));
    });

    group.finish();
}

fn bench_bitwise_xor(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitwise_xor");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba1 = BitArray::new(SIZE_STANDARD);
        let mut ba2 = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba1.random_set_pct(&mut rng, ACTIVATION_PCT);
        ba2.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(&ba1 ^ &ba2));
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba1 = BitArrayBitvec::new(SIZE_STANDARD);
        let mut ba2 = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba1.random_set_pct(&mut rng, ACTIVATION_PCT);
        ba2.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(&ba1 ^ &ba2));
    });

    group.finish();
}

fn bench_bitwise_not(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitwise_not");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(!&ba));
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(!&ba));
    });

    group.finish();
}

// =============================================================================
// Equality Comparison (CRITICAL for Phase 2 Change Tracking)
// =============================================================================

fn bench_equality_same(c: &mut Criterion) {
    let mut group = c.benchmark_group("equality_same");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba1 = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba1.random_set_pct(&mut rng, ACTIVATION_PCT);
        let ba2 = ba1.clone();

        b.iter(|| black_box(ba1 == ba2));
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba1 = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba1.random_set_pct(&mut rng, ACTIVATION_PCT);
        let ba2 = ba1.clone();

        b.iter(|| black_box(ba1 == ba2));
    });

    group.finish();
}

fn bench_equality_different(c: &mut Criterion) {
    let mut group = c.benchmark_group("equality_different");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba1 = BitArray::new(SIZE_STANDARD);
        let mut ba2 = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba1.random_set_pct(&mut rng, ACTIVATION_PCT);
        ba2.random_set_pct(&mut rng, ACTIVATION_PCT);
        ba2.toggle_bit(0); // Make them different

        b.iter(|| black_box(ba1 == ba2));
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba1 = BitArrayBitvec::new(SIZE_STANDARD);
        let mut ba2 = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba1.random_set_pct(&mut rng, ACTIVATION_PCT);
        ba2.random_set_pct(&mut rng, ACTIVATION_PCT);
        ba2.toggle_bit(0); // Make them different

        b.iter(|| black_box(ba1 == ba2));
    });

    group.finish();
}

// =============================================================================
// Word-Level Copy (CRITICAL for Phase 2 Lazy Copying)
// =============================================================================

fn bench_bitarray_copy_words(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitarray_copy_words");

    let num_words = SIZE_STANDARD / 32;

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut src = BitArray::new(SIZE_STANDARD);
        let mut dst = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        src.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| {
            bitarray_copy_words(black_box(&mut dst), black_box(&src), 0, 0, num_words);
        });
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut src = BitArrayBitvec::new(SIZE_STANDARD);
        let mut dst = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        src.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| {
            bitarray_copy_words_bitvec(black_box(&mut dst), black_box(&src), 0, 0, num_words);
        });
    });

    group.finish();
}

// =============================================================================
// Random Operations
// =============================================================================

fn bench_random_set_num(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_set_num");

    let num = (SIZE_STANDARD as f64 * ACTIVATION_PCT) as usize;

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);

        b.iter(|| ba.random_set_num(black_box(&mut rng), black_box(num)));
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);

        b.iter(|| ba.random_set_num(black_box(&mut rng), black_box(num)));
    });

    group.finish();
}

fn bench_random_shuffle(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_shuffle");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| {
            let mut rng_local = rand::rngs::StdRng::seed_from_u64(SEED);
            ba.random_shuffle(black_box(&mut rng_local));
        });
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| {
            let mut rng_local = rand::rngs::StdRng::seed_from_u64(SEED);
            ba.random_shuffle(black_box(&mut rng_local));
        });
    });

    group.finish();
}

// =============================================================================
// Find Operations
// =============================================================================

fn bench_find_next_set_bit(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_next_set_bit");

    // Custom implementation
    group.bench_function("custom", |b| {
        let mut ba = BitArray::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(ba.find_next_set_bit(0)));
    });

    // bitvec implementation
    group.bench_function("bitvec", |b| {
        let mut ba = BitArrayBitvec::new(SIZE_STANDARD);
        let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
        ba.random_set_pct(&mut rng, ACTIVATION_PCT);

        b.iter(|| black_box(ba.find_next_set_bit(0)));
    });

    group.finish();
}

criterion_group!(
    benches,
    // Hot path operations
    bench_set_bit,
    bench_get_bit,
    bench_clear_bit,
    bench_toggle_bit,
    // Counting
    bench_num_set,
    bench_num_similar,
    // Bulk operations
    bench_set_all,
    bench_clear_all,
    // Vector operations (CRITICAL)
    bench_set_acts,
    bench_get_acts,
    // Logical operations
    bench_bitwise_and,
    bench_bitwise_or,
    bench_bitwise_xor,
    bench_bitwise_not,
    // Equality (CRITICAL for change tracking)
    bench_equality_same,
    bench_equality_different,
    // Word-level copy (CRITICAL for Phase 2)
    bench_bitarray_copy_words,
    // Random operations
    bench_random_set_num,
    bench_random_shuffle,
    // Find operations
    bench_find_next_set_bit
);

criterion_main!(benches);
