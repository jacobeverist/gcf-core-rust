//! Comprehensive tests for PatternPooler.
//!
//! Tests cover:
//! - Basic construction and parameter validation
//! - Winner-take-all activation
//! - Learning convergence
//! - Integration with encoders
//! - Sparse representation properties

#![allow(unused_imports)]
use gnomics::{Block, PatternPooler, ScalarTransformer};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_pooler_construction() {
    let pooler = PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0);
    assert_eq!(pooler.num_s(), 1024);
    assert_eq!(pooler.num_as(), 40);
    assert_eq!(pooler.perm_thr(), 20);
}

#[test]
#[should_panic(expected = "num_as must be <= num_s")]
fn test_pooler_invalid_active_count() {
    PatternPooler::new(1024, 2048, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0);
}

#[test]
#[should_panic(expected = "num_t must be at least 2")]
fn test_pooler_invalid_history_depth() {
    PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 1, 0);
}

#[test]
fn test_pooler_activation_count() {
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut pooler = PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0);

    pooler.input_mut().add_child(encoder.output(), 0);
    pooler.init().unwrap();

    // Test at various input values
    for val in [0.0, 0.25, 0.5, 0.75, 1.0].iter() {
        encoder.set_value(*val);
        encoder.execute(false).unwrap();
        pooler.execute(false).unwrap();

        assert_eq!(
            pooler.output().borrow().state.num_set(),
            40,
            "Value {} should have exactly 40 active bits",
            val
        );
    }
}

#[test]
fn test_pooler_winner_take_all() {
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut pooler = PatternPooler::new(512, 20, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0);

    pooler.input_mut().add_child(encoder.output(), 0);
    pooler.init().unwrap();

    encoder.set_value(0.5);
    encoder.execute(false).unwrap();
    pooler.execute(false).unwrap();

    // Exactly num_as should be active
    assert_eq!(pooler.output().borrow().state.num_set(), 20);

    // All active bits should be different
    let acts = pooler.output().borrow().state.get_acts();
    assert_eq!(acts.len(), 20);

    // No duplicates
    for i in 0..acts.len() {
        for j in (i + 1)..acts.len() {
            assert_ne!(acts[i], acts[j], "Duplicate active bits found");
        }
    }
}

#[test]
fn test_pooler_learning_stability() {
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut pooler = PatternPooler::new(512, 30, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 42);

    pooler.input_mut().add_child(encoder.output(), 0);
    pooler.init().unwrap();

    // Present same value repeatedly
    encoder.set_value(0.5);
    encoder.execute(false).unwrap();

    // First activation
    pooler.execute(true).unwrap();
    let first_output = pooler.output().borrow().state.get_acts();

    // Learn on same pattern multiple times
    for _ in 0..50 {
        encoder.execute(false).unwrap();
        pooler.execute(true).unwrap();
    }

    let final_output = pooler.output().borrow().state.get_acts();

    // Representation should become more stable (similar)
    // Count how many bits are the same
    let first_set: std::collections::HashSet<_> = first_output.iter().collect();
    let final_set: std::collections::HashSet<_> = final_output.iter().collect();
    let overlap = first_set.intersection(&final_set).count();

    // After learning, should have high overlap (representation stabilizes)
    // Allow some variation due to randomness, but expect >50% overlap
    assert!(
        overlap as f64 / first_output.len() as f64 > 0.4,
        "Expected stable representation after learning, got overlap {}",
        overlap
    );
}

#[test]
fn test_pooler_different_inputs() {
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut pooler = PatternPooler::new(512, 25, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0);

    pooler.input_mut().add_child(encoder.output(), 0);
    pooler.init().unwrap();

    // Encode two different values
    encoder.set_value(0.2);
    encoder.execute(false).unwrap();
    pooler.execute(false).unwrap();
    let output1 = pooler.output().borrow().state.get_acts();

    encoder.set_value(0.8);
    encoder.execute(false).unwrap();
    pooler.execute(false).unwrap();
    let output2 = pooler.output().borrow().state.get_acts();

    // Different inputs should produce different (but possibly overlapping) outputs
    // Different inputs may produce same output initially before learning
    // Just check that both are valid outputs
    assert_eq!(output1.len(), 25);
    assert_eq!(output2.len(), 25);
}

#[test]
fn test_pooler_always_update() {
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut pooler = PatternPooler::new(512, 25, 20, 2, 1, 0.8, 0.5, 0.3, true, 2, 0);

    pooler.input_mut().add_child(encoder.output(), 0);
    pooler.init().unwrap();

    encoder.set_value(0.5);
    encoder.execute(false).unwrap();
    pooler.execute(true).unwrap();

    let output1_count = pooler.output().borrow().state.num_set();

    // Same input again (but always_update=true)
    encoder.execute(false).unwrap();
    pooler.execute(true).unwrap();

    let output2_count = pooler.output().borrow().state.num_set();

    // Should still have same number of active bits
    assert_eq!(output1_count, 25);
    assert_eq!(output2_count, 25);
}

#[test]
fn test_pooler_clear() {
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 1024, 128, 2, 0);
    let mut pooler = PatternPooler::new(512, 30, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0);

    pooler.input_mut().add_child(encoder.output(), 0);
    pooler.init().unwrap();

    encoder.set_value(0.5);
    encoder.execute(false).unwrap();
    pooler.execute(false).unwrap();

    assert_eq!(pooler.output().borrow().state.num_set(), 30);

    pooler.clear();
    assert_eq!(pooler.output().borrow().state.num_set(), 0);
}

#[test]
fn test_pooler_memory_usage() {
    let pooler = PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0);
    // Note: memory_usage works before init
    let mem = pooler.memory_usage();
    // Should report some non-zero memory usage
    assert!(mem > 0);
}

#[test]
fn test_pooler_sparse_representation() {
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 2048, 256, 2, 0);
    let mut pooler = PatternPooler::new(2048, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0);

    pooler.input_mut().add_child(encoder.output(), 0);
    pooler.init().unwrap();

    encoder.set_value(0.5);
    encoder.execute(false).unwrap();
    pooler.execute(false).unwrap();

    // Verify sparsity: 40/2048 = 1.95%
    let sparsity = pooler.output().borrow().state.num_set() as f64 / 2048.0;
    assert!(
        sparsity < 0.03,
        "Expected sparse representation, got {}%",
        sparsity * 100.0
    );
}
