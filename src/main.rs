use std::collections::BTreeMap;

use piim_common::mcp;
use rust_mcp_sdk::mcp_server::{McpServerOptions, server_runtime};
use rust_mcp_sdk::{McpServer, ToMcpServerHandler};
use rust_mcp_sdk::{
    StdioTransport, TransportOptions,
    error::SdkResult,
    schema::{ServerCapabilities, ServerCapabilitiesTools},
};

/// Main entry point for the `piim` MCP server.
#[tokio::main]
async fn main() -> SdkResult<()> {
    let memory_actor_ref = cfg_select! {
            feature = "qdrant_store" => {
                piim_qdrant::init_project_actor().await?
            }
            feature = "lance_store" => {
                piim_lance::init_project_actor().await?
            }
            _ => {
                return Err(rust_mcp_sdk::error::McpSdkError::Internal {
                    description: "No memory backend selected. Enable either `qdrant_store` or `lance_store` feature.".into(),
                })
            }
    };
    let server_details = server_details();

    let transport = StdioTransport::new(TransportOptions::default())?;
    let handler = mcp::MemoryToolHandler::new(memory_actor_ref, env!("CARGO_PKG_VERSION"))
        .to_mcp_server_handler();
    let server = server_runtime::create_server(McpServerOptions {
        transport,
        handler,
        server_details,
        message_observer: None,
    });
    server.start().await
}

pub fn server_details() -> rust_mcp_sdk::ServerDetails {
    rust_mcp_sdk::ServerDetails {
        server_info: rust_mcp_sdk::schema::Implementation {
            name: "piim".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("Project Info in MCP".into()),
            description: Some("Persistent project memory with vector search".into()),
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
