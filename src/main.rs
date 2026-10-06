mod actors;
mod config;
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

use crate::actors::embed;
use crate::actors::memory::MemoryActor;
use crate::config::Config;
use crate::mcp::MemoryToolHandler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::get_from_env();
    let db_conn = connect(&config.db_dir).execute().await?;

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

    let embed_actor_ref = embed::EmbeddingActor::spawn(embed::EmbeddingActor::new(
        Client::new(),
        config.embed_url,
        config.model_name,
        None,
    ));

    let memory_actor_ref = MemoryActor::spawn(MemoryActor::new(
        table,
        embed_actor_ref,
        config.vector_dimension,
    ));

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
