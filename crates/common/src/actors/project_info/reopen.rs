//! Message handler re-opening storage (used after snapshot restore).

use kameo::message::{Context, Message};

use crate::actors::ReopenMessage;
use crate::actors::project_info::ProjectInfoActor;

impl Message<ReopenMessage> for ProjectInfoActor {
    type Reply = Result<String, String>;

    /// Re-opens the backend storage and replies with a confirmation message.
    async fn handle(
        &mut self,
        _msg: ReopenMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.store.reopen().await
    }
}
