# Dev Container for Nostr Postgres DB

This directory contains the development container configuration for the nostr-postgres-db project.

## What's Included

- **Rust toolchain**: Latest stable Rust with rustfmt and clippy
- **PostgreSQL 16**: Preconfigured test database
- **Development tools**: cargo-watch, cargo-edit, git, curl
- **VS Code extensions**: 
  - rust-analyzer for Rust language support
  - even-better-toml for TOML syntax
  - crates for dependency management
  - vscode-lldb for debugging
  - Docker extension

## Quick Start

### Using VS Code

1. Install the [Dev Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) extension
2. Open this project in VS Code
3. Click "Reopen in Container" when prompted (or use Command Palette: "Dev Containers: Reopen in Container")
4. Wait for the container to build and start
5. You're ready to develop!

### Manual Docker Setup

If you prefer to use the container without VS Code:

```bash
cd .devcontainer
docker-compose up -d
docker-compose exec app bash
```

## Environment

The dev container provides:

- **DATABASE_URL**: Automatically set to `postgres://postgres:password@localhost:5432/nostr_test`
- **RUST_BACKTRACE**: Set to `1` for detailed error traces
- **PostgreSQL**: Running on port 5432 (forwarded to host)

## Running Tests

Inside the container:

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests
cargo test --test integration_tests

# Watch mode (requires cargo-watch)
cargo watch -x test
```

## Database Access

Connect to PostgreSQL from inside the container:

```bash
psql -h localhost -U postgres -d nostr_test
# Password: password
```

Or from your host machine:

```bash
psql -h localhost -U postgres -d nostr_test -p 5432
```

## Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Check without building
cargo check
```

## Linting and Formatting

```bash
# Format code
cargo fmt

# Run clippy lints
cargo clippy

# Fix automatically fixable issues
cargo clippy --fix
```

## Customization

### Modifying the Container

Edit `.devcontainer/devcontainer.json` to:
- Add more VS Code extensions
- Change environment variables
- Modify post-create commands

### Changing PostgreSQL Configuration

Edit `.devcontainer/docker-compose.yml` to:
- Change database credentials
- Add more environment variables
- Mount additional volumes

### Adding Development Tools

Edit `.devcontainer/Dockerfile` to:
- Install additional system packages
- Add more Rust tools
- Configure shell preferences

## Troubleshooting

### Container Won't Start

1. Check Docker is running: `docker ps`
2. Rebuild the container: Use Command Palette -> "Dev Containers: Rebuild Container"
3. Check logs: `docker-compose logs`

### Database Connection Issues

1. Verify PostgreSQL is running: `docker-compose ps`
2. Check logs: `docker-compose logs db`
3. Wait for health check: PostgreSQL takes a few seconds to start

### Performance Issues

If the container is slow:
1. Increase Docker memory allocation (Preferences -> Resources)
2. Use named volumes instead of bind mounts for Cargo cache
3. Consider using a local Rust installation instead

## VS Code Tips

- **Terminal**: Open a terminal inside the container with `Ctrl+\`` (Cmd+\` on Mac)
- **Debugging**: Use the Debug panel (F5) with pre-configured Rust debugging
- **IntelliSense**: rust-analyzer provides code completion, go-to-definition, and more
- **Problem Panel**: See compilation errors and warnings in real-time

## Further Reading

- [VS Code Dev Containers documentation](https://code.visualstudio.com/docs/devcontainers/containers)
- [Docker Compose documentation](https://docs.docker.com/compose/)
- [PostgreSQL Docker image](https://hub.docker.com/_/postgres)
