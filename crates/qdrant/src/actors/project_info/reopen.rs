//! Message handler for re-opening the Qdrant collection (used after snapshot restore).
//!
//! In Qdrant, "re-opening" means ensuring the collection still exists and is ready.

use kameo::message::{Context, Message};

use crate::actors::ReopenMessage;
use crate::actors::project_info::ProjectInfoActor;
use crate::{config::Config, helpers::ensure_collection};

impl Message<ReopenMessage> for ProjectInfoActor {
    type Reply = Result<String, String>;

    /// Ensures the Qdrant collection exists and is ready.
    async fn handle(
        &mut self,
        _msg: ReopenMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
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
}
