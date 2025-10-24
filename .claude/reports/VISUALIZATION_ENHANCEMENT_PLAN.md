# Visualization Enhancement Implementation Plan

## Overview

This document outlines the implementation plan for enhancing the WASM live visualizer with advanced features including input/output plotting, improved block representations, hierarchical layout, and interactive tooltips.

## Current State

**Completed Features:**
- ✅ Real-time network execution
- ✅ Force-directed network graph
- ✅ BitField heatmap visualization
- ✅ Basic metrics (FPS, step count)
- ✅ Demo-specific metrics (anomaly scores, class predictions)
- ✅ 4 demo networks

**Missing Features:**
- ❌ Input/output value plotting
- ❌ Block type visual distinctions
- ❌ Hierarchical layout
- ❌ Interactive tooltips (blocks and connections)
- ❌ Click-to-highlight functionality
- ❌ Directional arrows on edges
- ❌ Connection configuration display

## Enhancement Goals

1. **Data Capture**: Record and visualize scalar/discrete values flowing through the network
2. **Visual Hierarchy**: Organize blocks in meaningful layouts based on data flow
3. **Block Differentiation**: Make different block types visually distinct
4. **Interactivity**: Add tooltips, highlighting, and click interactions
5. **Real-time Plotting**: Show input/output values as time-series charts
6. **Connection Details**: Display connection metadata and directionality

---

## Implementation Phases

### Phase 1: Data Capture Enhancement (Backend)
**Goal**: Extend trace recording to capture input/output values

**Estimated Effort**: 4-6 hours

#### Task 1.1: Extend ExecutionTrace Structure
**File**: `src/execution_recorder.rs`

Add new fields to capture block input/output values:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockStepData {
    pub num_bits: usize,
    pub active_bits: Vec<usize>,
    pub num_active: usize,

    // NEW: Input/output values
    pub scalar_input: Option<f64>,
    pub discrete_input: Option<usize>,
    pub scalar_output: Option<f64>,
    pub discrete_output: Option<usize>,
    pub classifier_label: Option<usize>,
    pub classifier_probs: Option<Vec<usize>>,
    pub anomaly_score: Option<f64>,
}
```

**Implementation Steps:**
1. Add new fields to `BlockStepData` struct
2. Update serialization/deserialization
3. Modify `ExecutionRecorder::record_step()` to capture values
4. Add getter methods for each block type to expose values

#### Task 1.2: Modify Block Implementations
**Files**:
- `src/blocks/scalar_transformer.rs`
- `src/blocks/discrete_transformer.rs`
- `src/blocks/pattern_classifier.rs`
- `src/blocks/sequence_learner.rs`
- `src/blocks/context_learner.rs`

Add methods to expose current input/output values:

```rust
// ScalarTransformer
pub fn get_current_value(&self) -> f64 {
    self.value
}

// DiscreteTransformer
pub fn get_current_value(&self) -> usize {
    self.value
}

// PatternClassifier
pub fn get_current_label(&self) -> Option<usize> {
    self.label
}

pub fn get_probabilities(&self) -> Vec<usize> {
    self.probs.clone()
}

// SequenceLearner / ContextLearner
pub fn get_anomaly_score(&self) -> f64 {
    // Already implemented
}
```

#### Task 1.3: Update WASM Interface
**File**: `src/wasm_interface.rs`

Expose new data retrieval methods:

```rust
impl WasmNetwork {
    // Get scalar input value
    pub fn get_scalar_input(&self, handle: usize) -> Result<f64, JsValue> {
        // Implementation
    }

    // Get discrete input value
    pub fn get_discrete_input(&self, handle: usize) -> Result<usize, JsValue> {
        // Implementation
    }

    // Get classifier probabilities (already exists)
    // Get anomaly score (already exists)
}
```

**Subtasks:**
1. Add getter methods for all value types
2. Update trace JSON to include new fields
3. Test data capture with all block types
4. Verify JSON output includes new data

**Acceptance Criteria:**
- ✅ Trace JSON includes scalar/discrete values at each step
- ✅ All block types expose relevant values
- ✅ WASM interface provides access to captured data
- ✅ No performance degradation

---

### Phase 2: Visual Enhancements (Frontend)
**Goal**: Improve block and connection visualization

**Estimated Effort**: 6-8 hours

#### Task 2.1: Block Type Visual Distinctions
**File**: `visualization/viewer_live.html`

Create distinct visual styles for each block type:

```javascript
const blockStyles = {
    'ScalarTransformer': {
        fill: '#4a9eff',
        stroke: '#3a7ecf',
        shape: 'circle',
        icon: 'S'
    },
    'DiscreteTransformer': {
        fill: '#ff9a4a',
        stroke: '#cf7a2a',
        shape: 'circle',
        icon: 'D'
    },
    'PatternPooler': {
        fill: '#4aff4a',
        stroke: '#2acf2a',
        shape: 'rect',
        icon: 'P'
    },
    'PatternClassifier': {
        fill: '#ff4aff',
        stroke: '#cf2acf',
        shape: 'rect',
        icon: 'C'
    },
    'SequenceLearner': {
        fill: '#ffff4a',
        stroke: '#cfcf2a',
        shape: 'diamond',
        icon: 'SL'
    },
    'ContextLearner': {
        fill: '#4affff',
        stroke: '#2acfcf',
        shape: 'diamond',
        icon: 'CL'
    }
};

function drawNode(selection, d) {
    const style = blockStyles[d.type] || blockStyles.default;

    // Draw shape based on type
    if (style.shape === 'circle') {
        selection.append('circle')
            .attr('r', 30)
            .attr('fill', style.fill)
            .attr('stroke', style.stroke);
    } else if (style.shape === 'rect') {
        selection.append('rect')
            .attr('x', -30)
            .attr('y', -30)
            .attr('width', 60)
            .attr('height', 60)
            .attr('rx', 8)
            .attr('fill', style.fill)
            .attr('stroke', style.stroke);
    } else if (style.shape === 'diamond') {
        selection.append('polygon')
            .attr('points', '0,-35 35,0 0,35 -35,0')
            .attr('fill', style.fill)
            .attr('stroke', style.stroke);
    }

    // Add type icon
    selection.append('text')
        .attr('text-anchor', 'middle')
        .attr('dy', 5)
        .text(style.icon)
        .style('font-size', '14px')
        .style('font-weight', 'bold')
        .style('fill', 'white');
}
```

**Subtasks:**
1. Define color schemes for each block type
2. Create shape variations (circle, rectangle, diamond)
3. Add type icons or labels
4. Update CSS for new styles
5. Ensure good contrast and accessibility

#### Task 2.2: Hierarchical Layout
**File**: `visualization/viewer_live.html`

Replace force-directed layout with hierarchical layout:

```javascript
function drawNetworkGraphHierarchical() {
    // Compute node levels based on topological sort
    const levels = computeHierarchicalLevels(nodes, links);

    // Position nodes in layers
    const width = document.getElementById('network-panel').clientWidth;
    const height = document.getElementById('network-panel').clientHeight;

    const levelHeight = height / (levels.length + 1);

    nodes.forEach((node, i) => {
        const level = levels[node.id];
        const nodesInLevel = nodes.filter(n => levels[n.id] === level);
        const indexInLevel = nodesInLevel.indexOf(node);

        node.fx = width / (nodesInLevel.length + 1) * (indexInLevel + 1);
        node.fy = levelHeight * (level + 1);
    });

    // Use force simulation only for fine-tuning
    const simulation = d3.forceSimulation(nodes)
        .force('link', d3.forceLink(links).distance(levelHeight * 0.8))
        .force('collision', d3.forceCollide().radius(50))
        .alpha(0.1) // Low alpha for subtle adjustments
        .alphaDecay(0.05);
}

function computeHierarchicalLevels(nodes, links) {
    const levels = {};
    const visited = new Set();

    // Find root nodes (no incoming edges)
    const roots = nodes.filter(n =>
        !links.some(l => l.target === n.id)
    );

    // BFS to assign levels
    const queue = roots.map(n => ({ node: n, level: 0 }));

    while (queue.length > 0) {
        const { node, level } = queue.shift();

        if (visited.has(node.id)) continue;
        visited.add(node.id);
        levels[node.id] = level;

        // Add children
        const children = links
            .filter(l => l.source === node.id)
            .map(l => nodes.find(n => n.id === l.target));

        children.forEach(child => {
            queue.push({ node: child, level: level + 1 });
        });
    }

    return levels;
}
```

**Subtasks:**
1. Implement topological sort for network
2. Assign nodes to vertical layers
3. Distribute nodes horizontally within layers
4. Add option to toggle between force and hierarchical layouts
5. Animate transition between layouts

#### Task 2.3: Directional Arrows Enhancement
**File**: `visualization/viewer_live.html`

Improve arrow visibility and styling:

```javascript
// Enhanced arrowhead with better visibility
svg.append('defs').append('marker')
    .attr('id', 'arrowhead-input')
    .attr('viewBox', '-10 -5 10 10')
    .attr('refX', -8)
    .attr('refY', 0)
    .attr('markerWidth', 8)
    .attr('markerHeight', 8)
    .attr('orient', 'auto')
    .append('path')
    .attr('d', 'M-10,-5L0,0L-10,5Z')
    .attr('fill', '#4a9eff');

// Context connection arrows
svg.append('defs').append('marker')
    .attr('id', 'arrowhead-context')
    .attr('viewBox', '-10 -5 10 10')
    .attr('refX', -8)
    .attr('refY', 0)
    .attr('markerWidth', 8)
    .attr('markerHeight', 8)
    .attr('orient', 'auto')
    .append('path')
    .attr('d', 'M-10,-5L0,0L-10,5Z')
    .attr('fill', '#ff9a4a');

// Update link styling
link.attr('marker-end', d =>
    d.type === 'context' ? 'url(#arrowhead-context)' : 'url(#arrowhead-input)'
);
```

**Subtasks:**
1. Create distinct arrow styles for input/context connections
2. Adjust arrow size and positioning
3. Add animated flow indicators (optional)
4. Ensure arrows scale properly with zoom

**Acceptance Criteria:**
- ✅ Each block type has distinct visual appearance
- ✅ Network displays in hierarchical layout
- ✅ Arrows clearly show data flow direction
- ✅ Layout is intuitive and readable

---

### Phase 3: Interactive Tooltips and Highlighting
**Goal**: Add rich interactivity to the visualization

**Estimated Effort**: 5-7 hours

#### Task 3.1: Block Tooltips
**File**: `visualization/viewer_live.html`

Add comprehensive tooltips on block hover:

```javascript
// Add tooltip container to HTML
const tooltip = d3.select('body')
    .append('div')
    .attr('class', 'block-tooltip')
    .style('opacity', 0);

// Tooltip content generator
function getBlockTooltipContent(d) {
    const blockInfo = getBlockInfo(d.id); // From WASM

    return `
        <div class="tooltip-header">${d.name}</div>
        <div class="tooltip-type">${d.type}</div>
        <div class="tooltip-config">
            ${formatBlockConfig(d.type, blockInfo)}
        </div>
        <div class="tooltip-metrics">
            ${formatBlockMetrics(d.type, blockInfo)}
        </div>
    `;
}

function formatBlockConfig(type, info) {
    switch(type) {
        case 'ScalarTransformer':
            return `
                Range: ${info.min_val} - ${info.max_val}<br>
                Statelets: ${info.num_s}<br>
                Active: ${info.num_as}<br>
                History: ${info.num_t}
            `;
        case 'PatternPooler':
            return `
                Dendrites: ${info.num_s}<br>
                Winners: ${info.num_as}<br>
                Perm Threshold: ${info.perm_thr}<br>
                Learning Rate: ${info.pct_learn}
            `;
        // ... other block types
    }
}

// Add hover events
node.on('mouseover', function(event, d) {
    tooltip.transition()
        .duration(200)
        .style('opacity', 0.9);
    tooltip.html(getBlockTooltipContent(d))
        .style('left', (event.pageX + 10) + 'px')
        .style('top', (event.pageY - 28) + 'px');
})
.on('mouseout', function(d) {
    tooltip.transition()
        .duration(500)
        .style('opacity', 0);
});
```

**CSS for tooltip:**
```css
.block-tooltip {
    position: absolute;
    background: rgba(42, 42, 42, 0.95);
    border: 2px solid #4a9eff;
    border-radius: 6px;
    padding: 12px;
    pointer-events: none;
    font-size: 12px;
    z-index: 1000;
    max-width: 300px;
}

.tooltip-header {
    font-weight: 600;
    font-size: 14px;
    color: #4a9eff;
    margin-bottom: 4px;
}

.tooltip-type {
    font-size: 11px;
    color: #888;
    margin-bottom: 8px;
}

.tooltip-config {
    color: #e0e0e0;
    line-height: 1.4;
}

.tooltip-metrics {
    margin-top: 8px;
    padding-top: 8px;
    border-top: 1px solid #4a4a4a;
    color: #4aff4a;
}
```

#### Task 3.2: Connection Tooltips
**File**: `visualization/viewer_live.html`

Add tooltips for edges showing connection details:

```javascript
function getConnectionTooltipContent(d) {
    const sourceBlock = nodes.find(n => n.id === d.source.id);
    const targetBlock = nodes.find(n => n.id === d.target.id);

    return `
        <div class="tooltip-header">Connection</div>
        <div class="tooltip-detail">
            <strong>Type:</strong> ${d.type === 'context' ? 'Context' : 'Input'}<br>
            <strong>Source:</strong> ${sourceBlock.name}<br>
            <strong>Target:</strong> ${targetBlock.name}<br>
            <strong>Time Offset:</strong> ${d.time_offset || 0}<br>
            <strong>Data Flow:</strong> ${sourceBlock.type} → ${targetBlock.type}
        </div>
    `;
}

link.on('mouseover', function(event, d) {
    // Highlight connection
    d3.select(this)
        .attr('stroke-width', 4)
        .attr('stroke', d.type === 'context' ? '#ff9a4a' : '#4a9eff');

    tooltip.transition()
        .duration(200)
        .style('opacity', 0.9);
    tooltip.html(getConnectionTooltipContent(d))
        .style('left', (event.pageX + 10) + 'px')
        .style('top', (event.pageY - 28) + 'px');
})
.on('mouseout', function(event, d) {
    // Reset connection style
    d3.select(this)
        .attr('stroke-width', 2)
        .attr('stroke', d.type === 'context' ? '#ff9a4a' : '#555');

    tooltip.transition()
        .duration(500)
        .style('opacity', 0);
});
```

#### Task 3.3: Click-to-Highlight Functionality
**File**: `visualization/viewer_live.html`

Implement click selection with right-panel highlighting:

```javascript
let selectedBlockId = null;

node.on('click', function(event, d) {
    event.stopPropagation();

    // Toggle selection
    if (selectedBlockId === d.id) {
        selectedBlockId = null;
        unhighlightAll();
    } else {
        selectedBlockId = d.id;
        highlightBlock(d.id);
    }
});

function highlightBlock(blockId) {
    // Highlight node
    node.selectAll('circle, rect, polygon')
        .attr('stroke-width', n => n.id === blockId ? 4 : 2)
        .attr('filter', n => n.id === blockId ? 'url(#glow)' : null);

    // Scroll to and highlight corresponding BitField
    const bitfieldBlock = document.querySelector(`[data-block-id="${blockId}"]`);
    if (bitfieldBlock) {
        bitfieldBlock.scrollIntoView({ behavior: 'smooth', block: 'center' });
        bitfieldBlock.classList.add('highlighted');

        // Remove highlight after animation
        setTimeout(() => {
            bitfieldBlock.classList.remove('highlighted');
        }, 2000);
    }
}

function unhighlightAll() {
    node.selectAll('circle, rect, polygon')
        .attr('stroke-width', 2)
        .attr('filter', null);
}

// Add glow filter for selection
svg.append('defs').append('filter')
    .attr('id', 'glow')
    .html(`
        <feGaussianBlur stdDeviation="4" result="coloredBlur"/>
        <feMerge>
            <feMergeNode in="coloredBlur"/>
            <feMergeNode in="SourceGraphic"/>
        </feMerge>
    `);

// Click background to deselect
svg.on('click', function() {
    selectedBlockId = null;
    unhighlightAll();
});
```

**CSS for highlighting:**
```css
.bitfield-block.highlighted {
    animation: highlight-pulse 2s ease-out;
    border: 2px solid #4a9eff;
}

@keyframes highlight-pulse {
    0%, 100% { box-shadow: 0 0 0 0 rgba(74, 158, 255, 0); }
    50% { box-shadow: 0 0 20px 5px rgba(74, 158, 255, 0.5); }
}
```

**Subtasks:**
1. Implement tooltip system for blocks
2. Add tooltip system for connections
3. Create click selection mechanism
4. Add scroll-to and highlight effects
5. Add keyboard shortcuts (ESC to deselect)
6. Ensure tooltips don't obstruct view

**Acceptance Criteria:**
- ✅ Hovering shows detailed block configuration
- ✅ Hovering shows connection details
- ✅ Clicking block highlights corresponding BitField
- ✅ Selection state is clear and intuitive
- ✅ Tooltips are informative and well-formatted

---

### Phase 4: Real-Time Value Plotting
**Goal**: Add time-series plots for inputs and outputs

**Estimated Effort**: 8-10 hours

#### Task 4.1: Data Structure for Time Series
**File**: `visualization/viewer_live.html`

Create data structures to store historical values:

```javascript
// Global state for time series data
const timeSeriesData = {
    blocks: {},  // blockId -> { values: [], timestamps: [] }
    maxPoints: 200  // Keep last 200 points
};

function updateTimeSeriesData(blockId, value, timestamp) {
    if (!timeSeriesData.blocks[blockId]) {
        timeSeriesData.blocks[blockId] = {
            values: [],
            timestamps: []
        };
    }

    const data = timeSeriesData.blocks[blockId];
    data.values.push(value);
    data.timestamps.push(timestamp);

    // Keep only recent data
    if (data.values.length > timeSeriesData.maxPoints) {
        data.values.shift();
        data.timestamps.shift();
    }
}
```

#### Task 4.2: Plot Panel Layout
**File**: `visualization/viewer_live.html`

Restructure the right panel to include plots:

```html
<!-- Update HTML structure -->
<div id="bitfield-panel">
    <!-- New section for plots -->
    <div id="plots-section">
        <h3>Input/Output Values</h3>
        <div id="plots-container"></div>
    </div>

    <!-- Existing BitField section -->
    <div id="bitfields-section">
        <h3>BitField States</h3>
        <div id="bitfields-container"></div>
    </div>
</div>
```

**CSS:**
```css
#plots-section {
    margin-bottom: 30px;
    background: #2a2a2a;
    border-radius: 6px;
    padding: 15px;
}

#plots-section h3 {
    font-size: 14px;
    color: #4a9eff;
    margin-bottom: 10px;
}

.plot-container {
    margin-bottom: 20px;
    background: #1e1e1e;
    border-radius: 4px;
    padding: 10px;
}

.plot-title {
    font-size: 12px;
    color: #888;
    margin-bottom: 5px;
}

.plot-svg {
    width: 100%;
    height: 150px;
}
```

#### Task 4.3: D3.js Line Charts
**File**: `visualization/viewer_live.html`

Implement real-time line charts using D3:

```javascript
function createPlot(blockId, blockName, valueType) {
    const container = d3.select('#plots-container')
        .append('div')
        .attr('class', 'plot-container')
        .attr('data-block-id', blockId);

    container.append('div')
        .attr('class', 'plot-title')
        .text(`${blockName} - ${valueType}`);

    const svg = container.append('svg')
        .attr('class', 'plot-svg');

    const margin = { top: 10, right: 20, bottom: 30, left: 40 };
    const width = 400 - margin.left - margin.right;
    const height = 150 - margin.top - margin.bottom;

    const g = svg.append('g')
        .attr('transform', `translate(${margin.left},${margin.top})`);

    // Scales
    const xScale = d3.scaleLinear().range([0, width]);
    const yScale = d3.scaleLinear().range([height, 0]);

    // Axes
    const xAxis = d3.axisBottom(xScale).ticks(5);
    const yAxis = d3.axisLeft(yScale).ticks(5);

    g.append('g')
        .attr('class', 'x-axis')
        .attr('transform', `translate(0,${height})`)
        .call(xAxis);

    g.append('g')
        .attr('class', 'y-axis')
        .call(yAxis);

    // Line generator
    const line = d3.line()
        .x((d, i) => xScale(i))
        .y(d => yScale(d))
        .curve(d3.curveMonotoneX);

    // Path for line
    const path = g.append('path')
        .attr('class', 'plot-line')
        .attr('fill', 'none')
        .attr('stroke', '#4a9eff')
        .attr('stroke-width', 2);

    return { svg, g, xScale, yScale, xAxis, yAxis, line, path, width, height };
}

function updatePlot(plotObj, data) {
    const { xScale, yScale, xAxis, yAxis, line, path, g, height } = plotObj;

    // Update scales
    xScale.domain([0, data.length - 1]);
    yScale.domain([d3.min(data), d3.max(data)]);

    // Update axes
    g.select('.x-axis').call(xAxis);
    g.select('.y-axis').call(yAxis);

    // Update line
    path.datum(data)
        .attr('d', line);
}
```

#### Task 4.4: Integration with Execution Loop
**File**: `visualization/viewer_live.html`

Update execution loop to capture and plot values:

```javascript
function executeStep() {
    try {
        const learn = learningToggle.checked;

        // Set inputs and capture values
        let inputValue = null;
        switch (currentDemo) {
            case 'sequence':
                inputValue = executeSequenceStep(learn);
                updateTimeSeriesData(blockHandles.encoder, inputValue, executionStep);
                break;
            case 'classification':
                inputValue = executeClassificationStep(learn);
                updateTimeSeriesData(blockHandles.encoder, inputValue, executionStep);
                break;
            // ... other demos
        }

        // Execute network
        wasmNetwork.execute(learn);

        // Capture output values
        if (blockHandles.learner) {
            const anomaly = wasmNetwork.get_anomaly(blockHandles.learner);
            updateTimeSeriesData(`${blockHandles.learner}_anomaly`, anomaly, executionStep);
        }

        if (blockHandles.classifier) {
            const probs = wasmNetwork.get_probabilities(blockHandles.classifier);
            // Store probabilities for each class
            probs.forEach((prob, classIdx) => {
                updateTimeSeriesData(`${blockHandles.classifier}_class${classIdx}`, prob, executionStep);
            });
        }

        // Update all plots
        updateAllPlots();

        // ... rest of execution step
    } catch (err) {
        console.error('Execution error:', err);
        stopExecution();
    }
}

function updateAllPlots() {
    for (const [blockId, plotObj] of Object.entries(activePlots)) {
        const data = timeSeriesData.blocks[blockId];
        if (data && data.values.length > 0) {
            updatePlot(plotObj, data.values);
        }
    }
}
```

#### Task 4.5: Plot Management
**File**: `visualization/viewer_live.html`

Add controls for showing/hiding plots:

```javascript
const activePlots = {};

function initializePlots(demoType) {
    // Clear existing plots
    d3.select('#plots-container').selectAll('*').remove();
    activePlots = {};

    // Create plots based on demo type
    switch(demoType) {
        case 'sequence':
            activePlots.input = createPlot(
                blockHandles.encoder,
                'Input Value',
                'Discrete'
            );
            activePlots.anomaly = createPlot(
                `${blockHandles.learner}_anomaly`,
                'Anomaly Score',
                'Scalar'
            );
            break;

        case 'classification':
            activePlots.input = createPlot(
                blockHandles.encoder,
                'Input Value',
                'Scalar'
            );
            activePlots.class0 = createPlot(
                `${blockHandles.classifier}_class0`,
                'Class 0 Probability',
                'Scalar'
            );
            activePlots.class1 = createPlot(
                `${blockHandles.classifier}_class1`,
                'Class 1 Probability',
                'Scalar'
            );
            activePlots.class2 = createPlot(
                `${blockHandles.classifier}_class2`,
                'Class 2 Probability',
                'Scalar'
            );
            break;

        // ... other demos
    }
}
```

**Subtasks:**
1. Design data structure for time series
2. Implement D3.js line charts
3. Create plot containers and layout
4. Integrate with execution loop
5. Add plot management (show/hide, clear)
6. Add zoom and pan for plots
7. Add value labels on hover
8. Optimize for performance (only update visible plots)

**Acceptance Criteria:**
- ✅ Input values plotted in real-time
- ✅ Output values (anomaly, probabilities) plotted
- ✅ Plots update smoothly (30+ FPS)
- ✅ Plots are legible and informative
- ✅ Hovering shows exact values
- ✅ Auto-scaling works correctly

---

## Implementation Priority and Dependencies

### High Priority (Must Have)
1. **Task 2.1**: Block Type Visual Distinctions
   - Dependencies: None
   - Impact: High (improves readability)

2. **Task 3.1**: Block Tooltips
   - Dependencies: None
   - Impact: High (improves understanding)

3. **Task 3.3**: Click-to-Highlight
   - Dependencies: None
   - Impact: High (improves interactivity)

### Medium Priority (Should Have)
4. **Task 2.2**: Hierarchical Layout
   - Dependencies: None
   - Impact: Medium (improves organization)

5. **Task 3.2**: Connection Tooltips
   - Dependencies: Task 3.1 (tooltip system)
   - Impact: Medium (improves understanding)

6. **Task 4.3**: Real-Time Line Charts
   - Dependencies: Task 4.1, 4.2
   - Impact: High (adds new capability)

### Low Priority (Nice to Have)
7. **Task 1.1-1.3**: Data Capture Enhancement
   - Dependencies: None (but required for Task 4)
   - Impact: Medium (enables plotting)

8. **Task 2.3**: Directional Arrows Enhancement
   - Dependencies: None
   - Impact: Low (polish)

## Implementation Order

### Sprint 1: Visual Polish (Week 1)
- Task 2.1: Block Type Visual Distinctions
- Task 3.1: Block Tooltips
- Task 3.3: Click-to-Highlight

**Deliverable**: Enhanced visualizer with distinct block types and interactivity

### Sprint 2: Layout Improvements (Week 2)
- Task 2.2: Hierarchical Layout
- Task 2.3: Directional Arrows Enhancement
- Task 3.2: Connection Tooltips

**Deliverable**: Organized layout with comprehensive tooltips

### Sprint 3: Real-Time Plotting (Week 3)
- Task 1.1-1.3: Data Capture Enhancement (Backend)
- Task 4.1-4.2: Plot Infrastructure
- Task 4.3-4.5: Line Charts and Integration

**Deliverable**: Complete system with input/output plotting

## Testing Strategy

### Unit Tests
- Block configuration extraction
- Tooltip content generation
- Time series data management
- Plot rendering functions

### Integration Tests
- WASM data capture
- Plot updates during execution
- Click and hover interactions
- Layout switching

### Performance Tests
- Plot rendering performance (target: 60 FPS with 4 plots)
- Memory usage with long executions (target: <500MB)
- Network graph rendering (target: <100ms for 20 blocks)

### User Acceptance Tests
1. Can user distinguish block types at a glance?
2. Are tooltips informative and easy to read?
3. Does clicking block clearly highlight corresponding data?
4. Are plots readable and informative?
5. Does hierarchical layout improve understanding?

## Success Metrics

### Performance Metrics
- ✅ Rendering FPS: >30 FPS with all features enabled
- ✅ Tooltip response time: <100ms
- ✅ Plot update time: <16ms per plot
- ✅ Memory usage: <500MB after 1000 steps

### UX Metrics
- ✅ User can identify block types without reading labels
- ✅ Tooltips provide all necessary configuration info
- ✅ Click-to-highlight provides clear visual feedback
- ✅ Plots are legible at standard screen resolutions
- ✅ Layout clearly shows data flow direction

## File Modification Summary

### Backend (Rust)
- `src/execution_recorder.rs` - Extend trace data structure
- `src/blocks/scalar_transformer.rs` - Add value getters
- `src/blocks/discrete_transformer.rs` - Add value getters
- `src/blocks/pattern_classifier.rs` - Add output getters
- `src/blocks/sequence_learner.rs` - Ensure anomaly access
- `src/blocks/context_learner.rs` - Ensure anomaly access
- `src/wasm_interface.rs` - Add value retrieval methods

### Frontend (JavaScript/HTML)
- `visualization/viewer_live.html` - All enhancements
  - Add block styling system
  - Implement hierarchical layout
  - Add tooltip system
  - Add plotting infrastructure
  - Update execution loop

### Documentation
- `visualization/README.md` - Document new features
- `WASM_LIVE_VIEWER_COMPLETE.md` - Update completion status

## Risk Assessment

### Technical Risks

**Risk 1**: Performance degradation with multiple plots
- **Mitigation**: Implement plot virtualization, only update visible plots
- **Fallback**: Allow users to disable plots

**Risk 2**: Hierarchical layout may not work for all network topologies
- **Mitigation**: Implement both force and hierarchical layouts, allow toggle
- **Fallback**: Keep force-directed as default

**Risk 3**: WASM interface changes may require rebuilding
- **Mitigation**: Test changes incrementally, maintain backward compatibility
- **Fallback**: Separate data capture into optional feature

### UX Risks

**Risk 1**: Too much information may overwhelm users
- **Mitigation**: Progressive disclosure, collapsible sections
- **Fallback**: Make advanced features opt-in

**Risk 2**: Tooltips may obstruct view
- **Mitigation**: Smart positioning, auto-hide on scroll
- **Fallback**: Allow users to disable tooltips

## Timeline Estimate

**Total Time**: 23-31 hours

- **Sprint 1** (Visual Polish): 8-10 hours
- **Sprint 2** (Layout): 7-9 hours
- **Sprint 3** (Plotting): 8-12 hours

**Target Completion**: 3 weeks (part-time) or 1 week (full-time)

## Next Steps

1. **Review and Approve Plan**: Stakeholder review
2. **Setup Development Environment**: Ensure all tools ready
3. **Begin Sprint 1**: Start with high-impact visual enhancements
4. **Iterate**: Gather feedback after each sprint
5. **Document**: Update docs as features are completed

---

## Appendix: Code Snippets

### A. Getting Block Configuration from WASM

```javascript
// Extend WASM interface to expose block configuration
// (requires Rust changes)

const config = wasmNetwork.get_block_config(blockHandle);
// Returns JSON string with block configuration
const blockConfig = JSON.parse(config);
```

### B. Color Palette for Block Types

```javascript
const colorPalette = {
    encoders: ['#4a9eff', '#5aa5ff', '#6ab1ff'],     // Blues
    learners: ['#4aff4a', '#5aff5a', '#6aff6a'],     // Greens
    classifiers: ['#ff4aff', '#ff5aff', '#ff6aff'],  // Magentas
    poolers: ['#ffff4a', '#ffff5a', '#ffff6a']       // Yellows
};
```

### C. Plot Export Functionality (Future Enhancement)

```javascript
function exportPlotAsImage(plotId) {
    const svg = document.querySelector(`#plot-${plotId}`);
    const serializer = new XMLSerializer();
    const svgString = serializer.serializeToString(svg);

    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    const img = new Image();

    img.onload = () => {
        canvas.width = img.width;
        canvas.height = img.height;
        ctx.drawImage(img, 0, 0);

        canvas.toBlob(blob => {
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `plot-${plotId}.png`;
            a.click();
        });
    };

    img.src = 'data:image/svg+xml;base64,' + btoa(svgString);
}
```

---

**Document Version**: 1.0
**Last Updated**: 2025-10-23
**Status**: Ready for Implementation
