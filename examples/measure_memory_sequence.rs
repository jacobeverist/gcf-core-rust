//! Measure actual memory usage of networks using SequenceLearner blocks

use gnomics::{
    blocks::{DiscreteTransformer, SequenceLearner},
    Block, Network, Result,
};

fn main() -> Result<()> {
    println!("=== Network Memory Usage with SequenceLearner ===\n");
    println!("{:<10} {:<15} {:<20}", "Size", "Memory (bytes)", "Per-Block (bytes)");
    println!("{:-<50}", "");

    for size in [10, 50, 100, 250, 500] {
        let mut net = Network::new();
        let encoder = net.add(DiscreteTransformer::new(10, 512, 2, 0));
        let mut prev = encoder;

        for i in 0..size - 1 {
            let learner = net.add(SequenceLearner::new(
                512,   // columns
                4,     // statelets per column
                8,     // dendrites per statelet
                32,    // receptors per dendrite
                20,    // dendrite threshold
                20,    // perm_thr
                2,     // perm_inc
                1,     // perm_dec
                2,     // history depth
                false, // always_update
                i as u64,
            ));
            net.connect_to_input(prev, learner)?;
            prev = learner;
        }

        net.build()?;

        let memory = net.memory_usage();
        let per_block = memory / size;

        println!(
            "{:<10} {:<15} {:<20}",
            size,
            format_bytes(memory),
            format_bytes(per_block)
        );
    }

    Ok(())
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
