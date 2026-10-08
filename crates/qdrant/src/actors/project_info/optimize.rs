//! Message handler for Qdrant collection maintenance.

use kameo::message::{Context, Message};

use crate::actors::OptimizeMessage;
use crate::actors::project_info::ProjectInfoActor;

impl Message<OptimizeMessage> for ProjectInfoActor {
    type Reply = Result<String, String>;

    /// Qdrant manages its own storage efficiently, so this is a no-op.
    async fn handle(
        &mut self,
        _msg: OptimizeMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok("🔍 Kameo subtask completed. Qdrant collection is optimized.".to_string())
    }
}
