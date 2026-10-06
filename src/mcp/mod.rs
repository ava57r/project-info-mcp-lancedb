pub mod optimize;
pub mod search;
pub mod upsert;

use async_trait::async_trait;
use kameo::actor::ActorRef;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, CompleteRequestParams, CompleteResult,
    CompleteResultCompletion, GenericResult, GetPromptRequestParams, ListPromptsResult,
    ListPromptsResultCacheScope, ListResourceTemplatesResult,
    ListResourceTemplatesResultCacheScope, ListResourcesResult, ListResourcesResultCacheScope,
    ListToolsResult, ListToolsResultCacheScope, PaginatedRequestParams, ReadResourceRequestParams,
    RpcError, ServerResult, TextContent,
    schema_utils::{CallToolError, CustomRequest},
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

    /// This is a tools-only server: answer optional probes with empty
    /// success instead of `method_not_found` so strict clients (OpenCode)
    /// do not mark the server as failed during startup.
    async fn handle_list_resources_request(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: &RequestContext,
        _runtime: std::sync::Arc<dyn McpServer>,
    ) -> std::result::Result<ListResourcesResult, RpcError> {
        Ok(ListResourcesResult {
            resources: vec![],
            cache_scope: ListResourcesResultCacheScope::Private,
            result_type: "complete".to_string(),
            ttl_ms: 0,
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_list_resource_templates_request(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: &RequestContext,
        _runtime: std::sync::Arc<dyn McpServer>,
    ) -> std::result::Result<ListResourceTemplatesResult, RpcError> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![],
            cache_scope: ListResourceTemplatesResultCacheScope::Private,
            result_type: "complete".to_string(),
            ttl_ms: 0,
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_read_resource_request(
        &self,
        _params: ReadResourceRequestParams,
        _context: &RequestContext,
        _runtime: std::sync::Arc<dyn McpServer>,
    ) -> std::result::Result<ServerResult, RpcError> {
        Err(RpcError::method_not_found()
            .with_message("No resources are exposed by this server.".to_string()))
    }

    async fn handle_list_prompts_request(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: &RequestContext,
        _runtime: std::sync::Arc<dyn McpServer>,
    ) -> std::result::Result<ListPromptsResult, RpcError> {
        Ok(ListPromptsResult {
            prompts: vec![],
            cache_scope: ListPromptsResultCacheScope::Private,
            result_type: "complete".to_string(),
            ttl_ms: 0,
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_get_prompt_request(
        &self,
        _params: GetPromptRequestParams,
        _context: &RequestContext,
        _runtime: std::sync::Arc<dyn McpServer>,
    ) -> std::result::Result<ServerResult, RpcError> {
        Err(RpcError::method_not_found()
            .with_message("No prompts are exposed by this server.".to_string()))
    }

    async fn handle_complete_request(
        &self,
        _params: CompleteRequestParams,
        _context: &RequestContext,
        _runtime: std::sync::Arc<dyn McpServer>,
    ) -> std::result::Result<CompleteResult, RpcError> {
        Ok(CompleteResult {
            completion: CompleteResultCompletion {
                values: vec![],
                has_more: Some(false),
                total: Some(0),
            },
            result_type: "complete".to_string(),
            meta: None,
        })
    }

    async fn handle_custom_request(
        &self,
        _request: CustomRequest,
        _context: &RequestContext,
        _runtime: std::sync::Arc<dyn McpServer>,
    ) -> std::result::Result<GenericResult, RpcError> {
        Ok(GenericResult {
            result_type: "complete".to_string(),
            meta: None,
            extra: None,
        })
    }
}
