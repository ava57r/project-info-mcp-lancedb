pub mod optimize;
pub mod search;
pub mod upsert;

use async_trait::async_trait;
use kameo::actor::ActorRef;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, ListToolsResult, ListToolsResultCacheScope,
    PaginatedRequestParams, RpcError, ServerResult, TextContent, schema_utils::CallToolError,
};
use rust_mcp_sdk::{McpServer, RequestContext, tool_box};

use crate::actors::memory::MemoryActor;
use crate::mcp::optimize::OptimizeProjectInfo;
use crate::mcp::search::SearchProjectInfo;
use crate::mcp::upsert::UpsertProjectInfo;

// Generates `MemoryTools` enum with `tools()` and `TryFrom<CallToolRequestParams>`.
tool_box!(
    MemoryTools,
    [UpsertProjectInfo, SearchProjectInfo, OptimizeProjectInfo]
);

pub struct MemoryToolHandler {
    actor: ActorRef<MemoryActor>,
}

impl MemoryToolHandler {
    pub fn new(actor: ActorRef<MemoryActor>) -> Self {
        Self { actor }
    }
}

#[async_trait]
impl ServerHandler for MemoryToolHandler {
    async fn handle_list_tools_request(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: &RequestContext,
        _runtime: std::sync::Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: MemoryTools::tools(),
            cache_scope: ListToolsResultCacheScope::Private,
            result_type: "complete".to_string(),
            ttl_ms: 0,
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _context: &RequestContext,
        _runtime: std::sync::Arc<dyn McpServer>,
    ) -> std::result::Result<ServerResult, CallToolError> {
        let tool = MemoryTools::try_from(params).map_err(CallToolError::new)?;
        let text = match tool {
            MemoryTools::UpsertProjectInfo(args) => upsert::execute(self.actor.clone(), args).await,
            MemoryTools::SearchProjectInfo(args) => search::execute(self.actor.clone(), args).await,
            MemoryTools::OptimizeProjectInfo(_) => optimize::execute(self.actor.clone()).await,
        };
        Ok(ServerResult::from(CallToolResult::text_content(vec![
            TextContent::from(text),
        ])))
    }
}
