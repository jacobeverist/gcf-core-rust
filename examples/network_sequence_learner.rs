//! Online Learning with Scalar Sequence Anomaly Detection
//!
//! This example demonstrates:
//! - ScalarTransformer for encoding continuous values
//! - SequenceLearner for learning temporal sequences
//! - Anomaly detection in repeating patterns
//!
//! The system learns a repeating sequence (0.0, 0.2, 0.4, 0.6, 0.8, 1.0)
//! and detects when an unexpected value appears in the pattern.
#![allow(unused_imports)]


use gnomics::{blocks::{SequenceLearner, ScalarTransformer}, Block, BlockId, InputAccess, Network, OutputAccess, PatternClassifier, PatternPooler, Result};
use itertools::Itertools;

fn main() -> Result<()> {
    println!("\n=== Online Learning: Scalar Sequence Anomaly Detection ===\n");

    // Create network with encoder -> pooler
    let mut net = Network::new();


    // Create Scalar Transformer
    // Encodes continuous values [0.0, 1.0] into 64-bit patterns with 8 active bits
    let encoder = net.add( ScalarTransformer::new(
        0.0, // min_val: minimum input value
        1.0, // max_val: maximum input value
        64,  // num_s: number of statelets
        8,   // num_as: number of active statelets
        2,   // num_t: history depth
        42,  // seed: RNG seed for reproducibility
    ));

    // Create Sequence Learner
    // Learns temporal sequences and predicts next patterns
    let learner =  net.add(SequenceLearner::new(
        64,    // num_c: 64 columns (matches transformer output)
        10,    // num_spc: 10 statelets per column
        10,    // num_dps: 10 dendrites per statelet
        12,    // num_rpd: 12 receptors per dendrite
        6,     // d_thresh: dendrite threshold (activations needed)
        20,    // perm_thr: receptor permanence threshold
        2,     // perm_inc: receptor permanence increment
        1,     // perm_dec: receptor permanence decrement
        3,     // num_t: history depth
        false, // always_update: only update on changes
        42,    // seed: RNG seed for reproducibility
    ));


    // Connect blocks
    net.connect(encoder, learner)?;

    // Connect outputs to inputs manually
    {
        let enc_out = net.get::<ScalarTransformer>(encoder)?.output();
        net.get_mut::<SequenceLearner>(learner)?
            .input_mut()
            .add_child(enc_out, 0);
    }

    // Build and initialize
    net.build()?;
    net.get_mut::<SequenceLearner>(learner)?.init()?;

    // Execute
    net.get_mut::<ScalarTransformer>(encoder)?.set_value(42.0);
    net.execute(false)?;

    // Verify output
    let output = net.get::<SequenceLearner>(learner)?.output();
    assert!(output.borrow().state.num_set() > 0);





    // Connect encoder to learner
    // The learner's input reads from the transformer's output
    // learner.input_mut().add_child(transformer.output(), 0);
    // learner.init()?;

    // Define the repeating sequence with an anomaly at the end
    // Pattern: 0.0 → 0.2 → 0.4 → 0.6 → 0.8 → 1.0 (repeated)
    // Anomaly: Last sequence has 0.2 instead of 0.4 (position 122)
    let values = vec![
        // Repetitions 1-20 (normal pattern)
        0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0,
        0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0,
        0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0,
        0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0,
        0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0,
        0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0,
        0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0,
        // Repetition 21 with anomaly at position 4
        0.0, 0.2, 0.4, 0.2, 0.8, 1.0, // <-- Anomaly: 0.2 instead of 0.6
    ];

    let mut scores = Vec::new();
    let mut patterns = Vec::new();

    println!("Processing {} values in sequence...\n", values.len());

    // Execute training
    for (i, &value) in values.iter().enumerate() {

        // set scalar value
        net.get_mut::<ScalarTransformer>(encoder)?.set_value(value);

        // execute network a step
        net.execute(true)?;

        // get bitfield state of SequenceLearner
        let learner_pattern = net.get_mut::<SequenceLearner>(learner)?.get_output_state();

        // get computed anomaly score from SequenceLearner (0.0 = expected, 1.0 = completely unexpected)
        let score = net.get_mut::<SequenceLearner>(learner)?.get_anomaly_score();

        if score > 0.0 {
            println!(
                "⚠️  Step {}: value={:.1}, anomaly={:.1}",
                i,
                value,
                score
            );
            println!("{:?}", learner_pattern.get_bits().iter().format(""));
        }


        scores.push(score);
        patterns.push(learner_pattern);

    }


    // Summary statistics
    println!("\n=== Summary ===");
    let avg_score: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
    let max_score = scores.iter().fold(0.0f64, |a, &b| a.max(b));
    let max_idx = scores.iter().position(|&s| s == max_score).unwrap_or(0);

    println!("Total steps: {}", values.len());
    println!("Average anomaly score: {:.2}%", avg_score * 100.0);
    println!("Maximum anomaly score: {:.2}%", max_score * 100.0);
    println!(
        "Peak anomaly at step {} (value={:.1})",
        max_idx, values[max_idx]
    );

    // Verify the anomaly was detected at the expected position (step 123)
    // The anomaly is at index 123 where 0.2 appears instead of 0.6
    let anomaly_idx = 123;
    let anomaly_score = scores[anomaly_idx];

    if anomaly_score > 0.5 {
        // High anomaly score indicates detection
        println!(
            "\n✅ Anomaly successfully detected at step {} with score {:.2}%",
            anomaly_idx,
            anomaly_score * 100.0
        );
    } else {
        println!(
            "\n❌ Anomaly not detected at step {} (score: {:.2}%)",
            anomaly_idx,
            anomaly_score * 100.0
        );
    }

    Ok(())
}
