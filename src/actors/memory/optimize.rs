use kameo::message::{Context, Message};

use crate::actors::OptimizeMessage;
use crate::actors::memory::MemoryActor;

impl Message<OptimizeMessage> for MemoryActor {
    type Reply = Result<String, String>;

    async fn handle(
        &mut self,
        _msg: OptimizeMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let _ = self.table.optimize(Default::default()).await;

        Ok(format!("🔍 Kameo subtask completed. Database optimized",))
    }
}
