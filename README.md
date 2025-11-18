# measurement tool

A flexible runtime measurement tool for confidential computing environments that measures various system resources and communicates with Attestation Agents via ttrpc protocol.

## Overview

measurement tool is a Rust-based tool designed to perform runtime measurements of system resources (files, processes, container images, etc.) in confidential computing environments. It sends measurement results to an Attestation Agent through a secure ttrpc connection, enabling verification and attestation of system state.

## Currently Supported Measurers

- **File Measurer**: Measures file contents using cryptographic hashes
  - Supports SHA256 and SHA384 algorithms
  - Configurable PCR index for measurements
  - Glob pattern support for flexible file selection
  - Runtime: watches config for changes to `file_measurement.files` and measures only newly added patterns

## Installation

### Building from Source

```bash
git clone <repository-url>
cd measurement_tools
cargo build --release
```

The compiled binary will be available at `target/release/measurement_tool`.

## Configuration

Copy the example configuration file and modify it for your environment:

```bash
cp config.example.toml config.toml
```

### Configuration Options

```toml
# ttrpc endpoint for Attestation Agent
attestation_agent_socket = "unix:///run/confidential-containers/attestation-agent/attestation-agent.sock"

[file_measurement]
enable = true
pcr_index = 16
domain = "file"
operation = "measure"
hash_algorithm = "sha256"  # Options: sha256, sha384
files = [
  "/etc/hostname",
  "/usr/bin/attestation-agent",
  "/etc/*.conf"
]
```

#### Configuration Parameters

- `attestation_agent_socket`: ttrpc socket path for Attestation Agent communication
- `file_measurement.enable`: Enable/disable file measurement module
- `file_measurement.pcr_index`: PCR index to extend with measurements
- `file_measurement.domain`: Measurement domain identifier
- `file_measurement.operation`: Operation type (typically "measure")
- `file_measurement.hash_algorithm`: Hashing algorithm (sha256 or sha384)
- `file_measurement.files`: List of file paths to measure (supports glob patterns)

## Usage

### Basic Usage (Daemon)

```bash
# Use default config.toml in current directory
./measurement_tool

# Use custom configuration file
./measurement_tool /path/to/custom/config.toml
```

### Logging

Control logging output with the `RUST_LOG` environment variable:

```bash
# Info level logging (default)
RUST_LOG=info ./measurement_tool

# Debug level logging
RUST_LOG=debug ./measurement_tool

# Warning level only
RUST_LOG=warn ./measurement_tool
```

## Service

The tool is designed to run as a long-lived daemon. On startup it performs a one-time measurement run (equivalent to the previous oneshot behavior), then:
- Watches the configuration file for updates and measures any newly added patterns.

## Adding New Measurers

To add a new measurement module:

1. Create a new file in `src/modules/`
2. Implement the `Measurable` trait:

```rust
#[async_trait]
impl Measurable for YourMeasurer {
    fn name(&self) -> &str {
        "YourMeasurer"
    }

    fn is_enabled(&self, config: Arc<Config>) -> bool {
        // Check configuration
    }

    async fn measure(&self, config: Arc<Config>, aa_client: Arc<AAClient>) -> Result<()> {
        // Implement measurement logic
    }
}
```

3. Register it in `main.rs`:

```rust
let measurers: Vec<Box<dyn Measurable + Send + Sync>> = vec![
    Box::new(FileMeasurer::new()),
    Box::new(YourMeasurer::new()),
];
```

### Building and Testing

```bash
# Build in development mode
cargo build

# Run tests
cargo test

# Build optimized release
cargo build --release

# Check code formatting
cargo fmt

# Run clippy lints
cargo clippy
```
