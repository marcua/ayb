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

#### 2. Network Access Restrictions
**Status:** BLOCKING - Environment limitation

**Issue:**
```
curl: (56) CONNECT tunnel failed, response 403
HTTP/1.1 403 Forbidden
```

**Impact:**
- Cannot install system packages via apt-get (package sources unreachable)
- Cannot download external resources
- Playwright browser installation fails (gracefully handled)
- Blocks any test that might need external API calls

**Evidence:**
```bash
$ curl -I https://google.com
HTTP/1.1 403 Forbidden

$ sudo apt-get update
Err:1 http://security.ubuntu.com/ubuntu noble-security InRelease
  Temporary failure resolving 'security.ubuntu.com'
```

**Why this prevents fixes:**
- Cannot install missing packages (flex, protobuf, libnl) via apt-get
- Cannot download pre-built binaries from external sources
- Network isolation is at a fundamental level (HTTPS blocked with 403)

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

To run the full test suite in the Claude Code environment, the following infrastructure changes would be needed:

1. **Enable Docker or alternative container runtime** - Required for MinIO (S3 testing)
2. **Relax network restrictions** - Allow HTTPS to trusted domains (package repos, CDNs)
3. **Install system packages** - flex, libprotobuf-dev, protobuf-compiler, libnl-route-3-dev
4. **Provide PostgreSQL service** - For postgres integration tests
5. **Consider kernel upgrade** - Modern kernel with AppArmor support (for nsjail)

**Note:** The user explicitly stated not to modify the code, as tests pass locally and on GitHub Actions. These are purely environmental limitations of the Claude Code sandbox.
