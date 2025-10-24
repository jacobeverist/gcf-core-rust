//! WebAssembly interface for running Gnomics networks in the browser.
//!
//! This module provides JavaScript-friendly bindings for the Gnomics framework,
//! enabling real-time neural network execution and visualization in web browsers.

use wasm_bindgen::prelude::*;

use crate::{
    blocks::*,
    Block, BlockId, ExecutionTrace, InputAccess, Network, OutputAccess, Result as GnomicsResult,
};

/// WASM-friendly wrapper around Gnomics Network.
///
/// Provides a JavaScript-compatible API for building and executing neural networks
/// in the browser. Block handles (usize) are used instead of BlockIds for JS interop.
#[wasm_bindgen]
pub struct WasmNetwork {
    net: Network,
    // Map from JS handle (usize) to internal BlockId
    block_handles: Vec<(String, BlockId)>,
}

#[wasm_bindgen]
impl WasmNetwork {
    /// Create a new empty network.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const net = new WasmNetwork();
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Enable panic messages in browser console
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        Self {
            net: Network::new(),
            block_handles: Vec::new(),
        }
    }

    /// Add a ScalarTransformer block for encoding continuous values.
    ///
    /// Returns a handle (index) that can be used to reference this block.
    ///
    /// # Arguments
    /// * `name` - Human-readable name for visualization
    /// * `min_val` - Minimum input value
    /// * `max_val` - Maximum input value
    /// * `num_s` - Number of statelets
    /// * `num_as` - Number of active statelets
    /// * `num_t` - History depth
    /// * `seed` - Random seed
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const encoder = net.add_scalar_transformer(
    ///     "Temperature", 0.0, 100.0, 2048, 256, 2, 42
    /// );
    /// ```
    pub fn add_scalar_transformer(
        &mut self,
        name: &str,
        min_val: f64,
        max_val: f64,
        num_s: usize,
        num_as: usize,
        num_t: usize,
        seed: u32,
    ) -> usize {
        let block = ScalarTransformer::new(min_val, max_val, num_s, num_as, num_t, seed.into());
        let id = self.net.add(block);
        self.net.set_block_name(id, name);
        let handle = self.block_handles.len();
        self.block_handles.push((name.to_string(), id));
        handle
    }

    /// Add a DiscreteTransformer block for encoding categorical values.
    ///
    /// # Arguments
    /// * `name` - Human-readable name
    /// * `num_v` - Number of discrete values (categories)
    /// * `num_s` - Number of statelets
    /// * `num_t` - History depth
    /// * `seed` - Random seed
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const encoder = net.add_discrete_transformer(
    ///     "Day of Week", 7, 512, 2, 42
    /// );
    /// ```
    pub fn add_discrete_transformer(
        &mut self,
        name: &str,
        num_v: usize,
        num_s: usize,
        num_t: usize,
        seed: u32,
    ) -> usize {
        let block = DiscreteTransformer::new(num_v, num_s, num_t, seed.into());
        let id = self.net.add(block);
        self.net.set_block_name(id, name);
        let handle = self.block_handles.len();
        self.block_handles.push((name.to_string(), id));
        handle
    }

    /// Add a PatternPooler block for unsupervised feature learning.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const pooler = net.add_pattern_pooler(
    ///     "Feature Pooler", 1024, 40, 20, 2, 1, 0.8, 0.5, 0.3, false, 2, 0
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn add_pattern_pooler(
        &mut self,
        name: &str,
        num_s: usize,
        num_as: usize,
        perm_thr: u8,
        perm_inc: u8,
        perm_dec: u8,
        pct_pool: f64,
        pct_conn: f64,
        pct_learn: f64,
        always_update: bool,
        num_t: usize,
        seed: u32,
    ) -> usize {
        let block = PatternPooler::new(
            num_s, num_as, perm_thr, perm_inc, perm_dec, pct_pool, pct_conn, pct_learn,
            always_update, num_t, seed.into(),
        );
        let id = self.net.add(block);
        self.net.set_block_name(id, name);
        let handle = self.block_handles.len();
        self.block_handles.push((name.to_string(), id));
        handle
    }

    /// Add a PatternClassifier block for supervised classification.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const classifier = net.add_pattern_classifier(
    ///     "Weather Classifier", 3, 1024, 30, 20, 2, 1, 0.8, 0.5, 0.3, 2, 0
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn add_pattern_classifier(
        &mut self,
        name: &str,
        num_l: usize,
        num_s: usize,
        num_as: usize,
        perm_thr: u8,
        perm_inc: u8,
        perm_dec: u8,
        pct_pool: f64,
        pct_conn: f64,
        pct_learn: f64,
        num_t: usize,
        seed: u32,
    ) -> usize {
        let block = PatternClassifier::new(
            num_l, num_s, num_as, perm_thr, perm_inc, perm_dec, pct_pool, pct_conn, pct_learn,
            num_t, seed.into(),
        );
        let id = self.net.add(block);
        self.net.set_block_name(id, name);
        let handle = self.block_handles.len();
        self.block_handles.push((name.to_string(), id));
        handle
    }

    /// Add a SequenceLearner block for temporal sequence learning.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const learner = net.add_sequence_learner(
    ///     "Sequence Learner", 512, 4, 8, 32, 20, 20, 2, 1, 2, false, 42
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn add_sequence_learner(
        &mut self,
        name: &str,
        num_c: usize,
        num_spc: usize,
        num_dps: usize,
        num_rpd: usize,
        d_thresh: u32,
        perm_thr: u8,
        perm_inc: u8,
        perm_dec: u8,
        num_t: usize,
        always_update: bool,
        seed: u32,
    ) -> usize {
        let block = SequenceLearner::new(
            num_c, num_spc, num_dps, num_rpd, d_thresh, perm_thr, perm_inc, perm_dec, num_t,
            always_update, seed.into(),
        );
        let id = self.net.add(block);
        self.net.set_block_name(id, name);
        let handle = self.block_handles.len();
        self.block_handles.push((name.to_string(), id));
        handle
    }

    /// Add a ContextLearner block for contextual pattern recognition.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const learner = net.add_context_learner(
    ///     "Context Learner", 512, 4, 8, 32, 20, 20, 2, 1, 2, false, 42
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn add_context_learner(
        &mut self,
        name: &str,
        num_c: usize,
        num_spc: usize,
        num_dps: usize,
        num_rpd: usize,
        d_thresh: u32,
        perm_thr: u8,
        perm_inc: u8,
        perm_dec: u8,
        num_t: usize,
        always_update: bool,
        seed: u32,
    ) -> usize {
        let block = ContextLearner::new(
            num_c, num_spc, num_dps, num_rpd, d_thresh, perm_thr, perm_inc, perm_dec, num_t,
            always_update, seed.into(),
        );
        let id = self.net.add(block);
        self.net.set_block_name(id, name);
        let handle = self.block_handles.len();
        self.block_handles.push((name.to_string(), id));
        handle
    }

    /// Connect source block output to target block input.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// net.connect_to_input(encoder, pooler);
    /// ```
    pub fn connect_to_input(
        &mut self,
        source_handle: usize,
        target_handle: usize,
    ) -> Result<(), JsValue> {
        if source_handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid source handle"));
        }
        if target_handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid target handle"));
        }

        let source_id = self.block_handles[source_handle].1;
        let target_id = self.block_handles[target_handle].1;

        self.net
            .connect_to_input(source_id, target_id)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }

    /// Connect source block output to target block context input.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// net.connect_to_context(context_encoder, learner);
    /// ```
    pub fn connect_to_context(
        &mut self,
        source_handle: usize,
        target_handle: usize,
    ) -> Result<(), JsValue> {
        if source_handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid source handle"));
        }
        if target_handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid target handle"));
        }

        let source_id = self.block_handles[source_handle].1;
        let target_id = self.block_handles[target_handle].1;

        self.net
            .connect_to_context(source_id, target_id)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }

    /// Build the network (compute execution order).
    ///
    /// Must be called after adding all blocks and connections, before execute().
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// net.build();
    /// ```
    pub fn build(&mut self) -> Result<(), JsValue> {
        self.net
            .build()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }

    /// Initialize a learning block (allocates memory structures).
    ///
    /// Must be called for PatternPooler, PatternClassifier, SequenceLearner,
    /// and ContextLearner blocks before execution.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// net.init_block(pooler);
    /// net.init_block(learner);
    /// ```
    pub fn init_block(&mut self, handle: usize) -> Result<(), JsValue> {
        if handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid block handle"));
        }

        let block_id = self.block_handles[handle].1;

        // Try each block type that needs initialization
        if let Ok(block) = self.net.get_mut::<PatternPooler>(block_id) {
            return block
                .init()
                .map_err(|e| JsValue::from_str(&format!("{:?}", e)));
        }
        if let Ok(block) = self.net.get_mut::<PatternClassifier>(block_id) {
            return block
                .init()
                .map_err(|e| JsValue::from_str(&format!("{:?}", e)));
        }
        if let Ok(block) = self.net.get_mut::<SequenceLearner>(block_id) {
            return block
                .init()
                .map_err(|e| JsValue::from_str(&format!("{:?}", e)));
        }
        if let Ok(block) = self.net.get_mut::<ContextLearner>(block_id) {
            return block
                .init()
                .map_err(|e| JsValue::from_str(&format!("{:?}", e)));
        }

        // Transformers don't need initialization
        Ok(())
    }

    /// Start recording execution for visualization.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// net.start_recording();
    /// ```
    pub fn start_recording(&mut self) {
        self.net.start_recording();
    }

    /// Execute the network one step.
    ///
    /// # Arguments
    /// * `learn` - Whether to enable learning
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// net.execute(true);  // with learning
    /// net.execute(false); // inference only
    /// ```
    pub fn execute(&mut self, learn: bool) -> Result<(), JsValue> {
        self.net
            .execute(learn)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }

    /// Get current execution trace as JSON string.
    ///
    /// This stops and restarts recording to get a snapshot.
    /// Returns null if recording was not started.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const traceJson = net.get_trace_json();
    /// if (traceJson) {
    ///     const trace = JSON.parse(traceJson);
    ///     // Update visualization
    /// }
    /// ```
    pub fn get_trace_json(&mut self) -> Option<String> {
        if let Some(trace) = self.net.stop_recording() {
            let json = trace.to_json().ok()?;
            // Restart recording for next batch
            self.net.start_recording();
            Some(json)
        } else {
            None
        }
    }

    /// Set value for a ScalarTransformer block.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// net.set_scalar_value(encoder, 42.5);
    /// ```
    pub fn set_scalar_value(&mut self, handle: usize, value: f64) -> Result<(), JsValue> {
        if handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid block handle"));
        }

        let block_id = self.block_handles[handle].1;
        if let Ok(block) = self.net.get_mut::<ScalarTransformer>(block_id) {
            block.set_value(value);
            Ok(())
        } else {
            Err(JsValue::from_str("Block is not a ScalarTransformer"))
        }
    }

    /// Set value for a DiscreteTransformer block.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// net.set_discrete_value(encoder, 3);
    /// ```
    pub fn set_discrete_value(&mut self, handle: usize, value: usize) -> Result<(), JsValue> {
        if handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid block handle"));
        }

        let block_id = self.block_handles[handle].1;
        if let Ok(block) = self.net.get_mut::<DiscreteTransformer>(block_id) {
            block.set_value(value);
            Ok(())
        } else {
            Err(JsValue::from_str("Block is not a DiscreteTransformer"))
        }
    }

    /// Set label for a PatternClassifier block.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// net.set_classifier_label(classifier, 2);
    /// ```
    pub fn set_classifier_label(&mut self, handle: usize, label: usize) -> Result<(), JsValue> {
        if handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid block handle"));
        }

        let block_id = self.block_handles[handle].1;
        if let Ok(block) = self.net.get_mut::<PatternClassifier>(block_id) {
            block.set_label(label);
            Ok(())
        } else {
            Err(JsValue::from_str("Block is not a PatternClassifier"))
        }
    }

    /// Get anomaly score from a SequenceLearner or ContextLearner.
    ///
    /// Returns a value between 0.0 (expected) and 1.0 (anomalous).
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const anomaly = net.get_anomaly(learner);
    /// console.log(`Anomaly score: ${anomaly}`);
    /// ```
    pub fn get_anomaly(&self, handle: usize) -> Result<f64, JsValue> {
        if handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid block handle"));
        }

        let block_id = self.block_handles[handle].1;

        // Try SequenceLearner first
        if let Ok(block) = self.net.get::<SequenceLearner>(block_id) {
            return Ok(block.get_anomaly_score());
        }

        // Try ContextLearner
        if let Ok(block) = self.net.get::<ContextLearner>(block_id) {
            return Ok(block.get_anomaly_score());
        }

        Err(JsValue::from_str(
            "Block is not a SequenceLearner or ContextLearner",
        ))
    }

    /// Get classification probabilities from a PatternClassifier.
    ///
    /// Returns an array of probabilities (one per label).
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const probs = net.get_probabilities(classifier);
    /// console.log(`Class 0: ${probs[0]}, Class 1: ${probs[1]}`);
    /// ```
    pub fn get_probabilities(&self, handle: usize) -> Result<Vec<f64>, JsValue> {
        if handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid block handle"));
        }

        let block_id = self.block_handles[handle].1;
        if let Ok(block) = self.net.get::<PatternClassifier>(block_id) {
            Ok(block.get_probabilities())
        } else {
            Err(JsValue::from_str("Block is not a PatternClassifier"))
        }
    }

    /// Get the number of blocks in the network.
    pub fn num_blocks(&self) -> usize {
        self.block_handles.len()
    }

    /// Get the name of a block by handle.
    pub fn get_block_name(&self, handle: usize) -> Result<String, JsValue> {
        if handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid block handle"));
        }
        Ok(self.block_handles[handle].0.clone())
    }

    // ===== PHASE 2: Network Editing API =====

    /// Remove a block from the network.
    ///
    /// Note: All connections to/from this block must be removed first.
    /// The block handle will become invalid after removal.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// net.remove_block(block_handle);
    /// ```
    pub fn remove_block(&mut self, handle: usize) -> Result<(), JsValue> {
        if handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid block handle"));
        }

        let block_id = self.block_handles[handle].1;

        // Remove from network
        self.net
            .remove(block_id)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        // Mark handle as invalid (keep array size but clear entry)
        self.block_handles[handle] = ("<removed>".to_string(), BlockId::from_raw(u32::MAX));

        Ok(())
    }

    /// Remove a connection between two blocks.
    ///
    /// # Arguments
    /// * `source_handle` - Handle of the source block
    /// * `target_handle` - Handle of the target block
    /// * `connection_type` - "input" or "context"
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// net.remove_connection(encoder, pooler, "input");
    /// net.remove_connection(context_enc, learner, "context");
    /// ```
    pub fn remove_connection(
        &mut self,
        source_handle: usize,
        target_handle: usize,
        connection_type: &str,
    ) -> Result<(), JsValue> {
        if source_handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid source handle"));
        }
        if target_handle >= self.block_handles.len() {
            return Err(JsValue::from_str("Invalid target handle"));
        }

        let source_id = self.block_handles[source_handle].1;
        let target_id = self.block_handles[target_handle].1;

        // Remove connection from target's input or context
        match connection_type {
            "input" => {
                self.net
                    .disconnect_from_input(source_id, target_id)
                    .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
            }
            "context" => {
                self.net
                    .disconnect_from_context(source_id, target_id)
                    .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
            }
            _ => {
                return Err(JsValue::from_str(
                    "Invalid connection type. Use 'input' or 'context'",
                ));
            }
        }

        Ok(())
    }

    /// Export network configuration as JSON string.
    ///
    /// This saves the network topology and block parameters, but not learned state.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const configJson = net.export_config();
    /// localStorage.setItem('myNetwork', configJson);
    /// ```
    pub fn export_config(&self) -> Result<String, JsValue> {
        let config = self
            .net
            .to_config()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        config
            .to_json()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }

    /// Import network configuration from JSON string.
    ///
    /// This replaces the current network with the loaded configuration.
    /// All existing blocks and connections will be lost.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const configJson = localStorage.getItem('myNetwork');
    /// net.import_config(configJson);
    /// ```
    pub fn import_config(&mut self, config_json: &str) -> Result<(), JsValue> {
        use crate::network_config::NetworkConfig;

        let config = NetworkConfig::from_json(config_json)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        // Create new network from config
        let new_net = Network::from_config(&config)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        // Replace current network
        self.net = new_net;

        // Rebuild block handles
        self.block_handles.clear();
        for (i, block_info) in config.block_info.iter().enumerate() {
            let block_id = BlockId::from_raw(i as u32);
            self.block_handles
                .push((block_info.name.clone(), block_id));
        }

        Ok(())
    }

    /// Get list of all block handles, names, and types.
    ///
    /// Returns JSON array of {handle, name, type} objects.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const blocksJson = net.get_blocks_info();
    /// const blocks = JSON.parse(blocksJson);
    /// blocks.forEach(block => {
    ///     console.log(`${block.handle}: ${block.name} (${block.type})`);
    /// });
    /// ```
    pub fn get_blocks_info(&self) -> Result<String, JsValue> {
        let mut blocks = Vec::new();

        for (handle, (name, block_id)) in self.block_handles.iter().enumerate() {
            if name == "<removed>" {
                continue; // Skip removed blocks
            }

            // Determine block type by trying to get each type
            let block_type = if self.net.get::<ScalarTransformer>(*block_id).is_ok() {
                "ScalarTransformer"
            } else if self.net.get::<DiscreteTransformer>(*block_id).is_ok() {
                "DiscreteTransformer"
            } else if self.net.get::<PersistenceTransformer>(*block_id).is_ok() {
                "PersistenceTransformer"
            } else if self.net.get::<PatternPooler>(*block_id).is_ok() {
                "PatternPooler"
            } else if self.net.get::<PatternClassifier>(*block_id).is_ok() {
                "PatternClassifier"
            } else if self.net.get::<SequenceLearner>(*block_id).is_ok() {
                "SequenceLearner"
            } else if self.net.get::<ContextLearner>(*block_id).is_ok() {
                "ContextLearner"
            } else {
                "Unknown"
            };

            blocks.push(serde_json::json!({
                "handle": handle,
                "name": name,
                "type": block_type,
                "id": block_id.as_usize()
            }));
        }

        serde_json::to_string(&blocks).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Rebuild the network after modifications.
    ///
    /// This recomputes the execution order after adding/removing blocks or connections.
    /// Must be called before execute() after making changes.
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// net.remove_connection(encoder, pooler, "input");
    /// net.connect_to_input(encoder, classifier);
    /// net.rebuild();  // Recompute execution order
    /// ```
    pub fn rebuild(&mut self) -> Result<(), JsValue> {
        self.net
            .build()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }
}
