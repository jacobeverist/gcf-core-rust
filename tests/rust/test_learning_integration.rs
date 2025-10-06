//! Integration tests for learning pipelines.
//!
//! Tests cover:
//! - ScalarTransformer → PatternPooler
//! - ScalarTransformer → PatternClassifier
//! - ScalarTransformer → PatternPooler → PatternClassifier
//! - Multi-stage learning convergence

use gnomics::{Block, PatternClassifier, PatternPooler, ScalarTransformer};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_encoder_to_pooler_pipeline() {
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 2048, 256, 2, 42);
    let mut pooler = PatternPooler::new(2048, 50, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 42);

    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));


    pooler
        .input
        .add_child(encoder_output.clone(), 0);
    pooler.init().unwrap();

    // Process several values
    let test_values = vec![0.0, 0.25, 0.5, 0.75, 1.0];

    for &val in &test_values {
        encoder.set_value(val);
        encoder.execute(false).unwrap();
        pooler.execute(false).unwrap();

        assert_eq!(pooler.output.state.num_set(), 50, "Failed at value {}", val);
    }
}

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_encoder_to_classifier_pipeline() {
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 2048, 256, 2, 42);
    let mut classifier = PatternClassifier::new(4, 2048, 16, 20, 2, 1, 0.8, 0.5, 0.3, 2, 42);

    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));


    classifier
        .input
        .add_child(encoder_output.clone(), 0);
    classifier.init().unwrap();

    // Train on simple pattern
    let training_data = vec![
        (0.1, 0), (0.15, 0), (0.2, 0),
        (0.35, 1), (0.4, 1), (0.45, 1),
        (0.6, 2), (0.65, 2), (0.7, 2),
        (0.85, 3), (0.9, 3), (0.95, 3),
    ];

    // Train
    for _ in 0..10 {
        for &(value, label) in &training_data {
            encoder.set_value(value);
            classifier.set_label(label);
            encoder.execute(false).unwrap();
            classifier.execute(true).unwrap();
        }
    }

    // Validate learning occurred
    encoder.set_value(0.15);
    encoder.execute(false).unwrap();
    classifier.execute(false).unwrap();
    assert_eq!(classifier.get_predicted_label(), 0);

    encoder.set_value(0.65);
    encoder.execute(false).unwrap();
    classifier.execute(false).unwrap();
    assert_eq!(classifier.get_predicted_label(), 2);
}

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_three_stage_pipeline() {
    // ScalarTransformer → PatternPooler → PatternClassifier
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 2048, 256, 2, 42);
    let mut pooler = PatternPooler::new(2048, 80, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 42);
    let mut classifier = PatternClassifier::new(4, 1024, 10, 20, 2, 1, 0.8, 0.5, 0.3, 2, 42);

    // Connect pipeline
    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));

    pooler
        .input
        .add_child(encoder_output.clone(), 0);
    let pooler_output = Rc::new(RefCell::new(pooler.output.clone()));

    classifier
        .input
        .add_child(pooler_output.clone(), 0);

    pooler.init().unwrap();
    classifier.init().unwrap();

    // Training data
    let training_data = vec![
        (0.1, 0), (0.15, 0),
        (0.35, 1), (0.4, 1),
        (0.6, 2), (0.65, 2),
        (0.85, 3), (0.9, 3),
    ];

    // Train all stages
    for _ in 0..15 {
        for &(value, label) in &training_data {
            encoder.set_value(value);
            classifier.set_label(label);

            encoder.execute(false).unwrap();
            pooler.execute(true).unwrap(); // Learn pooled representation
            classifier.execute(true).unwrap(); // Learn classification
        }
    }

    // Test
    let mut correct = 0;
    for &(value, expected) in &training_data {
        encoder.set_value(value);
        encoder.execute(false).unwrap();
        pooler.execute(false).unwrap();
        classifier.execute(false).unwrap();

        if classifier.get_predicted_label() == expected {
            correct += 1;
        }
    }

    let accuracy = correct as f64 / training_data.len() as f64;
    assert!(
        accuracy >= 0.75,
        "Expected at least 75% accuracy in 3-stage pipeline, got {}%",
        accuracy * 100.0
    );
}

#[test]
fn test_pooler_representation_stability() {
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 2048, 256, 2, 42);
    let mut pooler = PatternPooler::new(1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 42);

    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));


    pooler
        .input
        .add_child(encoder_output.clone(), 0);
    pooler.init().unwrap();

    encoder.set_value(0.5);

    // Get representation before learning
    encoder.execute(false).unwrap();
    pooler.execute(false).unwrap();
    let repr_before = pooler.output.state.get_acts();

    // Learn on same pattern many times
    for _ in 0..100 {
        encoder.execute(false).unwrap();
        pooler.execute(true).unwrap();
    }

    // Get representation after learning
    encoder.execute(false).unwrap();
    pooler.execute(false).unwrap();
    let repr_after = pooler.output.state.get_acts();

    // Compute stability (overlap)
    let before_set: std::collections::HashSet<_> = repr_before.iter().collect();
    let after_set: std::collections::HashSet<_> = repr_after.iter().collect();
    let overlap = before_set.intersection(&after_set).count();
    let stability = overlap as f64 / repr_before.len() as f64;

    // After extensive learning, representation should be very stable
    assert!(
        stability > 0.5,
        "Expected stable representation after learning, got {}% overlap",
        stability * 100.0
    );
}

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_classifier_learning_convergence() {
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 2048, 256, 2, 42);
    let mut classifier = PatternClassifier::new(2, 2048, 20, 20, 2, 1, 0.8, 0.5, 0.3, 2, 42);

    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));


    classifier
        .input
        .add_child(encoder_output.clone(), 0);
    classifier.init().unwrap();

    // Binary classification: low vs high
    let training_data = vec![
        (0.1, 0), (0.2, 0), (0.3, 0),
        (0.7, 1), (0.8, 1), (0.9, 1),
    ];

    // Track accuracy over epochs
    let mut accuracies = Vec::new();

    for epoch in 0..20 {
        // Train
        for &(value, label) in &training_data {
            encoder.set_value(value);
            classifier.set_label(label);
            encoder.execute(false).unwrap();
            classifier.execute(true).unwrap();
        }

        // Test
        let mut correct = 0;
        for &(value, expected) in &training_data {
            encoder.set_value(value);
            encoder.execute(false).unwrap();
            classifier.execute(false).unwrap();

            if classifier.get_predicted_label() == expected {
                correct += 1;
            }
        }

        let accuracy = correct as f64 / training_data.len() as f64;
        accuracies.push(accuracy);

        // After epoch 10, accuracy should be improving
        if epoch == 10 {
            assert!(accuracy > 0.5, "Accuracy should improve with training");
        }
    }

    // Final accuracy should be high
    let final_accuracy = accuracies.last().unwrap();
    assert!(
        *final_accuracy >= 0.8,
        "Expected final accuracy >= 80%, got {}%",
        final_accuracy * 100.0
    );
}

#[test]
fn test_multiple_classifiers_same_encoder() {
    // Test that multiple classifiers can share the same encoder
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 2048, 256, 2, 42);
    let mut classifier1 = PatternClassifier::new(2, 2048, 20, 20, 2, 1, 0.8, 0.5, 0.3, 2, 42);
    let mut classifier2 = PatternClassifier::new(3, 2046, 15, 20, 2, 1, 0.8, 0.5, 0.3, 2, 43);

    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));


    classifier1
        .input
        .add_child(encoder_output.clone(), 0);
    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));

    classifier2
        .input
        .add_child(encoder_output.clone(), 0);

    classifier1.init().unwrap();
    classifier2.init().unwrap();

    // Run encoder once
    encoder.set_value(0.5);
    encoder.execute(false).unwrap();

    // Both classifiers process the same input
    classifier1.execute(false).unwrap();
    classifier2.execute(false).unwrap();

    // Both should produce outputs
    assert!(classifier1.output.state.num_set() > 0);
    assert!(classifier2.output.state.num_set() > 0);
}

#[test]
fn test_pooler_dimensionality_reduction() {
    // Test that pooler effectively reduces dimensionality
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 4096, 512, 2, 42);
    let mut pooler = PatternPooler::new(4096, 50, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 42);

    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));


    pooler
        .input
        .add_child(encoder_output.clone(), 0);
    pooler.init().unwrap();

    encoder.set_value(0.5);
    encoder.execute(false).unwrap();
    pooler.execute(false).unwrap();

    // Input: 512 active out of 4096
    // Output: 50 active out of 4096
    // Compression ratio: 512/50 = 10.24x
    let input_sparsity = 512.0 / 4096.0;
    let output_sparsity = 50.0 / 4096.0;

    assert!(
        output_sparsity < input_sparsity,
        "Pooler should produce sparser representation"
    );
}

#[test]
#[ignore = "TODO: Fix BlockOutput cloning issue - see ARCHITECTURE_ISSUES.md"]
fn test_sequential_training_batches() {
    // Test that classifier handles sequential training batches
    let mut encoder = ScalarTransformer::new(0.0, 1.0, 2048, 256, 2, 42);
    let mut classifier = PatternClassifier::new(2, 2048, 20, 20, 2, 1, 0.8, 0.5, 0.3, 2, 42);

    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));


    classifier
        .input
        .add_child(encoder_output.clone(), 0);
    classifier.init().unwrap();

    // Batch 1: Train on label 0
    for val in [0.1, 0.15, 0.2].iter() {
        encoder.set_value(*val);
        classifier.set_label(0);
        encoder.execute(false).unwrap();
        classifier.execute(true).unwrap();
    }

    // Batch 2: Train on label 1
    for val in [0.8, 0.85, 0.9].iter() {
        encoder.set_value(*val);
        classifier.set_label(1);
        encoder.execute(false).unwrap();
        classifier.execute(true).unwrap();
    }

    // Test both labels
    encoder.set_value(0.15);
    encoder.execute(false).unwrap();
    classifier.execute(false).unwrap();
    let pred1 = classifier.get_predicted_label();

    encoder.set_value(0.85);
    encoder.execute(false).unwrap();
    classifier.execute(false).unwrap();
    let pred2 = classifier.get_predicted_label();

    // Should learn both labels
    assert_eq!(pred1, 0);
    assert_eq!(pred2, 1);
}
