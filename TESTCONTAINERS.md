# Testcontainers Integration

This project uses [testcontainers-rs](https://github.com/testcontainers/testcontainers-rs) for integration testing with PostgreSQL.

## What is Testcontainers?

Testcontainers is a library that provides lightweight, throwaway instances of databases, message brokers, and other services that can run in Docker containers. It's perfect for integration testing because:

- **Automatic Setup**: No manual database installation or configuration required
- **Isolation**: Each test can have its own database instance
- **Consistency**: Tests run in the same environment everywhere (local, CI, etc.)
- **Cleanup**: Containers are automatically removed after tests complete
- **Fast**: Containers start quickly and are torn down immediately

## How It Works

### In Tests

The `tests/common/mod.rs` module provides a `setup_test_db()` function that:

1. Checks if `DATABASE_URL` environment variable is set
2. If not set, starts a PostgreSQL container automatically
3. Returns a `TestDatabase` struct that holds both the database connection and the container
4. When the `TestDatabase` is dropped, the container is automatically cleaned up

```rust
#[tokio::test]
async fn test_example() {
    let test_db = setup_test_db().await;
    
    // Use test_db.db for database operations
    let event = create_event();
    test_db.db.save_event(&event).await.unwrap();
    
    // Container automatically cleaned up when test_db goes out of scope
}
```

### Container Lifecycle

1. **Start**: Container starts when `setup_test_db()` is called
2. **Use**: Tests interact with the database
3. **Cleanup**: When `TestDatabase` is dropped, container is stopped and removed

### Fallback to Manual Database

For CI environments or when you prefer a manual setup, set:

```bash
export DATABASE_URL="postgres://postgres:password@localhost:5432/nostr_test"
```

The tests will detect this and skip container creation.

## Benefits

### For Developers

- **No Setup Required**: Just run `cargo test` with Docker running
- **Clean State**: Each test run starts with a fresh database
- **No Conflicts**: No port conflicts or leftover data from previous runs
- **Fast Iteration**: No need to manually create/drop databases

### For CI/CD

- **Consistent Environment**: Same PostgreSQL version everywhere
- **Parallel Testing**: Multiple test suites can run simultaneously
- **No External Dependencies**: CI doesn't need pre-configured databases
- **GitHub Actions Integration**: Works seamlessly with service containers

## Requirements

- Docker installed and running
- Docker daemon accessible to your user
- Network connectivity to pull PostgreSQL image (first run only)

## Configuration

### PostgreSQL Version

The tests start the official `postgres:16-alpine` image through testcontainers'
`GenericImage`; see `start_postgres()` in `tests/common/mod.rs`.

To use a different version, change the tag constant there:

```rust
pub const POSTGRES_TAG: &str = "15-alpine"; // Use PostgreSQL 15
```

### Connection Parameters

Default connection:
- **Host**: 127.0.0.1 (localhost)
- **Port**: Random available port (assigned by testcontainers)
- **Database**: postgres
- **User**: postgres
- **Password**: postgres

## Troubleshooting

### Docker Not Running

```
Error: failed to start container
```

**Solution**: Start Docker daemon
```bash
# Linux
sudo systemctl start docker

# macOS/Windows
# Start Docker Desktop application
```

### Permission Denied

```
Error: permission denied while trying to connect to Docker daemon
```

**Solution**: Add your user to the docker group
```bash
sudo usermod -aG docker $USER
# Log out and log back in
```

### Port Conflicts

Testcontainers uses random available ports, so conflicts are rare. If you encounter issues:

```bash
# Check what's using ports
netstat -tlnp | grep LISTEN
```

### Image Pull Failures

```
Error: failed to pull image
```

**Solution**: Check network connectivity and Docker Hub access
```bash
docker pull postgres:16-alpine
```

## Comparison with Alternatives

### vs. Manual Database Setup

| Feature | Testcontainers | Manual Setup |
|---------|----------------|--------------|
| Setup time | Automatic | Manual |
| Consistency | Always same version | Varies by system |
| Isolation | Per-test isolation possible | Shared database |
| Cleanup | Automatic | Manual |
| CI/CD | Easy integration | Requires configuration |

### vs. In-Memory Databases

| Feature | Testcontainers | In-Memory |
|---------|----------------|-----------|
| Accuracy | Real PostgreSQL | Simulated behavior |
| SQL Compatibility | 100% | May differ |
| Performance | Good | Excellent |
| Migration Testing | Yes | Limited |

## Best Practices

1. **Let containers start fresh**: Don't reuse containers between tests
2. **Use cleanup functions**: Always clean up test data with `cleanup_test_db()`
3. **Set timeouts**: Use reasonable timeouts for container startup
4. **Cache images**: Docker will cache PostgreSQL image after first pull
5. **CI optimization**: Consider using service containers in CI for faster startup

## Resources

- [testcontainers-rs documentation](https://docs.rs/testcontainers/)
- [`GenericImage`](https://docs.rs/testcontainers/latest/testcontainers/struct.GenericImage.html)
- [Testcontainers official site](https://www.testcontainers.org/)
