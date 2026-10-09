//! Structured stats / search handlers for the REST API + dashboard.

use std::collections::BTreeMap;

use kameo::message::{Context, Message};
use qdrant_client::qdrant::{Condition, Filter, ScrollPointsBuilder, SearchPointsBuilder};

use crate::actors::project_info::ProjectInfoActor;
use crate::actors::{
    EmbeddingMessage, SearchHit, StatsData, StatsStructuredMessage, StructuredSearchMessage,
};

impl Message<StatsStructuredMessage> for ProjectInfoActor {
    type Reply = Result<StatsData, String>;

    /// Counts points and aggregates per-category / per-project stats by scrolling through the collection.
    async fn handle(
        &mut self,
        msg: StatsStructuredMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let project = self.resolve_project(&msg.project);
        let all_projects = project == "*";

        // Build filter
        let filter = if project == "*" {
            None
        } else {
            Some(Filter::must([Condition::matches("project", project)]))
        };

        // Count total
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

        // Scroll through all points to aggregate stats
        let mut per_category: BTreeMap<String, usize> = BTreeMap::new();
        let mut per_project: BTreeMap<String, usize> = BTreeMap::new();
        let mut content_chars: usize = 0;
        let mut scanned: usize = 0;
        let mut has_more = true;

        while has_more {
            let scroll_result = self
                .client
                .scroll(
                    ScrollPointsBuilder::new(&self.collection_name)
                        .filter(filter.clone().unwrap_or_default())
                        .with_payload(true)
                        .limit(100),
                )
                .await;

            match scroll_result {
                Ok(result) => {
                    for point in result.result {
                        scanned += 1;
                        let payload = &point.payload;

                        if let Some(category) = payload.get("category").and_then(|v| v.as_str()) {
                            *per_category
                                .entry(category.to_string())
                                .or_insert(0) += 1;
                        }

                        if let Some(content) = payload.get("content").and_then(|v| v.as_str()) {
                            content_chars += content.len();
                        }

                        if all_projects {
                            if let Some(proj) = payload.get("project").and_then(|v| v.as_str()) {
                                *per_project
                                    .entry(proj.to_string())
                                    .or_insert(0) += 1;
                            }
                        }
                    }
                    has_more = result.next_page_offset.is_some();
                }
                Err(e) => {
                    return Err(format!("Error scanning collection for stats: {e}"));
                }
            }
        }

        Ok(StatsData {
            total,
            scanned,
            content_chars,
            avg_chars: content_chars / scanned.max(1),
            per_category,
            per_project: if all_projects {
                per_project
            } else {
                BTreeMap::new()
            },
        })
    }
}

impl Message<StructuredSearchMessage> for ProjectInfoActor {
    type Reply = Result<Vec<SearchHit>, String>;

    /// Embeds the query and runs vector search with payload filters, replying with structured hits.
    async fn handle(
        &mut self,
        msg: StructuredSearchMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let project = self.resolve_project(&msg.project);
        let query_vector = self
            .embed_actor_ref
            .ask(EmbeddingMessage { query: msg.query })
            .await
            .map_err(|e| e.to_string())?;

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
        let filter = (!conditions.is_empty()).then(|| Filter::must(conditions));

        // Build search request
        let mut search_request =
            SearchPointsBuilder::new(
                self.collection_name.clone(),
                query_vector,
                msg.limit as u64,
            )
            .vector_name(&self.vector_name);

        if let Some(f) = filter {
            search_request = search_request.filter(f);
        }

        let response = self
            .client
            .search_points(search_request)
            .await
            .map_err(|e| format!("Error during hybrid search: {e}"))?;

        let mut hits = Vec::new();
        for scored_point in response.result {
            let payload = scored_point.payload;
            let get_str = |key: &str| -> String {
                payload
                    .get(key)
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default()
            };

            hits.push(SearchHit {
                id: get_str("id"),
                project: get_str("project"),
                category: get_str("category"),
                content: get_str("content"),
                distance: Some(scored_point.score),
            });
        }

        Ok(hits)
    }
}
