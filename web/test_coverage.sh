#!/bin/bash

set -e  # Exit on error

echo "=== Running unit tests for coverage module ==="
cargo test --lib -- --nocapture coverage

echo -e "\n=== Running manual coverage test ==="
cargo run --bin test_coverage

echo -e "\n=== Tests completed ===" 