use crate::{BitField, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export BlockId from network module
pub use crate::network::BlockId;

/// Snapshot of a BitField state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitFieldSnapshot {
    /// Number of bits in the field
    pub num_bits: usize,
    /// List of active bit indices
    pub active_bits: Vec<usize>,
    /// Total number of active bits
    pub num_active: usize,
}

impl BitFieldSnapshot {
    pub fn from_bitfield(bf: &BitField) -> Self {
        Self {
            num_bits: bf.num_bits(),
            active_bits: bf.get_acts(),
            num_active: bf.num_set(),
        }
    }
}

/// Block metadata for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetadata {
    pub id: BlockId,
    pub name: String,
    pub block_type: String,
    pub num_statelets: usize,
    pub num_active: usize,
}

/// Connection between two blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockConnection {
    pub source_id: BlockId,
    pub target_id: BlockId,
    pub connection_type: ConnectionType,
    pub time_offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionType {
    Input,
    Context,
}

/// Single timestep in the execution trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_number: usize,
    pub block_states: HashMap<BlockId, BitFieldSnapshot>,
    pub block_metadata: HashMap<BlockId, BlockMetadata>,
}

/// Complete execution trace for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub connections: Vec<BlockConnection>,
    pub steps: Vec<ExecutionStep>,
    pub total_steps: usize,
}

impl ExecutionTrace {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            steps: Vec::new(),
            total_steps: 0,
        }
    }

    pub fn add_step(&mut self, step: ExecutionStep) {
        self.total_steps = step.step_number + 1;
        self.steps.push(step);
    }

    pub fn set_connections(&mut self, connections: Vec<BlockConnection>) {
        self.connections = connections;
    }

    /// Export trace to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            crate::GnomicsError::Other(format!("Failed to serialize trace to JSON: {}", e))
        })
    }

    /// Export trace to JSON file
    pub fn to_json_file(&self, path: &str) -> Result<()> {
        let json = self.to_json()?;
        std::fs::write(path, json).map_err(|e| {
            crate::GnomicsError::Other(format!("Failed to write trace to file: {}", e))
        })?;
        Ok(())
    }

    /// Import trace from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| {
            crate::GnomicsError::Other(format!("Failed to deserialize trace from JSON: {}", e))
        })
    }

    /// Import trace from JSON file
    pub fn from_json_file(path: &str) -> Result<Self> {
        let json = std::fs::read_to_string(path).map_err(|e| {
            crate::GnomicsError::Other(format!("Failed to read trace file: {}", e))
        })?;
        Self::from_json(&json)
    }
}

impl Default for ExecutionTrace {
    fn default() -> Self {
        Self::new()
    }
}

/// Records execution state for visualization
pub struct ExecutionRecorder {
    trace: ExecutionTrace,
    current_step: usize,
    recording: bool,
}

impl ExecutionRecorder {
    pub fn new() -> Self {
        Self {
            trace: ExecutionTrace::new(),
            current_step: 0,
            recording: true,
        }
    }

    pub fn start(&mut self) {
        self.recording = true;
    }

    pub fn stop(&mut self) {
        self.recording = false;
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    pub fn record_step(
        &mut self,
        block_states: HashMap<BlockId, BitFieldSnapshot>,
        block_metadata: HashMap<BlockId, BlockMetadata>,
    ) {
        if !self.recording {
            return;
        }

        let step = ExecutionStep {
            step_number: self.current_step,
            block_states,
            block_metadata,
        };

        self.trace.add_step(step);
        self.current_step += 1;
    }

    pub fn set_connections(&mut self, connections: Vec<BlockConnection>) {
        self.trace.set_connections(connections);
    }

    pub fn get_trace(&self) -> &ExecutionTrace {
        &self.trace
    }

    pub fn export_trace(self) -> ExecutionTrace {
        self.trace
    }

    pub fn reset(&mut self) {
        self.trace = ExecutionTrace::new();
        self.current_step = 0;
    }
}

impl Default for ExecutionRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitfield_snapshot() {
        let mut bf = BitField::new(100);
        bf.set_bit(5);
        bf.set_bit(10);
        bf.set_bit(15);

        let snapshot = BitFieldSnapshot::from_bitfield(&bf);
        assert_eq!(snapshot.num_bits, 100);
        assert_eq!(snapshot.num_active, 3);
        assert_eq!(snapshot.active_bits, vec![5, 10, 15]);
    }

    #[test]
    fn test_execution_recorder() {
        let mut recorder = ExecutionRecorder::new();
        assert!(recorder.is_recording());

        let mut states = HashMap::new();
        let mut metadata = HashMap::new();

        let mut bf = BitField::new(100);
        bf.set_bit(10);
        let block_id = BlockId::from_raw(1);
        states.insert(block_id, BitFieldSnapshot::from_bitfield(&bf));

        metadata.insert(
            block_id,
            BlockMetadata {
                id: block_id,
                name: "test_block".to_string(),
                block_type: "ScalarTransformer".to_string(),
                num_statelets: 100,
                num_active: 1,
            },
        );

        recorder.record_step(states, metadata);

        let trace = recorder.get_trace();
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(trace.total_steps, 1);
    }

    #[test]
    fn test_trace_json_roundtrip() {
        let mut trace = ExecutionTrace::new();

        let mut states = HashMap::new();
        let mut metadata = HashMap::new();

        let mut bf = BitField::new(50);
        bf.set_bit(5);
        bf.set_bit(10);
        let block_id = BlockId::from_raw(1);
        states.insert(block_id, BitFieldSnapshot::from_bitfield(&bf));

        metadata.insert(
            block_id,
            BlockMetadata {
                id: block_id,
                name: "encoder".to_string(),
                block_type: "ScalarTransformer".to_string(),
                num_statelets: 50,
                num_active: 2,
            },
        );

        let step = ExecutionStep {
            step_number: 0,
            block_states: states,
            block_metadata: metadata,
        };

        trace.add_step(step);

        let json = trace.to_json().unwrap();
        let parsed = ExecutionTrace::from_json(&json).unwrap();

        assert_eq!(parsed.steps.len(), 1);
        assert_eq!(parsed.total_steps, 1);
    }

    #[test]
    fn test_recorder_start_stop() {
        let mut recorder = ExecutionRecorder::new();
        recorder.stop();
        assert!(!recorder.is_recording());

        recorder.record_step(HashMap::new(), HashMap::new());
        assert_eq!(recorder.get_trace().steps.len(), 0);

        recorder.start();
        recorder.record_step(HashMap::new(), HashMap::new());
        assert_eq!(recorder.get_trace().steps.len(), 1);
    }
}
