//! Performance benchmarks for Phase 2 Block Infrastructure
//!
//! Tests critical paths to ensure performance targets are met:
//! - add_child: <10ns
//! - pull (per child): <120ns for 1024 bits
//! - children_changed: <10ns per child
//! - store with comparison: <100ns for 1024 bits

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gnomics::{BlockInput, BlockOutput, BlockMemory};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::cell::RefCell;
use std::rc::Rc;

fn bench_add_child(c: &mut Criterion) {
    c.bench_function("BlockInput::add_child", |b| {
        let mut output = BlockOutput::new();
        output.setup(2, 1024);

        let output = Rc::new(RefCell::new(output));

        b.iter(|| {
            let mut test_input = BlockInput::new();
            test_input.add_child(black_box(Rc::clone(&output)), black_box(0));
            black_box(test_input);
        });
    });
}

fn bench_pull(c: &mut Criterion) {
    let mut group = c.benchmark_group("BlockInput::pull");

    for num_children in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_children),
            num_children,
            |b, &num_children| {
                let mut input = BlockInput::new();
                let mut rng = StdRng::seed_from_u64(42);

                let outputs: Vec<_> = (0..num_children)
                    .map(|_| {
                        let mut out = BlockOutput::new();
                        out.setup(2, 1024);
                        out.state.random_set_num(&mut rng, 128);
                        out.store();
                        Rc::new(RefCell::new(out))
                    })
                    .collect();

                for output in &outputs {
                    input.add_child(Rc::clone(output), 0);
                }

                b.iter(|| {
                    input.pull();
                    black_box(&input.state);
                });
            },
        );
    }
    group.finish();
}

fn bench_pull_unchanged(c: &mut Criterion) {
    c.bench_function("BlockInput::pull (unchanged)", |b| {
        let mut input = BlockInput::new();
        let mut output = BlockOutput::new();
        output.setup(2, 1024);
        output.store(); // Mark as unchanged

        let output = Rc::new(RefCell::new(output));
        input.add_child(Rc::clone(&output), 0);

        // First pull to initialize
        input.pull();

        // Now benchmark unchanged pull
        b.iter(|| {
            input.pull(); // Should skip copy
            black_box(&input.state);
        });
    });
}

fn bench_children_changed(c: &mut Criterion) {
    let mut group = c.benchmark_group("BlockInput::children_changed");

    for num_children in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_children),
            num_children,
            |b, &num_children| {
                let mut input = BlockInput::new();

                let outputs: Vec<_> = (0..num_children)
                    .map(|_| {
                        let mut out = BlockOutput::new();
                        out.setup(2, 1024);
                        out.store(); // Unchanged
                        Rc::new(RefCell::new(out))
                    })
                    .collect();

                for output in &outputs {
                    input.add_child(Rc::clone(output), 0);
                }

                b.iter(|| {
                    black_box(input.children_changed());
                });
            },
        );
    }
    group.finish();
}

fn bench_store(c: &mut Criterion) {
    c.bench_function("BlockOutput::store (with comparison)", |b| {
        let mut output = BlockOutput::new();
        output.setup(2, 1024);

        // Set initial pattern
        for i in 0..128 {
            output.state.set_bit(i * 8);
        }
        output.store();
        output.step();

        // Modify slightly
        output.state.set_bit(0);

        b.iter(|| {
            output.store(); // Should compare and detect change
            black_box(&output);
        });
    });
}

fn bench_block_memory_overlap(c: &mut Criterion) {
    c.bench_function("BlockMemory::overlap", |b| {
        let mut memory = BlockMemory::new(100, 50, 20, 2, 1, 0.3);
        let mut rng = StdRng::seed_from_u64(42);
        memory.init_pooled(1024, &mut rng, 0.8, 0.5);

        let mut input = gnomics::BitArray::new(1024);
        input.random_set_num(&mut rng, 128);

        b.iter(|| {
            let overlap = memory.overlap(black_box(0), black_box(&input));
            black_box(overlap);
        });
    });
}

fn bench_block_memory_learn(c: &mut Criterion) {
    c.bench_function("BlockMemory::learn", |b| {
        let mut memory = BlockMemory::new(100, 50, 20, 2, 1, 0.3);
        let mut rng = StdRng::seed_from_u64(42);
        memory.init_pooled(1024, &mut rng, 0.8, 0.5);

        let mut input = gnomics::BitArray::new(1024);
        input.random_set_num(&mut rng, 128);

        b.iter(|| {
            memory.learn(black_box(0), black_box(&input), black_box(&mut rng));
        });
    });
}

fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("end-to-end pipeline");

    for change_rate in [0.0, 0.1, 0.5, 1.0].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:.0}% change", change_rate * 100.0)),
            change_rate,
            |b, &change_rate| {
                let mut encoder_output = BlockOutput::new();
                encoder_output.setup(2, 1024);

                let mut processor_input = BlockInput::new();

                let encoder_rc = Rc::new(RefCell::new(encoder_output.clone()));
                processor_input.add_child(Rc::clone(&encoder_rc), 0);

                let mut rng = StdRng::seed_from_u64(42);
                let mut step_count = 0;

                b.iter(|| {
                    // Encoder produces output
                    let mut encoder = encoder_rc.borrow_mut();

                    // Change pattern based on change rate
                    if rng.gen::<f64>() < change_rate {
                        encoder.state.random_set_num(&mut rng, 128);
                    }

                    encoder.step();
                    encoder.store();
                    drop(encoder);

                    // Processor pulls (should skip if unchanged)
                    processor_input.pull();

                    step_count += 1;
                    black_box(&processor_input);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_add_child,
    bench_pull,
    bench_pull_unchanged,
    bench_children_changed,
    bench_store,
    bench_block_memory_overlap,
    bench_block_memory_learn,
    bench_end_to_end
);
criterion_main!(benches);
