# Required Domains for Claude Code Test Environment

This document lists all domains that need to be accessible through the proxy to enable running the e2e test suite.

## Critical - Required to Build Project

### Rust/Cargo (crates.io)
**Status:** Currently blocked (403)
**Purpose:** Download Rust dependencies, build the project

- `crates.io` - Main crates.io site
- `index.crates.io` - Package index (sparse protocol)
- `static.crates.io` - Package downloads and static assets
- `github.com` - Many crates are hosted on GitHub
- `raw.githubusercontent.com` - Raw file access for GitHub-hosted crates

**Why critical:** Cannot run `cargo build` or `cargo test` without these. This is the #1 blocker.

## High Priority - Required for Test Infrastructure

### Docker Image Registry
**Status:** Docker not installed
**Purpose:** Pull MinIO image for S3 testing

- `registry-1.docker.io` - Docker Hub registry API
- `auth.docker.io` - Docker Hub authentication
- `production.cloudflare.docker.com` - Docker CDN
- `registry.hub.docker.com` - Docker Hub website

**Why important:** S3 snapshot tests require MinIO running in Docker container.

### Ubuntu Package Repositories
**Status:** Currently blocked (DNS failures)
**Purpose:** Install system dependencies (nsjail build tools, Docker, PostgreSQL)

- `archive.ubuntu.com` - Main Ubuntu package repository
- `security.ubuntu.com` - Ubuntu security updates
- `ports.ubuntu.com` - Ubuntu for ARM architectures (if needed)
- `ppa.launchpadcontent.net` - Personal Package Archives (PPAs)

**Why important:** Need to install libprotobuf-dev, protobuf-compiler, libnl-route-3-dev, flex, docker.io, postgresql

### PostgreSQL Repository (if not in Ubuntu repos)
**Status:** PostgreSQL not installed
**Purpose:** Install PostgreSQL for metadata backend testing

- `apt.postgresql.org` - Official PostgreSQL apt repository
- `yum.postgresql.org` - PostgreSQL yum repository (if needed)

**Why important:** Half of the tests use PostgreSQL as metadata backend.

## Medium Priority - Enhances Test Coverage

### GitHub (for nsjail build)
**Status:** May already be accessible
**Purpose:** Clone nsjail source code

- `github.com` - Git clone operations
- `raw.githubusercontent.com` - Raw file downloads

**Why useful:** Build nsjail from source for isolation testing. Tests can run without nsjail but lose isolation coverage.

### NPM/Node (if web UI tests need it)
**Status:** Unknown if needed
**Purpose:** Potential frontend dependencies

- `registry.npmjs.org` - NPM package registry
- `npm.pkg.github.com` - GitHub Packages for NPM

**Why useful:** May be needed for web UI testing dependencies, though Playwright is already installed.

## Already Working

### Python Package Index (PyPI)
**Status:** ✅ Working
**Purpose:** Install awscli, playwright

- `pypi.org` - PyPI website
- `files.pythonhosted.org` - Python package CDN

These are already accessible and working correctly.

## Summary by Priority

### Must Have (Tier 1) - Cannot run any tests without these:
```
crates.io
index.crates.io
static.crates.io
github.com
raw.githubusercontent.com
```

### Should Have (Tier 2) - Limited test coverage without these:
```
archive.ubuntu.com
security.ubuntu.com
ppa.launchpadcontent.net
registry-1.docker.io
auth.docker.io
production.cloudflare.docker.com
apt.postgresql.org
```

### Nice to Have (Tier 3) - Enhanced coverage:
```
registry.hub.docker.com
ports.ubuntu.com
yum.postgresql.org
registry.npmjs.org
```

## Minimal Configuration

If only enabling a **minimal set** for basic functionality:

```
# Tier 1: Rust development (absolute minimum)
crates.io
index.crates.io
static.crates.io
github.com
raw.githubusercontent.com

# Tier 2: System packages (needed for dependencies)
archive.ubuntu.com
security.ubuntu.com
```

This would enable:
- Building the Rust project ✓
- Running unit tests ✓
- Installing system dependencies ✓

This would NOT enable:
- Docker/MinIO (S3 tests would fail)
- PostgreSQL (only SQLite tests would run)
- Full isolation testing (nsjail may have issues)

## Recommended Configuration

For **full test suite compatibility**, enable all Tier 1 and Tier 2 domains above.
