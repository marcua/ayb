#!/bin/bash
# Don't exit on error - we want to continue and document what's blocked
set +e

echo "=== Quick Win Minimal Setup for Claude Code Environment ==="
echo ""

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check if a host is allowed
check_host() {
    local host=$1
    local protocol=${2:-http}

    local url="${protocol}://${host}/"
    local result=$(curl -s -m 5 -I "$url" 2>&1)

    if echo "$result" | grep -q "host_not_allowed"; then
        echo "✗ $host - BLOCKED (host_not_allowed)"
        return 1
    elif echo "$result" | grep -q "403"; then
        echo "✗ $host - BLOCKED (403 Forbidden)"
        return 1
    elif echo "$result" | grep -q "Temporary failure resolving"; then
        echo "✗ $host - DNS resolution failed"
        return 1
    elif echo "$result" | grep -qE "HTTP/[0-9.]+ (200|301|302|404)"; then
        echo "✓ $host - accessible"
        return 0
    else
        echo "? $host - unknown status"
        return 1
    fi
}

# Check critical hosts
echo "=== Checking Network Access ==="
echo ""
echo "Checking critical domains for test setup:"
echo ""

check_host "archive.ubuntu.com" "http"
check_host "security.ubuntu.com" "http"
check_host "dl.min.io" "http"
check_host "dl.min.io" "https"
check_host "registry-1.docker.io" "https"
check_host "github.com" "https"

echo ""

# Step 1: Install Docker if not present
echo "=== Step 1: Docker Installation ==="
echo ""
if command_exists docker; then
    echo "✓ Docker already installed"
    docker --version
    DOCKER_OK=true
else
    echo "✗ Docker not found"
    echo "Attempting to install docker.io..."
    if sudo apt-get update 2>&1 | grep -q "Failed to fetch"; then
        echo "✗ Failed to update package lists (network blocked)"
        echo ""
        echo "REQUIRED: Add these domains to allowlist:"
        echo "  - archive.ubuntu.com"
        echo "  - security.ubuntu.com"
        DOCKER_OK=false
    else
        if sudo apt-get install -y docker.io 2>&1; then
            echo "✓ Docker installed successfully"
            # Try to start docker
            if command_exists systemctl; then
                sudo systemctl start docker 2>/dev/null || echo "Note: systemctl not available"
            elif command_exists dockerd; then
                echo "Starting dockerd in background..."
                sudo dockerd >/dev/null 2>&1 &
                sleep 3
            fi
            DOCKER_OK=true
        else
            echo "✗ Failed to install Docker"
            DOCKER_OK=false
        fi
    fi
fi

# Step 2: Install system packages for nsjail
echo ""
echo "=== Step 2: nsjail Build Dependencies ==="
echo ""
PACKAGES="flex bison libprotobuf-dev protobuf-compiler libnl-route-3-dev"

# Check what's already installed
echo "Checking installed packages:"
for pkg in $PACKAGES; do
    if dpkg -l 2>/dev/null | grep -q "^ii  $pkg"; then
        echo "  ✓ $pkg already installed"
    else
        echo "  ✗ $pkg not installed"
    fi
done

echo ""
echo "Attempting to install missing packages..."
if sudo apt-get install -y $PACKAGES 2>&1 | grep -q "Unable to locate"; then
    echo "✗ Failed to install packages (network blocked)"
else
    echo "✓ Packages installed (or already present)"
fi

# Step 3: Run the standard test environment setup
echo ""
echo "=== Step 3: Test Environment Setup ==="
echo ""
if [ -f "tests/set_up_e2e_env.sh" ]; then
    # Run the setup script and capture output
    bash tests/set_up_e2e_env.sh 2>&1 | grep -E "(✓|✗|Starting|Failed|Error|Building)" || true

    # Check what succeeded
    if [ -d "tests/test-env" ]; then
        echo "✓ Python virtual environment created"
    fi
    if [ -f "tests/nsjail" ]; then
        echo "✓ nsjail binary built"
    fi
    if docker ps 2>/dev/null | grep -q minio-test; then
        echo "✓ MinIO container running"
    fi
else
    echo "✗ tests/set_up_e2e_env.sh not found"
fi

echo ""
echo "=== Setup Summary ==="
echo ""

# Check status of each component
DOCKER_STATUS=$(command_exists docker && echo '✓ Available' || echo '✗ Not available')
MINIO_STATUS=$(docker ps 2>/dev/null | grep -q minio-test && echo '✓ Running' || echo '✗ Not running')
NSJAIL_STATUS=$([ -f tests/nsjail ] && echo '✓ Built' || echo '✗ Not built')
PYTHON_STATUS=$([ -d tests/test-env ] && echo '✓ Created' || echo '✗ Not created')

echo "Component Status:"
echo "  Docker:       $DOCKER_STATUS"
echo "  MinIO:        $MINIO_STATUS"
echo "  nsjail:       $NSJAIL_STATUS"
echo "  Python venv:  $PYTHON_STATUS"
echo ""

# Determine if tests can run
CAN_RUN_TESTS=false
if command_exists docker && docker ps 2>/dev/null | grep -q minio-test; then
    CAN_RUN_TESTS=true
    echo "✓ Ready to run tests!"
    echo ""
    echo "Run SQLite e2e tests with:"
    echo "  cargo test client_server_integration_sqlite"
else
    echo "✗ Cannot run e2e tests yet"
    echo ""
    echo "BLOCKERS:"
    if ! command_exists docker; then
        echo "  - Docker not installed (requires network access to package repos)"
    fi
    if command_exists docker && ! docker ps 2>/dev/null | grep -q minio-test; then
        echo "  - MinIO not running (requires Docker)"
    fi
    echo ""
    echo "Required domains to unblock (see test-issues.md for full list):"
    echo "  - archive.ubuntu.com (for apt packages)"
    echo "  - security.ubuntu.com (for apt packages)"
    echo "  - registry-1.docker.io (for Docker images)"
    echo "  - docker.io (for Docker Hub)"
    echo "  - dl.min.io (for MinIO binary, if using native install)"
fi
echo ""
