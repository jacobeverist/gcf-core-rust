//! Integration tests for transformer blocks.
//!
//! Tests interaction between transformers and validates real-world usage patterns.

use gnomics::{Block, DiscreteTransformer, PersistenceTransformer, ScalarTransformer};

#[test]
fn test_scalar_vs_discrete_comparison() {
    // Compare encoding properties of scalar vs discrete transformers
    let mut scalar = ScalarTransformer::new(0.0, 10.0, 1024, 128, 2, 0);
    let mut discrete = DiscreteTransformer::new(10, 1024, 2, 0);

    // Encode similar concepts
    scalar.set_value(5.0);
    scalar.execute(false).unwrap();

    discrete.set_value(5);
    discrete.execute(false).unwrap();

    // Both should have output
    assert!(scalar.output().borrow().state.num_set() > 0);
    assert!(discrete.output().borrow().state.num_set() > 0);

    // Patterns will differ (scalar has overlapping, discrete doesn't)
    assert_ne!(scalar.output().borrow().state, discrete.output().borrow().state);
}

#[test]
fn test_multiple_transformers_pipeline() {
    // Simulate a multi-sensor system
    let mut temp = ScalarTransformer::new(0.0, 100.0, 1024, 128, 2, 0);
    let mut mode = DiscreteTransformer::new(4, 1024, 2, 0); // 4 operating modes
    let mut stability = PersistenceTransformer::new(0.0, 100.0, 1024, 128, 100, 2, 0);

    // Set all sensors
    temp.set_value(22.5);
    mode.set_value(2); // Mode 2
    stability.set_value(22.5);

    // Process all
    temp.execute(false).unwrap();
    mode.execute(false).unwrap();
    stability.execute(false).unwrap();

    // All should produce output
    assert_eq!(temp.output().borrow().state.num_set(), 128);
    assert_eq!(mode.output().borrow().state.num_set(), 256); // 1024/4
    assert_eq!(stability.output().borrow().state.num_set(), 128);
}

#[test]
#[ignore = "TODO: Fix floating-point precision in semantic similarity - see ARCHITECTURE_ISSUES.md"]
fn test_scalar_semantic_properties() {
    // Test that ScalarTransformer preserves semantic similarity
    let values = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
    let mut transformers: Vec<ScalarTransformer> = values
        .iter()
        .map(|_| ScalarTransformer::new(0.0, 1.0, 2048, 256, 2, 0))
        .collect();

    // Encode all values
    for (i, t) in transformers.iter_mut().enumerate() {
        t.set_value(values[i]);
        t.execute(false).unwrap();
    }

    // Adjacent values should have high overlap
    for i in 0..(transformers.len() - 1) {
        let overlap = transformers[i]
            .output().borrow().state
            .num_similar(&transformers[i + 1].output().borrow().state);
        let pct = (overlap as f64) / 256.0;

        assert!(
            pct > 0.5,
            "Adjacent values ({} vs {}) should have >50% overlap, got {:.1}%",
            values[i],
            values[i + 1],
            pct * 100.0
        );
    }

    // Distant values should have low overlap
    let overlap_0_1 = transformers[0]
        .output().borrow().state
        .num_similar(&transformers[5].output().borrow().state);
    let pct = (overlap_0_1 as f64) / 256.0;

    assert!(
        pct < 0.2,
        "Distant values (0.0 vs 1.0) should have <20% overlap, got {:.1}%",
        pct * 100.0
    );
}

#[test]
fn test_discrete_categorical_independence() {
    // Test that DiscreteTransformer creates independent categories
    let num_categories = 8;
    let mut transformers: Vec<DiscreteTransformer> = (0..num_categories)
        .map(|_| DiscreteTransformer::new(num_categories, 2048, 2, 0))
        .collect();

    // Encode all categories
    for (i, t) in transformers.iter_mut().enumerate() {
        t.set_value(i);
        t.execute(false).unwrap();
    }

    // All pairs should have zero overlap
    for i in 0..num_categories {
        for j in (i + 1)..num_categories {
            let overlap = transformers[i]
                .output().borrow().state
                .num_similar(&transformers[j].output().borrow().state);

            assert_eq!(
                overlap, 0,
                "Categories {} and {} should have zero overlap",
                i, j
            );
        }
    }
}

#[test]
fn test_persistence_temporal_tracking() {
    // Test that PersistenceTransformer tracks stability over time
    let mut pt = PersistenceTransformer::new(0.0, 1.0, 2048, 256, 50, 2, 0);

    // Stable period
    pt.set_value(0.5);
    let mut patterns = Vec::new();

    for i in 0..10 {
        pt.execute(false).unwrap();
        if i % 3 == 0 {
            patterns.push(pt.output().borrow().state.clone());
        }
    }

    // Patterns should differ as persistence builds
    for i in 1..patterns.len() {
        assert_ne!(
            patterns[i - 1], patterns[i],
            "Persistence patterns should change over time"
        );
    }
}

#[test]
#[ignore = "TODO: Fix PersistenceTransformer state output, number of active bits seems wrong"]
fn test_mixed_transformer_types() {
    // Real-world scenario: Multi-modal sensor fusion
    let mut temperature = ScalarTransformer::new(15.0, 30.0, 1024, 128, 2, 0);
    let mut weather_type = DiscreteTransformer::new(5, 1024, 2, 0); // sunny, cloudy, rainy, snowy, foggy
    let mut temp_stability = PersistenceTransformer::new(15.0, 30.0, 1024, 128, 100, 2, 0);

    // Sunny day, 22°C, stable
    temperature.set_value(22.0);
    weather_type.set_value(0); // sunny
    temp_stability.set_value(22.0);

    // Build stability
    for _ in 0..20 {
        temperature.execute(false).unwrap();
        weather_type.execute(false).unwrap();
        temp_stability.execute(false).unwrap();
    }

    assert_eq!(temperature.output().borrow().state.num_set(), 128);
    assert_eq!(weather_type.output().borrow().state.num_set(), 204); // 1024/5
    assert_eq!(temp_stability.output().borrow().state.num_set(), 128);
    assert_eq!(temp_stability.get_counter(), 20);

    // Weather changes to rainy
    weather_type.set_value(2); // rainy
    weather_type.execute(false).unwrap();

    // Temperature and stability continue
    temperature.execute(false).unwrap();
    temp_stability.execute(false).unwrap();

    // Weather should have different pattern now
    assert_eq!(weather_type.output().borrow().state.num_set(), 204);
    // Temperature stability should continue building
    assert_eq!(temp_stability.get_counter(), 22);
}

#[test]
fn test_transformer_state_independence() {
    // Verify transformers don't interfere with each other
    let mut t1 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut t2 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    t1.set_value(0.3);
    t2.set_value(0.7);

    t1.execute(false).unwrap();
    t2.execute(false).unwrap();

    // Should be independent
    assert_ne!(t1.output().borrow().state, t2.output().borrow().state);

    // Change t1 shouldn't affect t2
    t1.set_value(0.8);
    t1.execute(false).unwrap();

    let t2_before = t2.output().borrow().state.clone();
    t2.execute(false).unwrap();
    let t2_after = t2.output().borrow().state.clone();

    // t2 should be unchanged (same value)
    assert_eq!(t2_before, t2_after);
}

#[test]
fn test_clear_all_transformers() {
    let mut scalar = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut discrete = DiscreteTransformer::new(10, 1024, 2, 0);
    let mut persistence = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 0);

    // Set and encode
    scalar.set_value(0.5);
    discrete.set_value(5);
    persistence.set_value(0.5);

    for _ in 0..5 {
        scalar.execute(false).unwrap();
        discrete.execute(false).unwrap();
        persistence.execute(false).unwrap();
    }

    // Clear all
    scalar.clear();
    discrete.clear();
    persistence.clear();

    // All should be cleared
    assert_eq!(scalar.output().borrow().state.num_set(), 0);
    assert_eq!(discrete.output().borrow().state.num_set(), 0);
    assert_eq!(persistence.output().borrow().state.num_set(), 0);
    assert_eq!(persistence.get_counter(), 0);
}

#[test]
fn test_time_series_encoding() {
    // Simulate encoding a time series
    let mut scalar = ScalarTransformer::new(0.0, 100.0, 1024, 128, 3, 0);

    let time_series = [10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0];

    for &value in time_series.iter() {
        scalar.set_value(value);
        scalar.execute(false).unwrap();

        // Each encoding should have correct active count
        assert_eq!(scalar.output().borrow().state.num_set(), 128);
    }

    // Can access history
    let output = scalar.output();
    let output_borrow = output.borrow();
    let current = output_borrow.get_bitfield(0);
    let previous = output_borrow.get_bitfield(1);

    assert!(current.num_set() > 0);
    assert!(previous.num_set() > 0);
}

#[test]
fn test_categorical_time_series() {
    // Simulate categorical sequence (e.g., user actions)
    let mut discrete = DiscreteTransformer::new(5, 1024, 3, 0);

    let actions = [0, 1, 2, 1, 3, 4, 2]; // Click, scroll, type, scroll, search, submit, type

    for &action in actions.iter() {
        discrete.set_value(action);
        discrete.execute(false).unwrap();

        assert_eq!(discrete.output().borrow().state.num_set(), 204); // 1024/5
    }
}

#[test]
#[ignore = "TODO: Fix PersistenceTransformer initialization - see ARCHITECTURE_ISSUES.md"]
fn test_stability_detection() {
    // Use persistence transformer to detect stable vs unstable signals
    let mut stable_sensor = PersistenceTransformer::new(0.0, 100.0, 1024, 128, 50, 2, 0);
    let mut noisy_sensor = PersistenceTransformer::new(0.0, 100.0, 1024, 128, 50, 2, 0);

    // Stable signal
    for _ in 0..20 {
        stable_sensor.set_value(50.0);
        stable_sensor.execute(false).unwrap();
    }

    // Noisy signal (oscillates)
    for i in 0..20 {
        let value = if i % 2 == 0 { 30.0 } else { 70.0 };
        noisy_sensor.set_value(value);
        noisy_sensor.execute(false).unwrap();
    }

    // Stable should have high counter
    assert_eq!(stable_sensor.get_counter(), 20);

    // Noisy should have low counter (keeps resetting)
    assert_eq!(noisy_sensor.get_counter(), 0);
}

#[test]
fn test_memory_usage_comparison() {
    let scalar = ScalarTransformer::new(0.0, 1.0, 2048, 256, 3, 0);
    let discrete = DiscreteTransformer::new(10, 2048, 3, 0);
    let persistence = PersistenceTransformer::new(0.0, 1.0, 2048, 256, 100, 3, 0);

    let scalar_mem = scalar.memory_usage();
    let discrete_mem = discrete.memory_usage();
    let persistence_mem = persistence.memory_usage();

    // All should have reasonable memory footprint
    assert!(scalar_mem > 0 && scalar_mem < 20_000);
    assert!(discrete_mem > 0 && discrete_mem < 20_000);
    assert!(persistence_mem > 0 && persistence_mem < 20_000);

    // Should be roughly similar (same output size)
    let max_diff = scalar_mem.max(discrete_mem).max(persistence_mem);
    let min_diff = scalar_mem.min(discrete_mem).min(persistence_mem);

    assert!(
        max_diff < min_diff * 2,
        "Memory usage should be similar across transformers"
    );
}

#[test]
fn test_deterministic_encoding() {
    // Verify all transformers are deterministic
    let mut s1 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 42);
    let mut s2 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 42);

    let mut d1 = DiscreteTransformer::new(10, 1024, 2, 42);
    let mut d2 = DiscreteTransformer::new(10, 1024, 2, 42);

    let mut p1 = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 42);
    let mut p2 = PersistenceTransformer::new(0.0, 1.0, 1024, 128, 100, 2, 42);

    // Same inputs
    s1.set_value(0.5);
    s2.set_value(0.5);
    d1.set_value(5);
    d2.set_value(5);
    p1.set_value(0.5);
    p2.set_value(0.5);

    // Process
    for _ in 0..5 {
        s1.execute(false).unwrap();
        s2.execute(false).unwrap();
        d1.execute(false).unwrap();
        d2.execute(false).unwrap();
        p1.execute(false).unwrap();
        p2.execute(false).unwrap();
    }

    // Should be identical
    assert_eq!(s1.output().borrow().state, s2.output().borrow().state);
    assert_eq!(d1.output().borrow().state, d2.output().borrow().state);
    assert_eq!(p1.output().borrow().state, p2.output().borrow().state);
}

#[test]
fn test_boundary_value_analysis() {
    // Test transformers at boundary values
    let mut scalar = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    // Test boundaries
    for &val in [0.0, 0.001, 0.999, 1.0].iter() {
        scalar.set_value(val);
        scalar.execute(false).unwrap();
        assert_eq!(scalar.output().borrow().state.num_set(), 128);
    }

    let mut discrete = DiscreteTransformer::new(10, 1024, 2, 0);
    discrete.set_value(0);
    discrete.execute(false).unwrap();
    assert_eq!(discrete.output().borrow().state.num_set(), 102);

    discrete.set_value(9);
    discrete.execute(false).unwrap();
    assert_eq!(discrete.output().borrow().state.num_set(), 102);
}

#[test]
fn test_rapid_value_changes() {
    // Test transformers with rapid value changes
    let mut scalar = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    for i in 0..100 {
        let val = (i as f64) / 100.0;
        scalar.set_value(val);
        scalar.execute(false).unwrap();
        assert_eq!(scalar.output().borrow().state.num_set(), 128);
    }
}

#[test]
fn test_complete_workflow() {
    // Complete workflow: sensor reading → encoding → history
    let mut temp_sensor = ScalarTransformer::new(-20.0, 50.0, 2048, 256, 3, 0);
    let mut location = DiscreteTransformer::new(4, 2048, 3, 0); // 4 rooms
    let mut stability = PersistenceTransformer::new(-20.0, 50.0, 2048, 256, 100, 3, 0);

    // Simulate 10 time steps
    let readings = [
        (20.0, 0),
        (20.5, 0),
        (21.0, 0),
        (21.0, 0),
        (21.0, 1),
        (19.0, 1),
        (19.0, 1),
        (19.5, 1),
        (19.5, 2),
        (18.0, 2),
    ];

    for (temp, room) in readings.iter() {
        temp_sensor.set_value(*temp);
        location.set_value(*room);
        stability.set_value(*temp);

        temp_sensor.execute(false).unwrap();
        location.execute(false).unwrap();
        stability.execute(false).unwrap();
    }

    // All should have valid output
    assert_eq!(temp_sensor.output().borrow().state.num_set(), 256);
    assert_eq!(location.output().borrow().state.num_set(), 512); // 2048/4
    assert_eq!(stability.output().borrow().state.num_set(), 256);

    // Can access history
    let output = temp_sensor.output();
    let output_borrow = output.borrow();
    let temp_current = output_borrow.get_bitfield(0);
    let temp_prev = output_borrow.get_bitfield(1);
    assert_ne!(temp_current, temp_prev);
}
