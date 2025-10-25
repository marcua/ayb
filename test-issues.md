# Test Environment Issues in Claude Code

This document summarizes issues encountered when trying to run the e2e test suite in Claude Code, following the same process as GitHub Actions.

## GitHub Actions Test Process

The `.github/workflows/tests.yml` workflow performs the following steps:
1. Sets up PostgreSQL service container
2. Installs nsjail dependencies: `libprotobuf-dev`, `protobuf-compiler`, `libnl-route-3-dev`
3. Runs `tests/set_up_e2e_env.sh` which:
   - Creates Python virtual environment
   - Installs `awscli` and `playwright` via pip
   - Installs Playwright browsers (chromium)
   - Starts MinIO via Docker
   - Builds nsjail binary
   - Sets AppArmor sysctl parameters
4. Runs `cargo test --verbose` with `RUST_BACKTRACE=1`

## Issues Encountered

### Critical Network Issues

**1. Network DNS Resolution Failures**
- Environment cannot resolve external domain names
- Affects: apt package repositories, crates.io
- Errors:
  ```
  Temporary failure resolving 'archive.ubuntu.com'
  Temporary failure resolving 'security.ubuntu.com'
  Temporary failure resolving 'ppa.launchpadcontent.net'
  ```

**2. Cargo Cannot Access crates.io**
- HTTPS requests to crates.io index blocked
- Error: `failed to get successful HTTP response from https://index.crates.io/config.json (21.0.0.31), got 403 Access denied`
- Result: Cannot download dependencies, cannot build project
- Tried offline mode but no cached dependencies available

**3. PyPI Access Works**
- Successfully downloaded Python packages (awscli, playwright)
- Suggests PyPI traffic is allowed but apt/crates.io are not

### Missing System Dependencies

**4. nsjail Build Dependencies Not Installed**
- Required packages not available: `libprotobuf-dev`, `protobuf-compiler`, `libnl-route-3-dev`
- Cannot install due to network DNS failures (#1)
- Also missing `flex` tool needed for nsjail build:
  ```
  flex lexer.l
  make[2]: flex: No such file or directory
  ```

**5. Docker Not Available**
- Docker command not found
- MinIO cannot start (requires Docker)
- Affects: S3 snapshot tests

**6. PostgreSQL Not Available**
- No PostgreSQL service running
- GitHub Actions uses service container for this
- Missing: `pg_isready`, `psql` commands
- Affects: Tests using PostgreSQL metadata backend

### Platform/Configuration Issues

**7. AppArmor sysctl Parameters Don't Exist**
- Files don't exist:
  - `/proc/sys/kernel/apparmor_restrict_unprivileged_unconfined`
  - `/proc/sys/kernel/apparmor_restrict_unprivileged_userns`
- Required by nsjail on Ubuntu 24+
- May indicate kernel configuration differences

**8. Playwright Browsers Cannot Install**
- Message: "Using system browsers (Playwright browsers not available on this platform)"
- Chromium browser needed for web UI tests
- Platform incompatibility with headless environment

### What Works

- Python 3.11.14 available
- pip package installation works (PyPI accessible)
- awscli and playwright packages installed successfully
- curl available
- Basic bash scripting works

## Root Cause Analysis

The primary blocker is **proxy-based network access control** in the Claude Code environment:

### The Proxy Configuration
Environment uses an HTTP proxy for all external traffic:
```
HTTPS_PROXY=http://container_container_011CUUcuLoCTTFTo3wHHNS4t--bland-dismal-sharp-site:noauth@21.0.0.31:15002
NO_PROXY=localhost,127.0.0.1,169.254.169.254,metadata.google.internal,*.svc.cluster.local,*.local,*.googleapis.com,*.google.com
```

### Key Findings
1. **PyPI traffic allowed**: Successfully downloaded Python packages (awscli, playwright) through proxy
2. **crates.io traffic blocked**: Proxy returns `403 Access denied` for `https://index.crates.io/config.json`
3. **DNS only works through proxy**: Without proxy, cannot resolve any domain names
4. **apt repositories fail**: DNS resolution failures for Ubuntu package repos
5. **Docker unavailable**: Not installed in environment
6. **PostgreSQL unavailable**: No service running (GitHub Actions uses service container)

### What I Tried

**Attempt 1: Bypass proxy for crates.io**
- Added `crates.io` to `NO_PROXY` environment variable
- Result: DNS resolution failed (cannot resolve without proxy)
- Conclusion: Proxy is required for DNS, but proxy blocks crates.io

**Attempt 2: Use cargo sparse protocol**
- Created `.cargo/config.toml` with `protocol = "sparse"`
- Result: Still gets 403 from same URL
- Conclusion: Protocol change doesn't help; proxy blocks all crates.io access

**Attempt 3: Install nsjail dependencies**
- Tried `sudo apt-get install libprotobuf-dev protobuf-compiler libnl-route-3-dev`
- Result: DNS failures resolving apt repositories
- Conclusion: Cannot install system packages

## Impact Assessment

**Cannot run any tests** because:
1. **Cannot build the project**: Cargo blocked from accessing crates.io (403 via proxy)
2. **Cannot set up test infrastructure**: Docker/MinIO unavailable, nsjail cannot be built
3. **Missing required dependencies**: System packages cannot be installed, PostgreSQL unavailable

The environment is fundamentally incompatible with the test setup requirements due to proxy restrictions.

## Things I Fixed

**None** - All issues are environment-level restrictions that cannot be fixed without changing:
- Proxy allowlist configuration (to permit crates.io)
- Installing Docker
- Installing/starting PostgreSQL service
- Installing system packages for nsjail

## Things I Need Help With

### Critical Blockers (Cannot Proceed Without These)

**1. Enable crates.io Access Through Proxy**
- Current: Proxy returns 403 for all crates.io URLs
- Need: Add `crates.io`, `index.crates.io`, `static.crates.io` to proxy allowlist
- Impact: Completely blocks Rust development
- Evidence: PyPI works fine through same proxy, proving selective filtering

**2. Install Docker or Alternative Container Runtime**
- Current: Docker command not found
- Need: Docker installation or equivalent (podman, etc.)
- Impact: Cannot run MinIO for S3 snapshot tests
- Workaround possibility: Could skip S3 tests if environment variable is set

**3. Provide PostgreSQL Service**
- Current: No PostgreSQL server running
- Need: PostgreSQL 12+ running on localhost:5432
- Impact: Cannot run tests with PostgreSQL metadata backend
- GitHub Actions solution: Service container
- Alternative: SQLite-only testing (but loses PostgreSQL coverage)

### Secondary Issues (Can Work Around)

**4. System Package Installation Failures**
- Current: apt repositories unreachable via proxy
- Need: Enable access to Ubuntu package repos OR pre-install packages
- Packages needed: `libprotobuf-dev`, `protobuf-compiler`, `libnl-route-3-dev`, `flex`
- Impact: Cannot build nsjail for isolation testing
- Workaround: Tests can skip isolated mode on platforms without nsjail

**5. AppArmor sysctl Parameters Missing**
- Current: `/proc/sys/kernel/apparmor_restrict_unprivileged_*` files don't exist
- Need: Kernel with AppArmor support or skip sysctl commands
- Impact: Even if nsjail built, may not run properly
- Workaround: Tests already handle missing nsjail gracefully

**6. Playwright Browsers Cannot Install**
- Current: "Playwright browsers not available on this platform"
- Impact: Web UI tests may fail if they require chromium
- Workaround: Tests may work with system browsers if available

### Recommended Immediate Action

**Enable crates.io in proxy allowlist** - This single change would:
- Allow building the project with `cargo build`
- Enable running unit tests that don't require external services
- Make the environment minimally functional for Rust development

The other issues prevent full e2e test suite execution but could be worked around or documented as known limitations.
