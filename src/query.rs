use nostr::filter::Filter;
use nostr_database::*;

pub fn filter_to_sql_params(
    base_query: &str,
    filter: &Filter,
) -> (
    String,
    Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>>,
) {
    let mut sql = base_query.to_string();

    if !has_filters(filter) {
        return (sql, Vec::new());
    }

    let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();
    let mut idx = 1;

    if let Some(ids) = &filter.ids {
        let id_values = ids
            .iter()
            .map(|id| id.as_bytes().to_vec())
            .collect::<Vec<_>>();
        sql.push_str(&format!(" AND events.id = ANY (${})", idx));
        params.push(Box::new(id_values));
        idx += 1;
    }

    if let Some(authors) = &filter.authors {
        let values = authors
            .iter()
            .map(|id| id.as_bytes().to_vec())
            .collect::<Vec<_>>();
        sql.push_str(&format!(" AND events.pubkey = ANY (${})", idx));
        params.push(Box::new(values));
        idx += 1;
    }

    if let Some(kinds) = &filter.kinds {
        let values = kinds.iter().map(|v| v.as_u16() as i64).collect::<Vec<_>>();
        sql.push_str(&format!(" AND events.kind = ANY (${})", idx));
        params.push(Box::new(values));
        idx += 1;
    }

    if let Some(since) = filter.since {
        sql.push_str(&format!(" AND events.created_at >= ${}", idx));
        params.push(Box::new(since.as_u64() as i64));
        idx += 1;
    }

    if let Some(until) = filter.until {
        sql.push_str(&format!(" AND events.created_at <= ${}", idx));
        params.push(Box::new(until.as_u64() as i64));
        idx += 1;
    }

    for (tag, values) in &filter.generic_tags {
        sql.push_str(&format!(" AND event_tags.tag = ${}", idx));
        params.push(Box::new(tag.to_string()));
        idx += 1;

        let values = values.iter().map(|v| v.to_string()).collect::<Vec<_>>();

        sql.push_str(&format!(" AND event_tags.tag_value = ANY (${})", idx));
        params.push(Box::new(values));
        idx += 1;
    }

    sql.push_str(" ORDER BY events.created_at DESC");

    if let Some(limit) = filter.limit {
        sql.push_str(&format!(" LIMIT ${}", idx));
        params.push(Box::new(limit as i64));
    }

    (sql, params)
}

/// sets the given default limit on a Nostr filter if not set
pub fn with_limit(filter: Filter, default_limit: usize) -> Filter {
    if filter.limit.is_none() {
        return filter.limit(default_limit);
    }
    filter
}

// determine if the filter has any filters set
fn has_filters(filter: &Filter) -> bool {
    filter.ids.is_some()
        || filter.authors.is_some()
        || filter.kinds.is_some()
        || filter.since.is_some()
        || filter.until.is_some()
        || !filter.generic_tags.is_empty()
        || filter.limit.is_some()
}
