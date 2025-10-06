//! Integration tests for temporal blocks (ContextLearner and SequenceLearner)

use gnomics::blocks::{ContextLearner, DiscreteTransformer, SequenceLearner};
use gnomics::Block;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_sequence_learner_multistep_prediction() {
    let mut encoder = DiscreteTransformer::new(5, 5, 2, 0);
    let mut learner = SequenceLearner::new(5, 4, 8, 32, 20, 20, 2, 1, 2, false, 42);

    learner
        .input
        .add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);
    learner.init().unwrap();

    // Learn sequence: 0 → 1 → 2 → 3 → 4
    let sequence = vec![0, 1, 2, 3, 4];

    // Train multiple epochs
    for _ in 0..20 {
        for &value in &sequence {
            encoder.set_value(value);
            encoder.execute(false).unwrap();
            learner.execute(true).unwrap();
        }
    }

    // Test prediction accuracy (after training, transitions should be predicted)
    let mut anomalies = Vec::new();
    for &value in &sequence {
        encoder.set_value(value);
        encoder.execute(false).unwrap();
        learner.execute(false).unwrap();
        anomalies.push(learner.get_anomaly_score());
    }

    // Most transitions should have low anomaly (except first, which has no context)
    let avg_trained = anomalies.iter().skip(1).sum::<f64>() / (anomalies.len() - 1) as f64;
    assert!(
        avg_trained < 0.3,
        "Trained sequence should have low anomaly, got {:.3}",
        avg_trained
    );

    // Test novel sequence
    let novel_sequence = vec![0, 2, 4, 1, 3]; // Different order
    let mut novel_anomalies = Vec::new();
    for &value in &novel_sequence {
        encoder.set_value(value);
        encoder.execute(false).unwrap();
        learner.execute(false).unwrap();
        novel_anomalies.push(learner.get_anomaly_score());
    }

    let avg_novel = novel_anomalies.iter().skip(1).sum::<f64>() / (novel_anomalies.len() - 1) as f64;
    assert!(
        avg_novel > avg_trained,
        "Novel sequence should have higher anomaly: trained={:.3}, novel={:.3}",
        avg_trained,
        avg_novel
    );
}

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_context_learner_with_multiple_contexts() {
    let mut input_encoder = DiscreteTransformer::new(10, 10, 2, 0);
    let mut context_encoder = DiscreteTransformer::new(5, 128, 2, 0);
    let mut learner = ContextLearner::new(10, 4, 8, 32, 20, 20, 2, 1, 2, false, 42);

    learner
        .input
        .add_child(Rc::new(RefCell::new(input_encoder.output.clone())), 0);
    learner
        .context
        .add_child(Rc::new(RefCell::new(context_encoder.output.clone())), 0);
    learner.init().unwrap();

    // Learn multiple context-dependent patterns
    // Context 0: inputs 0, 1, 2
    // Context 1: inputs 3, 4, 5
    // Context 2: inputs 6, 7, 8
    let associations = vec![
        (0, 0),
        (1, 0),
        (2, 0),
        (3, 1),
        (4, 1),
        (5, 1),
        (6, 2),
        (7, 2),
        (8, 2),
    ];

    // Train
    for _ in 0..10 {
        for &(input_val, context_val) in &associations {
            input_encoder.set_value(input_val);
            context_encoder.set_value(context_val);
            input_encoder.execute(false).unwrap();
            context_encoder.execute(false).unwrap();
            learner.execute(true).unwrap();
        }
    }

    // Test correct associations have low anomaly
    for &(input_val, context_val) in &associations {
        input_encoder.set_value(input_val);
        context_encoder.set_value(context_val);
        input_encoder.execute(false).unwrap();
        context_encoder.execute(false).unwrap();
        learner.execute(false).unwrap();

        let anomaly = learner.get_anomaly_score();
        assert!(
            anomaly < 0.5,
            "Correct association ({}, {}) should have low anomaly, got {:.3}",
            input_val,
            context_val,
            anomaly
        );
    }

    // Test incorrect associations have higher anomaly
    // Input 0 with context 1 (should be context 0)
    input_encoder.set_value(0);
    context_encoder.set_value(1);
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(false).unwrap();
    let wrong_anomaly = learner.get_anomaly_score();

    assert!(
        wrong_anomaly > 0.7,
        "Incorrect context should have high anomaly, got {:.3}",
        wrong_anomaly
    );
}

#[test]
fn test_sequence_learner_cyclic_pattern() {
    let mut encoder = DiscreteTransformer::new(4, 4, 2, 0);
    let mut learner = SequenceLearner::new(4, 4, 8, 32, 20, 20, 2, 1, 2, false, 42);

    learner
        .input
        .add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);
    learner.init().unwrap();

    // Learn cyclic pattern: 0 → 1 → 2 → 3 → 0 → 1 → ...
    let cycle = vec![0, 1, 2, 3];

    // Train multiple full cycles
    for _ in 0..20 {
        for &value in &cycle {
            encoder.set_value(value);
            encoder.execute(false).unwrap();
            learner.execute(true).unwrap();
        }
    }

    // Test that cycle is learned (including 3 → 0 transition)
    let mut test_anomalies = Vec::new();
    for _ in 0..2 {
        // Test 2 full cycles
        for &value in &cycle {
            encoder.set_value(value);
            encoder.execute(false).unwrap();
            learner.execute(false).unwrap();
            test_anomalies.push(learner.get_anomaly_score());
        }
    }

    let avg_cycle_anomaly: f64 = test_anomalies.iter().skip(1).sum::<f64>() / (test_anomalies.len() - 1) as f64;
    assert!(
        avg_cycle_anomaly < 0.3,
        "Cyclic pattern should be well learned, got {:.3}",
        avg_cycle_anomaly
    );
}

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_context_learner_disambiguation() {
    // Test that context helps disambiguate same input with different meanings
    let mut input_encoder = DiscreteTransformer::new(5, 5, 2, 0);
    let mut context_encoder = DiscreteTransformer::new(5, 128, 2, 0);
    let mut learner = ContextLearner::new(5, 4, 8, 32, 20, 20, 2, 1, 2, false, 42);

    learner
        .input
        .add_child(Rc::new(RefCell::new(input_encoder.output.clone())), 0);
    learner
        .context
        .add_child(Rc::new(RefCell::new(context_encoder.output.clone())), 0);
    learner.init().unwrap();

    // Same input (0) appears in two different contexts (0 and 1)
    let associations = vec![(0, 0), (0, 1), (1, 0), (2, 1)];

    // Train
    for _ in 0..15 {
        for &(input_val, context_val) in &associations {
            input_encoder.set_value(input_val);
            context_encoder.set_value(context_val);
            input_encoder.execute(false).unwrap();
            context_encoder.execute(false).unwrap();
            learner.execute(true).unwrap();
        }
    }

    // Input 0 with context 0 should be learned
    input_encoder.set_value(0);
    context_encoder.set_value(0);
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(false).unwrap();
    let anomaly_c0 = learner.get_anomaly_score();

    // Input 0 with context 1 should also be learned
    input_encoder.set_value(0);
    context_encoder.set_value(1);
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(false).unwrap();
    let anomaly_c1 = learner.get_anomaly_score();

    // Both should have low anomaly
    assert!(
        anomaly_c0 < 0.5,
        "Input 0 with context 0 should be learned, got {:.3}",
        anomaly_c0
    );
    assert!(
        anomaly_c1 < 0.5,
        "Input 0 with context 1 should be learned, got {:.3}",
        anomaly_c1
    );

    // Input 0 with wrong context (2) should have high anomaly
    input_encoder.set_value(0);
    context_encoder.set_value(2);
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(false).unwrap();
    let anomaly_wrong = learner.get_anomaly_score();

    assert!(
        anomaly_wrong > 0.7,
        "Input 0 with novel context 2 should have high anomaly, got {:.3}",
        anomaly_wrong
    );
}

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_sequence_learner_branching_sequences() {
    let mut encoder = DiscreteTransformer::new(8, 8, 2, 0);
    let mut learner = SequenceLearner::new(8, 4, 8, 32, 20, 20, 2, 1, 2, false, 42);

    learner
        .input
        .add_child(Rc::new(RefCell::new(encoder.output.clone())), 0);
    learner.init().unwrap();

    // Learn two branching sequences from same start:
    // 0 → 1 → 2 → 3
    // 0 → 1 → 4 → 5
    let seq1 = vec![0, 1, 2, 3];
    let seq2 = vec![0, 1, 4, 5];

    // Train both sequences
    for _ in 0..15 {
        for &value in &seq1 {
            encoder.set_value(value);
            encoder.execute(false).unwrap();
            learner.execute(true).unwrap();
        }
        for &value in &seq2 {
            encoder.set_value(value);
            encoder.execute(false).unwrap();
            learner.execute(true).unwrap();
        }
    }

    // Both sequences should be learned (after 0 → 1, both 2 and 4 are valid)
    // Test sequence 1
    for &value in &seq1 {
        encoder.set_value(value);
        encoder.execute(false).unwrap();
        learner.execute(false).unwrap();
    }
    // Don't check individual anomalies since branching creates ambiguity

    // Test sequence 2
    for &value in &seq2 {
        encoder.set_value(value);
        encoder.execute(false).unwrap();
        learner.execute(false).unwrap();
    }

    // Novel sequence should have higher anomaly: 0 → 1 → 6 → 7
    let novel_seq = vec![0, 1, 6, 7];
    let mut novel_anomalies = Vec::new();
    for &value in &novel_seq {
        encoder.set_value(value);
        encoder.execute(false).unwrap();
        learner.execute(false).unwrap();
        novel_anomalies.push(learner.get_anomaly_score());
    }

    // Check that the novel branch (6 after 1) has high anomaly
    assert!(
        novel_anomalies[2] > 0.5,
        "Novel branch should have high anomaly, got {:.3}",
        novel_anomalies[2]
    );
}

#[test]
#[ignore = "TODO: Memory usage calculation needs review - see ARCHITECTURE_ISSUES.md"]
fn test_temporal_blocks_memory_efficiency() {
    // Test that temporal blocks don't use excessive memory
    let context_learner = ContextLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);
    let sequence_learner = SequenceLearner::new(512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0);

    let context_usage = context_learner.memory_usage();
    let sequence_usage = sequence_learner.memory_usage();

    // Both should use similar memory
    let diff = (context_usage as f64 - sequence_usage as f64).abs();
    let avg = (context_usage + sequence_usage) as f64 / 2.0;
    let relative_diff = diff / avg;

    assert!(
        relative_diff < 0.5,
        "Memory usage should be similar: context={}, sequence={}",
        context_usage,
        sequence_usage
    );

    // Should be under 2MB for this configuration
    assert!(
        context_usage < 2_000_000,
        "Context learner memory should be < 2MB, got {}",
        context_usage
    );
    assert!(
        sequence_usage < 2_000_000,
        "Sequence learner memory should be < 2MB, got {}",
        sequence_usage
    );
}
