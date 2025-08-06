# Squash - Docker Image Layer Compression Tool

[中文](README_CN.md) | **English**

A high-performance Docker image layer compression command-line tool written in Rust.

## 🚀 Features

- **🔄 Multiple Input Sources**: Support Docker image name:tag or exported/saved image files
- **📤 Flexible Output**: Save to file or load directly into Docker with specified image name and tag
- **🎯 Smart Layer Merging**: 
  - By count: Merge the latest n layers into one
  - By layer ID: Merge from specified layer ID to the latest layer
- **📁 Temporary Directory Support**: Configure storage location for intermediate files
- **📝 Verbose Output**: Detailed logging of operations
- **⚡ Memory Efficient**: Streaming processing for large files to prevent memory overflow
- **🔒 Safe Operations**: Path traversal protection and proper error handling
- **🧪 Well Tested**: Comprehensive unit tests, integration tests, and benchmarks

## 📦 Installation

### Prerequisites
- Rust 1.70+ (for building from source)
- Docker (for handling Docker images)

### Build from Source
```bash
git clone https://github.com/your-username/squash.git
cd squash
cargo build --release
```

The binary will be located at `target/release/squash`.

### Install via Cargo
```bash
cargo install --path .
```

## 🛠️ Usage

### Basic Usage

```bash
# Squash the latest 3 layers of an image and save to file
squash squash --source nginx:latest --output nginx-squashed.tar --layers 3

# Squash layers and load directly into Docker
squash squash --source nginx:latest --load my-nginx:squashed --layers 2

# Use a saved image file as source
squash squash --source /path/to/image.tar --output squashed.tar --layers 3

# Verbose output with custom temporary directory
squash squash --source nginx:latest --output nginx-squashed.tar --layers 3 --temp-dir /tmp/squash --verbose
```

### 📋 Command Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--source` | `-s` | Source image (name:tag or file path) |
| `--output` | `-o` | Output file path (required if not using --load) |
| `--load` | | Load result into Docker with specified name:tag |
| `--temp-dir` | `-t` | Temporary directory for intermediate files |
| `--layers` | `-l` | Layer specification (count or layer ID) |
| `--verbose` | `-v` | Enable verbose output |

### 🎯 Layer Specification Examples

```bash
# Merge the latest 3 layers
--layers 3

# Merge from specific layer ID to latest (minimum 8 characters required)
--layers "sha256:abc123def456"

# Merge layers using partial digest (8+ characters)
--layers "abc12345"
```

### 💡 Advanced Examples

```bash
# First export Docker image, then squash
docker save nginx:latest -o nginx.tar
squash squash --source nginx.tar --output nginx-squashed.tar --layers 2

# Squash and immediately load with new tag
squash squash --source nginx:latest --load nginx:optimized --layers 3 --verbose

# Use custom temporary directory for large images
squash squash --source large-image:latest --output optimized.tar --layers 5 --temp-dir /tmp/squash-work
```

## ✅ Core Features
- **🔧 CLI Interface**: Full-featured command-line interface
- **📦 Docker Integration**: Native Docker image export/import support
- **🔍 Image Parsing**: Complete Docker image format support
- **🏗️ Image Rebuilding**: Smart image reconstruction with merged layers
- **🎯 Flexible Merging**: Support for count-based and ID-based layer merging
- **🔄 Docker Loading**: Direct integration with Docker daemon
- **🔐 Integrity Checks**: SHA256 digest calculation and verification
- **📁 Archive Handling**: Complete tar archive operations

## 🚀 Performance Features
- **💾 Memory Efficient**: Streaming processing for large files
- **🛡️ Security**: Path traversal protection and input validation
- **🧹 Resource Management**: Automatic cleanup of temporary files

## 🔮 Planned Improvements
- **📊 Progress Indicators**: Progress bars for long-running operations
- **🗜️ Compression Options**: Configurable compression algorithms

## 🧪 Testing

### Unit Tests
```bash
# Run all unit tests (13 tests)
cargo test

# Run tests with verbose output
cargo test -- --nocapture

# Run specific test module
cargo test docker::layer
```

### Integration Tests
```bash
# Run integration tests
cargo test --test integration_test

# Run Docker-dependent tests (requires Docker)
cargo test --test integration_test -- --ignored

# Run all tests including ignored ones
cargo test --test integration_test -- --include-ignored
```

### Performance Benchmarks
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench layer_merger_creation
```

### 🔧 Development Testing

#### Test Image Generation
```bash
# Generate test Docker image
python3 create_test_image.py

# Test basic squashing functionality
cargo run -- squash --source test-docker-image.tar --output squashed.tar --layers 2 --verbose

# Test layer ID-based merging
cargo run -- squash --source test-docker-image.tar --output squashed-by-id.tar --layers "abc12345" --verbose
```

#### Code Quality Checks
```bash
# Run clippy for code quality checks
cargo clippy --all-targets --all-features

# Format code
cargo fmt

# Security audit
cargo audit
```

### 📝 Contributing Guidelines
1. Fork the repository
2. Create a feature branch
3. Make changes and add tests
4. Ensure all tests pass
5. Submit a pull request

## 🔍 Troubleshooting

### Common Issues

**Error: "Layer ID must be at least 8 characters long"**
- Solution: Provide at least 8 characters when using layer ID matching
- Example: Use `--layers "abc12345"` instead of `--layers "abc"`

**Error: "Cannot merge 0 layers"**
- Solution: Specify a valid number of layers to merge (1 or more)
- Example: Use `--layers 2` instead of `--layers 0`

**Memory issues with large images**
- Solution: Use a custom temporary directory on a disk with sufficient space
- Example: `--temp-dir /path/to/large/disk/temp`

**Docker daemon connection issues**
- Solution: Ensure Docker is running and accessible
- Check: `docker info` should work properly

### Debug Mode
```bash
# Enable verbose output for debugging
squash squash --source image:tag --output result.tar --layers 2 --verbose

# Check logs for detailed processing information
RUST_LOG=debug cargo run -- squash --source image:tag --output result.tar --layers 2
```

**Made with ❤️ and Rust** | **Please give a ⭐ Star if you find it useful!**

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
