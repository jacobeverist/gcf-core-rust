//! Tests for SequenceLearner block

use gnomics::blocks::{DiscreteTransformer, SequenceLearner};
use gnomics::{Block};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_sequence_learner_new() {
    let learner = SequenceLearner::new(
        512, // num_c
        4,   // num_spc
        8,   // num_dps
        32,  // num_rpd
        20,  // d_thresh
        20,  // perm_thr
        2,   // perm_inc
        1,   // perm_dec
        2,   // num_t
        false, // always_update
        0,   // seed
    );

    assert_eq!(learner.num_c(), 512);
    assert_eq!(learner.num_spc(), 4);
    assert_eq!(learner.num_dps(), 8);
    assert_eq!(learner.d_thresh(), 20);
}

#[test]
fn test_sequence_learner_self_feedback() {
    let learner = SequenceLearner::new(10, 2, 4, 16, 8, 20, 2, 1, 2, false, 0);
    // Context should have one child (self-feedback to output)
    assert_eq!(learner.context.num_children(), 1);
}

#[test]
fn test_sequence_learner_init() {
    let mut encoder = DiscreteTransformer::new(10, 512, 2, 0);

    let mut learner = SequenceLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);

    // Connect input
    learner
        .input
        .add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);

    // Initialize
    let result = learner.init();
    assert!(result.is_ok());
}

#[test]
fn test_sequence_learner_anomaly_score_initial() {
    let learner = SequenceLearner::new(10, 2, 4, 16, 8, 20, 2, 1, 2, false, 0);
    assert_eq!(learner.get_anomaly_score(), 0.0);
}

#[test]
fn test_sequence_learner_historical_count_empty() {
    let learner = SequenceLearner::new(10, 2, 4, 16, 8, 20, 2, 1, 2, false, 0);
    assert_eq!(learner.get_historical_count(), 0);
}

#[test]
fn test_sequence_learner_first_pattern_high_anomaly() {
    let mut encoder = DiscreteTransformer::new(10, 10, 2, 0);

    let mut learner = SequenceLearner::new(10, 2, 4, 16, 8, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);
    learner.init().unwrap();

    // First pattern should have high anomaly
    encoder.set_value(0);
    encoder.execute(false).unwrap();
    learner.execute(true).unwrap();

    let anomaly = learner.get_anomaly_score();
    assert!(anomaly > 0.9, "First pattern should have high anomaly, got {}", anomaly);
}

#[test]
fn test_sequence_learner_repeated_sequence_reduces_anomaly() {
    let mut encoder = DiscreteTransformer::new(5, 5, 2, 0);

    let mut learner = SequenceLearner::new(5, 2, 8, 32, 20, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);
    learner.init().unwrap();

    // Learn sequence: 0 → 1 → 2 → 0 → 1 → 2
    let sequence = vec![0, 1, 2];

    // Repeat sequence multiple times
    let mut anomalies = Vec::new();
    for _ in 0..10 {
        for &value in &sequence {
            encoder.set_value(value);
            encoder.execute(false).unwrap();
            learner.execute(true).unwrap();
            anomalies.push(learner.get_anomaly_score());
        }
    }

    // Average anomaly should decrease over time
    let early_avg: f64 = anomalies.iter().take(3).sum::<f64>() / 3.0;
    let late_avg: f64 = anomalies.iter().skip(27).take(3).sum::<f64>() / 3.0;

    assert!(late_avg < early_avg,
        "Average anomaly should decrease with learning: early={:.3}, late={:.3}",
        early_avg, late_avg);
}

#[test]
fn test_sequence_learner_broken_sequence_high_anomaly() {
    let mut encoder = DiscreteTransformer::new(5, 5, 2, 0);

    let mut learner = SequenceLearner::new(5, 2, 8, 32, 20, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);
    learner.init().unwrap();

    // Learn sequence: 0 → 1 → 2
    let sequence = vec![0, 1, 2];
    for _ in 0..10 {
        for &value in &sequence {
            encoder.set_value(value);
            encoder.execute(false).unwrap();
            learner.execute(true).unwrap();
        }
    }

    // Test learned sequence (should have low anomaly)
    encoder.set_value(0);
    encoder.execute(false).unwrap();
    learner.execute(false).unwrap();

    encoder.set_value(1);
    encoder.execute(false).unwrap();
    learner.execute(false).unwrap();
    let learned_anomaly = learner.get_anomaly_score();

    // Break sequence: after 0, expect 3 instead of 1
    encoder.set_value(0);
    encoder.execute(false).unwrap();
    learner.execute(false).unwrap();

    encoder.set_value(3);
    encoder.execute(false).unwrap();
    learner.execute(false).unwrap();
    let broken_anomaly = learner.get_anomaly_score();

    assert!(broken_anomaly > learned_anomaly,
        "Broken sequence should have higher anomaly: learned={:.3}, broken={:.3}",
        learned_anomaly, broken_anomaly);
}

#[test]
fn test_sequence_learner_historical_count_grows() {
    let mut encoder = DiscreteTransformer::new(5, 5, 2, 0);

    let mut learner = SequenceLearner::new(5, 2, 4, 16, 8, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);
    learner.init().unwrap();

    assert_eq!(learner.get_historical_count(), 0);

    // Process several patterns
    for value in 0..3 {
        encoder.set_value(value);
        encoder.execute(false).unwrap();
        learner.execute(true).unwrap();
    }

    let count = learner.get_historical_count();
    assert!(count > 0, "Historical count should grow after learning");
}

#[test]
fn test_sequence_learner_complex_sequence() {
    let mut encoder = DiscreteTransformer::new(10, 10, 2, 0);

    let mut learner = SequenceLearner::new(10, 4, 8, 32, 20, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);
    learner.init().unwrap();

    // Learn a longer sequence
    let sequence = vec![0, 1, 2, 3, 4, 5, 4, 3, 2, 1, 0];

    // Train on sequence multiple times
    for _ in 0..20 {
        for &value in &sequence {
            encoder.set_value(value);
            encoder.execute(false).unwrap();
            learner.execute(true).unwrap();
        }
    }

    // Test sequence has low anomaly
    let mut test_anomalies = Vec::new();
    for &value in &sequence {
        encoder.set_value(value);
        encoder.execute(false).unwrap();
        learner.execute(false).unwrap();
        test_anomalies.push(learner.get_anomaly_score());
    }

    let avg_anomaly: f64 = test_anomalies.iter().skip(1).sum::<f64>() / (test_anomalies.len() - 1) as f64;
    assert!(avg_anomaly < 0.3,
        "Learned sequence should have low average anomaly, got {:.3}", avg_anomaly);
}

#[test]
fn test_sequence_learner_clear() {
    let mut encoder = DiscreteTransformer::new(5, 5, 2, 0);

    let mut learner = SequenceLearner::new(5, 2, 4, 16, 8, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);
    learner.init().unwrap();

    // Process some data
    encoder.set_value(0);
    encoder.execute(false).unwrap();
    learner.execute(true).unwrap();

    // Clear
    learner.clear();

    // Check that state is cleared
    assert_eq!(learner.get_anomaly_score(), 0.0);
}

#[test]
fn test_sequence_learner_memory_usage() {
    let learner = SequenceLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);
    let usage = learner.memory_usage();
    assert!(usage > 0, "Memory usage should be non-zero");
    assert!(usage < 10_000_000, "Memory usage should be reasonable (<10MB)");
}

#[test]
fn test_sequence_learner_output_sparse() {
    let mut encoder = DiscreteTransformer::new(10, 10, 2, 0);

    let mut learner = SequenceLearner::new(10, 4, 8, 32, 20, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);
    learner.init().unwrap();

    // Process
    encoder.set_value(0);
    encoder.execute(false).unwrap();
    learner.execute(true).unwrap();

    // Output should be sparse
    let num_active = learner.output.borrow().state.num_set();
    let total_statelets = 10 * 4; // num_c * num_spc
    assert!(num_active > 0, "Output should have some active statelets");
    assert!(num_active < total_statelets, "Output should be sparse");
}

#[test]
fn test_sequence_learner_alternating_patterns() {
    let mut encoder = DiscreteTransformer::new(4, 4, 2, 0);

    let mut learner = SequenceLearner::new(4, 2, 8, 32, 20, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);
    learner.init().unwrap();

    // Learn alternating pattern: 0 → 1 → 0 → 1
    for _ in 0..15 {
        encoder.set_value(0);
        encoder.execute(false).unwrap();
        learner.execute(true).unwrap();

        encoder.set_value(1);
        encoder.execute(false).unwrap();
        learner.execute(true).unwrap();
    }

    // Test learned pattern
    encoder.set_value(0);
    encoder.execute(false).unwrap();
    learner.execute(false).unwrap();

    encoder.set_value(1);
    encoder.execute(false).unwrap();
    learner.execute(false).unwrap();
    let pattern_anomaly = learner.get_anomaly_score();

    // Test incorrect pattern: 0 → 0
    encoder.set_value(0);
    encoder.execute(false).unwrap();
    learner.execute(false).unwrap();

    encoder.set_value(0);
    encoder.execute(false).unwrap();
    learner.execute(false).unwrap();
    let wrong_anomaly = learner.get_anomaly_score();

    assert!(wrong_anomaly > pattern_anomaly,
        "Wrong pattern should have higher anomaly: correct={:.3}, wrong={:.3}",
        pattern_anomaly, wrong_anomaly);
}

#[test]
#[should_panic(expected = "num_c must be > 0")]
fn test_sequence_learner_zero_columns() {
    SequenceLearner::new(0, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);
}

#[test]
#[should_panic(expected = "num_spc must be > 0")]
fn test_sequence_learner_zero_statelets_per_column() {
    SequenceLearner::new(10, 0, 8, 32, 20, 20, 2, 1, 2, false, 0);
}

#[test]
#[should_panic(expected = "d_thresh must be < num_rpd")]
fn test_sequence_learner_invalid_threshold() {
    SequenceLearner::new(10, 4, 8, 32, 32, 20, 2, 1, 2, false, 0);
}

#[test]
#[should_panic(expected = "num_t must be at least 2")]
fn test_sequence_learner_insufficient_history() {
    SequenceLearner::new(10, 4, 8, 32, 20, 20, 2, 1, 1, false, 0);
}
