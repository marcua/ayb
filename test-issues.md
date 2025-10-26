# Test Environment Issues

This document describes issues encountered when attempting to run the test suite in the Claude Code environment, and what was fixed vs. what still needs help.

## Things Fixed

1. **Python Virtual Environment** ✓
   - Successfully created `tests/test-env` with Python 3.11.14
   - Installed awscli successfully
   - Installed playwright Python package successfully
   - All Python dependencies are working

2. **Unit Tests** ✓
   - All 11 unit tests pass successfully
   - No environmental issues with unit test execution
   - Tests cover: email templating, server config validation, configuration

## Things I Need Help With

### BLOCKING ISSUES (Cannot be fixed in current environment)

#### 1. Docker/Container Runtime Not Available
**Status:** BLOCKING - Cannot fix in environment

**Issue:**
```
tests/run_minio.sh: line 16: docker: command not found
```

**Impact:**
- MinIO cannot start (required for S3 snapshot testing)
- All e2e tests fail immediately at startup
- The test code uses `std::process::exit(1)` when MinIO setup fails, terminating the entire test run

**Evidence:**
```bash
$ which docker
# (not found)

$ docker --version
/bin/bash: line 1: docker: command not found
```

**Why this is blocking:**
- Every e2e test calls `ensure_minio_running()` (tests/utils/testing.rs:150-158)
- This function uses `Once::call_once()` to run `setup_minio()` exactly once
- If `setup_minio()` fails, it calls `std::process::exit(1)`, killing all tests
- There is no environment variable or feature flag to skip MinIO setup
- No alternative S3-compatible storage available without container runtime

#### 2. Network Access Restrictions **CRITICAL FINDING**
**Status:** BLOCKING - DNS resolution issue

**Issue:**
The environment uses an HTTP/HTTPS proxy with a JWT-based allowlist. **DNS resolution does not work**, even though HTTP/HTTPS requests work fine through the proxy.

```
$ sudo apt-get update
Err:1 http://security.ubuntu.com/ubuntu noble-security InRelease
  Temporary failure resolving 'security.ubuntu.com'

$ python3 -c "import socket; print(socket.gethostbyname('archive.ubuntu.com'))"
socket.gaierror: [Errno -3] Temporary failure in name resolution

$ curl -I http://archive.ubuntu.com/  # Works! Uses proxy
HTTP/1.1 301 Moved Permanently
```

**Impact:**
- apt-get cannot resolve hostnames (even though downloads would work via proxy)
- Cannot install Docker or any system packages
- Python/direct DNS lookups fail
- curl/wget work fine (they use the HTTP_PROXY)

**Root Cause:**
- Environment sets `HTTP_PROXY` and `HTTPS_PROXY` with allowed_hosts list
- The proxy JWT includes: archive.ubuntu.com, security.ubuntu.com, *.ubuntu.com, registry-1.docker.io, auth.docker.io, download.docker.com, github.com, ppa.launchpadcontent.net, and many others
- **BUT:** DNS resolution happens outside the proxy and fails
- apt-get needs working DNS to resolve package repository hosts

**Evidence of proxy allowlist:**
```bash
$ env | grep -i proxy
https_proxy=http://...jwt_eyJ...@21.0.0.101:15004
# JWT payload includes allowed_hosts with archive.ubuntu.com, *.ubuntu.com, etc.
```

**Missing from allowlist:**
- `dl.min.io` - Explicitly blocked with "host_not_allowed" error

**Why this prevents fixes:**
- apt-get requires DNS resolution before HTTP requests
- Cannot configure apt to use proxy without working DNS first
- This is a fundamental infrastructure limitation

#### 3. Missing Build Dependencies for nsjail
**Status:** PARTIALLY FIXABLE - But blocked by network issues

**Issue:**
```
make[2]: flex: No such file or directory
make[2]: *** [Makefile:70: lexer.h] Error 127
```

**Impact:**
- nsjail binary cannot be built
- Isolation tests that require nsjail will fail
- The kafel library (nsjail dependency) requires flex/bison to build

**What's installed:**
- bison ✓ (already installed)
- flex ✗ (missing)
- libprotobuf-dev ✗ (missing)
- protobuf-compiler ✗ (missing)
- libnl-route-3-dev ✗ (missing)

**Why can't fix:**
- Network is blocked (403), so apt-get cannot download packages
- No offline package cache available
- Cannot manually download .deb files due to network restrictions

#### 4. AppArmor sysctl Settings Not Available
**Status:** NOT FIXABLE - Kernel/security limitation

**Issue:**
```
sysctl: cannot stat /proc/sys/kernel/apparmor_restrict_unprivileged_unconfined: No such file or directory
sysctl: cannot stat /proc/sys/kernel/apparmor_restrict_unprivileged_userns: No such file or directory
```

**Impact:**
- Even if nsjail built, it might not run properly due to AppArmor restrictions
- Required for Ubuntu 24.x+ to run nsjail with user namespaces

**Environment details:**
```bash
$ uname -a
Linux runsc 4.4.0 #1 SMP Sun Jan 10 15:06:54 PST 2016 x86_64
```

**Why can't fix:**
- Very old kernel (4.4.0 from 2016)
- Environment appears to use gVisor's runsc (sandboxed runtime)
- AppArmor sysctl parameters don't exist in this kernel
- This is a fundamental infrastructure limitation

#### 5. PostgreSQL Service Not Available
**Status:** LIKELY BLOCKING - Not verified yet

**Issue:**
The postgres e2e test (`client_server_integration_postgres`) expects PostgreSQL at `localhost:5432`.

**Impact:**
- `client_server_integration_postgres` test will likely fail
- The test tries to run `dropdb` and `createdb` commands
- These commands need PostgreSQL client tools and a running server

**Not verified because:**
- Tests fail earlier due to MinIO requirement
- MinIO setup exits the entire test process before postgres tests run

**Would need:**
- PostgreSQL server running on localhost:5432
- Database user: postgres_user
- Password: test
- Client tools: `dropdb`, `createdb`

### NON-BLOCKING ISSUES (Informational)

#### 6. Playwright Browser Download Failed
**Status:** EXPECTED - Handled gracefully

**Issue:**
```
Using system browsers (Playwright browsers not available on this platform)
```

**Impact:**
- Minimal - the setup script handles this gracefully
- Browser e2e tests would use system browsers if available
- Tests fail earlier due to MinIO anyway

## Summary

**What works:**
- Unit tests (11/11 passing) ✓
- Python environment setup ✓
- Build system (cargo build/test) ✓

**What doesn't work (BLOCKING):**
- Docker/container runtime not available → MinIO can't start → All e2e tests fail
- Network access blocked (403 Forbidden) → Can't install missing packages
- Missing nsjail build dependencies (flex, protobuf, libnl)
- Kernel limitations (old kernel 4.4.0 with gVisor, no AppArmor support)

## Test Execution Results

### Unit Tests
**Status:** PASSING ✓

All unit tests pass successfully:
```
running 11 tests
test server::config::tests::test_email_backends_validation_both_configured ... ok
test server::config::tests::test_email_backends_validation_file_only ... ok
test server::config::tests::test_email_backends_validation_none_configured ... ok
test server::config::tests::test_email_backends_validation_smtp_only ... ok
test email::templating::tests::test_render_confirmation_with_public_url ... ok
test email::templating::tests::test_render_confirmation_with_web_default ... ok
test email::templating::tests::test_render_confirmation_without_web ... ok
test templating::tests::deserializes_properly ... ok
test templating::tests::executes_properly ... ok
test templating::tests::serializes_properly ... ok
test server::config::tests::test_env_var_override_public_url ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### E2E Tests
**Status:** FAILING ✗

E2E tests fail at startup due to MinIO setup:
```
test browser_e2e ... Starting MinIO (one-time setup)...
MinIO setup failed:
tests/run_minio.sh: line 16: docker: command not found

Failed to setup MinIO: Failed to run MinIO setup script
```

## Comparison with GitHub Actions

The GitHub Actions workflow (`.github/workflows/tests.yml`) successfully runs tests because it has:

1. **Docker service available** - Standard on GitHub Actions runners
2. **PostgreSQL service** - Started as a service container
3. **Network access** - Full internet access for downloading dependencies
4. **System packages** - Can install nsjail dependencies:
   ```yaml
   - name: Install nsjail requirements
     run: sudo apt-get install -y libprotobuf-dev protobuf-compiler libnl-route-3-dev
   ```
5. **Modern Ubuntu** - Ubuntu 22.04+ with proper kernel support

## Environment Information

```
Platform: Linux
Kernel: 4.4.0 (runsc - appears to be gVisor)
OS: Linux 4.4.0 #1 SMP Sun Jan 10 15:06:54 PST 2016 x86_64
Python: 3.11.14
Rust: stable toolchain
Working Directory: /home/user/ayb
Git Repo: Yes
Current Branch: claude/investigate-test-environment-011CUUgwRzWhZfDWLvJebUpM
```

## Recommended Next Steps

### **CRITICAL**: Fix DNS Resolution

**The #1 blocker is broken DNS resolution.**

The environment has an HTTP proxy with many domains allowlisted (including most we need!), but DNS resolution fails completely. This prevents apt-get from working even though the actual downloads would succeed via proxy.

**Required fix:**
- Enable DNS resolution for all hosts (or at least for allowlisted proxy hosts)
- OR: Configure the environment so apt can resolve hostnames through the proxy

**Current state:**
- `HTTP_PROXY`/`HTTPS_PROXY` set with JWT containing `allowed_hosts`
- Proxy allowlist already includes: `archive.ubuntu.com`, `*.ubuntu.com`, `registry-1.docker.io`, `download.docker.com`, `github.com`, `ppa.launchpadcontent.net`, and many others
- `curl`/`wget` work fine (use proxy)
- `apt-get` fails (needs DNS before it can use proxy)
- Direct DNS lookups fail: `socket.gaierror: [Errno -3] Temporary failure in name resolution`

### Additional Network Allowlist

**Missing domains that need to be added to the proxy allowlist:**

**Critical:**
- `dl.min.io` - MinIO binary downloads (currently blocked with `x-deny-reason: host_not_allowed`)

**Already in proxy allowlist (confirmed working via HTTP):**
- ✓ `archive.ubuntu.com`
- ✓ `security.ubuntu.com`
- ✓ `*.ubuntu.com`
- ✓ `registry-1.docker.io`
- ✓ `auth.docker.io`
- ✓ `download.docker.com`
- ✓ `hub.docker.com`
- ✓ `production.cloudflare.docker.com`
- ✓ `github.com`
- ✓ `raw.githubusercontent.com`
- ✓ `ppa.launchpadcontent.net`
- ✓ `static.crates.io`
- ✓ `index.crates.io`
- ✓ `crates.io`

**Nice to have:**
- `playwright.azureedge.net` - Playwright browser downloads

### Docker Solution

**Option 1: Install Docker (Recommended)**
```bash
# Install Docker in the sandbox
apt-get update
apt-get install -y docker.io
systemctl start docker  # or dockerd if systemd not available
```

**Option 2: Install Podman (Docker alternative)**
```bash
apt-get update
apt-get install -y podman
# Alias podman as docker: ln -s /usr/bin/podman /usr/bin/docker
```

**Option 3: Native MinIO binary (No containers)**
```bash
# Download MinIO standalone binary
wget https://dl.min.io/server/minio/release/linux-amd64/minio
chmod +x minio
# Start MinIO: MINIO_ROOT_USER=minioadmin MINIO_ROOT_PASSWORD=minioadmin ./minio server /tmp/minio-data
```
*Note: Would require modifying `tests/run_minio.sh` to use native binary instead of Docker*

### PostgreSQL Solution

**Option 1: Install PostgreSQL server (Recommended)**
```bash
apt-get update
apt-get install -y postgresql postgresql-client
# Configure to listen on localhost:5432
# Create user: sudo -u postgres createuser -s postgres_user
# Set password: sudo -u postgres psql -c "ALTER USER postgres_user PASSWORD 'test';"
```

**Option 2: PostgreSQL in Docker (requires Docker from above)**
```bash
docker run -d --name postgres-test \
  -e POSTGRES_USER=postgres_user \
  -e POSTGRES_PASSWORD=test \
  -e POSTGRES_DB=test_db \
  -p 5432:5432 \
  postgres:latest
```

**Option 3: Skip postgres tests (easiest)**
- Only run `cargo test client_server_integration_sqlite`
- Skip `cargo test client_server_integration_postgres`
- This still provides good coverage (postgres test is a variant, not unique functionality)

### System Packages

Once network is unblocked, install nsjail dependencies:
```bash
sudo apt-get update
sudo apt-get install -y flex bison libprotobuf-dev protobuf-compiler libnl-route-3-dev
```

### Minimal Setup for Core Tests

If you want to get **something** working quickly:

1. **Network allowlist** → Unblock Ubuntu repos + Docker Hub
2. **Install Docker** → Option 1 or 2 above
3. **Install system packages** → flex, protobuf, libnl packages
4. **Skip PostgreSQL** → Just run SQLite tests

This would enable:
- ✓ Unit tests (already working)
- ✓ SQLite e2e tests (with MinIO via Docker)
- ✓ Browser e2e tests (with MinIO via Docker)
- ✗ PostgreSQL e2e tests (optional, can skip)

**Note:** The user explicitly stated not to modify the code, as tests pass locally and on GitHub Actions. These are purely environmental limitations of the Claude Code sandbox.
