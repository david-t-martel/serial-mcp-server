//! New MCP implementation using official rust-mcp-sdk.
//! Legacy REST functionality has been deprecated and moved to `legacy_rest.rs` (to be removed in a future release).

#![allow(clippy::module_name_repetitions)]

use std::{io::Write, sync::{Arc, Mutex}, time::Duration};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use rust_mcp_sdk::{
    prelude::*,
    server_runtime,
    server_handler::ServerHandler,
};

use crate::{state::{PortState, PortConfig, AppState}, error::AppError};

// ------------------ Tool Definitions ------------------

#[mcp_tool(name = "list_ports", description = "List available serial ports on this system")] 
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListPortsTool {}

#[mcp_tool(name = "open_port", description = "Open a serial port with configuration")] 
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct OpenPortTool {
    pub port_name: String,
    pub baud_rate: u32,
    #[serde(default = "default_timeout_ms")] pub timeout_ms: u64,
}
fn default_timeout_ms() -> u64 { 1000 }

#[mcp_tool(name = "write", description = "Write UTF-8 data to the open serial port")] 
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct WriteTool { pub data: String }

#[mcp_tool(name = "read", description = "Read data from the open serial port (up to 1024 bytes)")] 
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReadTool {}

#[mcp_tool(name = "close", description = "Close the currently open serial port (idempotent)")] 
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CloseTool {}

#[mcp_tool(name = "status", description = "Return current port status and configuration")] 
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct StatusTool {}

// Future: binary read/write, streaming subscriptions, configure line endings, etc.

// ------------------ Handler ------------------
pub struct SerialServerHandler {
    pub state: AppState,
}

impl SerialServerHandler {
    fn list_ports_impl(&self) -> Result<CallToolResult, CallToolError> {
        let ports = serialport::available_ports().map_err(|e| CallToolError::internal_error(e.to_string()))?;
        let names: Vec<_> = ports.into_iter().map(|p| json!({"port_name": p.port_name})).collect();
        Ok(CallToolResult::json_content(vec![json!({"ports": names})]))
    }
    fn open_port_impl(&self, tool: OpenPortTool) -> Result<CallToolResult, CallToolError> {
        let mut st = self.state.lock().map_err(|_| CallToolError::internal_error("State lock poisoned".into()))?;
        if let PortState::Open { .. } = *st { return Err(CallToolError::invalid_request("Port already open".into())); }
        let port = serialport::new(&tool.port_name, tool.baud_rate)
            .timeout(Duration::from_millis(tool.timeout_ms))
            .open().map_err(|e| CallToolError::internal_error(e.to_string()))?;
        *st = PortState::Open { port, config: PortConfig { port_name: tool.port_name, baud_rate: tool.baud_rate, timeout_ms: tool.timeout_ms } };
        Ok(CallToolResult::text_content(vec![TextContent::from("opened".to_string())]))
    }
    fn write_impl(&self, tool: WriteTool) -> Result<CallToolResult, CallToolError> {
        let mut st = self.state.lock().map_err(|_| CallToolError::internal_error("State lock poisoned".into()))?;
        match &mut *st {
            PortState::Open { port, .. } => {
                let bytes = port.write(tool.data.as_bytes()).map_err(|e| CallToolError::internal_error(e.to_string()))?;
                Ok(CallToolResult::json_content(vec![json!({"bytes_written": bytes})]))
            }
            _ => Err(CallToolError::invalid_request("Port not open".into()))
        }
    }
    fn read_impl(&self) -> Result<CallToolResult, CallToolError> {
        let mut st = self.state.lock().map_err(|_| CallToolError::internal_error("State lock poisoned".into()))?;
        match &mut *st {
            PortState::Open { port, .. } => {
                let mut buffer = vec![0u8; 1024];
                let bytes_read = match port.read(buffer.as_mut_slice()) {
                    Ok(n) => n,
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => 0,
                    Err(e) => return Err(CallToolError::internal_error(e.to_string()))
                };
                let data = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                Ok(CallToolResult::json_content(vec![json!({"data": data, "bytes_read": bytes_read})]))
            }
            _ => Err(CallToolError::invalid_request("Port not open".into()))
        }
    }
    fn close_impl(&self) -> Result<CallToolResult, CallToolError> {
        let mut st = self.state.lock().map_err(|_| CallToolError::internal_error("State lock poisoned".into()))?;
        match &*st {
            PortState::Closed => Ok(CallToolResult::text_content(vec![TextContent::from("already closed".to_string())])),
            _ => { *st = PortState::Closed; Ok(CallToolResult::text_content(vec![TextContent::from("closed".to_string())])) }
        }
    }
    fn status_impl(&self) -> Result<CallToolResult, CallToolError> {
        let st = self.state.lock().map_err(|_| CallToolError::internal_error("State lock poisoned".into()))?;
        let val = serde_json::to_value(&*st).map_err(|e| CallToolError::internal_error(e.to_string()))?;
        Ok(CallToolResult::json_content(vec![json!({"status": val})]))
    }
}

#[async_trait]
impl ServerHandler for SerialServerHandler {
    async fn handle_list_tools_request(&self, _req: ListToolsRequest, _rt: Arc<dyn McpServer>) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult { tools: vec![
            ListPortsTool::tool(),
            OpenPortTool::tool(),
            WriteTool::tool(),
            ReadTool::tool(),
            CloseTool::tool(),
            StatusTool::tool(),
        ], meta: None, next_cursor: None })
    }

    async fn handle_call_tool_request(&self, req: CallToolRequest, _rt: Arc<dyn McpServer>) -> Result<CallToolResult, CallToolError> {
        match req.tool_name() {
            n if n == ListPortsTool::tool_name() => self.list_ports_impl(),
            n if n == OpenPortTool::tool_name() => {
                let parsed: OpenPortTool = req.deserialize_arguments()?; self.open_port_impl(parsed)
            }
            n if n == WriteTool::tool_name() => { let parsed: WriteTool = req.deserialize_arguments()?; self.write_impl(parsed) }
            n if n == ReadTool::tool_name() => self.read_impl(),
            n if n == CloseTool::tool_name() => self.close_impl(),
            n if n == StatusTool::tool_name() => self.status_impl(),
            other => Err(CallToolError::unknown_tool(other.to_string()))
        }
    }
}

/// Create and start the MCP server runtime (stdio or http depending on args)
pub async fn start_mcp_server_stdio(state: AppState) -> SdkResult<()> {
    let details = InitializeResult {
        server_info: Implementation { name: "Serial MCP Server".into(), version: env!("CARGO_PKG_VERSION").into(), title: Some("Serial Port MCP Server".into()) },
        capabilities: ServerCapabilities { tools: Some(ServerCapabilitiesTools { list_changed: None }), ..Default::default() },
        meta: None,
        instructions: Some("Use MCP tools to manage a single serial port".into()),
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    };
    let transport = StdioTransport::new(TransportOptions::default())?;
    let handler = SerialServerHandler { state };
    let server = server_runtime::create_server(details, transport, handler);
    server.start().await
}

