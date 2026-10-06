use kameo::message::{Context, Message};
use lancedb::index::{Index, scalar::FtsIndexBuilder};
use lancedb::query::{ExecutableQuery, QueryBase};
use tokio_stream::StreamExt;

use crate::actors::memory::MemoryActor;
use crate::actors::{EmbeddingMessage, SearchMessage};

impl Message<SearchMessage> for MemoryActor {
    type Reply = Result<String, String>;

    async fn handle(
        &mut self,
        msg: SearchMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let query_vector = self
            .embed_actor_ref
            .ask(EmbeddingMessage { query: msg.query })
            .await
            .map_err(|e| e.to_string())?;

        let _ = self
            .table
            .create_index(&["content"], Index::FTS(FtsIndexBuilder::default()))
            .execute()
            .await;

        let mut query_builder = self
            .table
            .query()
            .nearest_to(query_vector)
            .map_err(|e| e.to_string())?
            .limit(msg.limit);
        if let Some(cat) = msg.category {
            query_builder = query_builder.only_if(format!("category = '{}'", cat));
        }

        let mut stream = query_builder
            .execute()
            .await
            .map_err(|e| format!("Error during hybrid search: {}", e))?;
        let mut num_batches = 0usize;
        while let Some(batch) = stream.next().await {
            batch.map_err(|e| e.to_string())?;
            num_batches += 1;
        }

        Ok(format!(
            "🔍 Kameo subtask completed. Found Arrow fragments: {}",
            num_batches
        ))
    }
}
