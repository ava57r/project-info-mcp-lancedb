//! Message handler reporting usage statistics about the project memory collection.

use std::collections::BTreeMap;

use kameo::message::{Context, Message};
use qdrant_client::qdrant::{Condition, CountPointsBuilder, Filter, ScrollPointsBuilder};

use crate::actors::StatsMessage;
use crate::actors::project_info::ProjectInfoActor;

impl Message<StatsMessage> for ProjectInfoActor {
    type Reply = Result<String, String>;

    /// Counts points and aggregates per-category stats by scrolling through the collection.
    async fn handle(
        &mut self,
        msg: StatsMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let project = self.resolve_project(&msg.project);

        // Build scroll filter
        let scroll_filter = if project == "*" {
            None
        } else {
            Some(Filter::must([Condition::matches(
                "project",
                project.clone(),
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
            return Ok(format!(
                "📊 Memory stats for project '{project}': collection is empty (0 records)."
            ));
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
                        if all_projects {
                            if let Some(proj) = payload.get("project").and_then(|v| v.as_str()) {
                                *per_project.entry(proj.to_string()).or_insert(0) += 1;
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

        let avg_chars = content_chars / scanned.max(1);
        let mut out = format!(
            "📊 Memory stats for project '{project}': {total} record(s), ~{content_chars} content chars (avg {avg_chars}/record)\n"
        );
        if all_projects {
            out.push_str("Projects:\n");
            for (name, count) in &per_project {
                out.push_str(&format!("- {name}: {count}\n"));
            }
        }
        out.push_str("Categories:\n");
        for (category, count) in &per_category {
            out.push_str(&format!("- {category}: {count}\n"));
        }
        Ok(out)
    }
}
