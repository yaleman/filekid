# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FileKid is a web-based file manager with OAuth2 authentication built in Rust. It provides a secure web interface for browsing, uploading, and managing files with support for both local filesystem and temporary directory storage.

## Development Commands

### Building and Running
- `cargo run` - Run the application locally (requires filekid.json config)
- `just run` - Run local debug instance (uses just/justfile)
- `just run_docker` - Run in Docker container

### Code Quality
- `cargo clippy --all-features` - Run Clippy linter
- `cargo test` - Run all tests
- `just check` - Run comprehensive checks (codespell, clippy, test, doc_check)
- `just codespell` - Spell check code and documentation

### Documentation
- `cargo doc --document-private-items` - Generate documentation
- `just doc_check` - Check markdown formatting
- `just doc_fix` - Fix markdown formatting with deno

### Testing and Coverage
- `cargo tarpaulin --out Html` - Generate HTML coverage report
- `just coverage` - Run coverage analysis (outputs to tarpaulin-report.html)

### Release and Security
- `just release_prep` - Full release preparation (runs check, doc, semgrep, cargo deny, release build)
- `just semgrep` - Run security analysis with semgrep
- `just trivy_repo` - Run trivy security scan on repository

## Architecture

### Core Components

1. **Configuration System** (`src/config.rs`)
   - JSON-based configuration with server paths, OAuth settings, and TLS certificates
   - Supports multiple filesystem types (local, tempdir)
   - Runtime validation of paths and certificates

2. **Web Server** (`src/web.rs`)
   - Axum-based async web framework
   - TLS-only operation (no HTTP support)
   - OAuth2/OIDC authentication integration
   - Session management with SQLite backend

3. **Filesystem Abstraction** (`src/fs/`)
   - Trait-based design supporting multiple backends
   - Local filesystem (`local.rs`) and temporary directory (`tempdir.rs`) implementations
   - Async file operations with streaming support

4. **Views/Handlers** (`src/views/`)
   - Askama templates for server-side rendering
   - File browsing, upload, and deletion functionality
   - OAuth2 login/logout handling

### Key Features

- **OAuth2 Authentication**: Full OIDC integration with configurable providers
- **Multi-path Support**: Configure multiple named filesystem paths
- **Secure by Default**: TLS required, no insecure HTTP mode
- **File Operations**: Browse, upload, download, delete with web interface
- **Streaming Uploads**: Efficient large file handling
- **Session Management**: Secure session storage with SQLite

## Configuration

The application requires a `filekid.json` configuration file with:
- TLS certificate and key paths
- OAuth2 provider settings (issuer, client ID, optional secret)
- Server paths configuration
- Network binding settings

## Development Notes

- Uses `#![deny(warnings)]` and strict Clippy rules
- Forbids unsafe code
- Comprehensive error handling with custom Error enum
- Async/await throughout with tokio runtime
- Template-based HTML rendering with Askama
- SQLite for session storage via tower-sessions