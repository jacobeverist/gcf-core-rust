# Gnomics Network Visualization

This directory contains tools for visualizing the execution of Gnomics neural networks.

## Features

- **Network Graph Visualization**: Interactive force-directed graph showing blocks and connections
- **BitField Heatmaps**: Real-time visualization of sparse binary patterns for each block
- **Timeline Scrubbing**: Navigate through execution history with a timeline controller
- **Playback Controls**: Play/pause animation of network execution
- **Connection Types**: Visual distinction between input and context connections
- **Block Naming**: Custom labels for blocks for easier identification

## Quick Start

### 1. Record Network Execution

Add recording to your Rust code:

```rust
use gnomics::{Network, blocks::ScalarTransformer, Block, InputAccess};

let mut net = Network::new();

// Create and connect blocks
let encoder = net.add(ScalarTransformer::new(0.0, 100.0, 2048, 256, 2, 0));
// ... more blocks and connections ...

// Optional: name blocks for visualization
net.set_block_name(encoder, "Temperature Encoder");

net.build()?;

// Start recording before execution
net.start_recording();

// Execute your network
for value in data {
    net.get_mut::<ScalarTransformer>(encoder)?.set_value(value);
    net.execute(true)?;
}

// Export trace
if let Some(trace) = net.stop_recording() {
    trace.to_json_file("my_trace.json")?;
}
```

### 2. Visualize in Browser

1. Open `visualization/viewer.html` in a web browser
2. Click "Load Trace" and select your `my_trace.json` file
3. Use the timeline to explore execution

## Controls

- **Timeline Scrubber**: Click or drag to jump to any timestep
- **Play/Pause Button**: Animate through execution automatically
- **Keyboard**:
  - `Space`: Play/pause
  - `Left Arrow`: Previous step
  - `Right Arrow`: Next step
- **Network Graph**: Drag nodes to rearrange, scroll to zoom

## API Reference

### Recording Methods

```rust
// Start recording
net.start_recording();

// Check if recording
let is_recording = net.is_recording();

// Pause/resume
net.pause_recording();
net.resume_recording();

// Stop and get trace
let trace = net.stop_recording(); // Returns Option<ExecutionTrace>

// Set custom block names
net.set_block_name(block_id, "My Block Name");
```

### Export/Import

```rust
// Export to JSON string
let json = trace.to_json()?;

// Export to file
trace.to_json_file("trace.json")?;

// Import from JSON
let trace = ExecutionTrace::from_json(&json)?;

// Import from file
let trace = ExecutionTrace::from_json_file("trace.json")?;
```

## Examples

### Simple Sequence Learning

```bash
cargo run --example network_visualization
```

Creates a simple encoder → learner network, trains it on a sequence, introduces an anomaly, and exports the trace. Opens with `visualization/viewer.html`.

### Complex Multi-Block Network

```bash
cargo run --example complex_network_visualization
```

Creates a hierarchical network with:
- 3 input encoders (temperature, pressure, humidity)
- Feature pooler combining inputs
- Weather classifier (sunny/cloudy/rainy)

Demonstrates multi-input networks and classification.

## Trace Format

The exported JSON contains:

```json
{
  "connections": [
    {
      "source_id": 0,
      "target_id": 1,
      "connection_type": "input",
      "time_offset": 0
    }
  ],
  "steps": [
    {
      "step_number": 0,
      "block_states": {
        "0": {
          "num_bits": 2048,
          "active_bits": [10, 20, 30, ...],
          "num_active": 256
        }
      },
      "block_metadata": {
        "0": {
          "id": 0,
          "name": "Temperature Encoder",
          "block_type": "ScalarTransformer",
          "num_statelets": 2048,
          "num_active": 256
        }
      }
    }
  ],
  "total_steps": 100
}
```

## Visualization Components

### Network Graph

- **Nodes**: Circles represent blocks
  - Label: Block name
  - Subtitle: Block type
  - Color: Blue outline (#4a9eff)
- **Edges**: Arrows show data flow
  - Solid lines: Input connections
  - Dashed lines: Context connections (orange)
  - Arrowheads point to destination

### BitField Heatmap

- **Grid**: Each cell represents one bit
- **Colors**:
  - Dark gray (#2a2a2a): Inactive (0)
  - Blue (#4a9eff): Active (1)
- **Stats**: Shows active count and percentage

### Timeline

- **Track**: Gray bar showing full execution
- **Scrubber**: Blue circle indicating current position
- **Step Counter**: Shows current/total steps

## Performance Considerations

- **Large Networks**: Networks with >20 blocks may have cluttered graphs
- **Long Traces**: Traces with >1000 steps may slow timeline rendering
- **BitField Size**: Very large BitFields (>10,000 bits) use simplified grid

## Browser Compatibility

- Chrome/Edge: ✅ Fully supported
- Firefox: ✅ Fully supported
- Safari: ✅ Fully supported
- IE11: ❌ Not supported (uses modern JavaScript)

## Tips

1. **Name your blocks**: Makes graphs much easier to understand
2. **Pause during training**: Use `pause_recording()` to skip uninteresting periods
3. **Export subsets**: Record only the epochs/phases you want to visualize
4. **Multiple traces**: Compare different runs by loading different trace files

## Advanced Usage

### Selective Recording

```rust
// Train without recording
for epoch in 0..100 {
    for data in training_set {
        net.execute(true)?;
    }
}

// Record only test phase
net.start_recording();
for data in test_set {
    net.execute(false)?;
}
let trace = net.stop_recording().unwrap();
```

### Recording Callbacks

```rust
// Record every N steps
let mut step_count = 0;
for data in dataset {
    if step_count % 10 == 0 {
        net.resume_recording();
    } else {
        net.pause_recording();
    }
    net.execute(true)?;
    step_count += 1;
}
```

## Troubleshooting

**Problem**: "No file loaded" in viewer
- **Solution**: Make sure to click "Load Trace" and select a valid JSON file

**Problem**: Trace file is too large (>100MB)
- **Solution**: Record fewer steps or use `pause_recording()` to skip periods

**Problem**: Network graph is cluttered
- **Solution**: Drag nodes to rearrange, or consider visualizing subnetworks

**Problem**: BitField shows all zeros
- **Solution**: Ensure `execute()` was called after setting inputs

## Future Enhancements

Potential improvements for the visualization system:

- Real-time streaming (WebSocket support)
- Comparison mode (overlay multiple traces)
- Export to video
- Zoom into individual blocks
- Permanence value visualization
- Anomaly score overlay
- Learning rate heatmaps

## Contributing

To extend the visualizer:

1. **Backend (Rust)**: Modify `src/execution_recorder.rs` to capture additional data
2. **Frontend (JS)**: Edit `visualization/viewer.html` to display new features
3. **Examples**: Add new examples in `examples/` directory

## License

Same as Gnomics (MIT License)
