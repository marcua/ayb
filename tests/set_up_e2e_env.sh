#!/bin/bash

# Initialize and activate virtual environment
source tests/test-env/bin/activate || python3 -m venv tests/test-env && source tests/test-env/bin/activate

# Install requirements
pip install awscli localstack awscli-local

# Start LocalStack
tests/run_localstack.sh

# Build and install nsjail (Linux only)
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # On Ubuntu, assumes these requirements: sudo apt-get install -y libprotobuf-dev protobuf-compiler libnl-route-3-dev
    scripts/build_nsjail.sh
    mv nsjail tests/

    # Starting with Ubuntu 24.x, nsjail won't run with default permissions
    # (https://github.com/google/nsjail/issues/236).
    sudo sysctl -w kernel.apparmor_restrict_unprivileged_unconfined=0
    sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0
fi
