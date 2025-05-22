#!/bin/bash

# Exit on any error
set -e

echo "Setting up development environment for jackbot-sensor..."

# Update package lists
echo "Updating package lists..."
sudo apt-get update

# Install essential build tools
echo "Installing essential build tools..."
sudo apt-get install -y build-essential pkg-config libssl-dev curl git

# Install Rust and Cargo
echo "Installing Rust and Cargo..."
if ! command -v rustc &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "Rust is already installed. Updating..."
    rustup update
fi

# Install Redis
echo "Installing Redis..."
sudo apt-get install -y redis-server
sudo systemctl enable redis-server
sudo systemctl start redis-server

# Install AWS CLI for S3 interaction
echo "Installing AWS CLI..."
sudo apt-get install -y awscli

# Install Python and pip for any ML components or scripts
echo "Installing Python and pip..."
sudo apt-get install -y python3 python3-pip python3-venv

# Install development libraries that might be needed for Rust crates
echo "Installing development libraries..."
# For bindgen, building Rust crates, compression, and hardware interaction
sudo apt-get install -y libclang-dev cmake zlib1g-dev libudev-dev

# Install Apache Arrow dependencies (for Parquet support)
echo "Installing Apache Arrow dependencies..."
sudo apt-get install -y libatlas-base-dev liblapack-dev libsnappy-dev

echo "Development environment setup complete!"
echo "You may need to restart your terminal or run 'source $HOME/.cargo/env' to use Rust."
echo ""
echo "Next steps:"
echo "1. Configure your API keys for exchanges"
echo "2. Run 'cargo build' to compile the project"
echo "3. Run 'cargo test' to verify everything works" 