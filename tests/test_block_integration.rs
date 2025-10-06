//! Integration tests for block connections and data flow.
//!
//! These tests validate that blocks can be connected together and data flows
//! correctly through the hierarchy with lazy copying and change tracking.

use gnomics::{Block, BlockInput, BlockOutput, CURR, PREV};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

/// Mock encoder block that generates patterns
struct MockEncoder {
    output: BlockOutput,
    pattern_index: usize,
}

impl MockEncoder {
    fn new() -> Self {
        let mut output = BlockOutput::new();
        output.setup(2, 1024);

        Self {
            output,
            pattern_index: 0,
        }
    }

    fn set_pattern(&mut self, index: usize) {
        self.pattern_index = index;
    }

    fn get_output(&self) -> &BlockOutput {
        &self.output
    }
}

impl Block for MockEncoder {
    fn save(&self, _path: &Path) -> gnomics::Result<()> {
        Ok(())
    }

    fn load(&mut self, _path: &Path) -> gnomics::Result<()> {
        Ok(())
    }

    fn clear(&mut self) {
        self.output.clear();
    }

    fn step(&mut self) {
        self.output.step();
    }

    fn pull(&mut self) {
        // No children
    }

    fn compute(&mut self) {
        // Generate different pattern based on index
        self.output.state.clear_all();

        let base = self.pattern_index * 100;
        for i in 0..10 {
            self.output.state.set_bit(base + i);
        }
    }

    fn store(&mut self) {
        self.output.store();
    }

    fn memory_usage(&self) -> usize {
        self.output.memory_usage()
    }
}

/// Mock processor block that processes inputs
struct MockProcessor {
    input: BlockInput,
    output: BlockOutput,
    process_count: usize,
}

impl MockProcessor {
    fn new() -> Self {
        let mut output = BlockOutput::new();
        output.setup(2, 2048); // Larger to accommodate multiple children

        Self {
            input: BlockInput::new(),
            output,
            process_count: 0,
        }
    }

    fn get_process_count(&self) -> usize {
        self.process_count
    }
}

impl Block for MockProcessor {
    fn save(&self, _path: &Path) -> gnomics::Result<()> {
        Ok(())
    }

    fn load(&mut self, _path: &Path) -> gnomics::Result<()> {
        Ok(())
    }

    fn clear(&mut self) {
        self.input.clear();
        self.output.clear();
        self.process_count = 0;
    }

    fn step(&mut self) {
        self.output.step();
    }

    fn pull(&mut self) {
        self.input.pull();
    }

    fn compute(&mut self) {
        // CRITICAL: Skip processing if inputs haven't changed
        if !self.input.children_changed() {
            return;
        }

        self.process_count += 1;

        // Resize output if needed to match input size
        if self.output.state.num_bits() < self.input.num_bits() {
            self.output.state.resize(self.input.num_bits());
        }

        // Copy input to output (simple passthrough for testing)
        self.output.state.clear_all();
        for i in 0..self.input.num_bits() {
            if self.input.state.get_bit(i) > 0 {
                self.output.state.set_bit(i);
            }
        }
    }

    fn store(&mut self) {
        self.output.store();
    }

    fn memory_usage(&self) -> usize {
        self.input.memory_usage() + self.output.memory_usage()
    }
}

#[test]
fn test_basic_connection() {
    // Create encoder and processor
    let mut encoder = MockEncoder::new();
    let mut processor = MockProcessor::new();

    // Wrap encoder output in Rc<RefCell<>>
    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));

    // Connect processor to encoder
    processor.input.add_child(encoder_output.clone(), 0);

    // Set pattern and process
    encoder.set_pattern(0);
    encoder.execute(false).unwrap();

    // Update shared output
    *encoder_output.borrow_mut() = encoder.output.clone();

    processor.execute(false).unwrap();

    // Verify data flowed
    assert_eq!(processor.output.state.num_set(), 10);
    assert_eq!(processor.get_process_count(), 1);
}

#[test]
fn test_lazy_copying_skips_unchanged() {
    let mut encoder = MockEncoder::new();
    let mut processor = MockProcessor::new();

    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));
    processor.input.add_child(encoder_output.clone(), 0);

    // First feedforward - encoder produces output
    encoder.set_pattern(0);
    encoder.execute(false).unwrap();
    *encoder_output.borrow_mut() = encoder.output.clone();
    processor.execute(false).unwrap();

    assert_eq!(processor.get_process_count(), 1);

    // Second feedforward - encoder produces SAME output
    encoder.execute(false).unwrap();
    *encoder_output.borrow_mut() = encoder.output.clone();
    processor.execute(false).unwrap();

    // Process count should NOT increase (change tracking worked!)
    assert_eq!(processor.get_process_count(), 1);
}

#[test]
fn test_change_tracking_detects_changes() {
    let mut encoder = MockEncoder::new();
    let mut processor = MockProcessor::new();

    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));
    processor.input.add_child(encoder_output.clone(), 0);

    // First pattern
    encoder.set_pattern(0);
    encoder.execute(false).unwrap();
    *encoder_output.borrow_mut() = encoder.output.clone();
    processor.execute(false).unwrap();

    assert_eq!(processor.get_process_count(), 1);

    // Second pattern (different)
    encoder.set_pattern(1);
    encoder.execute(false).unwrap();
    *encoder_output.borrow_mut() = encoder.output.clone();
    processor.execute(false).unwrap();

    // Process count SHOULD increase (change detected)
    assert_eq!(processor.get_process_count(), 2);
}

#[test]
fn test_multiple_children_concatenation() {
    let mut encoder1 = MockEncoder::new();
    let mut encoder2 = MockEncoder::new();
    let mut processor = MockProcessor::new();

    let encoder1_output = Rc::new(RefCell::new(encoder1.output.clone()));
    let encoder2_output = Rc::new(RefCell::new(encoder2.output.clone()));

    processor.input.add_child(encoder1_output.clone(), 0);
    processor.input.add_child(encoder2_output.clone(), 0);

    // Generate different patterns
    encoder1.set_pattern(0);  // Bits 0-9
    encoder2.set_pattern(1);  // Bits 100-109

    encoder1.execute(false).unwrap();
    encoder2.execute(false).unwrap();

    *encoder1_output.borrow_mut() = encoder1.output.clone();
    *encoder2_output.borrow_mut() = encoder2.output.clone();

    processor.execute(false).unwrap();

    // Should have bits from both encoders (concatenated)
    assert_eq!(processor.output.state.num_set(), 20);

    // Check bits from first encoder (offset 0)
    assert_eq!(processor.output.state.get_bit(0), 1);
    assert_eq!(processor.output.state.get_bit(9), 1);

    // Check bits from second encoder (offset 1024)
    assert_eq!(processor.output.state.get_bit(1024 + 100), 1);
    assert_eq!(processor.output.state.get_bit(1024 + 109), 1);
}

#[test]
fn test_partial_change_optimization() {
    let mut encoder1 = MockEncoder::new();
    let mut encoder2 = MockEncoder::new();
    let mut processor = MockProcessor::new();

    let encoder1_output = Rc::new(RefCell::new(encoder1.output.clone()));
    let encoder2_output = Rc::new(RefCell::new(encoder2.output.clone()));

    processor.input.add_child(encoder1_output.clone(), 0);
    processor.input.add_child(encoder2_output.clone(), 0);

    // First round
    encoder1.set_pattern(0);
    encoder2.set_pattern(0);

    encoder1.execute(false).unwrap();
    encoder2.execute(false).unwrap();
    *encoder1_output.borrow_mut() = encoder1.output.clone();
    *encoder2_output.borrow_mut() = encoder2.output.clone();
    processor.execute(false).unwrap();

    assert_eq!(processor.get_process_count(), 1);

    // Second round - only encoder2 changes
    encoder2.set_pattern(1);

    encoder1.execute(false).unwrap();  // No change
    encoder2.execute(false).unwrap();  // Changed
    *encoder1_output.borrow_mut() = encoder1.output.clone();
    *encoder2_output.borrow_mut() = encoder2.output.clone();
    processor.execute(false).unwrap();

    // Should still process (at least one child changed)
    assert_eq!(processor.get_process_count(), 2);
}

#[test]
fn test_temporal_access() {
    let mut encoder = MockEncoder::new();

    // Generate sequence of patterns
    for i in 0..5 {
        encoder.set_pattern(i);
        encoder.execute(false).unwrap();
    }

    // Access current and previous
    let curr = encoder.output.get_bitarray(CURR);
    let prev = encoder.output.get_bitarray(PREV);

    // Current should have pattern 4
    assert_eq!(curr.get_bit(400), 1);
    assert_eq!(curr.get_bit(409), 1);

    // Previous should have pattern 3
    assert_eq!(prev.get_bit(300), 1);
    assert_eq!(prev.get_bit(309), 1);
}

#[test]
fn test_memory_usage() {
    let mut encoder = MockEncoder::new();
    let mut processor = MockProcessor::new();

    let encoder_output = Rc::new(RefCell::new(encoder.output.clone()));
    processor.input.add_child(encoder_output.clone(), 0);

    let encoder_mem = encoder.memory_usage();
    let processor_mem = processor.memory_usage();

    assert!(encoder_mem > 0);
    assert!(processor_mem > encoder_mem); // Processor has input + output
}
