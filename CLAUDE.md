# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**ayb** is a multi-tenant database management system written in Rust that makes it easy to create, host, and share embedded databases like SQLite and DuckDB. It provides both HTTP API and web interface for database operations.

## Development Commands

### Build and Run
```bash
# Build the project
cargo build

# Run server with debug logging
make server

# Generate default server config
ayb default_server_config > ayb.toml

# Run client commands
ayb client --url http://127.0.0.1:5433 <command>
```

See README.md for the full set of documented commands/use cases.

### Testing

**For Claude Code for Web / Constrained Environments:**
```bash
# First-time setup: Install system dependencies (requires sudo)
# Only needed in constrained environments (gVisor, containers, etc.)
# Skip this on local machines where packages are already installed
tests/claude_code_setup.sh

# Then run the standard test setup
tests/set_up_e2e_env.sh
```

**For Local Development:**
```bash
# Set up test environment (required before first test run)
tests/set_up_e2e_env.sh

# Run all tests
make test

# Run specific integration test
make test TEST=client_server_integration_sqlite

# The test setup script installs:
# - Python virtual environment with awscli
# - MinIO for S3-compatible storage testing
# - Native isolation (Landlock, cgroups, rlimit)
```

If tests fail with S3 errors, run `tests/run_minio.sh` and try again.

### Code Quality
CRITICAL: Run `make lint` any time you edit code. Your task is not complete if either of these report an error: fix all warnings and errors before reporting back.

```bash
# Format code and run clippy lints
make lint
```

Also CRITICAL: whenever you modify code, run the most relevant tests (or all tests) using the `make test` and `make test TEST=...` commands above.

## Architecture

### Core Structure
```
src/
├── ayb_db/           # Metadata database interfaces & models
├── client/           # CLI client implementation
├── server/           # HTTP server (API + web UI)
├── hosted_db/        # Database hosting logic (SQLite operations)
├── email/            # Email templating & sending
├── http/             # HTTP request/response structures
└── templating/       # Tera template utilities
```

### Key Components

**Server (`src/server/`)**
- HTTP API endpoints under `/v1/` routes
- Web UI endpoints serving HTML via Tera templates
- JWT-based authentication with fernet encryption
- Role-based permissions (no-access, read-only, read-write, manager)
- S3 snapshot backups with automated scheduling

**Database Layer (`src/ayb_db/`)**
- Supports both SQLite and PostgreSQL for metadata storage
- Entity models (users/organizations) with permission system
- Database migrations and schema evolution

**Hosted Database (`src/hosted_db/`)**
- SQLite query execution with safety constraints
- Native Linux isolation (Landlock, cgroups, rlimit) for multi-tenant security
- Database file organization and path management

**Client (`src/client/`)**
- Full-featured CLI with subcommands for all operations
- HTTP client for API communication
- Configuration management for server URL and tokens

### Technology Stack
- **Web Framework**: Actix Web 4.11.0
- **Database**: SQLite (rusqlite) and PostgreSQL (sqlx)
- **Authentication**: JWT via fernet encryption
- **Templating**: Tera for HTML templates
- **CLI**: clap for argument parsing
- **Async**: tokio runtime
- **Backup**: S3-compatible storage with zstd compression
- **Isolation**: Native Linux primitives (Landlock, cgroups, rlimit)

## Configuration

Server configuration uses TOML format (`ayb.toml`) with sections for:
- Database connection (SQLite or PostgreSQL)
- Authentication (fernet key, token expiration)
- Email (SMTP configuration)
- Snapshots (S3 configuration and scheduling)
- CORS settings

## Key Development Patterns

### Multi-Tenancy
- Entities represent users and organizations
- Permissions are granular: no-access, read-only, read-write, manager
- Database isolation via native Linux sandboxing and SQLite safety constraints

### Error Handling
- Uses Tera templates for consistent error snippet formatting
- Centralized error handling in server endpoints
- Comprehensive error types with derive_more for clear error messages

### Testing Strategy
- End-to-end integration tests that mirror realistic usage
- Mock services: SMTP server, MinIO S3, isolated environments
- Tests both SQLite and PostgreSQL metadata storage backends
- Minimal unit tests, focuses on comprehensive integration testing
