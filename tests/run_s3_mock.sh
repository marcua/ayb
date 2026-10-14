#!/bin/bash

# Run moto's S3 server as a local S3-compatible mock for snapshot tests.
# It's installed into tests/test-env by tests/set_up_e2e_env.sh.

echo "Starting S3 mock..."

# Determine script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MOTO_SERVER="$SCRIPT_DIR/test-env/bin/moto_server"
PORT=9000

if [ ! -x "$MOTO_SERVER" ]; then
  echo "moto_server not found at $MOTO_SERVER; run tests/set_up_e2e_env.sh first"
  exit 1
fi

# Kill any existing S3 mock
pkill -f "moto_server.*-p $PORT" || true
sleep 1

# Start the S3 mock in background
echo "Starting S3 mock server..."
"$MOTO_SERVER" -H 127.0.0.1 -p "$PORT" > /tmp/s3_mock.log 2>&1 &

S3_MOCK_PID=$!
echo "S3 mock started with PID $S3_MOCK_PID"

# Wait for the S3 mock to be ready
echo -n "Waiting for S3 mock to start"
for i in {1..30}; do
  if [ "$(curl -s -m 5 -o /dev/null -w '%{http_code}' http://localhost:$PORT/)" = "200" ]; then
    echo " Ready!"
    break
  fi

  if [ $i -eq 30 ]; then
    echo " Failed!"
    echo "S3 mock logs:"
    tail -20 /tmp/s3_mock.log 2>/dev/null || echo "No logs available"
    exit 1
  fi

  echo -n "."
  sleep 1
done

# Create the test bucket using AWS CLI
export AWS_ACCESS_KEY_ID=s3-mock-access-key
export AWS_SECRET_ACCESS_KEY=s3-mock-secret-key
export AWS_DEFAULT_REGION=us-east-1

# Use aws from virtual environment (works whether venv is activated or not)
AWS_CMD="$SCRIPT_DIR/test-env/bin/aws"

# Create bucket (ignore error if already exists)
echo -n "Creating test bucket..."
"$AWS_CMD" --endpoint-url http://localhost:$PORT s3 mb s3://bucket >/dev/null 2>&1 || true

# Verify bucket exists
if "$AWS_CMD" --endpoint-url http://localhost:$PORT s3 ls s3://bucket >/dev/null 2>&1; then
    echo " Done!"
else
    echo " Failed!"
    echo "Error: Could not access bucket"
    "$AWS_CMD" --endpoint-url http://localhost:$PORT s3 ls 2>&1
    exit 1
fi

echo "S3 mock setup complete!"
