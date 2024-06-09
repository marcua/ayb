#!/bin/bash

# Initialize and activate virtual environment
source tests/test-env/bin/activate || python3 -m venv tests/test-env && source tests/test-env/bin/activate

# Install requirements
pip install aiosmtpd awscli localstack

# Start localstack
localstack stop
echo "#!/bin/bash" > tests/localstack_init.sh
echo "awslocal s3 mb s3://bucket" >> tests/localstack_init.sh
chmod +x tests/localstack_init.sh
SCRIPT_PATH=$(realpath "tests/localstack_init.sh")
DOCKER_FLAGS="-v ${SCRIPT_PATH}:/etc/localstack/init/ready.d/init-aws.sh" localstack start -d --network ls

# Build and install nsjail
# On Ubuntu, assumes these requirements: sudo apt-get install -y libprotobuf-dev protobuf-compiler libnl-route-3-dev
scripts/build_nsjail.sh
mv nsjail tests/
