//! Tests for ContextLearner block

use gnomics::blocks::{ContextLearner, DiscreteTransformer};
use gnomics::Block;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_context_learner_new() {
    let learner = ContextLearner::new(
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
fn test_context_learner_init() {
    let input_encoder = DiscreteTransformer::new(10, 512, 2, 0);
    let context_encoder = DiscreteTransformer::new(5, 256, 2, 0);

    let mut learner = ContextLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);

    // Connect inputs
    learner
        .input
        .add_child(Rc::new(RefCell::new(input_encoder.output.clone())), 0);
    learner
        .context
        .add_child(Rc::new(RefCell::new(context_encoder.output.clone())), 0);

    // Initialize
    let result = learner.init();
    assert!(result.is_ok());
}

#[test]
fn test_context_learner_anomaly_score_initial() {
    let learner = ContextLearner::new(10, 2, 4, 16, 8, 20, 2, 1, 2, false, 0);
    assert_eq!(learner.get_anomaly_score(), 0.0);
}

#[test]
fn test_context_learner_historical_count_empty() {
    let learner = ContextLearner::new(10, 2, 4, 16, 8, 20, 2, 1, 2, false, 0);
    assert_eq!(learner.get_historical_count(), 0);
}

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_context_learner_first_exposure_high_anomaly() {
    let mut input_encoder = DiscreteTransformer::new(10, 10, 2, 0);
    let input_out = Rc::new(RefCell::new(input_encoder.output.clone()));
let mut context_encoder = DiscreteTransformer::new(5, 128, 2, 0);
    let context_out = Rc::new(RefCell::new(context_encoder.output.clone()));

    let input_out = Rc::new(RefCell::new(input_encoder.output.clone()));
    let context_out = Rc::new(RefCell::new(context_encoder.output.clone()));

    let mut learner = ContextLearner::new(10, 2, 4, 16, 8, 20, 2, 1, 2, false, 42);

    // Connect
    learner.input.add_child(input_out.clone(), 0);
    learner.context.add_child(context_out.clone(), 0);
    learner.init().unwrap();

    // First exposure should have high anomaly
    input_encoder.set_value(0);
    context_encoder.set_value(0);
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(true).unwrap();

    let anomaly = learner.get_anomaly_score();
    assert!(anomaly > 0.9, "First exposure should have high anomaly, got {}", anomaly);
}

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_context_learner_learning_reduces_anomaly() {
    let mut input_encoder = DiscreteTransformer::new(10, 10, 2, 0);
    let input_out = Rc::new(RefCell::new(input_encoder.output.clone()));
let mut context_encoder = DiscreteTransformer::new(5, 128, 2, 0);
    let context_out = Rc::new(RefCell::new(context_encoder.output.clone()));

    let mut learner = ContextLearner::new(10, 2, 8, 32, 20, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(input_out.clone(), 0);
    learner
        .context
        .add_child(context_out.clone(), 0);
    learner.init().unwrap();

    // Learn association multiple times
    input_encoder.set_value(0);
    context_encoder.set_value(0);

    let mut anomalies = Vec::new();
    for _ in 0..10 {
        input_encoder.execute(false).unwrap();
        context_encoder.execute(false).unwrap();
        learner.execute(true).unwrap();
        anomalies.push(learner.get_anomaly_score());
    }

    // Anomaly should decrease over time
    assert!(anomalies[9] < anomalies[0],
        "Anomaly should decrease with learning: first={:.3}, last={:.3}",
        anomalies[0], anomalies[9]);
}

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_context_learner_different_context_causes_anomaly() {
    let mut input_encoder = DiscreteTransformer::new(10, 10, 2, 0);
    let input_out = Rc::new(RefCell::new(input_encoder.output.clone()));
let mut context_encoder = DiscreteTransformer::new(5, 128, 2, 0);
    let context_out = Rc::new(RefCell::new(context_encoder.output.clone()));

    let mut learner = ContextLearner::new(10, 2, 8, 32, 20, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(input_out.clone(), 0);
    learner
        .context
        .add_child(context_out.clone(), 0);
    learner.init().unwrap();

    // Learn association: input=0 with context=0
    input_encoder.set_value(0);
    context_encoder.set_value(0);
    for _ in 0..10 {
        input_encoder.execute(false).unwrap();
        context_encoder.execute(false).unwrap();
        learner.execute(true).unwrap();
    }
    let learned_anomaly = learner.get_anomaly_score();

    // Test with different context: input=0 with context=1
    context_encoder.set_value(1);
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(false).unwrap(); // No learning
    let novel_context_anomaly = learner.get_anomaly_score();

    assert!(novel_context_anomaly > learned_anomaly,
        "Different context should cause higher anomaly: learned={:.3}, novel={:.3}",
        learned_anomaly, novel_context_anomaly);
}

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_context_learner_historical_count_grows() {
    let mut input_encoder = DiscreteTransformer::new(5, 5, 2, 0);
    let input_out = Rc::new(RefCell::new(input_encoder.output.clone()));
let mut context_encoder = DiscreteTransformer::new(3, 64, 2, 0);
    let context_out = Rc::new(RefCell::new(context_encoder.output.clone()));

    let mut learner = ContextLearner::new(5, 2, 4, 16, 8, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(input_out.clone(), 0);
    learner
        .context
        .add_child(context_out.clone(), 0);
    learner.init().unwrap();

    assert_eq!(learner.get_historical_count(), 0);

    // Learn a pattern
    input_encoder.set_value(0);
    context_encoder.set_value(0);
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(true).unwrap();

    let count1 = learner.get_historical_count();
    assert!(count1 > 0, "Historical count should grow after learning");

    // Learn another pattern
    input_encoder.set_value(1);
    context_encoder.set_value(1);
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(true).unwrap();

    let count2 = learner.get_historical_count();
    assert!(count2 >= count1, "Historical count should not decrease");
}

#[test]
fn test_context_learner_multiple_associations() {
    let mut input_encoder = DiscreteTransformer::new(10, 10, 2, 0);
    let input_out = Rc::new(RefCell::new(input_encoder.output.clone()));
let mut context_encoder = DiscreteTransformer::new(5, 128, 2, 0);
    let context_out = Rc::new(RefCell::new(context_encoder.output.clone()));

    let mut learner = ContextLearner::new(10, 4, 8, 32, 20, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(input_out.clone(), 0);
    learner
        .context
        .add_child(context_out.clone(), 0);
    learner.init().unwrap();

    // Learn multiple input-context associations
    let associations = vec![(0, 0), (1, 1), (2, 2), (3, 0), (4, 1)];

    for _ in 0..5 {
        for &(input_val, context_val) in &associations {
            input_encoder.set_value(input_val);
            context_encoder.set_value(context_val);
            input_encoder.execute(false).unwrap();
            context_encoder.execute(false).unwrap();
            learner.execute(true).unwrap();
        }
    }

    // Test each learned association has low anomaly
    for &(input_val, context_val) in &associations {
        input_encoder.set_value(input_val);
        context_encoder.set_value(context_val);
        input_encoder.execute(false).unwrap();
        context_encoder.execute(false).unwrap();
        learner.execute(false).unwrap();

        let anomaly = learner.get_anomaly_score();
        assert!(anomaly < 0.5,
            "Learned association ({}, {}) should have low anomaly, got {:.3}",
            input_val, context_val, anomaly);
    }
}

#[test]
fn test_context_learner_clear() {
    let mut input_encoder = DiscreteTransformer::new(5, 5, 2, 0);
    let input_out = Rc::new(RefCell::new(input_encoder.output.clone()));
let mut context_encoder = DiscreteTransformer::new(3, 64, 2, 0);
    let context_out = Rc::new(RefCell::new(context_encoder.output.clone()));

    let mut learner = ContextLearner::new(5, 2, 4, 16, 8, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(input_out.clone(), 0);
    learner
        .context
        .add_child(context_out.clone(), 0);
    learner.init().unwrap();

    // Process some data
    input_encoder.set_value(0);
    context_encoder.set_value(0);
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(true).unwrap();

    // Clear
    learner.clear();

    // Check that state is cleared
    assert_eq!(learner.get_anomaly_score(), 0.0);
}

#[test]
fn test_context_learner_memory_usage() {
    let learner = ContextLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);
    let usage = learner.memory_usage();
    assert!(usage > 0, "Memory usage should be non-zero");
    assert!(usage < 10_000_000, "Memory usage should be reasonable (<10MB)");
}

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_context_learner_output_sparse() {
    let mut input_encoder = DiscreteTransformer::new(10, 10, 2, 0);
    let input_out = Rc::new(RefCell::new(input_encoder.output.clone()));
let mut context_encoder = DiscreteTransformer::new(5, 128, 2, 0);
    let context_out = Rc::new(RefCell::new(context_encoder.output.clone()));

    let mut learner = ContextLearner::new(10, 4, 8, 32, 20, 20, 2, 1, 2, false, 42);

    // Connect
    learner
        .input
        .add_child(input_out.clone(), 0);
    learner
        .context
        .add_child(context_out.clone(), 0);
    learner.init().unwrap();

    // Process
    input_encoder.set_value(0);
    context_encoder.set_value(0);
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(true).unwrap();

    // Output should be sparse (some statelets active)
    let num_active = learner.output.borrow().state.num_set();
    let total_statelets = 10 * 4; // num_c * num_spc
    assert!(num_active > 0, "Output should have some active statelets");
    assert!(num_active < total_statelets, "Output should be sparse");
}

#[test]
#[should_panic(expected = "num_c must be > 0")]
fn test_context_learner_zero_columns() {
    ContextLearner::new(0, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);
}

#[test]
#[should_panic(expected = "num_spc must be > 0")]
fn test_context_learner_zero_statelets_per_column() {
    ContextLearner::new(10, 0, 8, 32, 20, 20, 2, 1, 2, false, 0);
}

#[test]
#[should_panic(expected = "d_thresh must be < num_rpd")]
fn test_context_learner_invalid_threshold() {
    ContextLearner::new(10, 4, 8, 32, 32, 20, 2, 1, 2, false, 0);
}

#[test]
#[should_panic(expected = "num_t must be at least 2")]
fn test_context_learner_insufficient_history() {
    ContextLearner::new(10, 4, 8, 32, 20, 20, 2, 1, 1, false, 0);
}
