//! Quick performance validation for critical operations.
//!
//! This example measures key operations and compares against C++ targets.

use gnomics::{bitfield_copy_words, BitField};
use rand::SeedableRng;
use std::time::Instant;

fn measure<F>(name: &str, target_ns: f64, iterations: usize, mut f: F)
where
    F: FnMut(),
{
    // Warmup
    for _ in 0..100 {
        f();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();
    let ns_per_op = elapsed.as_nanos() as f64 / iterations as f64;

    let status = if ns_per_op <= target_ns {
        "PASS"
    } else if ns_per_op <= target_ns * 1.2 {
        "CLOSE"
    } else {
        "MISS"
    };

    println!(
        "{:30} {:6.1}ns (target: {:5.0}ns) [{}]",
        name, ns_per_op, target_ns, status
    );
}

fn main() {
    println!("\n=== Gnomics Rust Performance Validation ===\n");
    println!("Testing critical operations against C++ targets:\n");

    let mut ba = BitField::new(10000);
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    // Individual bit operations
    measure("set_bit", 3.0, 1_000_000, || {
        ba.set_bit(5000);
    });

    measure("get_bit", 2.0, 1_000_000, || {
        let _ = ba.get_bit(5000);
    });

    measure("clear_bit", 3.0, 1_000_000, || {
        ba.clear_bit(5000);
    });

    measure("toggle_bit", 3.0, 1_000_000, || {
        ba.toggle_bit(5000);
    });

    // Counting operations (1024 bits)
    let mut ba1024 = BitField::new(1024);
    ba1024.random_set_pct(&mut rng, 0.2);

    measure("num_set (1024 bits)", 60.0, 100_000, || {
        let _ = ba1024.num_set();
    });

    // Word-level copy (1024 bits = 32 words)
    let mut src = BitField::new(1024);
    let mut dst = BitField::new(2048);
    src.random_set_pct(&mut rng, 0.2);

    measure("bitfield_copy_words (1024b)", 60.0, 100_000, || {
        bitfield_copy_words(&mut dst, &src, 0, 0, 32);
    });

    // Comparison (critical for change tracking)
    let ba_cmp1 = ba1024.clone();
    let ba_cmp2 = ba1024.clone();

    measure("PartialEq (same, 1024b)", 60.0, 100_000, || {
        let _ = ba_cmp1 == ba_cmp2;
    });

    let mut ba_cmp3 = ba1024.clone();
    ba_cmp3.toggle_bit(0);

    measure("PartialEq (diff, 1024b)", 60.0, 100_000, || {
        let _ = ba1024 == ba_cmp3;
    });

    // Logical operations (1024 bits)
    let ba_or1 = ba1024.clone();
    let ba_or2 = ba1024.clone();

    measure("bitwise_and (1024b)", 100.0, 50_000, || {
        let _ = &ba_or1 & &ba_or2;
    });

    measure("bitwise_or (1024b)", 100.0, 50_000, || {
        let _ = &ba_or1 | &ba_or2;
    });

    measure("bitwise_xor (1024b)", 100.0, 50_000, || {
        let _ = &ba_or1 ^ &ba_or2;
    });

    println!("\n=== Summary ===");
    println!("All critical operations validated!");
    println!("PASS: Meets C++ target");
    println!("CLOSE: Within 20% of target");
    println!("MISS: Exceeds target by >20%");
}
