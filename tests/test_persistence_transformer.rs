//! Comprehensive tests for PersistenceTransformer.
//!
//! Tests cover:
//! - Persistence counter behavior
//! - Reset on significant value changes
//! - Stable value tracking
//! - Temporal encoding properties

use gnomics::{Block, OutputAccess, PersistenceTransformer};

#[test]
fn test_persistence_basic_construction() {
    let pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);
    assert_eq!(pt.min_val(), 0.0);
    assert_eq!(pt.max_val(), 1.0);
    assert_eq!(pt.num_s(), 1024);
    assert_eq!(pt.num_as(), 128);
    assert_eq!(pt.max_step(), 100);
}

#[test]
#[should_panic(expected = "max_val must be greater than min_val")]
fn test_persistence_invalid_range() {
    PersistenceTransformer::new(1.0, 0.0, 1024, 128, 100, 2, 0);
}

#[test]
#[should_panic(expected = "max_step must be > 0")]
fn test_persistence_invalid_max_step() {
    PersistenceTransformer::new(0.0, 1.0, 1024, 128, 0, 2, 0);
}

#[test]
fn test_persistence_set_get_value() {
    let mut pt = PersistenceTransformer::new(0.0, 100.0, 1024, 128, 100, 2, 0);

    pt.set_value(50.0);
    assert_eq!(pt.get_value(), 50.0);

    pt.set_value(75.5);
    assert_eq!(pt.get_value(), 75.5);
}

#[test]
fn test_persistence_counter_increments_stable() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    pt.set_value(0.5);
    assert_eq!(pt.get_counter(), 0);

    // First encode resets due to initial change from 0.0 to 0.5
    pt.execute(false).unwrap();
    assert_eq!(pt.get_counter(), 0);

    // Now counter should increment each encode when value is stable
    pt.execute(false).unwrap();
    assert_eq!(pt.get_counter(), 1);

    pt.execute(false).unwrap();
    assert_eq!(pt.get_counter(), 2);

    pt.execute(false).unwrap();
    assert_eq!(pt.get_counter(), 3);
}

#[test]
fn test_persistence_counter_resets_on_large_change() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    // Build up persistence (first encode is reset, next 5 increment)
    pt.set_value(0.5);
    for _ in 0..6 {
        pt.execute(false).unwrap();
    }
    assert_eq!(pt.get_counter(), 5);

    // Large change (>10% of range) should reset counter
    pt.set_value(0.8); // 30% change
    pt.execute(false).unwrap();
    assert_eq!(pt.get_counter(), 0, "Counter should reset on large change");
}

#[test]
fn test_persistence_counter_no_reset_small_change() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    // Build up persistence (first encode is reset, next 3 increment)
    pt.set_value(0.5);
    for _ in 0..4 {
        pt.execute(false).unwrap();
    }
    assert_eq!(pt.get_counter(), 3);

    // Small change (<10% of range) should not reset
    pt.set_value(0.55); // 5% change
    pt.execute(false).unwrap();
    assert_eq!(
        pt.get_counter(),
        4,
        "Counter should continue on small change"
    );
}

#[test]
#[ignore = "TODO: Fix PersistenceTransformer initialization - see ARCHITECTURE_ISSUES.md"]
fn test_persistence_counter_exactly_10_percent_boundary() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    // Build up persistence
    pt.set_value(0.5);
    for _ in 0..3 {
        pt.execute(false).unwrap();
    }

    // Exactly 10% change (boundary case)
    pt.set_value(0.6); // Exactly 10%
    pt.execute(false).unwrap();
    assert_eq!(
        pt.get_counter(),
        4,
        "10% change should not reset (threshold is >10%)"
    );

    // Just over 10% should reset
    pt.set_value(0.71); // 11% change from 0.6
    pt.execute(false).unwrap();
    assert_eq!(
        pt.get_counter(),
        0,
        "Just over 10% should reset counter"
    );
}

#[test]
#[ignore = "TODO: Fix PersistenceTransformer initialization - see ARCHITECTURE_ISSUES.md"]
fn test_persistence_counter_caps_at_max() {
    let max_step = 10;
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, max_step, 2, 0);

    pt.set_value(0.5);

    // Encode more than max_step times
    for i in 0..20 {
        pt.execute(false).unwrap();

        if i < max_step {
            assert_eq!(pt.get_counter(), i + 1);
        } else {
            assert_eq!(
                pt.get_counter(),
                max_step,
                "Counter should cap at max_step"
            );
        }
    }
}

#[test]
fn test_persistence_encoding_num_active() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    pt.set_value(0.5);

    // Should always have num_as active bits regardless of counter
    for _ in 0..10 {
        pt.execute(false).unwrap();
        assert_eq!(
            pt.output().borrow().state.num_set(),
            128,
            "Should always have 128 active bits"
        );
    }
}

#[test]
fn test_persistence_encoding_changes_with_counter() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 2048, 256, 100, 2, 0);

    pt.set_value(0.5);

    // Get patterns at different persistence levels
    let mut patterns = Vec::new();
    for i in 0..=10 {
        pt.execute(false).unwrap();
        if i % 2 == 0 {
            patterns.push(pt.output().borrow().state.clone());
        }
    }

    // Patterns should differ as persistence increases
    for i in 1..patterns.len() {
        let same = patterns[i - 1] == patterns[i];
        assert!(
            !same,
            "Patterns should change as persistence counter increases"
        );
    }
}

#[test]
fn test_persistence_low_vs_high() {
    let mut pt_low = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);
    let mut pt_high = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    // Low persistence (1 step)
    pt_low.set_value(0.5);
    pt_low.execute(false).unwrap();

    // High persistence (50 steps)
    pt_high.set_value(0.5);
    for _ in 0..50 {
        pt_high.execute(false).unwrap();
    }

    // Different persistence levels should have different patterns
    let overlap = pt_low.output().borrow().state.num_similar(&pt_high.output().borrow().state);
    assert!(
        overlap < 100,
        "Different persistence levels should have different patterns, overlap={}",
        overlap
    );
}

#[test]
fn test_persistence_progression() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 50, 2, 0);

    pt.set_value(0.5);

    let mut prev_start = 0;

    // As persistence increases, encoded pattern should shift
    for i in 0..=50 {
        pt.execute(false).unwrap();

        if i % 10 == 0 {
            let acts = pt.output().borrow().state.get_acts();
            let start = acts[0];

            if i > 0 {
                assert!(
                    start > prev_start,
                    "Pattern should shift as persistence increases"
                );
            }
            prev_start = start;
        }
    }
}

#[test]
fn test_persistence_clear() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    pt.set_value(0.5);
    for _ in 0..10 {
        pt.execute(false).unwrap();
    }
    assert!(pt.get_counter() > 0);

    pt.clear();

    assert_eq!(pt.output().borrow().state.num_set(), 0, "Output should be cleared");
    assert_eq!(pt.get_counter(), 0, "Counter should be reset");
}

#[test]
#[ignore = "TODO: Fix PersistenceTransformer initialization - see ARCHITECTURE_ISSUES.md"]
fn test_persistence_multiple_stable_periods() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    // First stable period
    pt.set_value(0.3);
    for _ in 0..5 {
        pt.execute(false).unwrap();
    }
    assert_eq!(pt.get_counter(), 5);

    // Change value significantly
    pt.set_value(0.8);
    pt.execute(false).unwrap();
    assert_eq!(pt.get_counter(), 0, "Counter should reset");

    // Second stable period
    for _ in 0..7 {
        pt.execute(false).unwrap();
    }
    assert_eq!(pt.get_counter(), 7);
}

#[test]
#[ignore = "TODO: Fix PersistenceTransformer initialization - see ARCHITECTURE_ISSUES.md"]
fn test_persistence_gradual_drift() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    // Start at 0.5
    pt.set_value(0.5);
    pt.execute(false).unwrap();

    // Gradually drift upward by small increments (<10% each)
    for i in 1..10 {
        let new_val = 0.5 + (i as f64 * 0.02); // 2% increments
        pt.set_value(new_val);
        pt.execute(false).unwrap();

        // Counter should keep incrementing
        assert_eq!(pt.get_counter(), i + 1);
    }
}

#[test]
fn test_persistence_oscillation() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    // Oscillate between two values (>10% apart)
    for i in 0..10 {
        let val = if i % 2 == 0 { 0.3 } else { 0.8 };
        pt.set_value(val);
        pt.execute(false).unwrap();

        // Counter should reset each time (can't build persistence)
        assert_eq!(
            pt.get_counter(),
            0,
            "Oscillation should prevent persistence buildup"
        );
    }
}

#[test]
fn test_persistence_memory_usage() {
    let pt = PersistenceTransformer::new(0.0, 1.0, 2048, 256, 100, 3, 0);
    let usage = pt.memory_usage();

    // Should be reasonable (bitvec has more overhead than raw Vec<u32>)
    // 2048 bits * 3 time steps = 6144 bits = 768 bytes minimum
    // With bitvec overhead, expect ~40KB
    assert!(usage > 0);
    assert!(usage < 100_000, "Memory usage seems too high: {}", usage);
    assert!(usage > 1_000, "Memory usage seems too low: {}", usage);
}

#[test]
#[ignore = "TODO: Fix PersistenceTransformer initialization - see ARCHITECTURE_ISSUES.md"]
fn test_persistence_different_ranges() {
    // Temperature range
    let mut temp = PersistenceTransformer::new(0.0, 100.0, 1024, 128, 100, 2, 0);
    temp.set_value(50.0);
    for _ in 0..5 {
        temp.execute(false).unwrap();
    }
    assert_eq!(temp.get_counter(), 5);

    // Negative range
    let mut neg = PersistenceTransformer::new(-10.0, 10.0, 1024, 128, 100, 2, 0);
    neg.set_value(0.0);
    for _ in 0..5 {
        neg.execute(false).unwrap();
    }
    assert_eq!(neg.get_counter(), 5);
}

#[test]
fn test_persistence_zero_counter_encoding() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    pt.set_value(0.5);
    pt.execute(false).unwrap();

    // Counter is 1, but percentage is 1/100 = 0.01
    // Should activate bits near the start
    let acts = pt.output().borrow().state.get_acts();
    assert!(
        acts[0] < 50,
        "Low persistence should encode near start, got {}",
        acts[0]
    );
}

#[test]
fn test_persistence_max_counter_encoding() {
    let max_step = 10;
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, max_step, 2, 0);

    pt.set_value(0.5);
    for _ in 0..20 {
        pt.execute(false).unwrap();
    }

    // Counter should be at max
    assert_eq!(pt.get_counter(), max_step);

    // Should activate bits near the end
    let acts = pt.output().borrow().state.get_acts();
    assert!(
        acts[acts.len() - 1] > 900,
        "Max persistence should encode near end, got {}",
        acts[acts.len() - 1]
    );
}

#[test]
fn test_persistence_history_tracking() {
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 3, 0);

    // Low persistence
    pt.set_value(0.5);
    pt.execute(false).unwrap();
    let acts1 = pt.output().borrow().get_bitfield(0).get_acts();

    // Build more persistence
    for _ in 0..10 {
        pt.execute(false).unwrap();
    }
    let acts2 = pt.output().borrow().get_bitfield(0).get_acts();

    // Should be different
    assert_ne!(acts1, acts2, "Different persistence should encode differently");
}

#[test]
fn test_persistence_deterministic() {
    let mut pt1 = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);
    let mut pt2 = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    // Same sequence
    for _ in 0..5 {
        pt1.set_value(0.5);
        pt1.execute(false).unwrap();

        pt2.set_value(0.5);
        pt2.execute(false).unwrap();
    }

    // Should produce identical results
    assert_eq!(pt1.get_counter(), pt2.get_counter());
    assert_eq!(pt1.output().borrow().state, pt2.output().borrow().state);
}

#[test]
#[ignore = "TODO: Fix PersistenceTransformer initialization - see ARCHITECTURE_ISSUES.md"]
fn test_persistence_practical_temperature_example() {
    // Simulate temperature sensor with stability detection
    let mut temp = PersistenceTransformer::new(0.0, 100.0, 2048, 256, 100, 2, 0);

    // Room temperature stable at 22°C
    temp.set_value(22.0);
    for _ in 0..50 {
        temp.execute(false).unwrap();
    }

    let stable_pattern = temp.output().borrow().state.clone();
    let stable_counter = temp.get_counter();

    assert_eq!(stable_counter, 50, "Should build up persistence");

    // Small fluctuation (within 10% = 10°C)
    temp.set_value(25.0); // 3°C change
    temp.execute(false).unwrap();

    assert!(
        temp.get_counter() > 0,
        "Small fluctuation should not reset"
    );

    // Large change (heater turns on)
    temp.set_value(60.0); // 35°C change
    temp.execute(false).unwrap();

    assert_eq!(temp.get_counter(), 0, "Large change should reset");

    // New stable period at 60°C
    for _ in 0..30 {
        temp.execute(false).unwrap();
    }

    assert_eq!(temp.get_counter(), 30);

    let hot_pattern = temp.output().borrow().state.clone();

    // Stable patterns at different persistence should differ
    assert_ne!(stable_pattern, hot_pattern);
}
