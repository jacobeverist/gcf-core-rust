# Interactive Network Editor Enhancement Plan

**Goal**: Enable users to create, modify, and delete blocks and connections in the visualization dashboard, both offline and during real-time simulation.

**Date**: 2025-10-23
**Last Updated**: 2025-10-24
**Status**: Phase 3 Partially Complete - Block Creation Working

---

## Overview

This enhancement will transform the visualization dashboard from a read-only viewer into a full interactive network editor. Users will be able to:

- **Add new blocks** with configurable parameters
- **Create connections** between blocks via drag-and-drop
- **Remove blocks and connections**
- **Edit block parameters** on-the-fly
- **Save/export** modified network configurations
- **Hot-swap** network components during simulation

---

## Architecture Components

### 1. UI Components

#### 1.1 Block Creation Palette
**Location**: Left sidebar or floating toolbar

**Features**:
- Visual palette showing all available block types
- Drag-and-drop to canvas to create new block
- Click to add at default position
- Icon + name for each block type
- Organized by category:
  - Transformers (Scalar, Discrete, Persistence)
  - Learning (PatternPooler, PatternClassifier)
  - Temporal (ContextLearner, SequenceLearner)

**UI Design**:
```
┌─────────────────────┐
│ Block Palette       │
├─────────────────────┤
│ TRANSFORMERS        │
│  [▲] Scalar         │
│  [▲] Discrete       │
│  [▲] Persistence    │
├─────────────────────┤
│ LEARNING            │
│  [▬] Pattern Pooler │
│  [▬] Classifier     │
├─────────────────────┤
│ TEMPORAL            │
│  [■] Context Learn  │
│  [■] Sequence Learn │
└─────────────────────┘
```

#### 1.2 Connection Creation Tool
**Interaction**: Port-to-port drag

**Features**:
- Click and drag from source output port
- Visual guide line follows cursor
- Highlight valid target ports (input/context)
- Snap to target port on mouse up
- Connection type auto-detected (input vs context)
- Invalid connections rejected with visual feedback

**States**:
```javascript
{
    mode: 'idle' | 'dragging-connection',
    sourceBlock: blockId,
    sourcePort: 'output',
    targetPort: 'input' | 'context' | null,
    validTargets: [blockId, ...],
    cursorPos: {x, y}
}
```

#### 1.3 Block Parameter Editor
**Trigger**: Double-click block or right-click → "Edit Parameters"

**Features**:
- Modal dialog with form fields
- Parameter validation (ranges, types)
- Live preview of changes (if offline)
- "Apply" button to commit changes
- "Cancel" to discard
- Parameter templates/presets

**Example for ScalarTransformer**:
```
┌─────────────────────────────────┐
│ Edit ScalarTransformer          │
├─────────────────────────────────┤
│ Name: [temperature_encoder]     │
│ Min Value: [0.0        ]        │
│ Max Value: [100.0      ]        │
│ Statelets: [2048       ]        │
│ Active Statelets: [256 ]        │
│ History Depth: [2      ]        │
│ Random Seed: [0        ]        │
├─────────────────────────────────┤
│ [Apply]  [Cancel]  [Templates▼] │
└─────────────────────────────────┘
```

#### 1.4 Context Menu Enhancements
**Current**: Right-click block → Add Plot

**New Options**:
- Add Plot (existing)
- Edit Parameters
- Delete Block
- Duplicate Block
- View Block Info
- Set as Root (for hierarchical layout)

#### 1.5 Toolbar Actions
**Location**: Top of network panel

**Buttons**:
- 🖱️ Select Mode (default)
- ➕ Add Block Mode
- 🔗 Add Connection Mode
- 🗑️ Delete Mode
- 💾 Save Network Config
- 📂 Load Network Config
- ↩️ Undo
- ↪️ Redo
- 🔄 Reset Layout

---

### 2. WASM API Extensions

#### 2.1 Network Modification API

**Required Rust Functions** (expose to JavaScript):

```rust
// In wasm_interface.rs

#[wasm_bindgen]
impl WasmNetwork {
    /// Add a new block to the network
    pub fn add_block(&mut self,
        block_type: String,
        block_name: String,
        config_json: String
    ) -> Result<u32, JsValue> {
        // Parse block type and config
        // Create block instance
        // Add to network
        // Return block ID
    }

    /// Remove a block from the network
    pub fn remove_block(&mut self, block_id: u32) -> Result<(), JsValue> {
        // Check if block exists
        // Remove all connections to/from block
        // Remove from network
        // Update dependency graph
    }

    /// Add connection between blocks
    pub fn add_connection(&mut self,
        source_block: u32,
        target_block: u32,
        connection_type: String  // "input" or "context"
    ) -> Result<(), JsValue> {
        // Validate blocks exist
        // Validate connection type
        // Add connection
        // Rebuild network if needed
    }

    /// Remove connection
    pub fn remove_connection(&mut self,
        source_block: u32,
        target_block: u32,
        connection_type: String
    ) -> Result<(), JsValue> {
        // Remove connection
        // Update network topology
    }

    /// Update block parameters
    pub fn update_block_params(&mut self,
        block_id: u32,
        params_json: String
    ) -> Result<(), JsValue> {
        // Validate parameters
        // Update block configuration
        // Reinitialize block if needed
    }

    /// Rebuild network after modifications
    pub fn rebuild(&mut self) -> Result<(), JsValue> {
        // Recompute topology
        // Reinitialize modified blocks
        // Update execution order
    }

    /// Get current network configuration
    pub fn get_network_config(&self) -> Result<String, JsValue> {
        // Export NetworkConfig as JSON
    }

    /// Load network from configuration
    pub fn load_network_config(&mut self, config_json: String) -> Result<(), JsValue> {
        // Parse NetworkConfig
        // Rebuild network from config
        // Initialize all blocks
    }
}
```

#### 2.2 Block Configuration Templates

**Create block config builders in Rust**:

```rust
// In network_config.rs

impl BlockConfig {
    pub fn scalar_transformer_default() -> Self {
        BlockConfig::ScalarTransformer {
            min_val: 0.0,
            max_val: 100.0,
            num_s: 2048,
            num_as: 256,
            num_t: 2,
            seed: 0,
        }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self)
    }
}
```

---

### 3. State Management

#### 3.1 Editor State

**JavaScript state object**:

```javascript
const editorState = {
    mode: 'select',  // 'select', 'add-block', 'add-connection', 'delete'
    selectedBlocks: new Set(),
    selectedConnections: new Set(),
    clipboard: null,
    history: {
        undoStack: [],
        redoStack: [],
        maxSize: 50
    },
    isDirty: false,  // Has unsaved changes
    isSimulationRunning: false
};
```

#### 3.2 Undo/Redo System

**Operation types**:
```javascript
const operations = {
    ADD_BLOCK: { type, blockId, config },
    REMOVE_BLOCK: { type, blockId, config, connections },
    ADD_CONNECTION: { type, source, target, connType },
    REMOVE_CONNECTION: { type, source, target, connType },
    UPDATE_PARAMS: { type, blockId, oldParams, newParams },
    MOVE_BLOCK: { type, blockId, oldPos, newPos }
};
```

**Implementation**:
```javascript
function executeOperation(op) {
    // Apply operation
    // Push to undo stack
    // Clear redo stack
    // Update network
}

function undo() {
    // Pop from undo stack
    // Execute inverse operation
    // Push to redo stack
}

function redo() {
    // Pop from redo stack
    // Execute operation
    // Push to undo stack
}
```

#### 3.3 Auto-save and Export

**Features**:
- Auto-save to browser localStorage every N seconds
- Export to JSON file (NetworkConfig format)
- Import from JSON file
- Warning on page close if unsaved changes

---

### 4. Real-Time Modification Strategy

#### 4.1 Offline Mode (Simulation Stopped)
**Safe Operations**: All modifications allowed

**Workflow**:
1. User adds/removes blocks/connections
2. Changes reflected in UI immediately
3. Network rebuilt when user clicks "Apply" or "Start Simulation"
4. All blocks initialized with new topology

#### 4.2 Hot-Swap Mode (Simulation Running)
**Challenging**: Must maintain consistency

**Approach 1: Pause-Modify-Resume** (Recommended for v1)
1. User initiates modification
2. Pause simulation automatically
3. Apply changes and rebuild network
4. Resume simulation from current state

**Approach 2: Live Hot-Swap** (Advanced, future enhancement)
1. User adds block → Block created but not connected to execution
2. User adds connection → Connection queued for next step boundary
3. At step boundary, apply queued changes atomically
4. Continue simulation with new topology

**Constraints for Hot-Swap**:
- Cannot remove blocks that have dependents (must remove dependents first)
- Cannot modify parameters of blocks mid-step
- New blocks start with empty state
- Connections added at step boundary only

#### 4.3 Validation Rules

**Before Adding Block**:
- ✅ Block name is unique
- ✅ Parameters are valid (ranges, types)
- ✅ Block ID doesn't conflict

**Before Adding Connection**:
- ✅ Source block has output
- ✅ Target block has appropriate input (input/context)
- ✅ No circular dependencies created
- ✅ Connection doesn't already exist

**Before Removing Block**:
- ✅ All connections to/from block removed first OR
- ✅ Cascade delete all dependent connections

**Before Removing Connection**:
- ✅ Target block can function without this input (may need check)

---

## Implementation Phases

### Phase 1: UI Foundation (Week 1)
**Goal**: Basic editor UI without WASM integration

**Tasks**:
1. Create block palette component
2. Implement drag-and-drop for blocks
3. Add connection creation tool (visual only)
4. Create parameter editor modal
5. Implement context menu enhancements
6. Add toolbar with mode buttons
7. Visual feedback for selection, hover, invalid operations

**Deliverable**: Static UI that allows creating visual representations of blocks/connections

---

### Phase 2: WASM API (Week 2)
**Goal**: Rust backend support for network modifications

**Tasks**:
1. Implement `add_block()` in WasmNetwork
2. Implement `remove_block()`
3. Implement `add_connection()`
4. Implement `remove_connection()`
5. Implement `update_block_params()`
6. Implement `rebuild()`
7. Add network config export/import
8. Create block config JSON parsers
9. Add comprehensive error handling

**Deliverable**: WASM API that can modify network structure

---

### Phase 3: Integration (Week 3) - ✅ PARTIALLY COMPLETE
**Goal**: Connect UI to WASM API for offline editing

**Tasks**:
1. ✅ Wire block palette to `add_block()` API - **COMPLETE**
   - Click handlers on palette items
   - Parameter modal workflow
   - Dynamic form generation for all block types
2. ⏸️ Wire connection tool to `add_connection()` API - **PENDING**
   - Connection tool UI exists (Phase 1)
   - WASM API exists
   - Integration not yet implemented
3. ⏸️ Wire parameter editor to `update_block_params()` - **PENDING**
   - Currently only supports creation, not editing existing blocks
4. ⏸️ Wire delete operations to remove APIs - **PENDING**
5. ✅ Implement network rebuild on "Apply" - **COMPLETE**
   - Calls `rebuild()` after block creation
   - Updates block counter
   - Refreshes visualization
6. ✅ Add validation and error messages - **COMPLETE**
   - Parameter validation in forms
   - Error handling for init_block failures
   - Console logging for debugging
7. ⚠️ Test all CRUD operations offline - **PARTIAL**
   - **75/78 tests passing** (96% pass rate)
   - Single block creation fully working
   - Multi-block creation has issues (3 failing tests)

**Status**: Block creation via palette click is fully functional. Users can create individual blocks with custom parameters. Connection tool, editing, and deletion remain to be implemented.

**Known Issues**:
- Creating multiple blocks in quick succession may fail (rebuild() timing issue)
- Learning blocks (PatternPooler) without connections show as created but not visualized

---

### Phase 4: State Management (Week 4)
**Goal**: Undo/redo, save/load, auto-save

**Tasks**:
1. Implement operation history system
2. Add undo/redo functionality
3. Implement save to localStorage
4. Add export to JSON file
5. Add import from JSON file
6. Add unsaved changes warning
7. Implement clipboard (copy/paste blocks)

**Deliverable**: Production-ready editor with full state management

---

### Phase 5: Real-Time Editing (Week 5)
**Goal**: Modify network during simulation

**Tasks**:
1. Implement pause-modify-resume workflow
2. Add state preservation during rebuild
3. Handle simulation restart after changes
4. Add visual indicators for simulation state
5. Test hot-swap scenarios
6. Add safety checks and validation
7. Performance testing with large networks

**Deliverable**: Real-time network editing capability

---

### Phase 6: Polish & Advanced Features (Week 6)
**Goal**: UX improvements and advanced features

**Tasks**:
1. Add keyboard shortcuts (Ctrl+Z, Ctrl+C, Delete, etc.)
2. Implement block templates/presets
3. Add multi-select for batch operations
4. Implement align/distribute tools
5. Add connection routing hints
6. Improve visual feedback and animations
7. Add tooltips and help system
8. Performance optimization for large networks
9. Accessibility improvements

**Deliverable**: Polished, user-friendly editor

---

## Technical Considerations

### 1. Block ID Management
- **Current**: Block IDs assigned sequentially at network creation
- **New**: Need dynamic ID allocation
- **Solution**: Track max ID, increment for new blocks, reuse IDs carefully

### 2. Execution Order
- **Current**: Computed once at network build
- **New**: Recompute after topology changes
- **Solution**: Use existing dependency graph in Network::build()

### 3. BlockOutput Sharing
- **Current**: Blocks share outputs via `Rc<RefCell<BlockOutput>>`
- **New**: New connections must clone existing Rc
- **Challenge**: Track output references correctly

### 4. Initialization
- **Current**: All blocks initialized before first step
- **New**: New blocks initialized when added
- **Challenge**: Blocks need correct input sizes during init

### 5. Memory Management
- **Current**: Blocks live for entire session
- **New**: Blocks may be removed dynamically
- **Solution**: Rust Drop trait handles cleanup, but need to remove from Network

### 6. Serialization
- **Current**: NetworkConfig supports full topology
- **New**: Use existing infrastructure for save/load
- **Advantage**: Already implemented and tested!

---

## UI/UX Design Principles

### 1. Visual Feedback
- **Hover states**: Highlight valid drop targets
- **Selection**: Clear visual indication of selected elements
- **Validation**: Red outline for invalid operations
- **Success/Error**: Toast notifications for operations

### 2. Discoverability
- **Tooltips**: Explain each tool and button
- **Context menus**: Right-click for common operations
- **Keyboard shortcuts**: Display in tooltips
- **Help panel**: Toggle-able help overlay

### 3. Safety
- **Confirmation dialogs**: Before destructive operations
- **Undo/redo**: Always available escape hatch
- **Auto-save**: Prevent data loss
- **Validation**: Prevent invalid states

### 4. Performance
- **Debouncing**: Limit rebuild frequency
- **Lazy rendering**: Only update changed elements
- **Progress indicators**: For long operations
- **Responsive UI**: Keep 60 FPS during editing

---

## Testing Strategy

### Unit Tests
- Block creation with various configs
- Connection validation logic
- Undo/redo operation correctness
- Network rebuild with modified topology

### Integration Tests
- Add block → connect → simulate
- Remove block with connections
- Modify parameters → verify behavior
- Save/load round-trip

### UI Tests
- Drag-and-drop workflow
- Parameter form validation
- Keyboard shortcut handling
- Modal dialog interactions

### Performance Tests
- Large network editing (100+ blocks)
- Rapid successive operations
- Memory leak detection
- Simulation restart time

### User Acceptance Tests
- Can user add transformer and connect to pooler?
- Can user modify parameter and see effect?
- Can user save and reload custom network?
- Can user undo mistakes?

---

## Success Metrics

### Functionality
- ✅ All block types can be added via UI
- ✅ Connections created correctly (input/context)
- ✅ Parameters validated and applied
- ✅ Delete operations work without crashes
- ✅ Undo/redo works for all operations
- ✅ Save/load preserves network state
- ✅ Real-time editing works during simulation

### Performance
- ✅ Add block: < 100ms
- ✅ Add connection: < 50ms
- ✅ Network rebuild: < 500ms (50 blocks)
- ✅ UI remains responsive (60 FPS)
- ✅ No memory leaks over 1000 operations

### Usability
- ✅ New users can add block in < 30 seconds
- ✅ Creating network of 10 blocks takes < 5 minutes
- ✅ Zero data loss from crashes/closes
- ✅ Clear error messages for invalid operations
- ✅ Keyboard shortcuts work as expected

---

## Future Enhancements (Post-MVP)

### Advanced Features
1. **Subnetworks/Groups**: Group blocks into reusable modules
2. **Templates Library**: Save/load common network patterns
3. **Visual Programming**: Block code/logic editor
4. **Parameter Tuning**: Sliders for real-time parameter adjustment
5. **A/B Testing**: Compare multiple network configurations
6. **Version Control**: Git-like branching for network experiments

### Collaborative Features
1. **Multi-user Editing**: Real-time collaboration
2. **Commenting**: Annotate blocks and connections
3. **Sharing**: Publish networks to gallery
4. **Forking**: Build on others' networks

### Analysis Tools
1. **Performance Profiler**: Identify bottlenecks per block
2. **Data Flow Visualization**: Animate data through network
3. **Anomaly Inspector**: Drill into anomaly scores
4. **Learning Curves**: Track learning progress over time

---

## Risk Mitigation

### Risk 1: Real-Time Editing Complexity
**Mitigation**: Start with pause-modify-resume, defer true hot-swap to future

### Risk 2: Network Corruption
**Mitigation**: Extensive validation, auto-save, version history

### Risk 3: Performance Degradation
**Mitigation**: Profiling, optimization, lazy updates, debouncing

### Risk 4: Poor UX
**Mitigation**: User testing, iterative design, tooltips, help system

### Risk 5: WASM API Instability
**Mitigation**: Comprehensive error handling, fallback modes, logging

---

## Dependencies

### Rust Crates
- **serde/serde_json**: Already used for NetworkConfig serialization
- **wasm-bindgen**: Already integrated for WASM interface

### JavaScript Libraries
- **D3.js**: Already used for visualization
- **No additional dependencies needed**

### Browser APIs
- **localStorage**: For auto-save
- **File API**: For save/load JSON files
- **Drag and Drop API**: For block palette

---

## Documentation Plan

### User Documentation
1. **Tutorial**: Step-by-step network building guide
2. **Reference**: Complete block parameter documentation
3. **Examples**: Pre-built network templates
4. **FAQ**: Common questions and troubleshooting

### Developer Documentation
1. **WASM API Reference**: All exposed functions
2. **Architecture Guide**: How editor integrates with network
3. **Extension Guide**: How to add new block types
4. **Testing Guide**: How to test network modifications

---

## Conclusion

This enhancement will transform the Gnomics visualization dashboard into a powerful interactive network editor, enabling:

- **Rapid prototyping**: Build networks visually without writing code
- **Experimentation**: Modify and test architectures in real-time
- **Learning**: Understand network behavior through hands-on interaction
- **Productivity**: Reduce time from idea to working network

**Estimated Timeline**: 6 weeks (phases can overlap)
**Risk Level**: Medium (real-time editing is complex)
**Impact**: High (major usability improvement)
**Priority**: High (enables key use cases)

**Next Steps**:
1. Review and approve plan
2. Set up project tracking (issues, milestones)
3. Begin Phase 1: UI Foundation
4. Iterate based on user feedback

---

**Document Version**: 1.0
**Last Updated**: 2025-10-23
**Status**: Ready for Review
