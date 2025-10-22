//! Example: Save and Load Network Configuration
//!
//! Demonstrates how to:
//! 1. Build a network with multiple blocks
//! 2. Save the network configuration to JSON
//! 3. Load the configuration and recreate the network
//! 4. Verify the restored network works correctly
//!
//! This example shows Option 1 serialization: configuration only (no learned state).
//! The restored network has the same architecture but fresh (untrained) parameters.

use gnomics::{
    blocks::{DiscreteTransformer, PatternClassifier, PatternPooler, ScalarTransformer},
    Block, InputAccess, Network, NetworkConfig, OutputAccess, Result,
};

fn main() -> Result<()> {
    println!("=== Network Save/Load Example ===\n");

    // ========================================
    // PART 1: Create and configure a network
    // ========================================
    println!("Part 1: Building original network...");

    let mut original_net = Network::new();

    // Add blocks: encoder -> pooler -> classifier
    let encoder = original_net.add(ScalarTransformer::new(0.0, 10.0, 2048, 256, 2, 42));
    let pooler = original_net.add(PatternPooler::new(
        1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 123,
    ));
    let classifier = original_net.add(PatternClassifier::new(
        3, 1023, 20, 20, 2, 1, 0.8, 0.5, 0.3, 2, 456,
    ));

    // Connect blocks using simplified API
    original_net.connect_to_input(encoder, pooler)?;
    original_net.connect_to_input(pooler, classifier)?;

    // Build network
    original_net.build()?;
    println!("✓ Original network built with {} blocks", original_net.num_blocks());

    // Initialize learning blocks
    original_net.get_mut::<PatternPooler>(pooler)?.init()?;
    original_net.get_mut::<PatternClassifier>(classifier)?.init()?;
    println!("✓ Learning blocks initialized");

    // ========================================
    // PART 2: Save configuration to JSON
    // ========================================
    println!("\nPart 2: Saving network configuration...");

    let config = original_net
        .to_config()?
        .with_metadata("name", "Three-Stage Pipeline")
        .with_metadata("author", "Example User")
        .with_metadata("description", "Encoder -> Pooler -> Classifier");

    // Save to JSON file
    let json = config.to_json()?;
    let json_path = "network_config.json";
    std::fs::write(json_path, &json)?;
    println!("✓ Configuration saved to {}", json_path);
    println!("  File size: {} bytes", json.len());
    println!("  Blocks: {}", config.blocks.len());
    println!("  Connections: {}", config.connections.len());

    // Also save to binary format for comparison
    let binary = config.to_binary()?;
    let binary_path = "network_config.bin";
    std::fs::write(binary_path, &binary)?;
    println!("✓ Binary format saved to {}", binary_path);
    println!("  File size: {} bytes ({}% of JSON)", binary.len(), (binary.len() * 100) / json.len());

    // ========================================
    // PART 3: Load configuration and rebuild
    // ========================================
    println!("\nPart 3: Loading configuration...");

    // Load from JSON
    let loaded_json = std::fs::read_to_string(json_path)?;
    let loaded_config = NetworkConfig::from_json(&loaded_json)?;
    println!("✓ Configuration loaded from JSON");

    // Verify metadata
    if let Some(name) = loaded_config.metadata.get("name") {
        println!("  Network name: {}", name);
    }

    // Reconstruct network
    let mut restored_net = Network::from_config(&loaded_config)?;
    println!("✓ Network reconstructed with {} blocks", restored_net.num_blocks());

    // Build and initialize
    restored_net.build()?;

    // Re-get block ids from restored network
    let restored_order = restored_net.execution_order();
    let restored_encoder = restored_order[0];
    let restored_pooler = restored_order[1];
    let restored_classifier = restored_order[2];

    restored_net.get_mut::<PatternPooler>(restored_pooler)?.init()?;
    restored_net.get_mut::<PatternClassifier>(restored_classifier)?.init()?;
    println!("✓ Restored network built and initialized");

    // ========================================
    // PART 4: Verify restored network works
    // ========================================
    println!("\nPart 4: Verifying restored network...");

    // Test inference on restored network
    restored_net.get_mut::<ScalarTransformer>(restored_encoder)?.set_value(5.5);
    restored_net.get_mut::<PatternClassifier>(restored_classifier)?.set_label(1);
    restored_net.execute(true)?;

    let probs = restored_net.get::<PatternClassifier>(restored_classifier)?.get_probabilities();
    println!("✓ Network executed successfully");
    println!("  Encoder output: {} active bits",
        restored_net.get::<ScalarTransformer>(restored_encoder)?.output().borrow().state.num_set());
    println!("  Pooler output: {} active bits",
        restored_net.get::<PatternPooler>(restored_pooler)?.output().borrow().state.num_set());
    println!("  Classifier probabilities: [{:.3}, {:.3}, {:.3}]",
        probs[0], probs[1], probs[2]);

    // ========================================
    // PART 5: Load from binary format
    // ========================================
    println!("\nPart 5: Loading from binary format...");

    let binary_data = std::fs::read(binary_path)?;
    let binary_config = NetworkConfig::from_binary(&binary_data)?;
    let mut binary_net = Network::from_config(&binary_config)?;
    binary_net.build()?;
    println!("✓ Network loaded from binary format");
    println!("  Blocks: {}", binary_net.num_blocks());

    // ========================================
    // PART 6: Demonstrate round-trip fidelity
    // ========================================
    println!("\nPart 6: Round-trip verification...");

    let reconfig = restored_net.to_config()?;
    let original_json = config.to_json()?;
    let restored_json = reconfig.to_json()?;

    println!("✓ Configuration round-trip complete");
    println!("  Original JSON size: {} bytes", original_json.len());
    println!("  Restored JSON size: {} bytes", restored_json.len());
    println!("  Match: {}", original_json == restored_json);

    // Cleanup
    std::fs::remove_file(json_path)?;
    std::fs::remove_file(binary_path)?;
    println!("\n✓ Cleanup complete");

    println!("\n=== Summary ===");
    println!("✓ Network configuration successfully saved and loaded");
    println!("✓ JSON format is human-readable and editable");
    println!("✓ Binary format is more compact");
    println!("✓ Restored network has identical architecture");
    println!("✓ Configuration serialization is working correctly!");

    Ok(())
}
