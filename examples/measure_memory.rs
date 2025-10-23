//! Measure actual memory usage of networks of different sizes

use gnomics::{
    blocks::{PatternPooler, ScalarTransformer},
    Block, Network, Result,
};

fn main() -> Result<()> {
    println!("=== Network Memory Usage Measurement ===\n");
    println!("{:<10} {:<15} {:<20}", "Size", "Memory (bytes)", "Per-Block (bytes)");
    println!("{:-<50}", "");

    for size in [10, 50, 100, 250, 500] {
        let mut net = Network::new();
        let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
        let mut prev = encoder;

        for i in 0..size - 1 {
            let pooler = net.add(PatternPooler::new(
                1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, i as u64,
            ));
            net.connect_to_input(prev, pooler)?;
            prev = pooler;
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
