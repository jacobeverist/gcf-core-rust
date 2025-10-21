//! Simple direct tests for ContextLearner (without transformer dependencies)
#![allow(unused_imports)]

use gnomics::blocks::ContextLearner;
use gnomics::{Block, BlockOutput, ContextAccess, DiscreteTransformer, InputAccess, OutputAccess};
use std::cell::RefCell;
use std::rc::Rc;
use itertools::Itertools;

#[test]
fn test_context_learner_direct_activation() {
    let mut learner = ContextLearner::new(5, 2, 4, 16, 8, 20, 2, 1, 2, false, 42);

    // Setup dummy inputs directly (MUST setup BEFORE add_child)
    let input_out = Rc::new(RefCell::new(BlockOutput::new()));
    let context_out = Rc::new(RefCell::new(BlockOutput::new()));

    // Setup outputs BEFORE connecting them
    input_out.borrow_mut().setup(2, 5);
    context_out.borrow_mut().setup(2, 128);

    // Now connect them
    learner.input_mut().add_child(input_out.clone(), 0);
    learner.context_mut().add_child(context_out.clone(), 0);

    learner.init().unwrap();

    // Directly set input states
    input_out.borrow_mut().state.set_bit(0);  // Activate column 0
    input_out.borrow_mut().state.set_bit(2);  // Activate column 2
    input_out.borrow_mut().store();

    context_out.borrow_mut().state.set_bit(10);
    context_out.borrow_mut().state.set_bit(20);
    context_out.borrow_mut().store();

    // Run learner
    learner.execute(true).unwrap();

    // Should have high anomaly on first exposure
    let anomaly = learner.get_anomaly_score();
    assert!(anomaly > 0.9, "First exposure should have high anomaly, got {}", anomaly);

    // Should have some output activity
    let num_active = learner.output().borrow().state.num_set();
    assert!(num_active > 0, "Should have output activity");
}

#[test]
fn test_context_learner_learning_works() {
    let mut learner = ContextLearner::new(10, 4, 8, 32, 20, 20, 2, 1, 2, true, 42);

    // Setup outputs BEFORE connecting (critical for proper sizing)
    let input_out = Rc::new(RefCell::new(BlockOutput::new()));
    let context_out = Rc::new(RefCell::new(BlockOutput::new()));

    input_out.borrow_mut().setup(2, 10);
    context_out.borrow_mut().setup(2, 40);


    // let mut input_encoder = DiscreteTransformer::new(10, 10, 2, 0);
    // let mut context_encoder = DiscreteTransformer::new(5, 128, 2, 0);
    // let mut learner = ContextLearner::new(10, 2, 8, 32, 20, 20, 2, 1, 2, true, 42);

    // Connect after setup
    learner.input_mut().add_child(input_out.clone(), 0);
    learner.context_mut().add_child(context_out.clone(), 0);
    learner.init().unwrap();

    // Set pattern
    input_out.borrow_mut().state.set_bit(0);
    input_out.borrow_mut().state.set_bit(1);
    // context_out.borrow_mut().state.set_bit(5);
    // context_out.borrow_mut().state.set_bit(10);

    // Set at least 20 bits for threshold of 20:
    for i in 0..25 {
        context_out.borrow_mut().state.set_bit(i);
    }

    input_out.borrow_mut().store();
    context_out.borrow_mut().store();

    // First exposure - high anomaly
    learner.execute(true).unwrap();
    let first_anomaly = learner.get_anomaly_score();
    let first_count = learner.get_historical_count();
    // println!("{:?}", learner.output().borrow().state.clone().get_bits().iter().format(""));

    // Repeat same pattern multiple times
    for _ in 0..10 {
        learner.step();
        learner.pull();
        learner.compute();
        learner.store();
        learner.learn();
        // println!("{:?}", learner.output().borrow().state.clone().get_bits().iter().format(""));
    }

    let last_anomaly = learner.get_anomaly_score();
    let last_count = learner.get_historical_count();

    // Anomaly should decrease (learning occurred)
    assert!(last_anomaly < first_anomaly,
        "Anomaly should decrease: first={:.3}, last={:.3}", first_anomaly, last_anomaly);

    // Historical count should stay the same since the context is constant (dendrites assigned)
    assert!(last_count == first_count,
        "Historical count should grow: first={}, last={}", first_count, last_count);

    assert!(last_count > 0, "Should have learned some patterns");

}

#[test]
fn test_context_learner_get_anomaly_score() {
    let learner = ContextLearner::new(10, 2, 4, 16, 8, 20, 2, 1, 2, false, 0);
    assert_eq!(learner.get_anomaly_score(), 0.0);
}

#[test]
fn test_context_learner_get_historical_count() {
    let learner = ContextLearner::new(10, 2, 4, 16, 8, 20, 2, 1, 2, false, 0);
    assert_eq!(learner.get_historical_count(), 0);
}
