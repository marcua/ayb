#!/bin/bash

echo "Starting MinIO..."

# Determine script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINIO_BINARY="$SCRIPT_DIR/minio"
MINIO_DATA_DIR="/tmp/minio-data"

# Check if Docker is available and working
DOCKER_AVAILABLE=false
if command -v docker &> /dev/null; then
  if docker ps &> /dev/null; then
    DOCKER_AVAILABLE=true
    echo "Docker is available"
  else
    echo "Docker command exists but daemon is not accessible"
  fi
else
  echo "Docker is not installed"
fi

# If Docker is available, use it; otherwise use native binary
if [ "$DOCKER_AVAILABLE" = true ]; then
  echo "Using Docker-based MinIO..."

  # Stop any existing MinIO container
  docker stop minio-test >/dev/null 2>&1 || true
  docker rm minio-test >/dev/null 2>&1 || true

  # Give MinIO time to shut down
  sleep 3

  # Detect environment and choose networking approach
  if [[ "$CI" == "true" ]] || [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "Detected CI/Linux environment - using port mapping"
    # Use port mapping for CI/standard Linux Docker
    docker run -d \
      --name minio-test \
      -p 9000:9000 \
      -p 9001:9001 \
      -e "MINIO_ROOT_USER=minioadmin" \
      -e "MINIO_ROOT_PASSWORD=minioadmin" \
      minio/minio server /data --console-address ":9001" >/dev/null
  else
    echo "Detected macOS/Colima environment - using host networking"
    # Use host networking for Colima compatibility on macOS
    docker run -d \
      --name minio-test \
      --network host \
      -e "MINIO_ROOT_USER=minioadmin" \
      -e "MINIO_ROOT_PASSWORD=minioadmin" \
      minio/minio server /data --address :9000 --console-address :9001 >/dev/null
  fi
else
  echo "Using native MinIO binary..."

  # Download MinIO binary if it doesn't exist
  if [ ! -f "$MINIO_BINARY" ]; then
    echo "Downloading MinIO binary..."

    # Detect OS and architecture
    if [[ "$OSTYPE" == "darwin"* ]]; then
      # macOS
      if [[ $(uname -m) == "arm64" ]]; then
        MINIO_URL="https://dl.min.io/server/minio/release/darwin-arm64/minio"
      else
        MINIO_URL="https://dl.min.io/server/minio/release/darwin-amd64/minio"
      fi
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
      # Linux
      if [[ $(uname -m) == "aarch64" ]]; then
        MINIO_URL="https://dl.min.io/server/minio/release/linux-arm64/minio"
      else
        MINIO_URL="https://dl.min.io/server/minio/release/linux-amd64/minio"
      fi
    else
      echo "Unsupported OS: $OSTYPE"
      exit 1
    fi

    wget -q "$MINIO_URL" -O "$MINIO_BINARY" || curl -sL "$MINIO_URL" -o "$MINIO_BINARY"
    chmod +x "$MINIO_BINARY"
    echo "MinIO binary downloaded to $MINIO_BINARY"
  fi

  # Kill any existing MinIO process
  pkill -f "minio server" || true
  sleep 2

  # Create data directory
  mkdir -p "$MINIO_DATA_DIR"

  # Start MinIO in background
  echo "Starting MinIO server..."
  MINIO_ROOT_USER=minioadmin MINIO_ROOT_PASSWORD=minioadmin \
    "$MINIO_BINARY" server "$MINIO_DATA_DIR" \
    --address :9000 \
    --console-address :9001 \
    > /tmp/minio.log 2>&1 &

  MINIO_PID=$!
  echo "MinIO started with PID $MINIO_PID"
fi

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
    if [ "$DOCKER_AVAILABLE" = true ]; then
      docker logs --tail 20 minio-test
    else
      tail -20 /tmp/minio.log 2>/dev/null || echo "No logs available"
    fi
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

# Find aws command - check virtual environment first, then system
AWS_CMD="aws"
if [ -f "$SCRIPT_DIR/test-env/bin/aws" ]; then
  AWS_CMD="$SCRIPT_DIR/test-env/bin/aws"
elif ! command -v aws &> /dev/null; then
  echo "Error: aws CLI not found. Please run tests/set_up_e2e_env.sh first"
  exit 1
fi

# Create bucket (ignore error if already exists)
echo -n "Creating test bucket..."
"$AWS_CMD" --endpoint-url http://localhost:9000 s3 mb s3://bucket >/dev/null 2>&1 || true

# Verify bucket exists
if "$AWS_CMD" --endpoint-url http://localhost:9000 s3 ls s3://bucket >/dev/null 2>&1; then
    echo " Done!"
else
    echo " Failed!"
    echo "Error: Could not access bucket"
    "$AWS_CMD" --endpoint-url http://localhost:9000 s3 ls 2>&1
    exit 1
fi

echo "MinIO setup complete!"
