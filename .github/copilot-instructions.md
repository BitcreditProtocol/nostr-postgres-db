# GitHub Copilot Instructions

## Project Overview

This is **nostr-postgres-db**, a PostgreSQL storage backend for Nostr relays. It provides a high-performance, scalable database implementation for storing and querying Nostr events using PostgreSQL.

## Technology Stack

- **Language**: Rust (Edition 2024)
- **Database**: PostgreSQL
- **ORM**: Diesel 2.x with async support (diesel-async)
- **Connection Pooling**: deadpool
- **Nostr Libraries**: nostr 0.43, nostr-database 0.43
- **Testing**: tokio for async runtime, nostr-relay-builder for integration tests

## Project Structure

```
nostr-postgres-db/
├── src/
│   ├── lib.rs              # Main library entry point
│   ├── postgres.rs         # NostrPostgres implementation
│   ├── model.rs            # Database models (EventDb, EventTagDb)
│   ├── query.rs            # Query builders for filtering events
│   ├── schema/             # Diesel schema definitions
│   └── migrations/         # Migration utilities
├── migrations/
│   └── postgres/           # PostgreSQL migration files
├── examples/
│   └── postgres-relay.rs   # Example relay implementation
├── Cargo.toml
└── LICENSE
```

## Core Components

### NostrPostgres
The main database implementation that:
- Implements the `NostrDatabase` trait from nostr-database
- Manages connection pooling via deadpool
- Handles event storage, retrieval, and deletion
- Runs database migrations automatically on initialization

### Database Models
- `EventDb`: Database representation of Nostr events
- `EventTagDb`: Database representation of event tags
- `EventDataDb`: Container for event and its tags

### Query Building
The `query.rs` module contains query builders that:
- Build complex filter queries based on Nostr Filter specifications
- Support filtering by event IDs, authors, kinds, tags, and time ranges
- Handle proper indexing and efficient PostgreSQL queries

## Code Conventions

### Style Guidelines
- Follow standard Rust formatting (rustfmt)
- Use descriptive variable and function names
- Keep functions focused and modular
- Add doc comments for public APIs

### Naming Conventions
- Database-related structs end with `Db` suffix (e.g., `EventDb`, `EventTagDb`)
- Connection pool types use descriptive aliases (e.g., `PostgresConnectionPool`)
- Async functions should be clearly named

### Error Handling
- Use `DatabaseError` from nostr-database for database-related errors
- Convert Diesel errors to `DatabaseError` using `.map_err(DatabaseError::backend)`
- Provide meaningful error context

### Async Patterns
- All database operations are async using diesel-async
- Use connection pools for efficient resource management
- Transactions are handled via `db.transaction()`

## Development Workflow

### Building
```bash
cargo build
```

### Testing
```bash
cargo test
```

### Running Examples
```bash
# Requires PostgreSQL running at localhost:5432
cargo run --example postgres-relay
```

### Database Migrations
Migrations are automatically run when `NostrPostgres::new()` is called. Manual migrations can be triggered via `run_migrations()`.

## Key Design Decisions

1. **FlatBuffers for Event Payload**: Events are serialized using FlatBuffers for efficient storage and retrieval
2. **Tag Extraction**: Event tags are extracted into a separate table for efficient querying
3. **Soft Deletes**: Events use a `deleted` flag rather than actual deletion
4. **Connection Pooling**: deadpool manages connection lifecycle for optimal performance
5. **Diesel ORM**: Provides type-safe database queries and migrations

## Nostr Protocol Notes

- Events are uniquely identified by their 32-byte ID
- Authors are identified by their 32-byte public key
- Event kinds are 16-bit unsigned integers
- Timestamps are Unix timestamps (seconds since epoch)
- Tags are key-value pairs used for indexing and filtering

## Common Tasks

### Adding a New Query Filter
1. Update `build_filter_query()` in `query.rs`
2. Add appropriate filter logic
3. Ensure proper indexing in the database schema

### Adding a Migration
1. Create new migration directory in `migrations/postgres/`
2. Add `up.sql` and `down.sql` files
3. Test migration up and down paths

### Implementing New Database Methods
1. Add method to `NostrPostgres` in `postgres.rs`
2. Use connection pool via `self.get_connection().await?`
3. Implement proper error handling with `DatabaseError`
4. Consider transaction boundaries for multi-step operations

## Testing Considerations

- Tests require a running PostgreSQL instance
- Connection string format: `postgres://user:password@host:port/database`
- Clean up test data after each test
- Use unique database names for parallel test execution

## License

MIT License - See LICENSE file for details
