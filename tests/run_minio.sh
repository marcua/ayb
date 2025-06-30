#!/bin/bash

echo "Starting MinIO..."

# Stop any existing MinIO container
docker stop minio-test >/dev/null 2>&1 || true
docker rm minio-test >/dev/null 2>&1 || true

# Start MinIO container with host networking for Colima compatibility
docker run -d \
  --name minio-test \
  # **Note for Colima users:** The setup uses `--network host` for better compatibility with Colima's QEMU networking on macOS.
  --network host \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  minio/minio server /data --address :9000 --console-address :9001 >/dev/null

# Give MinIO time to start
sleep 3

# Wait for MinIO to be ready
echo -n "Waiting for MinIO to start"
for i in {1..30}; do
  # Check if MinIO is responding
  if curl -s -m 5 http://localhost:9000/ 2>&1 | grep -q "AccessDenied"; then
    echo " Ready!"
    break
  fi

  if [ $i -eq 30 ]; then
    echo " Failed!"
    echo "MinIO logs:"
    docker logs --tail 20 minio-test
    exit 1
  fi

  echo -n "."
  sleep 1
done

# Give MinIO a moment to fully initialize
sleep 2

# Create the test bucket using AWS CLI
export AWS_ACCESS_KEY_ID=minioadmin
export AWS_SECRET_ACCESS_KEY=minioadmin
export AWS_DEFAULT_REGION=us-east-1

# Create bucket (ignore error if already exists)
echo -n "Creating test bucket..."
aws --endpoint-url http://localhost:9000 s3 mb s3://bucket >/dev/null 2>&1 || true

# Verify bucket exists
if aws --endpoint-url http://localhost:9000 s3 ls s3://bucket >/dev/null 2>&1; then
    echo " Done!"
else
    echo " Failed!"
    echo "Error: Could not access bucket"
    aws --endpoint-url http://localhost:9000 s3 ls 2>&1
    exit 1
fi

echo "MinIO setup complete!"
