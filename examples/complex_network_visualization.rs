// Example demonstrating visualization of a complex multi-block network.
//
// This example creates a hierarchical network with multiple encoders,
// a pooler, and a classifier, simulating a simple classification pipeline.
//
// To run this example:
//   cargo run --example complex_network_visualization
//
// Then open visualization/viewer.html and load the generated
// complex_trace.json file.

use gnomics::{
    blocks::{PatternClassifier, PatternPooler, ScalarTransformer},
    Block, InputAccess, Network, Result,
};

fn main() -> Result<()> {
    println!("=== Complex Network Visualization Example ===\n");

    // Create network
    let mut net = Network::new();

    // Create multiple input encoders (simulating multi-sensor input)
    println!("Creating network blocks...");
    let temp_encoder = net.add(ScalarTransformer::new(
        0.0,   // min temp
        100.0, // max temp
        1024,  // statelets
        128,   // active statelets
        2,     // history
        42,    // seed
    ));

    let pressure_encoder = net.add(ScalarTransformer::new(
        900.0,  // min pressure (hPa)
        1100.0, // max pressure
        1024,   // statelets
        128,    // active statelets
        2,      // history
        43,     // seed
    ));

    let humidity_encoder = net.add(ScalarTransformer::new(
        0.0,   // min humidity (%)
        100.0, // max humidity
        1024,  // statelets
        128,   // active statelets
        2,     // history
        44,    // seed
    ));

    // Create pooler to combine inputs
    let pooler = net.add(PatternPooler::new(
        2048, // dendrites
        80,   // winners
        20,   // perm threshold
        2,    // perm inc
        1,    // perm dec
        0.8,  // pooling %
        0.5,  // connectivity %
        0.3,  // learning rate
        false, // always update
        2,    // history
        45,   // seed
    ));

    // Create classifier (3 weather conditions)
    let classifier = net.add(PatternClassifier::new(
        3,    // 3 labels (sunny, cloudy, rainy)
        3*64, // number of statelets
        64,   // active statelets
        20,   // perm threshold
        2,    // perm inc
        1,    // perm dec
        0.8,  // pooling %
        0.5,  // connectivity %
        0.3,  // learning rate
        2,    // history
        46,   // seed
    ));

    // Set human-readable names
    net.set_block_name(temp_encoder, "Temperature Sensor");
    net.set_block_name(pressure_encoder, "Pressure Sensor");
    net.set_block_name(humidity_encoder, "Humidity Sensor");
    net.set_block_name(pooler, "Feature Pooler");
    net.set_block_name(classifier, "Weather Classifier");

    // Connect network: encoders -> pooler -> classifier
    println!("Connecting network topology...");
    net.connect_many_to_input(&[temp_encoder, pressure_encoder, humidity_encoder], pooler)?;
    net.connect_to_input(pooler, classifier)?;

    // Build network
    println!("Building execution order...");
    net.build()?;

    // Initialize blocks
    println!("Initializing learning blocks...");
    net.get_mut::<PatternPooler>(pooler)?.init()?;
    net.get_mut::<PatternClassifier>(classifier)?.init()?;

    // Define training data: (temp, pressure, humidity, label)
    // Label 0 = Sunny, 1 = Cloudy, 2 = Rainy
    let training_data = vec![
        // Sunny conditions
        (25.0, 1013.0, 40.0, 0),
        (28.0, 1015.0, 35.0, 0),
        (30.0, 1018.0, 30.0, 0),
        (26.0, 1014.0, 38.0, 0),
        // Cloudy conditions
        (20.0, 1010.0, 60.0, 1),
        (18.0, 1008.0, 65.0, 1),
        (22.0, 1012.0, 55.0, 1),
        (19.0, 1009.0, 62.0, 1),
        // Rainy conditions
        (15.0, 1000.0, 85.0, 2),
        (12.0, 995.0, 90.0, 2),
        (14.0, 998.0, 88.0, 2),
        (13.0, 997.0, 92.0, 2),
    ];

    // Start recording
    println!("\nStarting execution recording...");
    net.start_recording();

    // Training phase
    println!("Training network (5 epochs)...");
    for epoch in 0..5 {
        for (temp, pressure, humidity, label) in &training_data {
            // Set inputs
            net.get_mut::<ScalarTransformer>(temp_encoder)?
                .set_value(*temp);
            net.get_mut::<ScalarTransformer>(pressure_encoder)?
                .set_value(*pressure);
            net.get_mut::<ScalarTransformer>(humidity_encoder)?
                .set_value(*humidity);

            // Set label
            net.get_mut::<PatternClassifier>(classifier)?
                .set_label(*label);

            // Execute with learning
            net.execute(true)?;
        }
        println!("  Epoch {}/5 complete", epoch + 1);
    }

    // Testing phase
    println!("\nTesting classification...");

    let test_data = vec![
        (27.0, 1016.0, 37.0, "Sunny"),
        (19.0, 1009.0, 63.0, "Cloudy"),
        (13.0, 996.0, 89.0, "Rainy"),
        (25.0, 1005.0, 70.0, "Ambiguous"), // borderline case
    ];

    for (temp, pressure, humidity, expected) in &test_data {
        // Set inputs (no label during testing)
        net.get_mut::<ScalarTransformer>(temp_encoder)?
            .set_value(*temp);
        net.get_mut::<ScalarTransformer>(pressure_encoder)?
            .set_value(*pressure);
        net.get_mut::<ScalarTransformer>(humidity_encoder)?
            .set_value(*humidity);

        // Execute without learning
        net.execute(false)?;

        // Get classification
        let probs = net.get::<PatternClassifier>(classifier)?.get_probabilities();
        let labels = ["Sunny", "Cloudy", "Rainy"];

        println!(
            "\n  Input: {:.1}°C, {:.0}hPa, {:.0}% humidity (Expected: {})",
            temp, pressure, humidity, expected
        );
        println!("  Classification probabilities:");
        for (i, label) in labels.iter().enumerate() {
            println!("    {}: {:.1}%", label, probs[i] * 100.0);
        }
    }

    // Stop recording and export
    println!("\n\nExporting execution trace...");
    if let Some(trace) = net.stop_recording() {
        let filename = "complex_trace.json";
        trace.to_json_file(filename)?;
        println!("Trace exported to: {}", filename);
        println!("Total steps recorded: {}", trace.total_steps);
    }

    println!("\n=== Visualization Instructions ===");
    println!("1. Open visualization/viewer.html in a web browser");
    println!("2. Click 'Load Trace' and select complex_trace.json");
    println!("3. Observe the hierarchical network structure:");
    println!("   - 3 input encoders (Temperature, Pressure, Humidity)");
    println!("   - Feature Pooler combining inputs");
    println!("   - Weather Classifier producing predictions");
    println!("4. Scrub through timeline to see:");
    println!("   - How input patterns change with different conditions");
    println!("   - How the pooler creates stable representations");
    println!("   - How the classifier learns and predicts");

    Ok(())
}
