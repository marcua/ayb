#!/bin/bash
set -e

echo "Setting up Claude Code environment for testing..."

# Fix HTTP_PROXY for apt-get (it uses HTTP protocol even for HTTPS repos)
if [ -n "$HTTPS_PROXY" ] && [ -z "$HTTP_PROXY" ]; then
    echo "Setting HTTP_PROXY=$HTTPS_PROXY for apt-get"
    export HTTP_PROXY="$HTTPS_PROXY"
fi

# Update package list
echo "Updating package list..."
sudo -E apt-get update

# Install required packages
echo "Installing required packages..."
sudo -E apt-get install -y \
    postgresql \
    flex \
    bison \
    libprotobuf-dev \
    protobuf-compiler \
    libnl-route-3-dev \
    wget \
    curl

echo "Claude Code environment setup complete!"
