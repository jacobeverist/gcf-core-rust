# Gnomic Computing

## Introduction

Gnomic Computing is a framework developed for building scalable Machine Learning (ML) applications using principles derived from computational neuroscience.  It models neuron activations with **binary patterns** (vectors of 1s and 0s) which form a kind of "cortical language".  Assemblages of computational **blocks** transmit these binary patterns between each other to create computational workflows that exploit these neuroscience principles.

Gnomic Computing is a single-thread C++ backend.  The design of Gnomic Computing represents the practical experience gained from solving machine learning problems using a [Hierarchical Temporal Memory](https://numenta.com/assets/pdf/biological-and-machine-intelligence/BAMI-Complete.pdf) (HTM) like approach. 

### Design

Gnomics is designed to be:

- **Usable**: solve practical ML applications
- **Scalable**: quick and easy to build block hierarchies of any size
- **Extensible**: improve existing or develop entirely new blocks
- **Fast**: leverages low-level bitwise operations
- **Low Memory**: maintain as low memory footprint as possible
- **Lightweight**: small project size

The current computational **blocks** provided are:
- **Transformers**: Encodes symbols, scalars, or vectors into binary patterns for processing by Gnomics.
- **PatternClassifier**: Supervised learning classifier for binary patterns.
- **PatternPooler**: Learns mapping from one representation to a pooled representation.
- **ContextLearner**: Learn inputs in provided contexts.  Flags anomaly if inputs are out-of-context.
- **SequenceLearner**: Learns input sequences.  Flags anomaly if previously unseen sequences are detected.  


## Getting Started

### System Requirements
Gnomics is known to run on the following platforms:

- Windows (7,8,10,11)
- MacOS (10.14 or higher)
- Linux (Ubuntu 16+, CentOS 7+)

If you want to develop Gnomics on your system, you need at least the following additional dependencies:
- `git`
- C/C++ compiler (`clang`, `visual studio`, `gcc`, etc.)


## Development

### Build
Alternatively, you can build Gnomics on your system following these steps.

Clone the repository
```bash
git clone https://github.com/the-aerospace-corporation/gnomics
```

Change to the project directory
```bash
cd gnomics
```

#### Build 
```bash
mkdir build
cd build
cmake ..
```

Build core C++ unit tests
```bash
mkdir build
cd build
cmake -DGnomics_TESTS=true ..
```

## Project Layout

```bash
.
├── src
│   ├── cpp               # Core C++ code
│   │   └── blocks        # Core C++ block algorithms
├── tests                 # Unit tests
│   ├── cpp               # C++ core unit tests
├── CMakeLists.txt        # CMake configuration for core C++ build
├── README.md             # README file
```
