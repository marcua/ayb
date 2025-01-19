#!/bin/bash

# Initialize and activate virtual environment
source tests/test-env/bin/activate || python3 -m venv tests/test-env && source tests/test-env/bin/activate

# Install requirements
pip install aiosmtpd awscli localstack

tests/run_localstack.sh

# Build and install nsjail
# On Ubuntu, assumes these requirements: sudo apt-get install -y libprotobuf-dev protobuf-compiler libnl-route-3-dev
scripts/build_nsjail.sh
mv nsjail tests/
