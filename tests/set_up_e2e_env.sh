#!/bin/bash

# Initialize and activate virtual environment
if [ ! -d "tests/test-env" ]; then
    python3 -m venv tests/test-env
fi
source tests/test-env/bin/activate

# Install requirements
pip install awscli playwright

# Try to install Playwright browsers, but don't fail if unsupported
echo "Installing Playwright browsers..."
if playwright install chromium >/dev/null 2>&1; then
    echo "Playwright browsers installed successfully"
else
    echo "Using system browsers (Playwright browsers not available on this platform)"
fi

# Start MinIO
tests/run_minio.sh
