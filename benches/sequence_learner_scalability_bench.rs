use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use gnomics::{
    blocks::{DiscreteTransformer, SequenceLearner},
    Block, Network, PatternPooler, ScalarTransformer,
};
use rand::Rng;
use std::time::Duration;

// ============================================================================
// Benchmark: Network Creation and Block Addition
// ============================================================================

fn bench_network_add_blocks(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequence_network_add_blocks");
    group.measurement_time(Duration::from_secs(50));

    for size in [10, 50, 100, 250, 500].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut net = Network::new();
                for i in 0..size {
                    black_box(net.add(SequenceLearner::new(
                        512, 4, 8, 32, 20, 20, 2, 1, 2, false, i as u64,
                    )));
                }
            });
        });
    }
    group.finish();
}

// ============================================================================
// Benchmark: Linear Pipeline (1 → 2 → 3 → ... → N)
// ============================================================================

fn bench_linear_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequence_linear_pipeline");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(20);

    for size in [5, 10, 25, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut net = Network::new();

                // Create linear chain: encoder → learner → learner → ...
                let encoder = net.add(DiscreteTransformer::new(10, 512, 2, 0));
                let mut prev = encoder;

                // Add sequence learners
                for i in 0..size - 1 {
                    let learner = net.add(SequenceLearner::new(
                        512, 4, 8, 32, 20, 20, 2, 1, 2, false, i as u64,
                    ));
                    net.connect_to_input(prev, learner).unwrap();
                    prev = learner;
                }

                // Build
                black_box(net.build().unwrap());
            });
        });
    }
    group.finish();
}

// ============================================================================
// Benchmark: Star Topology (1 encoder → N learners)
// ============================================================================

fn bench_star_topology(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequence_star_topology");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(20);

    for size in [5, 10, 25, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut net = Network::new();

                // Single encoder
                let encoder = net.add(DiscreteTransformer::new(10, 512, 2, 0));

                // Multiple learners all connected to same encoder
                for i in 0..size {
                    let learner = net.add(SequenceLearner::new(
                        512, 4, 8, 32, 20, 20, 2, 1, 2, false, i as u64,
                    ));
                    net.connect_to_input(encoder, learner).unwrap();
                }

                // Build
                black_box(net.build().unwrap());
            });
        });
    }
    group.finish();
}

// ============================================================================
// Benchmark: Diamond/Merge Topology (N encoders → 1 learner)
// ============================================================================

fn bench_diamond_topology(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequence_diamond_topology");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(20);

    for size in [5, 10, 25, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut net = Network::new();

                // Multiple encoders
                let mut encoders = Vec::new();
                for i in 0..size {
                    encoders.push(net.add(DiscreteTransformer::new(10, 256, 2, i as u64)));
                }

                // Single learner receiving from all encoders
                let learner = net.add(SequenceLearner::new(
                    size * 256,
                    4,
                    8,
                    32,
                    20,
                    20,
                    2,
                    1,
                    2,
                    false,
                    0,
                ));
                net.connect_many_to_input(&encoders, learner).unwrap();

                // Build
                black_box(net.build().unwrap());
            });
        });
    }
    group.finish();
}

// ============================================================================
// Benchmark: Execution Performance
// ============================================================================

fn bench_execution_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequence_execution_performance");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(20);

    for size in [5, 10, 25, 50].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            // Setup network once
            let mut net = Network::new();
            let num_v = 10; // Number of discrete values
            let encoder = net.add(DiscreteTransformer::new(num_v, 512, 2, 0));
            let mut prev = encoder;

            for i in 0..size - 1 {
                let learner = net.add(SequenceLearner::new(
                    512, 4, 8, 32, 20, 20, 2, 1, 2, false, i as u64,
                ));
                net.connect_to_input(prev, learner).unwrap();
                prev = learner;

                let pooler = net.add(PatternPooler::new(
                    512, 20, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, i as u64,
                ));
                net.connect_to_input(prev, pooler).unwrap();
                prev = pooler;
            }

            net.build().unwrap();

            // Initialize only SequenceLearner blocks (DiscreteTransformer auto-inits)
            for &block_id in net.block_ids().collect::<Vec<_>>().iter() {
                if let Ok(learner) = net.get_mut::<SequenceLearner>(block_id) {
                    learner.init().unwrap();
                }
                if let Ok(pooler) = net.get_mut::<PatternPooler>(block_id) {
                    pooler.init().unwrap();
                }
            }

            // Benchmark execution with random sampling
            let mut rng = rand::thread_rng();
            b.iter(|| {
                let value = rng.gen_range(0..num_v);
                net.get_mut::<DiscreteTransformer>(encoder)
                    .unwrap()
                    .set_value(value);
                black_box(net.execute(false).unwrap());
            });
        });
    }
    group.finish();
}

// ============================================================================
// Benchmark: Connection Operations
// ============================================================================

fn bench_connection_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequence_connection_operations");
    group.measurement_time(Duration::from_secs(10));

    for size in [10, 50, 100, 250, 500].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("sequential", size), size, |b, &size| {
            b.iter(|| {
                let mut net = Network::new();

                // Create blocks
                let mut blocks = Vec::new();
                for i in 0..size {
                    blocks.push(net.add(SequenceLearner::new(
                        512, 4, 8, 32, 20, 20, 2, 1, 2, false, i as u64,
                    )));
                }

                // Connect sequentially (linear chain)
                for i in 0..size - 1 {
                    black_box(net.connect_to_input(blocks[i], blocks[i + 1]).unwrap());
                }
            });
        });
    }
    group.finish();
}

// ============================================================================
// Benchmark: Build (Topological Sort) Performance
// ============================================================================

fn bench_build_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequence_build_performance");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(20);

    for size in [10, 25, 50, 100, 250].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    // Setup: Create network with linear topology
                    let mut net = Network::new();
                    let encoder = net.add(DiscreteTransformer::new(10, 512, 2, 0));
                    let mut prev = encoder;

                    for i in 0..size - 1 {
                        let learner = net.add(SequenceLearner::new(
                            512, 4, 8, 32, 20, 20, 2, 1, 2, false, i as u64,
                        ));
                        net.connect_to_input(prev, learner).unwrap();
                        prev = learner;
                    }
                    net
                },
                |mut net| {
                    // Benchmark: Just the build step
                    black_box(net.build().unwrap())
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

// ============================================================================
// Benchmark: Complex Multi-Stage Pipeline
// ============================================================================

fn bench_complex_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequence_complex_pipeline");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    for stages in [3, 5, 10].iter() {
        group.bench_with_input(BenchmarkId::new("stages", stages), stages, |b, &stages| {
            b.iter(|| {
                let mut net = Network::new();

                // Each stage: 3 encoders → learner
                let mut stage_outputs = Vec::new();

                for stage in 0..stages {
                    // 3 encoders per stage
                    let enc1 = net.add(DiscreteTransformer::new(10, 256, 2, (stage * 3) as u64));
                    let enc2 =
                        net.add(DiscreteTransformer::new(10, 256, 2, (stage * 3 + 1) as u64));
                    let enc3 =
                        net.add(DiscreteTransformer::new(10, 256, 2, (stage * 3 + 2) as u64));

                    // Learner for this stage
                    let learner = net.add(SequenceLearner::new(
                        768,
                        4,
                        8,
                        32,
                        20,
                        20,
                        2,
                        1,
                        2,
                        false,
                        stage as u64,
                    ));

                    net.connect_many_to_input(&[enc1, enc2, enc3], learner)
                        .unwrap();

                    // Connect to previous stage if not first
                    if !stage_outputs.is_empty() {
                        net.connect_many_to_input(&stage_outputs, learner).unwrap();
                    }

                    stage_outputs.clear();
                    stage_outputs.push(learner);
                }

                black_box(net.build().unwrap());
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_network_add_blocks,
    bench_linear_pipeline,
    bench_star_topology,
    bench_diamond_topology,
    bench_connection_operations,
    bench_build_performance,
    bench_execution_performance,
    bench_complex_pipeline,
);
criterion_main!(benches);
