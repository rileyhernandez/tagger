set shell := ["bash", "-c"]

# Run a command inside the 'lab' container
[private]
run-in-lab command:
    distrobox-enter -n lab -- {{command}}

# Build the project
build:
    @just run-in-lab "cargo build"

# Clean the project
clean:
    @just run-in-lab "cargo clean"

# Run the project
run:
    @just run-in-lab "cargo run"

# Run tests
test:
    @just run-in-lab "cargo test"

# Format code
fmt:
    @just run-in-lab "cargo fmt"

# Run clippy
clippy:
    @just run-in-lab "cargo clippy"

# Check the project
check:
    @just run-in-lab "cargo check"

# Add dependencies to project
add +args:
    @just run-in-lab "cargo add {{args}}"

# Removes dependencies from project
remove +args:
    @just run-in-lab "cargo remove {{args}}"
