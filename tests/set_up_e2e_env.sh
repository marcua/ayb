#!/bin/bash

# Initialize and activate virtual environment
source tests/test-env/bin/activate || python3 -m venv tests/test-env && source tests/test-env/bin/activate

# Install requirements
pip install aiosmtpd awscli localstack

# Install Playwright and browser binaries for browser testing
# Note: Playwright Rust bindings use the same browser binaries as the Python version
pip install playwright
playwright install chromium

# Start LocalStack
tests/run_localstack.sh

# Build and install nsjail
# On Ubuntu, assumes these requirements: sudo apt-get install -y libprotobuf-dev protobuf-compiler libnl-route-3-dev
scripts/build_nsjail.sh
mv nsjail tests/

# Starting with Ubuntu 24.x, nsjail won't run with default permissions
# (https://github.com/google/nsjail/issues/236).
sudo sysctl -w kernel.apparmor_restrict_unprivileged_unconfined=0
sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0
