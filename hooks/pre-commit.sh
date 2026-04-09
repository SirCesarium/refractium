#!/bin/bash
set -e

echo "Running cargo fmt..."
cargo fmt -- --check

echo "Running cargo check (including tests and benches)..."
cargo check --all-targets --all-features

echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all -W clippy::nursery -W clippy::pedantic

echo "Running sweet..."
swt --quiet

echo "All checks passed!"
