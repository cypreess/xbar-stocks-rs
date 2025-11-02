# Configuration
binary_name := "xbar-stocks"
release_binary := "target/release/" + binary_name
xbar_plugins_dir := env_var('HOME') + "/Library/Application Support/xbar/plugins"
xbar_plugin_name := "xbar-stocks.5m.o"

# Default recipe to display help
default:
    @just --list

# Build optimized release binary
build:
    @echo "Building release binary..."
    cargo build --release

# Build and strip binary for production
release: build
    @echo "Stripping debug symbols..."
    strip {{release_binary}}
    @echo "Production binary ready: {{release_binary}}"
    @ls -lh {{release_binary}}

# Install binary to xbar plugins directory
install: release
    @echo "Installing to xbar plugins directory..."
    @mkdir -p "{{xbar_plugins_dir}}"
    cp {{release_binary}} "{{xbar_plugins_dir}}/{{xbar_plugin_name}}"
    chmod +x "{{xbar_plugins_dir}}/{{xbar_plugin_name}}"
    @echo "Installed to: {{xbar_plugins_dir}}/{{xbar_plugin_name}}"
    @echo "Refresh xbar to see your stock portfolio!"

# Run tests
test:
    @echo "Running tests..."
    cargo test

# Run the production binary
run: release
    @echo "Running production binary..."
    ./{{release_binary}}

# Run in development mode
dev:
    @echo "Running in development mode..."
    cargo run

# Run the test_multiple example
example:
    @echo "Running test_multiple example..."
    cargo run --example test_multiple

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean

# Build, strip, and show binary info
all: release
    @echo ""
    @echo "Binary size:"
    @ls -lh {{release_binary}}
    @echo ""
    @echo "Binary info:"
    @file {{release_binary}}

# Check code formatting
fmt:
    @echo "Checking code formatting..."
    cargo fmt --check

# Format code
format:
    @echo "Formatting code..."
    cargo fmt

# Run clippy lints
lint:
    @echo "Running clippy lints..."
    cargo clippy -- -D warnings

# Run all checks (test, fmt, lint)
check: test fmt lint
    @echo "All checks passed!"
