// Integration tests for execution recording and visualization

use gnomics::{
    blocks::{DiscreteTransformer, SequenceLearner},
    Block, ExecutionTrace, InputAccess, Network, Result,
};

#[test]
fn test_recording_start_stop() -> Result<()> {
    let mut net = Network::new();

    // Create simple network
    let encoder = net.add(DiscreteTransformer::new(5, 256, 2, 0));
    let learner = net.add(SequenceLearner::new(256, 4, 8, 32, 20, 20, 2, 1, 2, false, 0));

    net.connect_to_input(encoder, learner)?;
    net.build()?;
    net.get_mut::<SequenceLearner>(learner)?.init()?;

    // Start recording
    net.start_recording();
    assert!(net.is_recording());

    // Execute a few steps
    for value in 0..5 {
        net.get_mut::<DiscreteTransformer>(encoder)?.set_value(value);
        net.execute(false)?;
    }

    // Stop and get trace
    let trace = net.stop_recording();
    assert!(trace.is_some());

    let trace = trace.unwrap();
    assert_eq!(trace.total_steps, 5);
    assert_eq!(trace.steps.len(), 5);

    Ok(())
}

#[test]
fn test_trace_json_export() -> Result<()> {
    let mut net = Network::new();

    let encoder = net.add(DiscreteTransformer::new(3, 128, 2, 0));
    let learner = net.add(SequenceLearner::new(128, 4, 8, 32, 20, 20, 2, 1, 2, false, 0));

    net.set_block_name(encoder, "TestEncoder");
    net.set_block_name(learner, "TestLearner");

    net.connect_to_input(encoder, learner)?;
    net.build()?;
    net.get_mut::<SequenceLearner>(learner)?.init()?;

    // Record execution
    net.start_recording();

    for value in 0..3 {
        net.get_mut::<DiscreteTransformer>(encoder)?.set_value(value);
        net.execute(false)?;
    }

    let trace = net.stop_recording().unwrap();

    // Export to JSON
    let json = trace.to_json()?;
    assert!(!json.is_empty());

    // Import back
    let imported = ExecutionTrace::from_json(&json)?;
    assert_eq!(imported.total_steps, 3);
    assert_eq!(imported.steps.len(), 3);
    // SequenceLearner has both input connection + self-feedback
    assert!(imported.connections.len() >= 1);

    Ok(())
}

#[test]
fn test_trace_contains_connections() -> Result<()> {
    let mut net = Network::new();

    let encoder = net.add(DiscreteTransformer::new(5, 256, 2, 0));
    let learner = net.add(SequenceLearner::new(256, 4, 8, 32, 20, 20, 2, 1, 2, false, 0));

    net.connect_to_input(encoder, learner)?;
    net.build()?;
    net.get_mut::<SequenceLearner>(learner)?.init()?;

    net.start_recording();
    net.get_mut::<DiscreteTransformer>(encoder)?.set_value(0);
    net.execute(false)?;

    let trace = net.stop_recording().unwrap();

    // Check connections were recorded
    assert!(!trace.connections.is_empty());

    // Should have at least the input connection (plus self-feedback for SequenceLearner)
    assert!(trace.connections.len() >= 1);

    Ok(())
}

#[test]
fn test_trace_contains_block_states() -> Result<()> {
    let mut net = Network::new();

    let encoder = net.add(DiscreteTransformer::new(5, 256, 2, 0));
    net.build()?;

    net.start_recording();

    net.get_mut::<DiscreteTransformer>(encoder)?.set_value(2);
    net.execute(false)?;

    let trace = net.stop_recording().unwrap();

    // Check that we have recorded state
    assert_eq!(trace.steps.len(), 1);

    let step = &trace.steps[0];
    assert!(!step.block_states.is_empty());
    assert!(!step.block_metadata.is_empty());

    // Check block metadata
    for metadata in step.block_metadata.values() {
        assert!(!metadata.name.is_empty());
        assert!(!metadata.block_type.is_empty());
        assert!(metadata.num_statelets > 0);
    }

    Ok(())
}

#[test]
fn test_pause_resume_recording() -> Result<()> {
    let mut net = Network::new();

    let encoder = net.add(DiscreteTransformer::new(10, 256, 2, 0)); // 10 values (0-9)
    net.build()?;

    net.start_recording();

    // Record 2 steps
    for value in 0..2 {
        net.get_mut::<DiscreteTransformer>(encoder)?.set_value(value);
        net.execute(false)?;
    }

    // Pause recording
    net.pause_recording();
    assert!(!net.is_recording()); // recorder exists but is paused (not actively recording)

    // Execute 2 more steps (should not be recorded)
    for value in 2..4 {
        net.get_mut::<DiscreteTransformer>(encoder)?.set_value(value);
        net.execute(false)?;
    }

    // Resume recording
    net.resume_recording();

    // Record 2 more steps
    for value in 4..6 {
        net.get_mut::<DiscreteTransformer>(encoder)?.set_value(value);
        net.execute(false)?;
    }

    let trace = net.stop_recording().unwrap();

    // Should only have 4 steps (2 before pause + 2 after resume)
    assert_eq!(trace.total_steps, 4);
    assert_eq!(trace.steps.len(), 4);

    Ok(())
}

#[test]
fn test_block_naming() {
    let mut net = Network::new();

    let encoder = net.add(DiscreteTransformer::new(5, 256, 2, 0));

    // Default name
    let default_name = net.get_block_name(encoder);
    assert!(default_name.contains("Block_"));

    // Custom name
    net.set_block_name(encoder, "MyCustomEncoder");
    let custom_name = net.get_block_name(encoder);
    assert_eq!(custom_name, "MyCustomEncoder");
}

#[test]
fn test_trace_file_roundtrip() -> Result<()> {
    let mut net = Network::new();

    let encoder = net.add(DiscreteTransformer::new(3, 128, 2, 0));
    net.build()?;

    net.start_recording();

    for value in 0..3 {
        net.get_mut::<DiscreteTransformer>(encoder)?.set_value(value);
        net.execute(false)?;
    }

    let trace = net.stop_recording().unwrap();

    // Write to file
    let filename = "test_trace.json";
    trace.to_json_file(filename)?;

    // Read back from file
    let loaded = ExecutionTrace::from_json_file(filename)?;

    // Verify
    assert_eq!(loaded.total_steps, trace.total_steps);
    assert_eq!(loaded.steps.len(), trace.steps.len());

    // Clean up
    std::fs::remove_file(filename)?;

    Ok(())
}
