use nostr::filter::Filter;
use nostr_database::*;

pub fn filter_to_sql_params(
    base_query: &str,
    filter: &Filter,
    with_order: bool,
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
        params.push(Box::new(i64::try_from(since.as_secs()).unwrap_or(i64::MAX)));
        idx += 1;
    }

    if let Some(until) = filter.until {
        sql.push_str(&format!(" AND events.created_at <= ${}", idx));
        params.push(Box::new(i64::try_from(until.as_secs()).unwrap_or(i64::MAX)));
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

    if with_order {
        sql.push_str(" ORDER BY events.created_at DESC");
    }

    if let Some(limit) = filter.limit {
        sql.push_str(&format!(" LIMIT ${}", idx));
        params.push(Box::new(limit as i64));
    }

    (sql, params)
}

pub fn count_query_for_filter(filter: &Filter) -> &'static str {
    if uses_tag_filters(filter) {
        "SELECT count(DISTINCT events.id) FROM events LEFT JOIN event_tags ON events.id = event_tags.event_id WHERE events.deleted = FALSE"
    } else {
        "SELECT count(*) FROM events WHERE events.deleted = FALSE"
    }
}

pub fn select_events_query_for_filter(filter: &Filter) -> &'static str {
    if uses_tag_filters(filter) {
        "SELECT DISTINCT events.* FROM events LEFT JOIN event_tags ON events.id = event_tags.event_id WHERE events.deleted = FALSE"
    } else {
        "SELECT events.* FROM events WHERE events.deleted = FALSE"
    }
}

pub fn select_event_ids_query_for_filter(filter: &Filter) -> &'static str {
    if uses_tag_filters(filter) {
        "SELECT DISTINCT events.id FROM events LEFT JOIN event_tags ON events.id = event_tags.event_id WHERE events.deleted = FALSE"
    } else {
        "SELECT events.id FROM events WHERE events.deleted = FALSE"
    }
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

fn uses_tag_filters(filter: &Filter) -> bool {
    !filter.generic_tags.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr::prelude::*;

    #[test]
    fn test_filter_to_sql_params_no_filters() {
        let filter = Filter::new();
        let base_query = "SELECT * FROM events";
        let (sql, params) = filter_to_sql_params(base_query, &filter, true);

        assert_eq!(sql, base_query);
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_filter_to_sql_params_with_ids() {
        let event_id = EventId::all_zeros();
        let filter = Filter::new().ids([event_id]);
        let base_query = "SELECT * FROM events WHERE deleted = FALSE";
        let (sql, params) = filter_to_sql_params(base_query, &filter, true);

        assert!(sql.contains("AND events.id = ANY ($1)"));
        assert!(sql.contains("ORDER BY events.created_at DESC"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_filter_to_sql_params_with_authors() {
        let keys = Keys::generate();
        let filter = Filter::new().author(keys.public_key());
        let base_query = "SELECT * FROM events WHERE deleted = FALSE";
        let (sql, params) = filter_to_sql_params(base_query, &filter, true);

        assert!(sql.contains("AND events.pubkey = ANY ($1)"));
        assert!(sql.contains("ORDER BY events.created_at DESC"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_filter_to_sql_params_with_kinds() {
        let filter = Filter::new().kinds([Kind::TextNote, Kind::Metadata]);
        let base_query = "SELECT * FROM events WHERE deleted = FALSE";
        let (sql, params) = filter_to_sql_params(base_query, &filter, true);

        assert!(sql.contains("AND events.kind = ANY ($1)"));
        assert!(sql.contains("ORDER BY events.created_at DESC"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_filter_to_sql_params_with_since() {
        let timestamp = Timestamp::from(1234567890);
        let filter = Filter::new().since(timestamp);
        let base_query = "SELECT * FROM events WHERE deleted = FALSE";
        let (sql, params) = filter_to_sql_params(base_query, &filter, true);

        assert!(sql.contains("AND events.created_at >= $1"));
        assert!(sql.contains("ORDER BY events.created_at DESC"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_filter_to_sql_params_with_until() {
        let timestamp = Timestamp::from(1234567890);
        let filter = Filter::new().until(timestamp);
        let base_query = "SELECT * FROM events WHERE deleted = FALSE";
        let (sql, params) = filter_to_sql_params(base_query, &filter, true);

        assert!(sql.contains("AND events.created_at <= $1"));
        assert!(sql.contains("ORDER BY events.created_at DESC"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_filter_to_sql_params_with_limit() {
        let filter = Filter::new().limit(50);
        let base_query = "SELECT * FROM events WHERE deleted = FALSE";
        let (sql, params) = filter_to_sql_params(base_query, &filter, true);

        assert!(sql.contains("ORDER BY events.created_at DESC"));
        assert!(sql.contains("LIMIT $1"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_filter_to_sql_params_with_limit_but_no_order() {
        let filter = Filter::new().limit(50);
        let base_query = "SELECT * FROM events WHERE deleted = FALSE";
        let (sql, params) = filter_to_sql_params(base_query, &filter, false);

        assert!(!sql.contains("ORDER BY events.created_at DESC"));
        assert!(sql.contains("LIMIT $1"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_filter_to_sql_params_with_generic_tags() {
        use std::collections::BTreeSet;
        // Create a filter with a custom tag by using the builder
        let mut filter = Filter::new();
        let mut values = BTreeSet::new();
        values.insert("value1".to_string());
        values.insert("value2".to_string());
        filter
            .generic_tags
            .insert(SingleLetterTag::lowercase(Alphabet::E), values);
        let base_query = "SELECT * FROM events WHERE deleted = FALSE";
        let (sql, params) = filter_to_sql_params(base_query, &filter, true);

        assert!(sql.contains("AND event_tags.tag = $1"));
        assert!(sql.contains("AND event_tags.tag_value = ANY ($2)"));
        assert!(sql.contains("ORDER BY events.created_at DESC"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_filter_to_sql_params_combined() {
        let keys = Keys::generate();
        let timestamp = Timestamp::from(1234567890);
        let filter = Filter::new()
            .author(keys.public_key())
            .kinds([Kind::TextNote])
            .since(timestamp)
            .limit(100);
        let base_query = "SELECT * FROM events WHERE deleted = FALSE";
        let (sql, params) = filter_to_sql_params(base_query, &filter, true);

        assert!(sql.contains("AND events.pubkey = ANY ($1)"));
        assert!(sql.contains("AND events.kind = ANY ($2)"));
        assert!(sql.contains("AND events.created_at >= $3"));
        assert!(sql.contains("ORDER BY events.created_at DESC"));
        assert!(sql.contains("LIMIT $4"));
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn test_with_limit_sets_default() {
        let filter = Filter::new();
        let limited = with_limit(filter, 1000);

        assert_eq!(limited.limit, Some(1000));
    }

    #[test]
    fn test_with_limit_preserves_existing() {
        let filter = Filter::new().limit(50);
        let limited = with_limit(filter, 1000);

        assert_eq!(limited.limit, Some(50));
    }

    #[test]
    fn test_has_filters_empty() {
        let filter = Filter::new();
        assert!(!has_filters(&filter));
    }

    #[test]
    fn test_has_filters_with_ids() {
        let filter = Filter::new().ids([EventId::all_zeros()]);
        assert!(has_filters(&filter));
    }

    #[test]
    fn test_has_filters_with_authors() {
        let keys = Keys::generate();
        let filter = Filter::new().author(keys.public_key());
        assert!(has_filters(&filter));
    }

    #[test]
    fn test_has_filters_with_kinds() {
        let filter = Filter::new().kind(Kind::TextNote);
        assert!(has_filters(&filter));
    }

    #[test]
    fn test_has_filters_with_since() {
        let filter = Filter::new().since(Timestamp::from(1234567890));
        assert!(has_filters(&filter));
    }

    #[test]
    fn test_has_filters_with_until() {
        let filter = Filter::new().until(Timestamp::from(1234567890));
        assert!(has_filters(&filter));
    }

    #[test]
    fn test_has_filters_with_tags() {
        use std::collections::BTreeSet;
        let mut filter = Filter::new();
        let mut values = BTreeSet::new();
        values.insert("value".to_string());
        filter
            .generic_tags
            .insert(SingleLetterTag::lowercase(Alphabet::E), values);
        assert!(has_filters(&filter));
    }

    #[test]
    fn test_has_filters_with_limit() {
        let filter = Filter::new().limit(10);
        assert!(has_filters(&filter));
    }

    #[test]
    fn test_count_query_for_filter_without_tags() {
        let filter = Filter::new().kind(Kind::TextNote);
        let sql = count_query_for_filter(&filter);

        assert_eq!(
            sql,
            "SELECT count(*) FROM events WHERE events.deleted = FALSE"
        );
    }

    #[test]
    fn test_count_query_for_filter_with_tags() {
        use std::collections::BTreeSet;

        let mut filter = Filter::new();
        let mut values = BTreeSet::new();
        values.insert("value".to_string());
        filter
            .generic_tags
            .insert(SingleLetterTag::lowercase(Alphabet::E), values);

        let sql = count_query_for_filter(&filter);

        assert_eq!(
            sql,
            "SELECT count(DISTINCT events.id) FROM events LEFT JOIN event_tags ON events.id = event_tags.event_id WHERE events.deleted = FALSE"
        );
    }

    #[test]
    fn test_select_events_query_for_filter_without_tags() {
        let filter = Filter::new().limit(25);
        let sql = select_events_query_for_filter(&filter);

        assert_eq!(
            sql,
            "SELECT events.* FROM events WHERE events.deleted = FALSE"
        );
    }

    #[test]
    fn test_select_events_query_for_filter_with_tags() {
        use std::collections::BTreeSet;

        let mut filter = Filter::new();
        let mut values = BTreeSet::new();
        values.insert("value".to_string());
        filter
            .generic_tags
            .insert(SingleLetterTag::lowercase(Alphabet::E), values);

        let sql = select_events_query_for_filter(&filter);

        assert_eq!(
            sql,
            "SELECT DISTINCT events.* FROM events LEFT JOIN event_tags ON events.id = event_tags.event_id WHERE events.deleted = FALSE"
        );
    }

    #[test]
    fn test_select_event_ids_query_for_filter_without_tags() {
        let filter = Filter::new().author(Keys::generate().public_key());
        let sql = select_event_ids_query_for_filter(&filter);

        assert_eq!(
            sql,
            "SELECT events.id FROM events WHERE events.deleted = FALSE"
        );
    }

    #[test]
    fn test_select_event_ids_query_for_filter_with_tags() {
        use std::collections::BTreeSet;

        let mut filter = Filter::new();
        let mut values = BTreeSet::new();
        values.insert("value".to_string());
        filter
            .generic_tags
            .insert(SingleLetterTag::lowercase(Alphabet::E), values);

        let sql = select_event_ids_query_for_filter(&filter);

        assert_eq!(
            sql,
            "SELECT DISTINCT events.id FROM events LEFT JOIN event_tags ON events.id = event_tags.event_id WHERE events.deleted = FALSE"
        );
    }
}
