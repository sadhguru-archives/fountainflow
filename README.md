# Fountainflow

A high-performance, cross-platform file transfer tool using fountain codes for reliable UDP-based transfer.

## Overview

Fountainflow is a command-line tool that enables fast and reliable file transfers between computers without requiring mounted network shares or TCP connections. It uses fountain codes (specifically Raptor codes based on RFC 5053) over UDP for efficient, reliable data transfer with automatic error correction.

### Key Features

- High-speed file transfer utilizing full available bandwidth
- No network share requirements
- Built-in error correction using fountain codes
- Rate limiting capabilities
- Progress monitoring and statistics
- Checksum verification
- Cross-platform support (Linux, macOS, Windows)
- Simple command-line interface

## How It Works

Fountainflow uses fountain codes (a type of rateless erasure code) to enable reliable file transfer over UDP. Unlike traditional TCP-based transfers, this approach:

1. Doesn't require acknowledgment for every packet
2. Can reconstruct the original file from any subset of received packets (given enough packets)
3. Automatically handles packet loss and reordering
4. Scales well with high-latency or lossy connections

## Installation

```bash
cargo install fountainflow
```

Or build from source:

```bash
git clone https://github.com/swamikevala/fountainflow.git
cd fountainflow
cargo build --release
```

## Usage

### Sending a file:
```bash
fountainflow send --file path/to/file --target 192.168.1.100:3000 --rate-limit 1000
```

### Receiving a file:
```bash
fountainflow receive --file output/path --port 3000
```

### Options:
- `--rate-limit`: Maximum transfer rate in Mbps (default: unlimited)
- `--port`: UDP port to use (default: 3000)
- `--checksum`: Enable checksum verification (default: true)
- `--verbose`: Show detailed progress information

## Technical Details

### Implementation
- Written in Rust for performance and cross-platform compatibility
- Uses Raptor codes (RFC 5053) for reliable transfer
- Async I/O with Tokio
- UDP-based transport layer
- Built-in rate limiting and congestion control
- Checksum verification using Blake3

### Performance
- Scales to available bandwidth
- Minimal CPU overhead
- Low memory footprint
- Efficient handling of large files

## Building from Source

### Prerequisites
- Rust 1.70 or newer
- Cargo
- C compiler (for native dependencies)

### Build Steps
1. Clone the repository
2. Run `cargo build --release`
3. Find the binary in `target/release/`

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) first.

### Development Setup
1. Fork the repository
2. Create a new branch
3. Make your changes
4. Submit a pull request

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details

## Project Status

Currently in active development. Features being worked on:
- [ ] Core fountain code implementation
- [ ] UDP transport layer
- [ ] CLI interface
- [ ] Cross-platform testing
- [ ] Performance optimization
- [ ] Documentation

## Acknowledgments

- Based on the Raptor code specification (RFC 5053)
- Inspired by the need for efficient, reliable file transfer tools

## Contact

- GitHub Issues: For bug reports and feature requests
- Discussions: For questions and community interaction