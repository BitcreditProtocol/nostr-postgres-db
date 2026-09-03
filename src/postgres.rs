use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;

use deadpool_postgres::{Manager, ManagerConfig, Object, Pool, RecyclingMethod};
use nostr::event::{Event, EventId};
use nostr::filter::Filter;
use nostr::types::Timestamp;
use nostr_database::error::Error;
use nostr_database::{
    DatabaseEventStatus, Features, NostrDatabase, RejectedReason, SaveEventStatus,
};
use tokio_postgres::NoTls;
use tokio_postgres::types::ToSql;
use tracing::warn;

use super::model::{EventDataDb, EventDb};
use crate::flatbuffers;
use crate::query::{
    count_query_for_filter, filter_to_sql_params, select_event_ids_query_for_filter,
    select_events_query_for_filter, with_limit,
};

/// Shorthand for a pooled database connection
pub type PostgresConnection = Object;

/// The future type returned by the [`NostrDatabase`] trait methods
type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Inplements NostrDatabase trait for a Postgres database backend
#[derive(Clone)]
pub struct NostrPostgres {
    pool: Pool,
}

impl NostrPostgres {
    /// Create a new [`NostrPostgres`] instance
    pub async fn new<C>(connection_string: C) -> Result<Self, Error>
    where
        C: AsRef<str>,
    {
        let pool = postgres_connection_pool(connection_string.as_ref()).await?;
        crate::migrations::run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    /// Create a new [`NostrPostgres`] instance from an existing connection pool
    ///
    /// This method will run database migrations on the provided pool.
    pub async fn from_pool(pool: Pool) -> Result<Self, Error> {
        crate::migrations::run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    pub(crate) async fn get_connection(&self) -> Result<PostgresConnection, Error> {
        self.pool.get().await.map_err(Error::storage)
    }

    pub(crate) async fn save(&self, event_data: EventDataDb) -> Result<SaveEventStatus, Error> {
        let mut db = self.get_connection().await?;
        let tx = db.transaction().await.map_err(Error::storage)?;
        if tx.execute(r#"INSERT INTO events (id, pubkey, created_at, kind, payload, deleted) VALUES ($1, $2, $3, $4, $5, $6)"#, &[
            &event_data.event.id,
            &event_data.event.pubkey,
            &event_data.event.created_at,
            &event_data.event.kind,
            &event_data.event.payload,
            &event_data.event.deleted
        ])
            .await.is_err() {
            return Ok(SaveEventStatus::Rejected(RejectedReason::Duplicate));
        }

        let stmt = tx
            .prepare(r#"INSERT INTO event_tags (tag, tag_value, event_id) VALUES ($1, $2, $3)"#)
            .await
            .map_err(Error::storage)?;

        for tag in event_data.tags {
            if let Err(e) = tx
                .execute(&stmt, &[&tag.tag, &tag.tag_value, &tag.event_id])
                .await
            {
                warn!("Failed to insert tag: {e}");
            }
        }

        tx.commit().await.map_err(Error::storage)?;
        Ok(SaveEventStatus::Success)
    }

    pub(crate) async fn event_by_id(&self, event_id: &EventId) -> Result<Option<EventDb>, Error> {
        let db = self.get_connection().await?;
        let query =
            r#"SELECT id, pubkey, created_at, kind, payload, deleted FROM events WHERE id = $1"#;

        let result: Option<EventDb> = db
            .query_opt(query, &[&event_id.as_bytes().to_vec()])
            .await
            .map_err(Error::storage)?
            .map(|row| row.into());
        Ok(result)
    }
}

impl NostrDatabase for NostrPostgres {
    fn backend(&self) -> &'static str {
        "postgres"
    }

    fn features(&self) -> Features {
        Features {
            persistent: true,
            event_expiration: false,
            full_text_search: false,
            request_to_vanish: false,
        }
    }

    /// Save [`Event`] into store
    ///
    /// **This method assumes that [`Event`] was already verified**
    fn save_event<'a>(
        &'a self,
        event: &'a Event,
    ) -> BoxedFuture<'a, Result<SaveEventStatus, Error>> {
        Box::pin(async move {
            let result = self.save(EventDataDb::try_from(event)?).await;
            let until = if event.created_at.is_zero() {
                event.created_at
            } else {
                Timestamp::from_secs(event.created_at.as_secs() - 1)
            };
            if event.kind.is_replaceable()
                && matches!(result, Ok(SaveEventStatus::Success))
                && let Err(e) = self
                    .delete(
                        Filter::new()
                            .author(event.pubkey)
                            .kind(event.kind)
                            .until(until),
                    )
                    .await
            {
                warn!("Failed to delete old replaceable events: {e}");
            }
            result
        })
    }

    /// Check event status by ID
    ///
    /// Check if the event is saved, deleted or not existent.
    fn check_id<'a>(
        &'a self,
        event_id: &'a EventId,
    ) -> BoxedFuture<'a, Result<DatabaseEventStatus, Error>> {
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
    ) -> BoxedFuture<'a, Result<Option<Event>, Error>> {
        Box::pin(async move {
            let event = match self.event_by_id(event_id).await? {
                Some(e) if !e.deleted => {
                    Some(flatbuffers::decode_event(&e.payload).map_err(Error::storage)?)
                }
                _ => None,
            };
            Ok(event)
        })
    }

    /// Count the number of events found with [`Filter`].
    ///
    /// Use `Filter::new()` or `Filter::default()` to count all events.
    fn count(&self, filter: Filter) -> BoxedFuture<'_, Result<usize, Error>> {
        Box::pin(async move {
            let base_query = count_query_for_filter(&filter);
            let (sql, params) = filter_to_sql_params(base_query, &filter, false);
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
    ///
    /// The result is ordered like [`Event`] itself: newest first, ties broken by id.
    fn query(&self, filter: Filter) -> BoxedFuture<'_, Result<BTreeSet<Event>, Error>> {
        let filter = with_limit(filter, 10000);
        Box::pin(async move {
            let base_query = select_events_query_for_filter(&filter);
            let (sql, params) = filter_to_sql_params(base_query, &filter, true);

            let param_slice = &params
                .iter()
                .map(|x| x.as_ref() as &(dyn ToSql + Sync))
                .collect::<Vec<_>>();

            let result: Vec<EventDb> = self
                .get_connection()
                .await?
                .query(&sql, param_slice.as_slice())
                .await
                .map_err(Error::storage)?
                .into_iter()
                .map(|e| e.into())
                .collect();

            let mut events = BTreeSet::new();
            for item in result {
                match flatbuffers::decode_event(&item.payload) {
                    Ok(event) => {
                        events.insert(event);
                    }
                    Err(e) => warn!("Failed to decode stored event: {e}"),
                }
            }
            Ok(events)
        })
    }

    /// Delete all events that match the [Filter]
    fn delete(&self, filter: Filter) -> BoxedFuture<'_, Result<(), Error>> {
        let filter = with_limit(filter, 999);
        Box::pin(async move {
            let base_query = select_event_ids_query_for_filter(&filter);
            let (sql, params) = filter_to_sql_params(base_query, &filter, false);
            let param_slice = &params
                .iter()
                .map(|x| x.as_ref() as &(dyn ToSql + Sync))
                .collect::<Vec<_>>();

            let delete_ids: Vec<Vec<u8>> = self
                .get_connection()
                .await?
                .query(&sql, param_slice.as_slice())
                .await
                .map_err(Error::storage)?
                .into_iter()
                .map(|e| e.get(0))
                .collect();

            if delete_ids.is_empty() {
                return Ok(());
            }

            let update_query = "UPDATE events SET deleted = TRUE WHERE events.id = ANY ($1)";
            self.get_connection()
                .await?
                .execute(update_query, &[&delete_ids])
                .await
                .map_err(Error::storage)?;

            Ok(())
        })
    }

    fn wipe(&self) -> BoxedFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            Err(Error::unsupported(
                "wipe is not supported by the postgres backend",
            ))
        })
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
) -> Result<deadpool_postgres::Pool, Error> {
    let cfg: tokio_postgres::Config = connection_string.parse().map_err(Error::storage)?;
    let mgr_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };
    let pool = Pool::builder(Manager::from_config(cfg, NoTls, mgr_config))
        .max_size(16)
        .build()
        .map_err(Error::storage)?;
    Ok(pool)
}
