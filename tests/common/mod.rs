// Common utilities for integration tests
use std::env;

use nostr::prelude::*;
use nostr_database::prelude::*;
use nostr_postgres_db::NostrPostgres;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

/// Official Postgres image the throwaway test database runs on
pub const POSTGRES_IMAGE: &str = "postgres";
pub const POSTGRES_TAG: &str = "16-alpine";

pub struct TestDatabase {
    pub db: NostrPostgres,
    #[allow(dead_code)]
    container: Option<ContainerAsync<GenericImage>>,
}

/// Start a throwaway Postgres container and return it together with its connection string
///
/// This is what `testcontainers_modules::postgres::Postgres` used to do for us: the
/// official image with `postgres` as user, password and database name. The image
/// starts a temporary server during initialisation (its log goes to stdout) and the
/// real one afterwards (logging to stderr), so both streams have to report readiness.
pub async fn start_postgres() -> (ContainerAsync<GenericImage>, String) {
    let ready = "database system is ready to accept connections";
    let container = GenericImage::new(POSTGRES_IMAGE, POSTGRES_TAG)
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(ready))
        .with_wait_for(WaitFor::message_on_stdout(ready))
        .with_env_var("POSTGRES_DB", "postgres")
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .start()
        .await
        .expect("Failed to start PostgreSQL container");

    let host_port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get container port");

    let db_url = format!("postgres://postgres:postgres@127.0.0.1:{host_port}/postgres");
    (container, db_url)
}

/// Setup a test database connection using testcontainers
///
/// If DATABASE_URL environment variable is set, uses that instead of starting a container.
/// This allows for CI environments or manual testing with existing databases.
pub async fn setup_test_db() -> TestDatabase {
    // Check if DATABASE_URL is set (for CI or manual testing)
    if let Ok(db_url) = env::var("DATABASE_URL") {
        let db = NostrPostgres::new(&db_url)
            .await
            .expect("Failed to connect to test database");
        return TestDatabase {
            db,
            container: None,
        };
    }

    // Start PostgreSQL container using testcontainers
    let (container, db_url) = start_postgres().await;

    // Create database connection
    let db = NostrPostgres::new(&db_url)
        .await
        .expect("Failed to connect to test database");

    TestDatabase {
        db,
        container: Some(container),
    }
}

/// Clean up test data (deletes all events)
pub async fn cleanup_test_db(test_db: &TestDatabase) {
    // Query all events and delete them
    let filter = Filter::new();
    let _ = test_db.db.delete(filter).await;
}
