//! Comprehensive tests for ScalarTransformer.
//!
//! Tests cover:
//! - Basic encoding functionality
//! - Semantic properties (overlapping patterns for similar values)
//! - Edge cases (boundaries, clamping)
//! - Performance characteristics

use gnomics::{Block, ScalarTransformer};

#[test]
fn test_scalar_basic_construction() {
    let st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    assert_eq!(st.min_val(), 0.0);
    assert_eq!(st.max_val(), 1.0);
    assert_eq!(st.num_s(), 1024);
    assert_eq!(st.num_as(), 128);
}

#[test]
#[should_panic(expected = "max_val must be greater than min_val")]
fn test_scalar_invalid_range() {
    ScalarTransformer::new(1.0, 0.0, 1024, 128, 2, 0);
}

#[test]
#[should_panic(expected = "num_as must be <= num_s")]
fn test_scalar_invalid_active_count() {
    ScalarTransformer::new(0.0, 1.0, 1024, 2048, 2, 0);
}

#[test]
#[should_panic(expected = "num_t must be at least 2")]
fn test_scalar_invalid_history_depth() {
    ScalarTransformer::new(0.0, 1.0, 1024, 128, 1, 0);
}

#[test]
fn test_scalar_set_get_value() {
    let mut st = ScalarTransformer::new(0.0, 100.0, 1024, 128, 2, 0);

    st.set_value(50.0);
    assert_eq!(st.get_value(), 50.0);

    st.set_value(75.5);
    assert_eq!(st.get_value(), 75.5);

    st.set_value(0.0);
    assert_eq!(st.get_value(), 0.0);
}

#[test]
fn test_scalar_value_clamping() {
    let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    // Below minimum
    st.set_value(-0.5);
    assert_eq!(st.get_value(), 0.0);

    // Above maximum
    st.set_value(1.5);
    assert_eq!(st.get_value(), 1.0);

    // Exactly at boundaries
    st.set_value(0.0);
    assert_eq!(st.get_value(), 0.0);

    st.set_value(1.0);
    assert_eq!(st.get_value(), 1.0);
}

#[test]
fn test_scalar_encoding_num_active() {
    let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    // Test at various values
    for val in [0.0, 0.25, 0.5, 0.75, 1.0].iter() {
        st.set_value(*val);
        st.execute(false).unwrap();
        assert_eq!(
            st.get_output().borrow().state.num_set(),
            128,
            "Value {} should have 128 active bits",
            val
        );
    }
}

#[test]
fn test_scalar_encoding_minimum_value() {
    let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    st.set_value(0.0);
    st.execute(false).unwrap();

    assert_eq!(st.get_output().borrow().state.num_set(), 128);

    // Should start at bit 0
    let acts = st.get_output().borrow().state.get_acts();
    assert_eq!(acts[0], 0, "Minimum value should start at bit 0");
    assert_eq!(acts[127], 127, "Should activate first 128 bits");
}

#[test]
fn test_scalar_encoding_maximum_value() {
    let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    st.set_value(1.0);
    st.execute(false).unwrap();

    assert_eq!(st.get_output().borrow().state.num_set(), 128);

    // Should end at bit 1023
    let acts = st.get_output().borrow().state.get_acts();
    assert_eq!(acts[0], 896, "Maximum value should start at bit 896");
    assert_eq!(
        acts[acts.len() - 1],
        1023,
        "Maximum value should end at bit 1023"
    );
}

#[test]
fn test_scalar_encoding_midpoint() {
    let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    st.set_value(0.5);
    st.execute(false).unwrap();

    assert_eq!(st.get_output().borrow().state.num_set(), 128);

    let acts = st.get_output().borrow().state.get_acts();
    // Midpoint should activate bits around position 448 ((1024-128)/2)
    assert!(
        acts[0] >= 400 && acts[0] <= 500,
        "Midpoint should start around bit 448, got {}",
        acts[0]
    );
}

#[test]
fn test_scalar_semantic_similarity_close_values() {
    let mut st1 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut st2 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    // Very similar values (1% apart)
    st1.set_value(0.50);
    st1.execute(false).unwrap();

    st2.set_value(0.51);
    st2.execute(false).unwrap();

    // Should have high overlap
    let overlap = st1.get_output().borrow().state.num_similar(&st2.get_output().borrow().state);
    assert!(
        overlap > 100,
        "Similar values (0.50 vs 0.51) should have >100 overlapping bits, got {}",
        overlap
    );

    // Calculate overlap percentage
    let overlap_pct = (overlap as f64) / 128.0;
    assert!(
        overlap_pct > 0.75,
        "Similar values should have >75% overlap, got {:.1}%",
        overlap_pct * 100.0
    );
}

#[test]
fn test_scalar_semantic_similarity_distant_values() {
    let mut st1 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut st2 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    // Very different values
    st1.set_value(0.0);
    st1.execute(false).unwrap();

    st2.set_value(1.0);
    st2.execute(false).unwrap();

    // Should have minimal overlap
    let overlap = st1.get_output().borrow().state.num_similar(&st2.get_output().borrow().state);
    assert!(
        overlap < 20,
        "Distant values (0.0 vs 1.0) should have <20 overlapping bits, got {}",
        overlap
    );
}

#[test]
#[ignore = "TODO: Fix floating-point precision in semantic similarity - see ARCHITECTURE_ISSUES.md"]
fn test_scalar_semantic_similarity_gradient() {
    // Test that overlap decreases as values become more distant
    let mut st_base = ScalarTransformer::new(0.0, 1.0, 2048, 256, 2, 0);
    st_base.set_value(0.5);
    st_base.execute(false).unwrap();

    let test_values = [0.5, 0.51, 0.55, 0.6, 0.7, 0.8, 1.0];
    let mut overlaps = Vec::new();

    for &val in test_values.iter() {
        let mut st = ScalarTransformer::new(0.0, 1.0, 2048, 256, 2, 0);
        st.set_value(val);
        st.execute(false).unwrap();

        let overlap = st_base.get_output().borrow().state.num_similar(&st.get_output().borrow().state);
        overlaps.push(overlap);
    }

    // Verify overlap generally decreases with distance
    assert_eq!(overlaps[0], 256, "Identical values should have 100% overlap");
    assert!(
        overlaps[1] > overlaps[2],
        "Closer values should have more overlap"
    );
    assert!(
        overlaps[2] > overlaps[4],
        "Overlap should decrease with distance"
    );
    assert!(
        overlaps[4] > overlaps[6],
        "Overlap should continue decreasing"
    );
}

#[test]
fn test_scalar_encoding_change_detection() {
    let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    st.set_value(0.5);
    st.execute(false).unwrap();
    let acts1 = st.get_output().borrow().state.get_acts();

    // Feedforward again without changing value
    st.execute(false).unwrap();
    let acts2 = st.get_output().borrow().state.get_acts();

    // Should be identical (optimization check)
    assert_eq!(acts1, acts2, "Repeated encoding should be identical");
}

#[test]
fn test_scalar_different_ranges() {
    // Test temperature range
    let mut temp = ScalarTransformer::new(0.0, 100.0, 1024, 128, 2, 0);
    temp.set_value(50.0);
    temp.execute(false).unwrap();
    assert_eq!(temp.get_output().borrow().state.num_set(), 128);

    // Test negative range
    let mut neg = ScalarTransformer::new(-10.0, 10.0, 1024, 128, 2, 0);
    neg.set_value(0.0);
    neg.execute(false).unwrap();
    assert_eq!(neg.get_output().borrow().state.num_set(), 128);

    // Test very small range
    let mut small = ScalarTransformer::new(0.0, 0.1, 1024, 128, 2, 0);
    small.set_value(0.05);
    small.execute(false).unwrap();
    assert_eq!(small.get_output().borrow().state.num_set(), 128);
}

#[test]
fn test_scalar_clear() {
    let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    st.set_value(0.75);
    st.execute(false).unwrap();
    assert_eq!(st.get_output().borrow().state.num_set(), 128);

    st.clear();

    assert_eq!(st.get_output().borrow().state.num_set(), 0, "Output should be cleared");
    assert_eq!(st.get_value(), st.min_val(), "Value should reset to minimum");
}

#[test]
fn test_scalar_history_tracking() {
    let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 3, 0);

    // Encode first value
    st.set_value(0.3);
    st.execute(false).unwrap();
    let acts1 = {
        let output = st.get_output();
        let output_borrow = output.borrow();
        output_borrow.get_bitfield(0).get_acts()
    };

    // Encode second value
    st.set_value(0.7);
    st.execute(false).unwrap();
    let (acts2_curr, acts2_prev) = {
        let output = st.get_output();
        let output_borrow = output.borrow();
        (output_borrow.get_bitfield(0).get_acts(), output_borrow.get_bitfield(1).get_acts())
    };

    // Current should be different from previous
    assert_ne!(acts2_curr, acts2_prev);
    // Previous should match first encoding
    assert_eq!(acts2_prev, acts1);
}

#[test]
fn test_scalar_memory_usage() {
    let st = ScalarTransformer::new(0.0, 1.0, 2048, 256, 3, 0);
    let usage = st.memory_usage();

    // Should be reasonable (bitvec has more overhead than raw Vec<u32>)
    // 2048 bits * 3 time steps = 6144 bits = 768 bytes minimum
    // With bitvec overhead, expect ~40KB
    assert!(usage > 0);
    assert!(usage < 100_000, "Memory usage seems too high: {}", usage);
    assert!(usage > 1_000, "Memory usage seems too low: {}", usage);
}

#[test]
fn test_scalar_multiple_encodings() {
    let mut st = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    let test_sequence = [0.0, 0.25, 0.5, 0.75, 1.0, 0.5, 0.0];

    for &val in test_sequence.iter() {
        st.set_value(val);
        st.execute(false).unwrap();
        assert_eq!(
            st.get_output().borrow().state.num_set(),
            128,
            "Each encoding should have 128 active bits"
        );
    }
}

#[test]
fn test_scalar_same_value_identical_encoding() {
    let mut st1 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut st2 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    st1.set_value(0.42);
    st1.execute(false).unwrap();

    st2.set_value(0.42);
    st2.execute(false).unwrap();

    // Identical values should produce identical encodings
    assert_eq!(
        st1.get_output().borrow().state, st2.get_output().borrow().state,
        "Same value should produce identical encoding"
    );
}

#[test]
#[ignore = "TODO: Fix floating-point precision in semantic similarity - see ARCHITECTURE_ISSUES.md"]
fn test_scalar_precision() {
    let mut st1 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut st2 = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);

    // Very close values (within floating point precision)
    st1.set_value(0.123456789);
    st1.execute(false).unwrap();

    st2.set_value(0.123456788);
    st2.execute(false).unwrap();

    // Should still have very high overlap
    let overlap = st1.get_output().borrow().state.num_similar(&st2.get_output().borrow().state);
    assert!(
        overlap > 120,
        "Tiny differences should still have very high overlap"
    );
}

#[test]
fn test_scalar_large_statelet_count() {
    let mut st = ScalarTransformer::new(0.0, 1.0, 4096, 512, 2, 0);

    st.set_value(0.5);
    st.execute(false).unwrap();

    assert_eq!(st.get_output().borrow().state.num_set(), 512);
}

#[test]
fn test_scalar_small_active_percentage() {
    // Test with very sparse representation (2.5% active)
    let mut st = ScalarTransformer::new(0.0, 1.0, 4096, 100, 2, 0);

    st.set_value(0.5);
    st.execute(false).unwrap();

    assert_eq!(st.get_output().borrow().state.num_set(), 100);
}
