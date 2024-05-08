#!/bin/bash

mkdir tests/localstack_data_$1
source tests/localstack_data/test-env/bin/activate || python3 -m venv tests/localstack_data_$1/test-env && source tests/localstack_data_$1/test-env/bin/activate
pip install localstack awscli
echo "#!/bin/bash" > tests/localstack_init.sh
echo "awslocal s3 mb s3://bucket$1" >> tests/localstack_init.sh
chmod +x tests/localstack_init.sh
SCRIPT_PATH=$(realpath "tests/localstack_init.sh")
DOCKER_FLAGS="-v ${SCRIPT_PATH}:/etc/localstack/init/ready.d/init-aws.sh" localstack start -d --network ls
