use std::time::{SystemTime, UNIX_EPOCH};

use arrow_array::cast::AsArray;
use kameo::actor::{Actor, ActorRef};
use kameo::message::{Context, Message};
use lancedb::index::{Index, scalar::FtsIndexBuilder};
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::Table;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio_stream::StreamExt;

use crate::actors::{SearchMessage, UpsertMessage};
use crate::helpers::build_arrow_record;

pub const EMBED_FORMAT: &str = "float";

pub struct MemoryActor {
    pub table: Table,
    pub http_client: Client,
    pub embed_url: String,
    pub model_name: String,
    pub vector_dimension: usize,
    pub pooling: Option<String>, // "LAST" or "MEAN"
}

#[derive(Serialize)]
struct EmbedParams {
    pooling: String, // "LAST" or "MEAN"
}

#[derive(Serialize)]
struct Req<'a> {
    input: &'a str,
    model: &'a str,
    encoding_format: &'a str,
    params: Option<EmbedParams>,
}

#[derive(Deserialize)]
struct Data {
    embedding: Vec<f32>,
}
#[derive(Deserialize)]
struct Res {
    data: Vec<Data>,
}

impl MemoryActor {
    pub async fn get_embedding_from_ovms(&self, text: &str) -> Result<Vec<f32>, String> {
        let response = self
            .http_client
            .post(&self.embed_url)
            .json(&Req {
                input: text,
                model: &self.model_name,
                encoding_format: EMBED_FORMAT,
                params: self.pooling.as_ref().map(|p| EmbedParams {
                    pooling: p.to_string(),
                }),
            })
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let parsed: Res = response.json().await.map_err(|e| e.to_string())?;

        if let Some(d) = parsed.data.first() {
            Ok(d.embedding.clone())
        } else {
            Err("Model returned empty vectors array".to_string())
        }
    }
}

impl Actor for MemoryActor {
    type Args = MemoryActor;

    type Error = anyhow::Error;

    async fn on_start(args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(args)
    }
}

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

        let vector = self.get_embedding_from_ovms(&msg.content).await?;

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

impl Message<SearchMessage> for MemoryActor {
    type Reply = Result<String, String>;

    async fn handle(
        &mut self,
        msg: SearchMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let query_vector = self.get_embedding_from_ovms(&msg.query).await?;

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
            .map_err(|e| format!("Ошибка гибридного поиска: {}", e))?;
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
