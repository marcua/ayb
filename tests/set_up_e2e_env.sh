#!/bin/bash

# Initialize and activate virtual environment
if [ ! -d "tests/test-env" ]; then
    python3 -m venv tests/test-env
fi
source tests/test-env/bin/activate

# Install requirements
pip install awscli playwright
playwright install chromium

# Start MinIO
tests/run_minio.sh

# Build and install nsjail (Linux only)
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # On Ubuntu, assumes these requirements: sudo apt-get install -y libprotobuf-dev protobuf-compiler libnl-route-3-dev
    if [ ! -f "tests/nsjail" ] || [ "$FORCE_NSJAIL" = "1" ]; then
        echo "Building nsjail..."
        scripts/build_nsjail.sh
        mv nsjail tests/
    else
        echo "nsjail already exists, skipping build (set FORCE_NSJAIL=1 to rebuild)"
    fi

    # Starting with Ubuntu 24.x, nsjail won't run with default permissions
    # (https://github.com/google/nsjail/issues/236).
    sudo sysctl -w kernel.apparmor_restrict_unprivileged_unconfined=0
    sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0
fi
