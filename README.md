# nostr-postgres-db

Postgres SQL storage backend for Nostr relays.

It implements `nostr-database`'s `NostrDatabase` trait, so it drops into anything
built on [rust-nostr](https://github.com/rust-nostr/nostr) that takes a database —
`nostr-relay-builder`, for instance — in place of the in-memory or LMDB backends.

The code was extracted from the Nostr SDK; the MIT licence and its copyright
(Rust Nostr Developers) come with it.

## Status

Version `0.1.1`, Rust edition 2024. **Not published on crates.io** — depend on it by
path or git reference.

## Public API

Six lines, all of `src/lib.rs`:

| Item | |
|---|---|
| `NostrPostgres::new(connection_string)` | connect and run any pending migrations |
| `NostrPostgres::from_pool(pool)` | reuse an existing `deadpool_postgres::Pool` — this also runs migrations |
| `postgres_connection_pool(..)` | build a pool if you want to hold it yourself |
| `run_migrations(&pool)` | apply migrations on their own |

`NostrPostgres` implements `NostrDatabase`, which is what makes it usable as a relay
backend rather than something you call directly.

## Example

`examples/postgres-relay.rs`, runnable with `cargo run --example postgres-relay`:

```rust
use std::time::Duration;

use nostr_database::prelude::*;
use nostr_postgres_db::NostrPostgres;
use nostr_relay_builder::prelude::*;

// Your database URL
const DB_URL: &str = "postgres://postgres:password@localhost:5432";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Create a nostr db instance and run pending db migrations if any
    let db = NostrPostgres::new(DB_URL).await?;

    // Add db to builder
    let builder = RelayBuilder::default().database(db);

    // Create local relay
    let relay = LocalRelay::new(builder);
    relay.run().await?;
    println!("Url: {}", relay.url().await);

    // Keep up the program
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
```

## Requirements

Postgres, reachable at the connection string you pass. Both `NostrPostgres::new` and
`from_pool` apply the migrations in `src/migrations.rs`, so an empty database is fine.

Dependency floor, from `Cargo.toml`: `nostr` 0.44, `nostr-database` 0.44 (with
`flatbuf`), `tokio-postgres` 0.7, `deadpool-postgres` 0.14.

## Tests

```bash
cargo test
```

Integration tests start a throwaway Postgres in Docker via
[testcontainers](https://github.com/testcontainers/testcontainers-rs), so Docker has
to be running and no local database setup is needed. The details, including the
permission errors you may hit on Linux, are in [TESTCONTAINERS.md](TESTCONTAINERS.md).

## Licence

MIT. See [LICENSE](LICENSE).
