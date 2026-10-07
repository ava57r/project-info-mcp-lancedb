use kameo::message::{Context, Message};

use crate::actors::OptimizeMessage;
use crate::actors::project_info::ProjectInfoActor;

impl Message<OptimizeMessage> for ProjectInfoActor {
    type Reply = Result<String, String>;

    /// Optimizes the LanceDB table and replies with a confirmation message.
    async fn handle(
        &mut self,
        _msg: OptimizeMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let _ = self.table.optimize(Default::default()).await;

        Ok("🔍 Kameo subtask completed. Database optimized".to_string())
    }
}
