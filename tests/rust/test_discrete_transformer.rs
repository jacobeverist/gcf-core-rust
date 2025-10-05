//! Comprehensive tests for DiscreteTransformer.
//!
//! Tests cover:
//! - Basic encoding functionality
//! - Semantic properties (distinct patterns for different categories)
//! - Edge cases (boundary categories)
//! - Categorical distinctness verification

use gnomics::{Block, DiscreteTransformer};

#[test]
fn test_discrete_basic_construction() {
    let dt = DiscreteTransformer::new(10, 1024, 2, 0);
    assert_eq!(dt.num_v(), 10);
    assert_eq!(dt.num_s(), 1024);
    assert_eq!(dt.num_as(), 102); // 1024 / 10
}

#[test]
#[should_panic(expected = "num_v must be > 0")]
fn test_discrete_invalid_num_v() {
    DiscreteTransformer::new(0, 1024, 2, 0);
}

#[test]
#[should_panic(expected = "num_s must be > 0")]
fn test_discrete_invalid_num_s() {
    DiscreteTransformer::new(10, 0, 2, 0);
}

#[test]
#[should_panic(expected = "num_t must be at least 2")]
fn test_discrete_invalid_history_depth() {
    DiscreteTransformer::new(10, 1024, 1, 0);
}

#[test]
fn test_discrete_set_get_value() {
    let mut dt = DiscreteTransformer::new(10, 1024, 2, 0);

    dt.set_value(0);
    assert_eq!(dt.get_value(), 0);

    dt.set_value(5);
    assert_eq!(dt.get_value(), 5);

    dt.set_value(9);
    assert_eq!(dt.get_value(), 9);
}

#[test]
#[should_panic(expected = "value must be < num_v")]
fn test_discrete_value_out_of_range() {
    let mut dt = DiscreteTransformer::new(10, 1024, 2, 0);
    dt.set_value(10); // Invalid: should be 0-9
}

#[test]
fn test_discrete_encoding_num_active() {
    let mut dt = DiscreteTransformer::new(4, 1024, 2, 0);

    // Each category should have exactly num_s / num_v active bits
    for cat in 0..4 {
        dt.set_value(cat);
        dt.feedforward(false).unwrap();
        assert_eq!(
            dt.output.state.num_set(),
            256,
            "Category {} should have 256 active bits",
            cat
        );
    }
}

#[test]
fn test_discrete_categories_no_overlap() {
    let mut dt1 = DiscreteTransformer::new(4, 1024, 2, 0);
    let mut dt2 = DiscreteTransformer::new(4, 1024, 2, 0);

    // Test all pairs of different categories
    for cat1 in 0..4 {
        dt1.set_value(cat1);
        dt1.feedforward(false).unwrap();

        for cat2 in 0..4 {
            if cat1 == cat2 {
                continue;
            }

            dt2.set_value(cat2);
            dt2.feedforward(false).unwrap();

            let overlap = dt1.output.state.num_similar(&dt2.output.state);
            assert_eq!(
                overlap, 0,
                "Categories {} and {} should have zero overlap",
                cat1, cat2
            );
        }
    }
}

#[test]
fn test_discrete_same_category_identical() {
    let mut dt1 = DiscreteTransformer::new(8, 1024, 2, 0);
    let mut dt2 = DiscreteTransformer::new(8, 1024, 2, 0);

    for cat in 0..8 {
        dt1.set_value(cat);
        dt1.feedforward(false).unwrap();

        dt2.set_value(cat);
        dt2.feedforward(false).unwrap();

        assert_eq!(
            dt1.output.state, dt2.output.state,
            "Category {} should encode identically",
            cat
        );
    }
}

#[test]
fn test_discrete_all_categories_distinct() {
    let num_v = 16;
    let mut transformers: Vec<DiscreteTransformer> = (0..num_v)
        .map(|_| DiscreteTransformer::new(num_v, 2048, 2, 0))
        .collect();

    // Encode each category
    for (i, dt) in transformers.iter_mut().enumerate() {
        dt.set_value(i);
        dt.feedforward(false).unwrap();
    }

    // Verify all pairs are distinct (no overlap)
    for i in 0..num_v {
        for j in (i + 1)..num_v {
            let overlap = transformers[i]
                .output
                .state
                .num_similar(&transformers[j].output.state);
            assert_eq!(
                overlap, 0,
                "Categories {} and {} should have no overlap",
                i, j
            );
        }
    }
}

#[test]
fn test_discrete_category_zero() {
    let mut dt = DiscreteTransformer::new(10, 1024, 2, 0);

    dt.set_value(0);
    dt.feedforward(false).unwrap();

    assert_eq!(dt.output.state.num_set(), 102);

    let acts = dt.output.state.get_acts();
    // Category 0 should start at or near bit 0
    assert!(
        acts[0] < 10,
        "Category 0 should start near bit 0, got {}",
        acts[0]
    );
}

#[test]
fn test_discrete_last_category() {
    let num_v = 10;
    let mut dt = DiscreteTransformer::new(num_v, 1024, 2, 0);

    dt.set_value(num_v - 1);
    dt.feedforward(false).unwrap();

    assert_eq!(dt.output.state.num_set(), 102);

    let acts = dt.output.state.get_acts();
    // Last category should end at or near bit 1023
    assert!(
        acts[acts.len() - 1] > 1000,
        "Last category should end near bit 1023, got {}",
        acts[acts.len() - 1]
    );
}

#[test]
fn test_discrete_encoding_change_detection() {
    let mut dt = DiscreteTransformer::new(10, 1024, 2, 0);

    dt.set_value(5);
    dt.feedforward(false).unwrap();
    let acts1 = dt.output.state.get_acts();

    // Feedforward again without changing value
    dt.feedforward(false).unwrap();
    let acts2 = dt.output.state.get_acts();

    // Should be identical (optimization check)
    assert_eq!(acts1, acts2, "Repeated encoding should be identical");
}

#[test]
fn test_discrete_binary_choice() {
    let mut dt = DiscreteTransformer::new(2, 1024, 2, 0);

    // Category 0
    dt.set_value(0);
    dt.feedforward(false).unwrap();
    let acts0 = dt.output.state.get_acts();
    assert_eq!(dt.output.state.num_set(), 512);

    // Category 1
    dt.set_value(1);
    dt.feedforward(false).unwrap();
    let acts1 = dt.output.state.get_acts();
    assert_eq!(dt.output.state.num_set(), 512);

    // Verify no overlap
    let overlap = acts0.iter().filter(|&&a| acts1.contains(&a)).count();
    assert_eq!(overlap, 0, "Binary categories should have no overlap");
}

#[test]
fn test_discrete_many_categories() {
    let num_v = 100;
    let mut dt = DiscreteTransformer::new(num_v, 10000, 2, 0);

    // Test a few categories
    for cat in [0, 25, 50, 75, 99].iter() {
        dt.set_value(*cat);
        dt.feedforward(false).unwrap();
        assert_eq!(
            dt.output.state.num_set(),
            100,
            "Category {} should have 100 active bits",
            cat
        );
    }
}

#[test]
fn test_discrete_few_categories() {
    let mut dt = DiscreteTransformer::new(3, 1024, 2, 0);

    // 3 categories: each gets 341 bits (1024 / 3)
    dt.set_value(0);
    dt.feedforward(false).unwrap();
    assert_eq!(dt.output.state.num_set(), 341);

    dt.set_value(1);
    dt.feedforward(false).unwrap();
    assert_eq!(dt.output.state.num_set(), 341);

    dt.set_value(2);
    dt.feedforward(false).unwrap();
    assert_eq!(dt.output.state.num_set(), 341);
}

#[test]
fn test_discrete_clear() {
    let mut dt = DiscreteTransformer::new(10, 1024, 2, 0);

    dt.set_value(7);
    dt.feedforward(false).unwrap();
    assert_eq!(dt.output.state.num_set(), 102);

    dt.clear();

    assert_eq!(dt.output.state.num_set(), 0, "Output should be cleared");
    assert_eq!(dt.get_value(), 0, "Value should reset to 0");
}

#[test]
fn test_discrete_history_tracking() {
    let mut dt = DiscreteTransformer::new(10, 1024, 3, 0);

    // Encode first category
    dt.set_value(3);
    dt.feedforward(false).unwrap();
    let acts1 = dt.output.get_bitarray(0).get_acts();

    // Encode second category
    dt.set_value(7);
    dt.feedforward(false).unwrap();
    let acts2_curr = dt.output.get_bitarray(0).get_acts();
    let acts2_prev = dt.output.get_bitarray(1).get_acts();

    // Current should be different from previous
    assert_ne!(acts2_curr, acts2_prev);
    // Previous should match first encoding
    assert_eq!(acts2_prev, acts1);
}

#[test]
fn test_discrete_memory_usage() {
    let dt = DiscreteTransformer::new(10, 2048, 3, 0);
    let usage = dt.memory_usage();

    // Should be reasonable
    assert!(usage > 0);
    assert!(usage < 10_000, "Memory usage seems too high: {}", usage);
}

#[test]
fn test_discrete_sequential_encodings() {
    let mut dt = DiscreteTransformer::new(5, 1024, 2, 0);

    // Encode sequence
    let sequence = [0, 1, 2, 3, 4, 3, 2, 1, 0];

    for &cat in sequence.iter() {
        dt.set_value(cat);
        dt.feedforward(false).unwrap();
        assert_eq!(
            dt.output.state.num_set(),
            204, // 1024 / 5
            "Each encoding should have correct active count"
        );
    }
}

#[test]
fn test_discrete_coverage_complete() {
    // Verify that all categories together cover the full statelet space
    let num_v = 8;
    let num_s = 1024;
    let mut dt = DiscreteTransformer::new(num_v, num_s, 2, 0);

    let mut all_bits = vec![false; num_s];

    // Mark all bits used by each category
    for cat in 0..num_v {
        dt.set_value(cat);
        dt.feedforward(false).unwrap();

        let acts = dt.output.state.get_acts();
        for &bit in acts.iter() {
            all_bits[bit] = true;
        }
    }

    // Count covered bits
    let covered = all_bits.iter().filter(|&&b| b).count();

    // Should cover most of the space (may not be exact due to integer division)
    assert!(
        covered >= 950,
        "Categories should cover most of statelet space, covered {} of {}",
        covered,
        num_s
    );
}

#[test]
fn test_discrete_single_category() {
    // Edge case: only 1 category
    let mut dt = DiscreteTransformer::new(1, 1024, 2, 0);

    dt.set_value(0);
    dt.feedforward(false).unwrap();

    // All bits should be active
    assert_eq!(dt.output.state.num_set(), 1024);
}

#[test]
fn test_discrete_category_spacing() {
    // Test that categories are evenly spaced
    let num_v = 4;
    let mut transformers: Vec<DiscreteTransformer> = (0..num_v)
        .map(|_| DiscreteTransformer::new(num_v, 1024, 2, 0))
        .collect();

    let mut starts = Vec::new();

    for (i, dt) in transformers.iter_mut().enumerate() {
        dt.set_value(i);
        dt.feedforward(false).unwrap();

        let acts = dt.output.state.get_acts();
        starts.push(acts[0]);
    }

    // Check spacing is roughly equal
    for i in 1..starts.len() {
        let spacing = starts[i] - starts[i - 1];
        // Should be around 256 (1024 / 4)
        assert!(
            spacing >= 200 && spacing <= 300,
            "Category spacing should be ~256, got {}",
            spacing
        );
    }
}

#[test]
fn test_discrete_deterministic() {
    // Same category should always produce same encoding
    let mut dt = DiscreteTransformer::new(10, 1024, 2, 42);

    let mut encodings = Vec::new();

    for _ in 0..5 {
        dt.set_value(5);
        dt.feedforward(false).unwrap();
        encodings.push(dt.output.state.get_acts());
    }

    // All encodings should be identical
    for i in 1..encodings.len() {
        assert_eq!(
            encodings[0], encodings[i],
            "Encoding should be deterministic"
        );
    }
}

#[test]
fn test_discrete_day_of_week_example() {
    // Practical example: encoding day of week
    let mut dow = DiscreteTransformer::new(7, 2048, 2, 0);

    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    for (i, &day) in days.iter().enumerate() {
        dow.set_value(i);
        dow.feedforward(false).unwrap();

        assert_eq!(
            dow.output.state.num_set(),
            292, // 2048 / 7
            "Day {} should have 292 active bits",
            day
        );
    }

    // Verify Mon and Sun have no overlap
    dow.set_value(0);
    dow.feedforward(false).unwrap();
    let mon = dow.output.state.clone();

    dow.set_value(6);
    dow.feedforward(false).unwrap();
    let sun = dow.output.state.clone();

    assert_eq!(
        mon.num_similar(&sun),
        0,
        "Different days should have no overlap"
    );
}
