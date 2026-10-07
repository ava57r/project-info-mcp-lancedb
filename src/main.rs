mod actors;
mod config;
mod helpers;
mod mcp;

use std::collections::BTreeMap;

use arrow_array::RecordBatch;
use kameo::actor::Spawn;
use lancedb::connect;
use reqwest::Client;
use rust_mcp_sdk::error::SdkResult;
use rust_mcp_sdk::mcp_server::{McpServerOptions, server_runtime};
use rust_mcp_sdk::schema::{Implementation, ServerCapabilities, ServerCapabilitiesTools};
use rust_mcp_sdk::{
    McpServer, ServerDetails, StdioTransport, ToMcpServerHandler, TransportOptions,
};

use crate::actors::embed;
use crate::actors::project_info::ProjectInfoActor;
use crate::config::Config;
use crate::mcp::MemoryToolHandler;

#[tokio::main]
async fn main() -> SdkResult<()> {
    let config = Config::get_from_env();
    let db_conn = connect(&config.db_dir).execute().await.map_err(|e| {
        rust_mcp_sdk::error::McpSdkError::Internal {
            description: e.to_string(),
        }
    })?;

    let table = match db_conn.open_table("project_memory").execute().await {
        Ok(t) => t,
        Err(_) => db_conn
            .create_table(
                "project_memory",
                RecordBatch::new_empty(helpers::table_schema(config.vector_dimension)),
            )
            .execute()
            .await
            .map_err(|e| rust_mcp_sdk::error::McpSdkError::Internal {
                description: e.to_string(),
            })?,
    };

    let embed_actor_ref = embed::EmbeddingActor::spawn(embed::EmbeddingActor::new(
        Client::new(),
        config.embeddings_url,
        config.model,
        None,
    ));

    let memory_actor_ref = ProjectInfoActor::spawn(ProjectInfoActor::new(
        table,
        embed_actor_ref,
        config.vector_dimension,
    ));

    let server_details = ServerDetails {
        server_info: Implementation {
            name: "piil".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("Project Info in LanceDB MCP".into()),
            description: Some("Persistent project memory over LanceDB with hybrid search".into()),
            icons: vec![],
            website_url: None,
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools {
                list_changed: Some(true),
            }),
            experimental: Some({
                let mut exp = BTreeMap::new();
                exp.insert(
                    "customRequests".to_string(),
                    rust_mcp_sdk::schema::JsonObject(BTreeMap::new()),
                );
                exp
            }),
            ..Default::default()
        },
        instructions: None,
        meta: None,
    };

    let transport = StdioTransport::new(TransportOptions::default())?;
    let handler = MemoryToolHandler::new(memory_actor_ref).to_mcp_server_handler();
    let server = server_runtime::create_server(McpServerOptions {
        transport,
        handler,
        server_details,
        message_observer: None,
    });
    server.start().await
}
