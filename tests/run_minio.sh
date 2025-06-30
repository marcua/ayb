#!/bin/bash

echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting MinIO container setup..."

# Log environment details
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Environment Detection:"
echo "  CI: ${CI:-not set}"
echo "  OSTYPE: ${OSTYPE:-not set}"
echo "  USER: ${USER:-not set}"
echo "  Docker version: $(docker --version 2>/dev/null || echo 'Docker not available')"

# Stop any existing MinIO container
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Cleaning up existing MinIO containers..."
docker stop minio-test >/dev/null 2>&1 || true
docker rm minio-test >/dev/null 2>&1 || true

# Verify no existing container remains
if docker ps -a --format '{{.Names}}' | grep -q "minio-test"; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: Failed to remove existing minio-test container"
    docker ps -a --filter name=minio-test
    exit 1
fi

# Detect environment and choose networking approach  
if [[ "$CI" == "true" ]] || [[ "$OSTYPE" == "linux-gnu"* ]]; then
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] Detected CI/Linux environment - using port mapping"
  
  # Check if ports are available
  if netstat -tuln 2>/dev/null | grep -q ":9000 "; then
      echo "[$(date '+%Y-%m-%d %H:%M:%S')] WARNING: Port 9000 appears to be in use"
      netstat -tuln | grep ":9000 " || true
  fi
  
  # Use port mapping for CI/standard Linux Docker
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting MinIO container with port mapping..."
  docker run -d \
    --name minio-test \
    -p 9000:9000 \
    -p 9001:9001 \
    -e "MINIO_ROOT_USER=minioadmin" \
    -e "MINIO_ROOT_PASSWORD=minioadmin" \
    minio/minio server /data --console-address ":9001" 2>&1 | tee /tmp/minio-startup.log
else
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] Detected macOS/Colima environment - using host networking"
  
  # Use host networking for Colima compatibility on macOS
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting MinIO container with host networking..."
  docker run -d \
    --name minio-test \
    --network host \
    -e "MINIO_ROOT_USER=minioadmin" \
    -e "MINIO_ROOT_PASSWORD=minioadmin" \
    minio/minio server /data --address :9000 --console-address :9001 2>&1 | tee /tmp/minio-startup.log
fi

# Verify container started
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Verifying container started..."
container_id=$(docker ps --filter name=minio-test --format '{{.ID}}')
if [[ -z "$container_id" ]]; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: MinIO container failed to start"
    echo "Container logs:"
    docker logs minio-test 2>&1 || echo "No logs available"
    exit 1
fi

echo "[$(date '+%Y-%m-%d %H:%M:%S')] Container started with ID: $container_id"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Container status: $(docker inspect --format='{{.State.Status}}' minio-test)"

# Give MinIO time to start
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Waiting 3 seconds for initial MinIO startup..."
sleep 3

# Wait for MinIO to be ready
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Testing MinIO readiness..."
for i in {1..30}; do
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] Readiness check attempt $i/30"
  
  # Check container is still running
  if ! docker ps --filter name=minio-test --format '{{.Status}}' | grep -q "Up"; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: MinIO container is not running"
    echo "Container status: $(docker ps -a --filter name=minio-test --format '{{.Status}}')"
    echo "Container logs:"
    docker logs --tail 50 minio-test
    exit 1
  fi
  
  # Test HTTP connectivity
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] Testing HTTP connectivity to localhost:9000..."
  curl_output=$(curl -s -m 5 http://localhost:9000/ 2>&1)
  curl_exit_code=$?
  
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] Curl exit code: $curl_exit_code"
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] Curl output: $curl_output"
  
  # Check if MinIO is responding with expected response
  if echo "$curl_output" | grep -q "AccessDenied"; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] MinIO is ready! (AccessDenied response received)"
    break
  fi
  
  # Check for other potential success indicators
  if echo "$curl_output" | grep -q "MinIO"; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] MinIO response detected but not AccessDenied: $curl_output"
  fi

  if [ $i -eq 30 ]; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: MinIO failed to become ready after 30 attempts"
    echo "Final curl output: $curl_output"
    echo "Container status: $(docker inspect --format='{{.State.Status}}' minio-test)"
    echo "Container logs:"
    docker logs --tail 50 minio-test
    echo "Network connectivity test:"
    netstat -tuln | grep ":9000" || echo "Port 9000 not listening"
    exit 1
  fi

  echo "[$(date '+%Y-%m-%d %H:%M:%S')] Not ready yet, waiting 1 second..."
  sleep 1
done

# Give MinIO a moment to fully initialize
echo "[$(date '+%Y-%m-%d %H:%M:%S')] MinIO basic readiness confirmed, waiting 2 seconds for full initialization..."
sleep 2

# Set up AWS CLI credentials
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Setting up AWS CLI credentials..."
export AWS_ACCESS_KEY_ID=minioadmin
export AWS_SECRET_ACCESS_KEY=minioadmin
export AWS_DEFAULT_REGION=us-east-1

# Test AWS CLI connectivity
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Testing AWS CLI connectivity..."
aws_test_output=$(aws --endpoint-url http://localhost:9000 s3 ls 2>&1)
aws_test_exit_code=$?
echo "[$(date '+%Y-%m-%d %H:%M:%S')] AWS CLI test exit code: $aws_test_exit_code"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] AWS CLI test output: $aws_test_output"

# Create bucket
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Creating test bucket..."
bucket_create_output=$(aws --endpoint-url http://localhost:9000 s3 mb s3://bucket 2>&1)
bucket_create_exit_code=$?
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Bucket creation exit code: $bucket_create_exit_code"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Bucket creation output: $bucket_create_output"

# Verify bucket exists and is accessible
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Verifying bucket access..."
bucket_list_output=$(aws --endpoint-url http://localhost:9000 s3 ls s3://bucket 2>&1)
bucket_list_exit_code=$?
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Bucket list exit code: $bucket_list_exit_code"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Bucket list output: $bucket_list_output"

if [ $bucket_list_exit_code -eq 0 ]; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] SUCCESS: Bucket verification passed"
else
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: Could not access bucket"
    echo "All available buckets:"
    aws --endpoint-url http://localhost:9000 s3 ls 2>&1
    echo "Container final logs:"
    docker logs --tail 20 minio-test
    exit 1
fi

# Test a basic S3 operation (put/get/delete)
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Testing basic S3 operations..."
test_file="/tmp/minio-test-file-$$"
echo "test content" > "$test_file"

# Test upload
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Testing file upload..."
upload_output=$(aws --endpoint-url http://localhost:9000 s3 cp "$test_file" s3://bucket/test-file 2>&1)
upload_exit_code=$?
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Upload exit code: $upload_exit_code"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Upload output: $upload_output"

# Test download
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Testing file download..."
download_file="/tmp/minio-download-test-$$"
download_output=$(aws --endpoint-url http://localhost:9000 s3 cp s3://bucket/test-file "$download_file" 2>&1)
download_exit_code=$?
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Download exit code: $download_exit_code"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Download output: $download_output"

# Verify content
if [ -f "$download_file" ] && [ "$(cat "$download_file")" = "test content" ]; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] SUCCESS: S3 operations test passed"
else
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: S3 operations test failed"
    echo "Downloaded file exists: $([ -f "$download_file" ] && echo "yes" || echo "no")"
    echo "Downloaded content: $(cat "$download_file" 2>/dev/null || echo "unable to read")"
    exit 1
fi

# Cleanup test files
rm -f "$test_file" "$download_file"

# Delete test object
aws --endpoint-url http://localhost:9000 s3 rm s3://bucket/test-file >/dev/null 2>&1 || true

echo "[$(date '+%Y-%m-%d %H:%M:%S')] SUCCESS: MinIO setup complete and verified!"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Container ID: $(docker ps --filter name=minio-test --format '{{.ID}}')"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Container status: $(docker inspect --format='{{.State.Status}}' minio-test)"
