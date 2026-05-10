#!/usr/bin/env bash

# Clean the target/doc directory to remove old documentation files
rm -rf target/doc

# Generate documentation for the project
cargo doc --no-deps

# Serve the generated documentation on localhost:$1 (default to 8000 if no port is provided)
PORT=${1:-8000}
echo "Serving documentation on http://localhost:$PORT/codecrafters_interpreter/"
python3 -m http.server -d "target/doc" $PORT