//! Message handler listing stored points with filters (REST API / dashboard).

use kameo::message::{Context, Message};
use qdrant_client::qdrant::{Condition, Filter, ScrollPointsBuilder};

use crate::actors::project_info::ProjectInfoActor;
use crate::actors::{ListPointsMessage, PointRecord};

impl Message<ListPointsMessage> for ProjectInfoActor {
    type Reply = Result<(Vec<PointRecord>, usize), String>;

    /// Scrolls the collection (project/category predicate + offset/limit + substring filter)
    /// and replies with points + total count.
    async fn handle(
        &mut self,
        msg: ListPointsMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let project = self.resolve_project(&msg.project);

        // Build filter for project/category
        let mut conditions = Vec::new();
        if project != "*" {
            conditions.push(Condition::matches("project", project));
        }
        if let Some(ref cat) = msg.category {
            if !cat.trim().is_empty() {
                conditions.push(Condition::matches("category", cat.clone()));
            }
        }
        let filter = (!conditions.is_empty())
            .then(|| Filter::must(conditions));

        // Count total matching points
        let total = if let Some(ref f) = filter {
            match self
                .client
                .count(
                    qdrant_client::qdrant::CountPointsBuilder::new(&self.collection_name)
                        .filter(f.clone())
                        .exact(true),
                )
                .await
            {
                Ok(r) => r.result.map_or(0, |v| v.count as usize),
                Err(_) => 0,
            }
        } else {
            // No filter — count all points
            match self
                .client
                .count(
                    qdrant_client::qdrant::CountPointsBuilder::new(&self.collection_name)
                        .exact(true),
                )
                .await
            {
                Ok(r) => r.result.map_or(0, |v| v.count as usize),
                Err(_) => 0,
            }
        };

        // Scroll through points with pagination
        let mut all = Vec::new();
        let mut has_more = true;

        while has_more {
            let scroll_result = self
                .client
                .scroll(
                    ScrollPointsBuilder::new(&self.collection_name)
                        .filter(filter.clone().unwrap_or_default())
                        .with_payload(true)
                        .limit(msg.limit.min(u32::MAX as usize) as u32),
                )
                .await;

            match scroll_result {
                Ok(result) => {
                    for point in result.result {
                        all.push(point_to_record(&point));
                    }
                    has_more = result.next_page_offset.is_some();
                }
                Err(e) => {
                    return Err(format!("Error scanning collection: {e}"));
                }
            }
        }

        // Substring filter on id + content (case-insensitive).
        let has_filter = msg.query.as_deref().is_some_and(|q| !q.trim().is_empty());
        let filtered: Vec<PointRecord> = match msg.query {
            Some(ref q) if !q.trim().is_empty() => {
                let needle = q.to_lowercase();
                all.into_iter()
                    .filter(|p| {
                        p.id.to_lowercase().contains(&needle)
                            || p.content.to_lowercase().contains(&needle)
                    })
                    .collect()
            }
            _ => all,
        };
        let filtered_total = if has_filter { filtered.len() } else { total };
        let page: Vec<PointRecord> = filtered
            .into_iter()
            .skip(msg.offset)
            .take(msg.limit.max(1))
            .collect();
        Ok((page, filtered_total))
    }
}

fn point_to_record(point: &qdrant_client::qdrant::RetrievedPoint) -> PointRecord {
    let payload = &point.payload;
    let get_str = |key: &str| -> String {
        payload
            .get(key)
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_default()
    };

    let get_i64 = |key: &str| -> i64 {
        payload
            .get(key)
            .and_then(|v| {
                // qdrant_client::qdrant::Value has methods for each type
                if let Some(s) = v.as_str() {
                    s.parse::<i64>().ok()
                } else if let Some(i) = v.as_integer() {
                    Some(i)
                } else {
                    None
                }
            })
            .unwrap_or(0)
    };

    PointRecord {
        id: get_str("id"),
        project: get_str("project"),
        content: get_str("content"),
        category: get_str("category"),
        file_hash: get_str("file_hash"),
        timestamp: get_i64("timestamp"),
    }
}
