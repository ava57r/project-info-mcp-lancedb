//! Qdrant implementation of the shared [`MemoryStore`] trait.

use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use piim_common::actors::{PointRecord, SearchHit, StatsData};
use piim_common::store::{MemoryStore, Record};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    Condition, CountPointsBuilder, DeletePointsBuilder, Filter, RetrievedPoint,
    ScrollPointsBuilder, SearchPointsBuilder, UpsertPointsBuilder, Value,
};

use crate::config::Config;
use crate::helpers::{build_filter, build_point, ensure_collection};

/// Storage backed by a Qdrant collection.
pub struct QdrantStore {
    client: Qdrant,
    collection_name: String,
    vector_name: String,
}

impl QdrantStore {
    /// Creates a store for an already-initialized Qdrant collection.
    pub fn new(client: Qdrant, collection_name: String, vector_name: String) -> Self {
        Self {
            client,
            collection_name,
            vector_name,
        }
    }
}

#[async_trait]
impl MemoryStore for QdrantStore {
    async fn prepare_upsert(&self, id: &str, project: &str, file_hash: &str) -> bool {
        // Build filter to find existing point with this id and project
        let mut id_filter = Filter::must([Condition::matches("id", id.to_string())]);
        if project != "*" {
            id_filter
                .must
                .push(Condition::matches("project", project.to_string()));
        }

        // Check if existing point has same hash
        let existing_hash = self
            .client
            .scroll(
                ScrollPointsBuilder::new(&self.collection_name)
                    .filter(id_filter)
                    .with_payload(true)
                    .limit(1),
            )
            .await
            .ok()
            .and_then(|scroll_result| {
                scroll_result
                    .result
                    .first()
                    .and_then(|p| p.payload.get("file_hash"))
                    .and_then(|v| v.as_str())
                    .map(String::from)
            });

        if let Some(old_hash) = existing_hash
            && old_hash == file_hash
        {
            return false;
        }
        true
    }

    async fn write_record(&self, record: Record<'_>, vector: Vec<f32>) -> Result<(), String> {
        let point = build_point(
            record.id,
            record.project,
            record.content,
            record.category,
            record.file_hash,
            record.timestamp,
            &self.vector_name,
            vector,
        );

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection_name, vec![point]).wait(true))
            .await
            .map_err(|e| format!("Error writing to Qdrant: {e}"))?;
        Ok(())
    }

    async fn search_text(
        &self,
        query_vector: Vec<f32>,
        project: &str,
        category: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>, String> {
        let mut search_request =
            SearchPointsBuilder::new(self.collection_name.clone(), query_vector, limit as u64)
                .vector_name(&self.vector_name);

        if let Some(f) = build_filter(project, category) {
            search_request = search_request.filter(f);
        }

        let response = self
            .client
            .search_points(search_request)
            .await
            .map_err(|e| format!("Error during Qdrant search: {e}"))?;

        Ok(response
            .result
            .into_iter()
            .map(|scored_point| SearchHit {
                id: payload_str(&scored_point.payload, "id"),
                project: payload_str(&scored_point.payload, "project"),
                category: payload_str(&scored_point.payload, "category"),
                content: payload_str(&scored_point.payload, "content"),
                distance: Some(scored_point.score),
            })
            .collect())
    }

    async fn search_structured(
        &self,
        query_vector: Vec<f32>,
        project: &str,
        category: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>, String> {
        // Build filter for project/category
        let mut conditions = Vec::new();
        if project != "*" {
            conditions.push(Condition::matches("project", project.to_string()));
        }
        if let Some(cat) = category
            && !cat.trim().is_empty()
        {
            conditions.push(Condition::matches("category", cat.to_string()));
        }
        let filter = (!conditions.is_empty()).then(|| Filter::must(conditions));

        // Build search request
        let mut search_request =
            SearchPointsBuilder::new(self.collection_name.clone(), query_vector, limit as u64)
                .vector_name(&self.vector_name);

        if let Some(f) = filter {
            search_request = search_request.filter(f);
        }

        let response = self
            .client
            .search_points(search_request)
            .await
            .map_err(|e| format!("Error during hybrid search: {e}"))?;

        Ok(response
            .result
            .into_iter()
            .map(|scored_point| SearchHit {
                id: payload_str(&scored_point.payload, "id"),
                project: payload_str(&scored_point.payload, "project"),
                category: payload_str(&scored_point.payload, "category"),
                content: payload_str(&scored_point.payload, "content"),
                distance: Some(scored_point.score),
            })
            .collect())
    }

    async fn count(&self, project: &str, category: Option<&str>) -> Result<usize, String> {
        // Build filter for project/category
        let mut conditions = Vec::new();
        if project != "*" {
            conditions.push(Condition::matches("project", project.to_string()));
        }
        if let Some(cat) = category
            && !cat.trim().is_empty()
        {
            conditions.push(Condition::matches("category", cat.to_string()));
        }
        let filter = (!conditions.is_empty()).then(|| Filter::must(conditions));

        // Count total matching points
        let total = if let Some(ref f) = filter {
            match self
                .client
                .count(
                    CountPointsBuilder::new(&self.collection_name)
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
                .count(CountPointsBuilder::new(&self.collection_name).exact(true))
                .await
            {
                Ok(r) => r.result.map_or(0, |v| v.count as usize),
                Err(_) => 0,
            }
        };
        Ok(total)
    }

    async fn list(
        &self,
        project: &str,
        category: Option<&str>,
        limit: usize,
    ) -> Result<Vec<PointRecord>, String> {
        // Build filter for project/category
        let mut conditions = Vec::new();
        if project != "*" {
            conditions.push(Condition::matches("project", project.to_string()));
        }
        if let Some(cat) = category
            && !cat.trim().is_empty()
        {
            conditions.push(Condition::matches("category", cat.to_string()));
        }
        let filter = (!conditions.is_empty()).then(|| Filter::must(conditions));

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
                        .limit(limit.min(u32::MAX as usize) as u32),
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
        Ok(all)
    }

    async fn stats(&self, project: &str) -> Result<StatsData, String> {
        // Build scroll filter
        let scroll_filter = if project == "*" {
            None
        } else {
            Some(Filter::must([Condition::matches(
                "project",
                project.to_string(),
            )]))
        };

        // Try to count points efficiently
        let total = if let Some(ref filter) = scroll_filter {
            match self
                .client
                .count(
                    CountPointsBuilder::new(&self.collection_name)
                        .filter(filter.clone())
                        .exact(true),
                )
                .await
            {
                Ok(count_result) => count_result.result.map_or(0, |r| r.count as usize),
                Err(_) => 0,
            }
        } else {
            // For wildcard, we need to scroll and count manually
            let mut total_count = 0;
            let mut has_more = true;
            while has_more {
                let scroll_result = self
                    .client
                    .scroll(
                        ScrollPointsBuilder::new(&self.collection_name)
                            .with_payload(true)
                            .limit(100),
                    )
                    .await;

                match scroll_result {
                    Ok(result) => {
                        total_count += result.result.len();
                        has_more = result.next_page_offset.is_some();
                    }
                    Err(_) => break,
                }
            }
            total_count
        };

        if total == 0 {
            return Ok(StatsData::default());
        }

        // Now scroll through to collect per-category stats
        let mut per_category: BTreeMap<String, usize> = BTreeMap::new();
        let mut content_chars: usize = 0;
        let mut scanned: usize = 0;
        let all_projects = project == "*";
        let mut per_project: BTreeMap<String, usize> = BTreeMap::new();

        let mut has_more = true;
        while has_more {
            let scroll_result = self
                .client
                .scroll(
                    ScrollPointsBuilder::new(&self.collection_name)
                        .filter(scroll_filter.clone().unwrap_or_default())
                        .with_payload(true)
                        .limit(100),
                )
                .await;

            match scroll_result {
                Ok(result) => {
                    for scored_point in result.result {
                        scanned += 1;
                        let payload = &scored_point.payload;
                        if let Some(category) = payload.get("category").and_then(|v| v.as_str()) {
                            *per_category.entry(category.to_string()).or_insert(0) += 1;
                        }
                        if let Some(content) = payload.get("content").and_then(|v| v.as_str()) {
                            content_chars += content.len();
                        }
                        if all_projects
                            && let Some(proj) = payload.get("project").and_then(|v| v.as_str())
                        {
                            *per_project.entry(proj.to_string()).or_insert(0) += 1;
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

    async fn delete(&self, id: &str, project: &str) -> Result<bool, String> {
        // Build filter to find existing point
        let mut filter = Filter::must([Condition::matches("id", id.to_string())]);
        if project != "*" {
            filter
                .must
                .push(Condition::matches("project", project.to_string()));
        }

        // Check if the point exists
        let existing = self
            .client
            .scroll(
                ScrollPointsBuilder::new(&self.collection_name)
                    .filter(filter.clone())
                    .limit(1),
            )
            .await
            .map_err(|e| format!("Error checking point: {e}"))?;

        if existing.result.is_empty() {
            return Ok(false);
        }

        // Extract the point ID from the found record
        let point_id = existing
            .result
            .first()
            .ok_or("No point found")?
            .id
            .as_ref()
            .ok_or("Point has no ID")?;

        // Delete the point by ID
        self.client
            .delete_points(
                DeletePointsBuilder::new(&self.collection_name).points(vec![point_id.clone()]),
            )
            .await
            .map_err(|e| format!("Error deleting point: {e}"))?;

        Ok(true)
    }

    async fn optimize(&self) -> Result<String, String> {
        // Qdrant manages its own storage efficiently, so this is a no-op.
        Ok("🔍 Kameo subtask completed. Qdrant collection is optimized.".to_string())
    }

    async fn reopen(&self) -> Result<String, String> {
        // In Qdrant, "re-opening" means ensuring the collection still exists and is ready.
        ensure_collection(
            &self.client,
            &self.collection_name,
            &self.vector_name,
            Config::get_from_env().vector_dimension,
        )
        .await
        .map_err(|e| format!("Error re-opening collection: {e}"))?;

        Ok(format!(
            "Collection '{}' reopened and ready.",
            self.collection_name
        ))
    }

    fn stats_empty_noun(&self) -> &'static str {
        "collection"
    }
}

/// Extracts a string field from a point payload.
fn payload_str(payload: &HashMap<String, Value>, key: &str) -> String {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_default()
}

/// Converts a retrieved point into a `PointRecord`.
fn point_to_record(point: &RetrievedPoint) -> PointRecord {
    let payload = &point.payload;

    let get_i64 = |key: &str| -> i64 {
        payload
            .get(key)
            .and_then(|v| {
                // qdrant_client::qdrant::Value has methods for each type
                if let Some(s) = v.as_str() {
                    s.parse::<i64>().ok()
                } else {
                    v.as_integer()
                }
            })
            .unwrap_or(0)
    };

    PointRecord {
        id: payload_str(payload, "id"),
        project: payload_str(payload, "project"),
        content: payload_str(payload, "content"),
        category: payload_str(payload, "category"),
        file_hash: payload_str(payload, "file_hash"),
        timestamp: get_i64("timestamp"),
    }
}
