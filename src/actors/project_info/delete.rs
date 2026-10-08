//! Message handler deleting a single point by `(id, project)`.

use kameo::message::{Context, Message};

use crate::actors::DeletePointMessage;
use crate::actors::project_info::ProjectInfoActor;
use crate::helpers::escape_literal;

impl Message<DeletePointMessage> for ProjectInfoActor {
    type Reply = Result<bool, String>;

    /// Deletes the row matching `(id, project)`; replies `true` when a row existed.
    async fn handle(
        &mut self,
        msg: DeletePointMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let project = self.resolve_project(&msg.project);
        let predicate = format!(
            "id = {} AND project = {}",
            escape_literal(&msg.id),
            escape_literal(&project)
        );
        let existing = self
            .table
            .count_rows(Some(predicate.clone()))
            .await
            .map_err(|e| format!("Error checking point: {e}"))?;
        if existing == 0 {
            return Ok(false);
        }
        self.table
            .delete(&predicate)
            .await
            .map_err(|e| format!("Error deleting point: {e}"))?;
        Ok(true)
    }
}
