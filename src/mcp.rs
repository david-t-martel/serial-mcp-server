//! New MCP implementation using official rust-mcp-sdk.
//! Legacy REST functionality has been deprecated and moved to `legacy_rest.rs` (to be removed in a future release).

#![allow(clippy::module_name_repetitions)]

use std::{io::Write, sync::Arc, time::Duration};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use rust_mcp_sdk::{
    macros::{mcp_tool, JsonSchema},
    mcp_server::{server_runtime, ServerHandler},
    schema::{
        InitializeResult, Implementation, ServerCapabilities, ServerCapabilitiesTools,
        LATEST_PROTOCOL_VERSION, RpcError, CallToolRequest, CallToolResult,
        ListToolsRequest, ListToolsResult, TextContent,
    },
    error::SdkResult,
    StdioTransport, TransportOptions, McpServer,
};

// CallToolError lives under schema_utils submodule path
use rust_mcp_sdk::schema::mcp_2025_06_18::schema_utils::CallToolError;

use crate::state::{PortState, PortConfig, AppState, DataBitsCfg, ParityCfg, StopBitsCfg, FlowControlCfg};

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
    #[serde(default = "default_data_bits")] pub data_bits: DataBitsCfg,
    #[serde(default = "default_parity")] pub parity: ParityCfg,
    #[serde(default = "default_stop_bits")] pub stop_bits: StopBitsCfg,
    #[serde(default = "default_flow_control")] pub flow_control: FlowControlCfg,
}
fn default_timeout_ms() -> u64 { 1000 }
fn default_data_bits() -> DataBitsCfg { DataBitsCfg::Eight }
fn default_parity() -> ParityCfg { ParityCfg::None }
fn default_stop_bits() -> StopBitsCfg { StopBitsCfg::One }
fn default_flow_control() -> FlowControlCfg { FlowControlCfg::None }

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
    let ports = serialport::available_ports().map_err(|e| CallToolError::from_message(e.to_string()))?;
        let names: Vec<_> = ports.into_iter().map(|p| json!({"port_name": p.port_name})).collect();
        let mut structured = serde_json::Map::new();
        structured.insert("ports".into(), serde_json::Value::Array(names));
        Ok(CallToolResult::text_content(vec![TextContent::from("ports listed".to_string())])
            .with_structured_content(structured))
    }
    fn open_port_impl(&self, tool: OpenPortTool) -> Result<CallToolResult, CallToolError> {
    let mut st = self.state.lock().map_err(|_| CallToolError::from_message("State lock poisoned"))?;
    if let PortState::Open { .. } = *st { return Err(CallToolError::from_message("Port already open")); }
        let mut builder = serialport::new(&tool.port_name, tool.baud_rate)
            .timeout(Duration::from_millis(tool.timeout_ms));
        // Apply extended configuration
        builder = builder
            .data_bits(match tool.data_bits { DataBitsCfg::Five => serialport::DataBits::Five, DataBitsCfg::Six => serialport::DataBits::Six, DataBitsCfg::Seven => serialport::DataBits::Seven, DataBitsCfg::Eight => serialport::DataBits::Eight })
            .parity(match tool.parity { ParityCfg::None => serialport::Parity::None, ParityCfg::Odd => serialport::Parity::Odd, ParityCfg::Even => serialport::Parity::Even })
            .stop_bits(match tool.stop_bits { StopBitsCfg::One => serialport::StopBits::One, StopBitsCfg::Two => serialport::StopBits::Two })
            .flow_control(match tool.flow_control { FlowControlCfg::None => serialport::FlowControl::None, FlowControlCfg::Hardware => serialport::FlowControl::Hardware, FlowControlCfg::Software => serialport::FlowControl::Software });
        let port = builder.open().map_err(|e| CallToolError::from_message(e.to_string()))?;
        *st = PortState::Open { port, config: PortConfig { port_name: tool.port_name, baud_rate: tool.baud_rate, timeout_ms: tool.timeout_ms, data_bits: tool.data_bits, parity: tool.parity, stop_bits: tool.stop_bits, flow_control: tool.flow_control } };
        Ok(CallToolResult::text_content(vec![TextContent::from("opened".to_string())]))
    }
    fn write_impl(&self, tool: WriteTool) -> Result<CallToolResult, CallToolError> {
    let mut st = self.state.lock().map_err(|_| CallToolError::from_message("State lock poisoned"))?;
        match &mut *st {
            PortState::Open { port, .. } => {
                let bytes = port.write(tool.data.as_bytes()).map_err(|e| CallToolError::from_message(e.to_string()))?;
                let mut structured = serde_json::Map::new();
                structured.insert("bytes_written".into(), serde_json::Value::Number(bytes.into()));
                Ok(CallToolResult::text_content(vec![TextContent::from(format!("wrote {} bytes", bytes))])
                    .with_structured_content(structured))
            }
            _ => Err(CallToolError::from_message("Port not open"))
        }
    }
    fn read_impl(&self) -> Result<CallToolResult, CallToolError> {
    let mut st = self.state.lock().map_err(|_| CallToolError::from_message("State lock poisoned"))?;
        match &mut *st {
            PortState::Open { port, .. } => {
                let mut buffer = vec![0u8; 1024];
                let bytes_read = match port.read(buffer.as_mut_slice()) {
                    Ok(n) => n,
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => 0,
                    Err(e) => return Err(CallToolError::from_message(e.to_string()))
                };
                let data = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                let mut structured = serde_json::Map::new();
                structured.insert("data".into(), serde_json::Value::String(data.clone()));
                structured.insert("bytes_read".into(), serde_json::Value::Number(bytes_read.into()));
                Ok(CallToolResult::text_content(vec![TextContent::from(format!("read {} bytes", bytes_read))])
                    .with_structured_content(structured))
            }
            _ => Err(CallToolError::from_message("Port not open"))
        }
    }
    fn close_impl(&self) -> Result<CallToolResult, CallToolError> {
    let mut st = self.state.lock().map_err(|_| CallToolError::from_message("State lock poisoned"))?;
        match &*st {
            PortState::Closed => Ok(CallToolResult::text_content(vec![TextContent::from("already closed".to_string())])),
            _ => { *st = PortState::Closed; Ok(CallToolResult::text_content(vec![TextContent::from("closed".to_string())])) }
        }
    }
    fn status_impl(&self) -> Result<CallToolResult, CallToolError> {
    let st = self.state.lock().map_err(|_| CallToolError::from_message("State lock poisoned"))?;
    let val = serde_json::to_value(&*st).map_err(|e| CallToolError::from_message(e.to_string()))?;
        let mut structured = serde_json::Map::new();
        structured.insert("status".into(), val);
        Ok(CallToolResult::text_content(vec![TextContent::from("status".to_string())])
            .with_structured_content(structured))
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
                // Manually parse args from request params
                let args = req.params.arguments.clone().unwrap_or_default();
                let port_name = args.get("port_name").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::invalid_arguments(OpenPortTool::tool_name(), Some("port_name missing".into())))?.to_string();
                let baud_rate = args.get("baud_rate").and_then(|v| v.as_u64()).ok_or_else(|| CallToolError::invalid_arguments(OpenPortTool::tool_name(), Some("baud_rate missing".into())))? as u32;
                let timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(1000);
                // Helper to parse enum from string
                let parse_enum = |key: &str| args.get(key).and_then(|v| v.as_str()).map(|s| s.to_lowercase());
                let data_bits = match parse_enum("data_bits").as_deref() {
                    None => default_data_bits(),
                    Some("5" | "five") => DataBitsCfg::Five,
                    Some("6" | "six") => DataBitsCfg::Six,
                    Some("7" | "seven") => DataBitsCfg::Seven,
                    Some("8" | "eight") => DataBitsCfg::Eight,
                    Some(other) => return Err(CallToolError::invalid_arguments(OpenPortTool::tool_name(), Some(format!("invalid data_bits: {other}"))))
                };
                let parity = match parse_enum("parity").as_deref() {
                    None => default_parity(),
                    Some("none") => ParityCfg::None,
                    Some("odd") => ParityCfg::Odd,
                    Some("even") => ParityCfg::Even,
                    Some(other) => return Err(CallToolError::invalid_arguments(OpenPortTool::tool_name(), Some(format!("invalid parity: {other}"))))
                };
                let stop_bits = match parse_enum("stop_bits").as_deref() {
                    None => default_stop_bits(),
                    Some("1" | "one") => StopBitsCfg::One,
                    Some("2" | "two") => StopBitsCfg::Two,
                    Some(other) => return Err(CallToolError::invalid_arguments(OpenPortTool::tool_name(), Some(format!("invalid stop_bits: {other}"))))
                };
                let flow_control = match parse_enum("flow_control").as_deref() {
                    None => default_flow_control(),
                    Some("none") => FlowControlCfg::None,
                    Some("hardware" | "rtscts") => FlowControlCfg::Hardware,
                    Some("software" | "xonxoff") => FlowControlCfg::Software,
                    Some(other) => return Err(CallToolError::invalid_arguments(OpenPortTool::tool_name(), Some(format!("invalid flow_control: {other}"))))
                };
                self.open_port_impl(OpenPortTool { port_name, baud_rate, timeout_ms, data_bits, parity, stop_bits, flow_control })
            }
            n if n == WriteTool::tool_name() => {
                let args = req.params.arguments.clone().unwrap_or_default();
                let data = args.get("data").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::invalid_arguments(WriteTool::tool_name(), Some("data missing".into())))?.to_string();
                self.write_impl(WriteTool { data })
            }
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

