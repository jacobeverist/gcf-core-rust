// Example demonstrating network execution recording and visualization.
//
// This example creates a simple network with a sequence learner,
// runs it through a training sequence, and exports the execution trace
// for visualization.
//
// To run this example:
//   cargo run --example network_visualization
//
// Then open visualization/viewer.html in a browser and load the generated
// trace.json file.

use gnomics::{
    blocks::{DiscreteTransformer, SequenceLearner},
    Block, InputAccess, Network, Result,
};

fn main() -> Result<()> {
    println!("=== Gnomics Network Visualization Example ===\n");

    // Create network
    let mut net = Network::new();

    // Create blocks
    println!("Creating network blocks...");
    let encoder = net.add(DiscreteTransformer::new(
        10,   // 10 discrete values (0-9)
        512,  // 512 statelets
        2,    // 2 time steps of history
        42,   // seed
    ));

    let learner = net.add(SequenceLearner::new(
        512,  // 512 columns
        4,    // 4 statelets per column
        8,    // 8 dendrites per statelet
        32,   // 32 receptors per dendrite
        20,   // dendrite threshold
        20,   // permanence threshold
        2,    // permanence increment
        1,    // permanence decrement
        2,    // 2 time steps of history
        false, // don't always update
        42,   // seed
    ));

    // Set human-readable names for visualization
    net.set_block_name(encoder, "Digit Encoder");
    net.set_block_name(learner, "Sequence Learner");

    // Connect blocks
    println!("Connecting blocks...");
    net.connect_to_input(encoder, learner)?;

    // Build network
    println!("Building network execution order...");
    net.build()?;

    // Initialize learning blocks
    println!("Initializing blocks...");
    net.get_mut::<SequenceLearner>(learner)?.init()?;

    // Define a repeating sequence: 0 -> 1 -> 2 -> 3
    let sequence = vec![0, 1, 2, 3];

    // Start recording
    println!("\nStarting execution recording...");
    net.start_recording();

    // Training phase - learn the sequence
    println!("Training sequence learner (10 epochs)...");
    for epoch in 0..10 {
        for &value in &sequence {
            // Set input value
            net.get_mut::<DiscreteTransformer>(encoder)?
                .set_value(value);

            // Execute with learning
            net.execute(true)?;
        }

        if (epoch + 1) % 2 == 0 {
            println!("  Epoch {}/10 complete", epoch + 1);
        }
    }

    // Testing phase - run sequence and introduce anomaly
    println!("\nTesting with anomaly detection...");

    // Normal sequence
    for &value in &sequence {
        net.get_mut::<DiscreteTransformer>(encoder)?
            .set_value(value);
        net.execute(false)?;

        let anomaly = net.get::<SequenceLearner>(learner)?.get_anomaly_score();
        println!("  Value: {} -> Anomaly: {:.3}", value, anomaly);
    }

    // Introduce anomaly
    println!("\n  Introducing anomaly (value 7 out of sequence)...");
    net.get_mut::<DiscreteTransformer>(encoder)?.set_value(7);
    net.execute(false)?;

    let anomaly = net.get::<SequenceLearner>(learner)?.get_anomaly_score();
    println!("  Value: 7 -> Anomaly: {:.3} (HIGH!)", anomaly);

    // Continue normal sequence
    println!("\n  Returning to normal sequence...");
    for &value in &sequence {
        net.get_mut::<DiscreteTransformer>(encoder)?
            .set_value(value);
        net.execute(false)?;

        let anomaly = net.get::<SequenceLearner>(learner)?.get_anomaly_score();
        println!("  Value: {} -> Anomaly: {:.3}", value, anomaly);
    }

    // Stop recording and export
    println!("\nExporting execution trace...");
    if let Some(trace) = net.stop_recording() {
        let filename = "trace.json";
        trace.to_json_file(filename)?;
        println!("Trace exported to: {}", filename);
        println!("Total steps recorded: {}", trace.total_steps);
    }

    println!("\n=== Visualization Instructions ===");
    println!("1. Open visualization/viewer.html in a web browser");
    println!("2. Click 'Load Trace' and select trace.json");
    println!("3. Use the timeline to scrub through execution");
    println!("4. Press Space to play/pause animation");
    println!("5. Use arrow keys to step forward/backward");
    println!("\nYou should see:");
    println!("  - Network graph showing Encoder -> Learner connection");
    println!("  - BitField states for each block at each timestep");
    println!("  - Anomaly spike when value 7 is introduced");

    Ok(())
}
