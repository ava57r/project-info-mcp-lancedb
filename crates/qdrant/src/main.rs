//! `piim`: MCP server exposing persistent project memory over Qdrant with vector search.
//!
//! Wires up the embeddings and project-info actors, then serves the memory tools over stdio.

use std::collections::BTreeMap;

use rust_mcp_sdk::error::SdkResult;
use rust_mcp_sdk::mcp_server::{McpServerOptions, server_runtime};
use rust_mcp_sdk::schema::{Implementation, ServerCapabilities, ServerCapabilitiesTools};
use rust_mcp_sdk::{
    McpServer, ServerDetails, StdioTransport, ToMcpServerHandler, TransportOptions,
};

use piim_common::mcp::MemoryToolHandler;

/// Main entry point for the `piim` MCP server.
#[tokio::main]
async fn main() -> SdkResult<()> {
    let memory_actor_ref = piim_qdrant::init_project_actor().await?;
    let server_details = server_details();

    let transport = StdioTransport::new(TransportOptions::default())?;
    let handler =
        MemoryToolHandler::new(memory_actor_ref, env!("CARGO_PKG_VERSION")).to_mcp_server_handler();
    let server = server_runtime::create_server(McpServerOptions {
        transport,
        handler,
        server_details,
        message_observer: None,
    });
    server.start().await
}

fn server_details() -> ServerDetails {
    ServerDetails {
        server_info: Implementation {
            name: "piim".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("Project Info in MCP".into()),
            description: Some("Persistent project memory over Qdrant with vector search".into()),
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
    }
}
