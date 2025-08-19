#!/bin/bash

echo "Building DeepSeek CLI..."
cargo build --release

echo "Installing to /usr/local/bin..."
sudo cp target/release/deepseek-cli /usr/local/bin/deepseek-cli

echo "DeepSeek CLI installed!"
echo "Usage: deepseek-cli --help"