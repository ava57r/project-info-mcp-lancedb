//! Message handler inserting or replacing a project info record in Qdrant.

use std::time::{SystemTime, UNIX_EPOCH};

use kameo::message::{Context, Message};
use qdrant_client::qdrant::{Condition, Filter, ScrollPointsBuilder, UpsertPointsBuilder};
use sha2::{Digest, Sha256};

use crate::actors::project_info::ProjectInfoActor;
use crate::actors::{EmbeddingMessage, UpsertMessage};
use crate::helpers::build_point;

impl Message<UpsertMessage> for ProjectInfoActor {
    type Reply = Result<String, String>;

    /// Hashes the content, skips writes when unchanged, then embeds and upserts the point into Qdrant.
    async fn handle(
        &mut self,
        msg: UpsertMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let project = self.resolve_project(&msg.project);
        let mut hasher = Sha256::new();
        hasher.update(msg.content.as_bytes());
        let current_hash = hex::encode(hasher.finalize());

        // Build filter to find existing point with this id and project
        let mut id_filter = Filter::must([Condition::matches("id", msg.id.clone())]);
        if project != "*" {
            id_filter
                .must
                .push(Condition::matches("project", project.clone()));
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

        if let Some(old_hash) = existing_hash {
            if old_hash == current_hash {
                return Ok(format!(
                    "ℹ️ [Kameo] Data for id '{}' didn't change (hash matches). Model inference skipped.",
                    msg.id
                ));
            }
        }

        let vector = self
            .embed_actor_ref
            .ask(EmbeddingMessage {
                query: msg.content.clone(),
            })
            .await
            .map_err(|e| e.to_string())?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let point = build_point(
            &msg.id,
            &project,
            &msg.content,
            &msg.category,
            &current_hash,
            timestamp,
            &self.vector_name,
            vector,
        );

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection_name, vec![point]).wait(true))
            .await
            .map_err(|e| format!("Error writing to Qdrant: {e}"))?;

        Ok(format!(
            "✅ [Kameo] Data '{}' successfully updated in project memory (project '{}').",
            msg.id, project
        ))
    }
}
