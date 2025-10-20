# Running Tests

This document describes how to run the tests for nostr-postgres-db.

## Prerequisites

- Rust toolchain (1.90.0 or later)
- PostgreSQL database (16 or later)

## Unit Tests

Unit tests do not require a database connection and can be run with:

```bash
cargo test --lib
```

These tests cover:
- Query filter generation and SQL parameter binding
- Event and tag data model conversions
- Filter validation logic

## Integration Tests

Integration tests use [testcontainers-rs](https://github.com/testcontainers/testcontainers-rs) to automatically manage PostgreSQL containers. **No manual database setup is required!**

### Automatic Container Management

The integration tests will automatically:
1. Start a PostgreSQL container using Docker
2. Run migrations
3. Execute tests
4. Clean up the container when tests complete

**Requirements:**
- Docker installed and running
- Docker daemon accessible to your user

### Manual Database Configuration (Optional)

If you prefer to use an existing database instead of testcontainers, set the `DATABASE_URL` environment variable:

```bash
export DATABASE_URL="postgres://postgres:password@localhost:5432/nostr_test"
```

The tests will detect this and skip container creation, using your provided database instead.

### Setting up Docker

If you don't have Docker installed:

**Linux:**
```bash
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo usermod -aG docker $USER
```

**macOS:**
Install [Docker Desktop](https://www.docker.com/products/docker-desktop)

**Windows:**
Install [Docker Desktop](https://www.docker.com/products/docker-desktop)

### Running Integration Tests

```bash
# Run all tests (unit + integration)
cargo test

# Run only integration tests
cargo test --test integration_tests

# Run specific integration test
cargo test --test integration_tests test_save_and_retrieve_event

# Run with output
cargo test -- --nocapture
```

## Test Coverage

### Unit Tests (26 tests)

**Query Module:**
- Filter to SQL conversion with various filter types
- Parameter binding for IDs, authors, kinds, timestamps
- Generic tag filtering
- Limit handling
- Filter validation

**Model Module:**
- Event data conversion from nostr Event to database representation
- Tag extraction from events
- FlatBuffer encoding/decoding
- Database model field validation

### Integration Tests (22 tests)

These tests verify end-to-end database operations:

1. **Event Operations:**
   - Save and retrieve events
   - Handle duplicate events
   - Check event status (saved/deleted/not existent)

2. **Query Operations:**
   - Filter by author
   - Filter by event kinds
   - Filter by event IDs
   - Filter by timestamp (since/until)
   - Filter by tags
   - Limit results
   - Complex multi-filter queries

3. **Delete Operations:**
   - Delete single events
   - Delete by author
   - Verify deleted events are not returned

4. **Special Cases:**
   - Events with multiple tags
   - Backend identification
   - Unsupported operations (wipe)

## Development Environment

The project includes dev-container configuration for VS Code development:

- `.devcontainer/devcontainer.json` - VS Code dev container configuration
- `.devcontainer/docker-compose.yml` - PostgreSQL service definition
- `.devcontainer/Dockerfile` - Development environment setup

This is separate from testcontainers and provides a complete development environment.

## Troubleshooting

### Docker Issues

If tests fail to start containers:

1. Verify Docker is running:
   ```bash
   docker ps
   ```

2. Check Docker daemon logs:
   ```bash
   docker logs
   ```

3. Ensure your user has Docker permissions:
   ```bash
   docker run hello-world
   ```

### Using Manual Database

If you prefer not to use testcontainers:

1. Set the DATABASE_URL environment variable:
   ```bash
   export DATABASE_URL="postgres://postgres:password@localhost:5432/nostr_test"
   ```

2. Ensure PostgreSQL is running and accessible

3. Run tests - they will use your database instead of containers

### Test Database Cleanup

With testcontainers, cleanup is automatic. If using a manual database:

```sql
-- Connect to the test database
\c nostr_test

-- Delete all test data
DELETE FROM event_tags;
DELETE FROM events;
```

## Performance Testing

To test with larger datasets:

```bash
# Run tests with release optimizations
cargo test --release

# Run specific performance-critical tests
cargo test --release test_query_with_limit
```
