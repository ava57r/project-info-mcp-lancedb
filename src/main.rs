mod actors;
mod helpers;
mod mcp;

use std::sync::Arc;

use arrow_array::RecordBatch;
use kameo::actor::Spawn;
use lancedb::connect;
use mcp_sdk_rs::server::Server;
use mcp_sdk_rs::transport::{Transport, stdio::StdioTransport};
use reqwest::Client;
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::actors::memory::MemoryActor;
use crate::mcp::MemoryToolHandler;

const DB_DIR: &str = "./.opencode_memory/kameo_db";
const EMBED_URL: &str = "http://localhost:8002/v1/embeddings";
const MODEL_NAME: &str = "qwen3-embed";
const VECTOR_DIMENSION: usize = 1024;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_conn = connect(DB_DIR).execute().await?;

    let table = match db_conn.open_table("project_memory").execute().await {
        Ok(t) => t,
        Err(_) => {
            db_conn
                .create_table(
                    "project_memory",
                    RecordBatch::new_empty(helpers::table_schema()),
                )
                .execute()
                .await?
        }
    };

    let memory_actor_ref = MemoryActor::spawn(MemoryActor {
        table,
        http_client: Client::new(),
        embed_url: EMBED_URL.to_string(),
        model_name: MODEL_NAME.to_string(),
        vector_dimension: VECTOR_DIMENSION,
        pooling: None,
    });

    let (read_tx, read_rx) = tokio::sync::mpsc::channel::<String>(64);
    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<String>(64);

    tokio::spawn(async move {
        let mut lines = BufReader::new(tokio::io::stdin()).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if read_tx.send(line).await.is_err() {
                break;
            }
        }
    });

    tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(line) = write_rx.recv().await {
            let failed = stdout.write_all(line.as_bytes()).await.is_err()
                || stdout.write_all(b"\n").await.is_err()
                || stdout.flush().await.is_err();
            if failed {
                break;
            }
        }
    });

    let transport = Arc::new(StdioTransport::new(read_rx, write_tx)) as Arc<dyn Transport>;
    let handler = Arc::new(MemoryToolHandler::new(memory_actor_ref));
    let server = Server::new(transport, handler);
    server.start().await?;
    Ok(())
}
