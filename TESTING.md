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

Integration tests require a running PostgreSQL database. 

### Setting up the Test Database

#### Option 1: Using Docker

The easiest way to run integration tests is using the provided dev-container configuration:

1. Open the project in VS Code
2. Install the "Dev Containers" extension
3. Click "Reopen in Container" when prompted
4. The PostgreSQL database will be automatically configured

Or run PostgreSQL manually with Docker:

```bash
docker run -d \
  --name nostr-postgres-test \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_DB=nostr_test \
  -p 5432:5432 \
  postgres:16-alpine
```

#### Option 2: Local PostgreSQL Installation

1. Install PostgreSQL 16 or later
2. Create a test database:

```sql
CREATE DATABASE nostr_test;
```

3. Set the connection string:

```bash
export DATABASE_URL="postgres://postgres:password@localhost:5432/nostr_test"
```

### Running Integration Tests

Once the database is set up:

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

## Continuous Integration

The project includes dev-container configuration for consistent testing environments:

- `.devcontainer/devcontainer.json` - VS Code dev container configuration
- `.devcontainer/docker-compose.yml` - PostgreSQL service definition
- `.devcontainer/Dockerfile` - Development environment setup

## Troubleshooting

### Connection Errors

If tests fail with connection errors:

1. Verify PostgreSQL is running:
   ```bash
   psql -U postgres -h localhost -p 5432 -d nostr_test
   ```

2. Check the DATABASE_URL environment variable:
   ```bash
   echo $DATABASE_URL
   ```

3. Ensure the database exists and is accessible

### Test Database Cleanup

The integration tests automatically clean up after themselves, but if needed:

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
