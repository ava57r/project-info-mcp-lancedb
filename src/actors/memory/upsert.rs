use std::time::{SystemTime, UNIX_EPOCH};

use arrow_array::cast::AsArray;
use kameo::message::{Context, Message};
use lancedb::query::{ExecutableQuery, QueryBase};
use sha2::{Digest, Sha256};
use tokio_stream::StreamExt;

use crate::actors::memory::MemoryActor;
use crate::actors::{EmbeddingMessage, UpsertMessage};
use crate::helpers::build_arrow_record;

impl Message<UpsertMessage> for MemoryActor {
    type Reply = Result<String, String>;

    async fn handle(
        &mut self,
        msg: UpsertMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mut hasher = Sha256::new();
        hasher.update(msg.content.as_bytes());
        let current_hash = hex::encode(hasher.finalize());

        let predicate = format!("id = '{}'", msg.id);
        if let Ok(mut stream) = self
            .table
            .query()
            .only_if(&predicate)
            .limit(1)
            .execute()
            .await
            && let Some(Ok(batch)) = stream.next().await
            && batch.num_rows() > 0
        {
            if let Ok(hash_col_idx) = batch.schema().index_of("file_hash") {
                let hash_array = batch.column(hash_col_idx).as_string::<i32>();
                let old_hash = hash_array.value(0);
                if old_hash == current_hash {
                    return Ok(format!(
                        "ℹ️ [Kameo] Data for id '{}' didn't change (hash matches). Model inference skipped.",
                        msg.id
                    ));
                }
            }

            let _ = self.table.delete(&predicate).await;
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
        let record_batch = build_arrow_record(
            &msg.id,
            &msg.content,
            &msg.category,
            &current_hash,
            timestamp,
            vector,
            self.vector_dimension,
        )?;

        self.table
            .add(record_batch)
            .execute()
            .await
            .map_err(|e| format!("Error writing to LanceDB: {}", e))?;

        Ok(format!(
            "✅ [Kameo] Data '{}' successfully updated in project memory.",
            msg.id
        ))
    }
}
