//! Message handler re-opening the LanceDB table (used after snapshot restore).

use arrow_array::RecordBatch;
use kameo::message::{Context, Message};
use lancedb::{Connection, connect};

use crate::actors::ReopenMessage;
use crate::actors::project_info::ProjectInfoActor;
use crate::helpers::table_schema;

impl Message<ReopenMessage> for ProjectInfoActor {
    type Reply = Result<String, String>;

    /// Reconnects to `db_dir` and re-opens (or creates) `project_memory`.
    async fn handle(
        &mut self,
        _msg: ReopenMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let db: Connection = connect(&self.db_dir)
            .execute()
            .await
            .map_err(|e| format!("Error reconnecting to LanceDB: {e}"))?;
        let table = match db.open_table("project_memory").execute().await {
            Ok(t) => t,
            Err(_) => db
                .create_table(
                    "project_memory",
                    RecordBatch::new_empty(table_schema(self.vector_dimension)),
                )
                .execute()
                .await
                .map_err(|e| format!("Error recreating table: {e}"))?,
        };
        self.table = table;
        Ok("Table reopened".to_string())
    }
}
