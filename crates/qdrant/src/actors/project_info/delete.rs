//! Message handler deleting a single point by `(id, project)`.

use kameo::message::{Context, Message};
use qdrant_client::qdrant::{Condition, DeletePointsBuilder, Filter, ScrollPointsBuilder};

use crate::actors::DeletePointMessage;
use crate::actors::project_info::ProjectInfoActor;

impl Message<DeletePointMessage> for ProjectInfoActor {
    type Reply = Result<bool, String>;

    /// Deletes the point matching `(id, project)`; replies `true` when a point existed.
    async fn handle(
        &mut self,
        msg: DeletePointMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let project = self.resolve_project(&msg.project);

        // Build filter to find existing point
        let mut filter = Filter::must([Condition::matches("id", msg.id.clone())]);
        if project != "*" {
            filter
                .must
                .push(Condition::matches("project", project.clone()));
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
}
