//! Message handler searching project memory with vector similarity search via Qdrant.

use kameo::message::{Context, Message};
use qdrant_client::qdrant::SearchPointsBuilder;

use crate::actors::project_info::ProjectInfoActor;
use crate::actors::{EmbeddingMessage, SearchMessage};
use crate::helpers::build_filter;

/// Single search hit extracted from a Qdrant result.
struct SearchMatch {
    id: String,
    project: String,
    category: String,
    content: String,
    distance: f32,
}

impl Message<SearchMessage> for ProjectInfoActor {
    type Reply = Result<String, String>;

    /// Embeds the query, runs vector search on Qdrant, and replies with formatted matches.
    async fn handle(
        &mut self,
        msg: SearchMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let project = self.resolve_project(&msg.project);
        let query_vector = self
            .embed_actor_ref
            .ask(EmbeddingMessage { query: msg.query })
            .await
            .map_err(|e| e.to_string())?;

        let mut search_request =
            SearchPointsBuilder::new(self.collection_name.clone(), query_vector, msg.limit as u64);

        if let Some(f) = build_filter(&project, msg.category.as_deref()) {
            search_request = search_request.filter(f);
        }

        let response = self
            .client
            .search_points(search_request)
            .await
            .map_err(|e| format!("Error during Qdrant search: {}", e))?;

        let mut found = Vec::new();
        for scored_point in response.result {
            let payload = scored_point.payload;
            let get_str = |key: &str| -> String {
                payload
                    .get(key)
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default()
            };
            found.push(SearchMatch {
                id: get_str("id"),
                project: get_str("project"),
                category: get_str("category"),
                content: get_str("content"),
                distance: scored_point.score,
            });
        }

        Ok(format_matches(&found))
    }
}

fn format_matches(found: &[SearchMatch]) -> String {
    if found.is_empty() {
        return "🔍 No matches found.".to_string();
    }
    let mut out = format!("🔍 Found {} match(es):\n", found.len());
    for (i, m) in found.iter().enumerate() {
        out.push_str(&format!(
            "{}. [{}:{}] {} (distance: {:.4})\n   {}\n",
            i + 1,
            m.project,
            m.category,
            m.id,
            m.distance,
            m.content
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_matches_and_empty_result() {
        assert_eq!(format_matches(&[]), "🔍 No matches found.");

        let mut found = Vec::new();
        found.push(SearchMatch {
            id: "src/auth.rs".to_string(),
            project: "my-proj".to_string(),
            category: "file".to_string(),
            content: "JWT validation helpers".to_string(),
            distance: 0.875,
        });
        found.push(SearchMatch {
            id: "src/main.rs".to_string(),
            project: "my-proj".to_string(),
            category: "file".to_string(),
            content: "stdio wiring and actor spawn".to_string(),
            distance: 0.5,
        });
        let text = format_matches(&found);

        assert!(text.contains("Found 2 match(es)"));
        assert!(text.contains("1. [my-proj:file] src/auth.rs (distance: 0.8750)"));
        assert!(text.contains("JWT validation helpers"));
        assert!(text.contains("2. [my-proj:file] src/main.rs (distance: 0.5000)"));
    }
}
