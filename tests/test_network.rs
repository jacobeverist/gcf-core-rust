//! Integration tests for Network architecture.
//!
//! Tests the Network struct with real blocks to verify:
//! - Block management
//! - Dependency resolution
//! - Execution ordering
//! - Type-safe access

use gnomics::{
    blocks::{DiscreteTransformer, PatternClassifier, PatternPooler, ScalarTransformer},
    Block, InputAccess, Network, OutputAccess, Result,
};

#[test]
fn test_network_simple_pipeline() -> Result<()> {
    // Create network with encoder -> pooler
    let mut net = Network::new();

    let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
    let pooler = net.add(PatternPooler::new(
        1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0,
    ));

    // Connect outputs to inputs (dependencies auto-discovered)
    {
        let enc_out = net.get::<ScalarTransformer>(encoder)?.output();
        net.get_mut::<PatternPooler>(pooler)?
            .input_mut()
            .add_child(enc_out, 0);
    }

    // Build and initialize
    net.build()?;
    net.get_mut::<PatternPooler>(pooler)?.init()?;

    // Execute
    net.get_mut::<ScalarTransformer>(encoder)?.set_value(42.0);
    net.execute(false)?;

    // Verify output
    let output = net.get::<PatternPooler>(pooler)?.output();
    assert!(output.borrow().state.num_set() > 0);

    Ok(())
}

#[test]
fn test_network_three_stage_pipeline() -> Result<()> {
    // Create encoder -> pooler -> classifier
    let mut net = Network::new();

    let encoder = net.add(ScalarTransformer::new(0.0, 10.0, 2048, 256, 2, 0));
    let pooler = net.add(PatternPooler::new(
        1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0,
    ));
    let classifier = net.add(PatternClassifier::new(
        3, 1023, 20, 20, 2, 1, 0.8, 0.5, 0.3, 2, 0,  // 1023 is divisible by 3
    ));

    // Connect outputs to inputs (dependencies auto-discovered)
    {
        let enc_out = net.get::<ScalarTransformer>(encoder)?.output();
        net.get_mut::<PatternPooler>(pooler)?
            .input_mut()
            .add_child(enc_out, 0);

        let pool_out = net.get::<PatternPooler>(pooler)?.output();
        net.get_mut::<PatternClassifier>(classifier)?
            .input_mut()
            .add_child(pool_out, 0);
    }

    // Build execution plan
    net.build()?;

    // Initialize
    net.get_mut::<PatternPooler>(pooler)?.init()?;
    net.get_mut::<PatternClassifier>(classifier)?.init()?;

    // Verify execution order
    let order = net.execution_order();
    assert_eq!(order.len(), 3);
    assert_eq!(order[0], encoder);
    assert_eq!(order[1], pooler);
    assert_eq!(order[2], classifier);

    // Execute training
    for value in [1.0, 2.0, 3.0, 1.5, 2.5, 3.5] {
        net.get_mut::<ScalarTransformer>(encoder)?.set_value(value);
        let label = if value < 2.0 {
            0
        } else if value < 3.0 {
            1
        } else {
            2
        };
        net.get_mut::<PatternClassifier>(classifier)?
            .set_label(label);
        net.execute(true)?;
    }

    // Test inference
    net.get_mut::<ScalarTransformer>(encoder)?.set_value(2.2);
    net.execute(false)?;

    let probs = net.get::<PatternClassifier>(classifier)?.get_probabilities();
    assert_eq!(probs.len(), 3);

    Ok(())
}

#[test]
fn test_network_multiple_inputs() -> Result<()> {
    // Create two encoders feeding into one pooler
    let mut net = Network::new();

    let encoder1 = net.add(ScalarTransformer::new(0.0, 100.0, 1024, 128, 2, 0));
    let encoder2 = net.add(ScalarTransformer::new(0.0, 100.0, 1024, 128, 2, 1));
    let pooler = net.add(PatternPooler::new(
        1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0,
    ));

    // Connect outputs to inputs (dependencies auto-discovered)
    {
        let enc1_out = net.get::<ScalarTransformer>(encoder1)?.output();
        let enc2_out = net.get::<ScalarTransformer>(encoder2)?.output();
        let pooler_input = net.get_mut::<PatternPooler>(pooler)?.input_mut();
        pooler_input.add_child(enc1_out, 0);
        pooler_input.add_child(enc2_out, 0);
    }

    net.build()?;
    net.get_mut::<PatternPooler>(pooler)?.init()?;

    // Verify execution order: both encoders before pooler
    let order = net.execution_order();
    assert_eq!(order.len(), 3);

    let pooler_pos = order.iter().position(|&x| x == pooler).unwrap();
    let enc1_pos = order.iter().position(|&x| x == encoder1).unwrap();
    let enc2_pos = order.iter().position(|&x| x == encoder2).unwrap();

    assert!(enc1_pos < pooler_pos);
    assert!(enc2_pos < pooler_pos);

    // Execute
    net.get_mut::<ScalarTransformer>(encoder1)?.set_value(42.0);
    net.get_mut::<ScalarTransformer>(encoder2)?.set_value(58.0);
    net.execute(false)?;

    Ok(())
}

#[test]
fn test_network_diamond_dependency() -> Result<()> {
    // Create diamond pattern:
    //     encoder
    //     /     \
    // pooler1  pooler2
    //     \     /
    //   classifier

    let mut net = Network::new();

    let encoder = net.add(ScalarTransformer::new(0.0, 10.0, 2048, 256, 2, 0));
    let pooler1 = net.add(PatternPooler::new(
        512, 20, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0,
    ));
    let pooler2 = net.add(PatternPooler::new(
        512, 20, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 1,
    ));
    let classifier = net.add(PatternClassifier::new(
        2, 1024, 20, 20, 2, 1, 0.8, 0.5, 0.3, 2, 0,
    ));

    // Connect outputs to inputs (dependencies auto-discovered)
    {
        let enc_out = net.get::<ScalarTransformer>(encoder)?.output();

        net.get_mut::<PatternPooler>(pooler1)?
            .input_mut()
            .add_child(enc_out.clone(), 0);

        net.get_mut::<PatternPooler>(pooler2)?
            .input_mut()
            .add_child(enc_out, 0);

        let pool1_out = net.get::<PatternPooler>(pooler1)?.output();
        let pool2_out = net.get::<PatternPooler>(pooler2)?.output();

        let class_input = net.get_mut::<PatternClassifier>(classifier)?.input_mut();
        class_input.add_child(pool1_out, 0);
        class_input.add_child(pool2_out, 0);
    }

    net.build()?;

    // Initialize
    net.get_mut::<PatternPooler>(pooler1)?.init()?;
    net.get_mut::<PatternPooler>(pooler2)?.init()?;
    net.get_mut::<PatternClassifier>(classifier)?.init()?;

    // Verify execution order
    let order = net.execution_order();
    assert_eq!(order.len(), 4);

    // Encoder must be first
    assert_eq!(order[0], encoder);

    // Poolers must come before classifier
    let pool1_pos = order.iter().position(|&x| x == pooler1).unwrap();
    let pool2_pos = order.iter().position(|&x| x == pooler2).unwrap();
    let class_pos = order.iter().position(|&x| x == classifier).unwrap();

    assert!(pool1_pos < class_pos);
    assert!(pool2_pos < class_pos);

    // Execute
    net.get_mut::<ScalarTransformer>(encoder)?.set_value(5.0);
    net.get_mut::<PatternClassifier>(classifier)?.set_label(0);
    net.execute(true)?;

    Ok(())
}

#[test]
fn test_network_cycle_detection() {
    let mut net = Network::new();

    // Create two poolers that feed into each other (cycle)
    let pooler1 = net.add(PatternPooler::new(512, 20, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0));
    let pooler2 = net.add(PatternPooler::new(512, 20, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 1));

    // Create cycle: pooler1 depends on pooler2, pooler2 depends on pooler1
    {
        let out1 = net.get::<PatternPooler>(pooler1).unwrap().output();
        let out2 = net.get::<PatternPooler>(pooler2).unwrap().output();

        net.get_mut::<PatternPooler>(pooler1).unwrap().input_mut().add_child(out2, 0);
        net.get_mut::<PatternPooler>(pooler2).unwrap().input_mut().add_child(out1, 0);
    }

    // Should fail with cycle detection
    let result = net.build();
    assert!(result.is_err());
}

#[test]
fn test_network_execute_without_build() {
    let mut net = Network::new();
    net.add(ScalarTransformer::new(0.0, 10.0, 1024, 128, 2, 0));

    // Should fail - not built
    let result = net.execute(false);
    assert!(result.is_err());
}

#[test]
fn test_network_get_wrong_type() {
    let mut net = Network::new();
    let id = net.add(ScalarTransformer::new(0.0, 10.0, 1024, 128, 2, 0));

    // Try to get as wrong type
    let result = net.get::<PatternPooler>(id);
    assert!(result.is_err());
}

#[test]
fn test_network_clear() -> Result<()> {
    let mut net = Network::new();

    net.add(ScalarTransformer::new(0.0, 10.0, 1024, 128, 2, 0));
    net.add(ScalarTransformer::new(0.0, 10.0, 1024, 128, 2, 1));

    net.build()?;

    assert_eq!(net.num_blocks(), 2);
    assert!(net.is_built());

    net.clear();

    assert_eq!(net.num_blocks(), 0);
    assert!(!net.is_built());

    Ok(())
}

#[test]
fn test_network_training_loop() -> Result<()> {
    // Realistic training scenario
    let mut net = Network::new();

    let encoder = net.add(DiscreteTransformer::new(10, 512, 2, 0));
    let pooler = net.add(PatternPooler::new(
        1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0,
    ));

    // Connect outputs to inputs (dependencies auto-discovered)
    {
        let enc_out = net.get::<DiscreteTransformer>(encoder)?.output();
        net.get_mut::<PatternPooler>(pooler)?
            .input_mut()
            .add_child(enc_out, 0);
    }

    net.build()?;
    net.get_mut::<PatternPooler>(pooler)?.init()?;

    // Training epochs
    for _epoch in 0..3 {
        for value in 0..10 {
            net.get_mut::<DiscreteTransformer>(encoder)?
                .set_value(value);
            net.execute(true)?;
        }
    }

    // Inference
    net.get_mut::<DiscreteTransformer>(encoder)?.set_value(5);
    net.execute(false)?;

    let output = net.get::<PatternPooler>(pooler)?.output();
    assert!(output.borrow().state.num_set() > 0);

    Ok(())
}
