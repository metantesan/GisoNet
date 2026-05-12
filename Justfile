# GisoNet — just command reference
# ────────────────────────────────────────────────────────────

# ── Build ──────────────────────────────────────────────────

# Build all workspace members (debug)
build:
    cargo build

# Build all workspace members (release)
build-release:
    cargo build --release

# Build a specific crate: `just build-crate daemon`
build-crate crate:
    cargo build -p gisonet-{{crate}}

# Check compilation (fast, no codegen)
check:
    cargo check --all-targets

# ── Lint ───────────────────────────────────────────────────

# Run clippy on all targets
clippy:
    cargo clippy --all-targets -- -D warnings

# Check formatting
fmt-check:
    cargo fmt --check

# Format all code
fmt:
    cargo fmt

# ── Clean ──────────────────────────────────────────────────

# Clean all build artifacts
clean:
    cargo clean

# ── Run ────────────────────────────────────────────────────

# Run daemon (needs root for ports 53/80/443)
run-daemon *args:
    sudo env "CARGO_MANIFEST_DIR=$PWD/daemon" cargo run -p gisonet-daemon -- {{args}}

# Run daemon release binary (needs root)
run-daemon-release:
    sudo ./target/release/gisonet-daemon

# Run UI (as user)
run-ui:
    cargo run -p gisonet-ui -j $(nproc)

# Run UI release binary
run-ui-release:
    ./target/release/gisonet-ui

# Run both daemon (root) and UI (user) in background
# Usage: just run
run:
    @echo "Starting daemon in background (requires sudo)..."
    sudo env "CARGO_MANIFEST_DIR=$PWD/daemon" cargo run -p gisonet-daemon &
    @sleep 2
    @echo "Starting UI..."
    cargo run -p gisonet-ui

# ── Systemd (Linux) ────────────────────────────────────────

# Install systemd service (needs root)
install-service:
    sudo cp dist/gisonet-daemon.service /etc/systemd/system/
    sudo systemctl daemon-reload
    @echo "Run: sudo systemctl enable --now gisonet-daemon"

# Remove systemd service
uninstall-service:
    sudo systemctl disable --now gisonet-daemon 2>/dev/null || true
    sudo rm -f /etc/systemd/system/gisonet-daemon.service
    sudo systemctl daemon-reload

# ── Test ───────────────────────────────────────────────────

# Run all tests
test:
    cargo test --all-targets

# ── Utility ────────────────────────────────────────────────

# Show all available commands
default:
    @just --list

# Check for outdated dependencies
outdated:
    cargo outdated --exit-code 1 2>/dev/null || cargo install cargo-outdated
    cargo outdated

# Audit dependencies for vulnerabilities
audit:
    cargo audit 2>/dev/null || cargo install cargo-audit
    cargo audit
