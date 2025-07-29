#!/bin/bash

# Synker Server Build and Deployment Script

set -e

echo "Building Synker Server..."

# Build the project
cargo build --release

echo "Build completed successfully!"

# Create necessary directories
mkdir -p storage temp

echo "Setting up database..."
# Initialize database
./target/release/synker-server --init-db

echo "Setup complete!"
echo ""
echo "To start the server:"
echo "  ./target/release/synker-server"
echo ""
echo "To create an admin user:"
echo "  ./target/release/synker-server --create-admin"
echo ""
echo "Remember to edit config.toml with your MyCloud settings before starting!"
