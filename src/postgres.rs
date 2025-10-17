use deadpool::managed::Object;
use deadpool_postgres::Pool;
use deadpool_postgres::{Manager, ManagerConfig, RecyclingMethod};
use nostr::event::*;
use nostr::filter::Filter;
use nostr_database::*;
use prelude::BoxedFuture;
use tokio_postgres::NoTls;
use tokio_postgres::types::ToSql;

use super::model::{EventDataDb, EventDb};
use crate::query::{filter_to_sql_params, with_limit};

/// Shorthand for a database connection pool type
pub type PostgresConnection = Object<deadpool_postgres::Manager>;

/// Inplements NostrDatabase trait for a Postgres database backend
#[derive(Clone)]
pub struct NostrPostgres {
    pool: Pool,
}

impl NostrPostgres {
    /// Create a new [`NostrPostgres`] instance
    pub async fn new<C>(connection_string: C) -> Result<Self, DatabaseError>
    where
        C: AsRef<str>,
    {
        let pool = postgres_connection_pool(connection_string.as_ref()).await?;
        crate::migrations::run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    pub(crate) async fn get_connection(&self) -> Result<PostgresConnection, DatabaseError> {
        self.pool.get().await.map_err(DatabaseError::backend)
    }

    pub(crate) async fn save(
        &self,
        event_data: EventDataDb,
    ) -> Result<SaveEventStatus, DatabaseError> {
        let mut db = self.get_connection().await?;
        let tx = db.transaction().await.map_err(DatabaseError::backend)?;
        tx.execute(r#"INSERT INTO events (id, pubkey, created_at, kind, payload, deleted) VALUES ($1, $2, $3, $4, $5, $6)"#, &[
            &event_data.event.id,
            &event_data.event.pubkey,
            &event_data.event.created_at,
            &event_data.event.kind,
            &event_data.event.payload,
            &event_data.event.deleted
        ])
            .await
            .map_err(DatabaseError::backend)?;

        // could not find a reasonable way to have values escaped in batch insert
        for tag in event_data.tags {
            tx.execute(
                r#"INSERT INTO event_tags (tag, tag_value, event_id) VALUES ($1, $2, $3)"#,
                &[&tag.tag, &tag.tag_value, &tag.event_id],
            )
            .await
            .map_err(DatabaseError::backend)?;
        }

        match tx.commit().await {
            Ok(_) => Ok(SaveEventStatus::Success),
            Err(_) => Ok(SaveEventStatus::Rejected(RejectedReason::Duplicate)),
        }
    }

    pub(crate) async fn event_by_id(
        &self,
        event_id: &EventId,
    ) -> Result<Option<EventDb>, DatabaseError> {
        let db = self.get_connection().await?;
        let query =
            r#"SELECT id, pubkey, created_at, kind, payload, deleted FROM events WHERE id = $1"#;

        let result: Option<EventDb> = db
            .query_opt(query, &[&event_id.as_bytes().to_vec()])
            .await
            .map_err(DatabaseError::backend)?
            .map(|row| row.into());
        Ok(result)
    }
}

impl NostrDatabase for NostrPostgres {
    fn backend(&self) -> Backend {
        Backend::Custom("Postgres".to_string())
    }

    /// Save [`Event`] into store
    ///
    /// **This method assumes that [`Event`] was already verified**
    fn save_event<'a>(
        &'a self,
        event: &'a Event,
    ) -> BoxedFuture<'a, Result<SaveEventStatus, DatabaseError>> {
        Box::pin(async move { self.save(EventDataDb::try_from(event)?).await })
    }

    /// Check event status by ID
    ///
    /// Check if the event is saved, deleted or not existent.
    fn check_id<'a>(
        &'a self,
        event_id: &'a EventId,
    ) -> BoxedFuture<'a, Result<DatabaseEventStatus, DatabaseError>> {
        Box::pin(async move {
            let status = match self.event_by_id(event_id).await? {
                Some(e) if e.deleted => DatabaseEventStatus::Deleted,
                Some(_) => DatabaseEventStatus::Saved,
                None => DatabaseEventStatus::NotExistent,
            };
            Ok(status)
        })
    }

    /// Get [`Event`] by [`EventId`]
    fn event_by_id<'a>(
        &'a self,
        event_id: &'a EventId,
    ) -> BoxedFuture<'a, Result<Option<Event>, DatabaseError>> {
        Box::pin(async move {
            let event = match self.event_by_id(event_id).await? {
                Some(e) if !e.deleted => {
                    Some(Event::decode(&e.payload).map_err(DatabaseError::backend)?)
                }
                _ => None,
            };
            Ok(event)
        })
    }

    /// Count the number of events found with [`Filter`].
    ///
    /// Use `Filter::new()` or `Filter::default()` to count all events.
    fn count(&self, filter: Filter) -> BoxedFuture<'_, Result<usize, DatabaseError>> {
        Box::pin(async move {
            let base_query = "SELECT DISTINCT count(*) FROM events LEFT JOIN event_tags ON events.id = event_tags.event_id WHERE events.deleted = FALSE";
            let (sql, params) = filter_to_sql_params(base_query, &filter);
            let param_slice = &params
                .iter()
                .map(|x| x.as_ref() as &(dyn ToSql + Sync))
                .collect::<Vec<_>>();
            let db = self.get_connection().await?;
            let result = match db.query_one(&sql, param_slice.as_slice()).await {
                Ok(row) => {
                    let count: i64 = row.get(0);
                    count
                }
                Err(_) => 0,
            };
            Ok(result as usize)
        })
    }

    /// Query stored events.
    fn query(&self, filter: Filter) -> BoxedFuture<'_, Result<Events, DatabaseError>> {
        let filter = with_limit(filter, 10000);
        Box::pin(async move {
            let base_query = "SELECT DISTINCT events.* FROM events LEFT JOIN event_tags ON events.id = event_tags.event_id WHERE events.deleted = FALSE";
            let mut events = Events::new(&filter);
            let (sql, params) = filter_to_sql_params(base_query, &filter);
            let param_slice = &params
                .iter()
                .map(|x| x.as_ref() as &(dyn ToSql + Sync))
                .collect::<Vec<_>>();

            let result: Vec<EventDb> = self
                .get_connection()
                .await?
                .query(&sql, param_slice.as_slice())
                .await
                .map_err(DatabaseError::backend)?
                .into_iter()
                .map(|e| e.into())
                .collect();

            for item in result.into_iter() {
                if let Ok(event) = Event::decode(&item.payload) {
                    events.insert(event);
                }
            }
            Ok(events)
        })
    }

    /// Delete all events that match the [Filter]
    fn delete(&self, filter: Filter) -> BoxedFuture<'_, Result<(), DatabaseError>> {
        let filter = with_limit(filter, 999);
        Box::pin(async move {
            let base_query = "SELECT DISTINCT events.id FROM events LEFT JOIN event_tags ON events.id = event_tags.event_id WHERE events.deleted = FALSE";
            let (sql, params) = filter_to_sql_params(base_query, &filter);
            let param_slice = &params
                .iter()
                .map(|x| x.as_ref() as &(dyn ToSql + Sync))
                .collect::<Vec<_>>();

            let delete_ids: Vec<Box<Vec<u8>>> = self
                .get_connection()
                .await?
                .query(&sql, param_slice.as_slice())
                .await
                .map_err(DatabaseError::backend)?
                .into_iter()
                .map(|e| Box::new(e.get(0)))
                .collect();

            let param_slice = &delete_ids
                .iter()
                .map(|x| x.as_ref() as &(dyn ToSql + Sync))
                .collect::<Vec<_>>();

            let update_query = "UPDATE events SET deleted = TRUE WHERE events.id = ANY (${})";
            self.get_connection()
                .await?
                .execute(update_query, param_slice.as_slice())
                .await
                .map_err(DatabaseError::backend)?;

            Ok(())
        })
    }

    fn wipe(&self) -> BoxedFuture<'_, prelude::Result<(), DatabaseError>> {
        Box::pin(async move { Err(DatabaseError::NotSupported) })
    }
}

/// Create a new [`NostrPostgres`] instance from an existing connection pool
impl From<Pool> for NostrPostgres {
    fn from(pool: Pool) -> Self {
        Self { pool }
    }
}

impl std::fmt::Debug for NostrPostgres {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NostrPostgres")
            .field("pool", &self.pool.status())
            .finish()
    }
}

pub async fn postgres_connection_pool(
    connection_string: &str,
) -> Result<deadpool_postgres::Pool, DatabaseError> {
    let cfg: tokio_postgres::Config = connection_string.parse().map_err(DatabaseError::backend)?;
    let mgr_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };
    let pool = Pool::builder(Manager::from_config(cfg, NoTls, mgr_config))
        .max_size(16)
        .build()
        .map_err(DatabaseError::backend)?;
    Ok(pool)
}
