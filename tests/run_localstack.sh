#!/bin/bash

localstack stop
echo "#!/bin/bash" > tests/localstack_init.sh
echo "awslocal s3 mb s3://bucket" >> tests/localstack_init.sh
chmod +x tests/localstack_init.sh
SCRIPT_PATH=$(realpath "tests/localstack_init.sh")
DOCKER_FLAGS="-v ${SCRIPT_PATH}:/etc/localstack/init/ready.d/init-aws.sh" localstack start -d --network ls
