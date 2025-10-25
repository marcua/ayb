# Test Environment Issues

This document summarizes issues encountered when attempting to run e2e tests in the Claude Code environment, comparing against the GitHub Actions workflow.

## Environment Comparison

### GitHub Actions Workflow (`.github/workflows/tests.yml`)
The CI environment includes:
1. PostgreSQL service running on port 5432
2. Installed nsjail dependencies (libprotobuf-dev, protobuf-compiler, libnl-route-3-dev)
3. Docker for running MinIO
4. Python environment with awscli and playwright
5. Full network access to package registries

### Claude Code Environment
```
Kernel: Linux 4.4.0 (runsc - gVisor sandbox)
Architecture: x86_64
Python: 3.11.14
Cargo: 1.90.0
Sudo: Available
Docker: Not available
```

---

## Things I Fixed

### 1. Python Virtual Environment Setup
**Status**: FIXED
**What was done**: Successfully created Python venv and installed awscli and playwright packages
**Result**: `tests/test-env/` directory created with working Python environment
**Evidence**: pip successfully downloaded and installed all required Python packages

### 2. Cargo Build System
**Status**: FIXED (Clarified)
**Initial concern**: Web access to crates.io HTML interface is blocked with "data access policy" error
**Resolution**: Despite web interface being blocked, cargo's package download protocol works fine
**Evidence**:
- `cargo build` completed successfully in 58.46s
- All dependencies downloaded from crates.io registry
- Project compiles without errors
**Conclusion**: The crates.io web restriction does not affect cargo functionality

---

## Things I Need Help With

### 1. Playwright Driver Download (CRITICAL BLOCKER)
**Status**: BLOCKS ALL TESTS
**Description**: The `playwright` Rust crate (v0.0.20) tries to download its driver binary from `playwright.azureedge.net` during compilation, which fails because the proxy explicitly blocks this domain.

**Cargo build error**:
```
thread 'main' panicked at playwright-0.0.20/src/build.rs:50:48:
called `Result::unwrap()` on an `Err` value: reqwest::Error {
  kind: Request,
  url: "https://playwright.azureedge.net/builds/driver/next/playwright-1.11.0-1620331022000-linux.zip",
  source: hyper::Error(Connect, "unsuccessful tunnel")
}
```

**Proxy error (verified with curl)**:
```
HTTP/1.1 403 Forbidden
x-deny-reason: host_not_allowed
```

**Impact**: Cannot compile tests at all - this is a complete blocker
**Affected**: All tests (since tests depend on the playwright crate)
**Root cause**: The environment uses an HTTP proxy with JWT authentication. The proxy's `allowed_hosts` list does not include `playwright.azureedge.net` or `*.azureedge.net`. The proxy explicitly rejects connections to this domain.

**What needs to be done**: Add `playwright.azureedge.net` (or `*.azureedge.net`) to the proxy's allowed hosts list in the JWT token configuration.

**Possible solutions**:
1. **Recommended**: Add `playwright.azureedge.net` or `*.azureedge.net` to the proxy's allowed hosts
2. Pre-download the playwright driver and cache it in the environment
3. Mock/stub the playwright dependency for non-browser tests (requires code changes to test suite)

### 2. Docker Not Available
**Status**: BLOCKS S3/SNAPSHOT TESTS
**Description**: Docker command not found in environment
**Impact**: Cannot run MinIO container for S3 snapshot tests
**Test**: `which docker` fails
**Setup script failure**:
```bash
tests/run_minio.sh: line 16: docker: command not found
```
**Affected tests**: All snapshot tests that require S3 storage (`tests/e2e_tests/snapshot_tests.rs`, `tests/browser_e2e_tests/snapshots.rs`)
**Possible solutions**:
1. Enable Docker in Claude Code environment
2. Use alternative S3 mock that doesn't require Docker
3. Skip S3 tests in Claude Code environment

### 3. PostgreSQL Not Running
**Status**: BLOCKS POSTGRESQL BACKEND TESTS
**Description**: PostgreSQL service not available
**Impact**: Tests that use PostgreSQL as metadata backend will fail
**Test**: `pg_isready -h localhost -p 5432` command not found
**Note**: GitHub Actions starts PostgreSQL as a service container
**Affected tests**: Any test using PostgreSQL metadata database (tests work with SQLite by default, so this is lower priority)
**Possible solutions**:
1. Start PostgreSQL service in Claude Code environment
2. Install PostgreSQL locally in environment
3. Skip PostgreSQL-specific tests, run only SQLite tests

### 4. nsjail Cannot Be Built
**Status**: BLOCKS ISOLATION TESTS
**Description**: Missing build dependencies for nsjail:
- `flex` (lexer generator)
- `libprotobuf-dev`
- `protobuf-compiler`
- `libnl-route-3-dev`

**Build error**:
```
make[2]: flex: No such file or directory
make[2]: *** [Makefile:70: lexer.h] Error 127
```

**Additional problem**: Cannot install via apt-get due to network restrictions
```
Err:2 http://archive.ubuntu.com/ubuntu noble InRelease
  Temporary failure resolving 'archive.ubuntu.com'
```

**Affected tests**: Tests requiring nsjail sandboxing (isolation features)
**Note**: nsjail is Linux-only, used for query sandboxing
**Possible solutions**:
1. Pre-install nsjail build dependencies in Claude Code environment
2. Provide pre-built nsjail binary
3. Skip isolation tests in Claude Code environment

### 5. AppArmor sysctl Parameters Not Available
**Status**: KERNEL/SECURITY RESTRICTION
**Description**: Cannot set required sysctl parameters for nsjail on Ubuntu 24.x+
```
sysctl: cannot stat /proc/sys/kernel/apparmor_restrict_unprivileged_unconfined
sysctl: cannot stat /proc/sys/kernel/apparmor_restrict_unprivileged_userns
```
**Impact**: Even if nsjail were built, it might not run
**Root cause**: gVisor sandbox doesn't expose these kernel parameters
**Possible solutions**:
1. Run with different AppArmor configuration
2. Disable AppArmor checks for nsjail
3. Accept that isolation tests won't work in this environment

### 6. Network Restrictions (APT repositories)
**Status**: PREVENTS PACKAGE INSTALLATION
**Description**: Cannot update apt or install packages
**Blocked domains**:
- archive.ubuntu.com
- security.ubuntu.com
- ppa.launchpadcontent.net

**Impact**: Cannot install any missing system dependencies via apt-get
**Possible solutions**:
1. Allow these Ubuntu repository domains
2. Pre-install required packages in Claude Code base environment
3. Provide alternative package installation method

---

## Summary

### What Works
- Python virtual environment setup
- Cargo build system (despite crates.io web interface being blocked)
- Basic compilation (non-test code)
- Source code downloads successfully

### Primary Blocker
**Playwright driver download failure** - This prevents test compilation entirely. Until this is resolved, no tests can run.

### Secondary Blockers (would affect tests after playwright is fixed)
1. No Docker - blocks S3/snapshot tests
2. No PostgreSQL - blocks PostgreSQL backend tests
3. No nsjail - blocks isolation tests
4. No apt packages - prevents installing dependencies

### Recommendation
To run tests in Claude Code environment, need to either:
1. **Option A**: Fix network restrictions (allow playwright.azureedge.net, Docker, apt repositories)
2. **Option B**: Restructure test dependencies to avoid requiring playwright binary download during build
3. **Option C**: Run a subset of tests that don't require these external dependencies (would require test refactoring)

**Bottom line**: The environment is fundamentally incompatible with the current test setup due to network restrictions and missing container runtime. This is a sandbox/environment issue, not a code issue.
