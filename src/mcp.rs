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
use crate::session::{SessionStore};

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
    #[serde(default)] pub terminator: Option<String>,
    #[serde(default)] pub idle_disconnect_ms: Option<u64>,
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

#[mcp_tool(name = "metrics", description = "Return cumulative port IO metrics and timing")] 
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct MetricsTool {}

// Reconfigure (close+open) an existing port with new settings, resetting metrics
#[mcp_tool(name = "reconfigure_port", description = "Reopen (or open) the serial port with new configuration, resetting runtime metrics")] 
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReconfigurePortTool {
    #[serde(default)] pub port_name: Option<String>,
    #[serde(default = "default_reconfig_baud")] pub baud_rate: u32,
    #[serde(default = "default_timeout_ms")] pub timeout_ms: u64,
    #[serde(default = "default_data_bits")] pub data_bits: DataBitsCfg,
    #[serde(default = "default_parity")] pub parity: ParityCfg,
    #[serde(default = "default_stop_bits")] pub stop_bits: StopBitsCfg,
    #[serde(default = "default_flow_control")] pub flow_control: FlowControlCfg,
    #[serde(default)] pub terminator: Option<String>,
    #[serde(default)] pub idle_disconnect_ms: Option<u64>,
}
fn default_reconfig_baud() -> u32 { 9600 }

// --- Session Tool Schemas ---
#[mcp_tool(name = "create_session", description = "Create a new session for a logical device id")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CreateSessionTool { pub device_id: String, pub port_name: Option<String> }

#[mcp_tool(name = "append_message", description = "Append a message to a session timeline")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AppendMessageTool { pub session_id: String, pub role: String, pub content: String, pub direction: Option<String>, pub features: Option<String>, pub latency_ms: Option<i64> }

#[mcp_tool(name = "list_messages", description = "List messages for a session (ascending)")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListMessagesTool { pub session_id: String, pub limit: Option<u64> }

#[mcp_tool(name = "export_session", description = "Export full session with messages as JSON")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ExportSessionTool { pub session_id: String }

#[mcp_tool(name = "filter_messages", description = "Filter messages by role / feature substring / direction")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FilterMessagesTool { pub session_id: String, pub role: Option<String>, pub feature: Option<String>, pub direction: Option<String>, pub limit: Option<u64> }

#[mcp_tool(name = "feature_index", description = "Build an index of feature tag counts for a session")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FeatureIndexTool { pub session_id: String }

#[mcp_tool(name = "session_stats", description = "Lightweight stats for a session (count, last id, rate)")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SessionStatsTool { pub session_id: String }

#[mcp_tool(name = "list_ports_extended", description = "List serial ports with extended metadata (VID/PID, manufacturer, product, serial number, type)")]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListPortsExtendedTool {}

// Future: binary read/write, streaming subscriptions, configure line endings, etc.

// ------------------ Handler ------------------
pub struct SerialServerHandler {
    pub state: AppState,
    pub sessions: SessionStore,
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
    fn list_ports_extended_impl(&self) -> Result<CallToolResult, CallToolError> {
        use serialport::SerialPortType;
        let ports = serialport::available_ports().map_err(|e| CallToolError::from_message(e.to_string()))?;
        let detailed: Vec<_> = ports.into_iter().map(|p| {
            let mut obj = serde_json::Map::new();
            obj.insert("port_name".into(), json!(p.port_name));
            match p.port_type {
                SerialPortType::UsbPort(info) => {
                    obj.insert("transport".into(), json!("usb"));
                    obj.insert("vid".into(), json!(format!("0x{:04x}", info.vid)));
                    obj.insert("pid".into(), json!(format!("0x{:04x}", info.pid)));
                    if let Some(sn) = info.serial_number { obj.insert("serial_number".into(), json!(sn)); }
                    if let Some(mf) = info.manufacturer { obj.insert("manufacturer".into(), json!(mf)); }
                    if let Some(prod) = info.product { obj.insert("product".into(), json!(prod)); }
                }
                SerialPortType::BluetoothPort => { obj.insert("transport".into(), json!("bluetooth")); }
                SerialPortType::PciPort => { obj.insert("transport".into(), json!("pci")); }
                SerialPortType::Unknown => { obj.insert("transport".into(), json!("unknown")); }
            }
            json!(obj)
        }).collect();
        let mut structured = serde_json::Map::new();
        structured.insert("ports".into(), serde_json::Value::Array(detailed));
        Ok(CallToolResult::text_content(vec![TextContent::from("ports detailed".to_string())]).with_structured_content(structured))
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
        *st = PortState::Open { 
            port, 
            config: PortConfig { 
                port_name: tool.port_name, 
                baud_rate: tool.baud_rate, 
                timeout_ms: tool.timeout_ms, 
                data_bits: tool.data_bits, 
                parity: tool.parity, 
                stop_bits: tool.stop_bits, 
                flow_control: tool.flow_control, 
                terminator: tool.terminator, 
                idle_disconnect_ms: tool.idle_disconnect_ms 
            }, 
            last_activity: std::time::Instant::now(), 
            timeout_streak: 0,
            bytes_read_total: 0,
            bytes_written_total: 0,
            idle_close_count: 0,
            open_started: std::time::Instant::now(),
        };
        Ok(CallToolResult::text_content(vec![TextContent::from("opened".to_string())]))
    }
    fn write_impl(&self, tool: WriteTool) -> Result<CallToolResult, CallToolError> {
    let mut st = self.state.lock().map_err(|_| CallToolError::from_message("State lock poisoned"))?;
        match &mut *st {
            PortState::Open { port, config, last_activity, bytes_written_total, .. } => {
                let mut data = tool.data;
                if let Some(term) = &config.terminator { if !data.ends_with(term) { data.push_str(term); } }
                let bytes = port.write(data.as_bytes()).map_err(|e| CallToolError::from_message(e.to_string()))?;
                *bytes_written_total += bytes as u64;
                *last_activity = std::time::Instant::now();
                let mut structured = serde_json::Map::new();
                structured.insert("bytes_written".into(), serde_json::Value::Number(bytes.into()));
                structured.insert("bytes_written_total".into(), serde_json::Value::Number((*bytes_written_total).into()));
                Ok(CallToolResult::text_content(vec![TextContent::from(format!("wrote {} bytes", bytes))])
                    .with_structured_content(structured))
            }
            _ => Err(CallToolError::from_message("Port not open"))
        }
    }
    fn read_impl(&self) -> Result<CallToolResult, CallToolError> {
    let mut st = self.state.lock().map_err(|_| CallToolError::from_message("State lock poisoned"))?;
        match &mut *st {
            PortState::Open { port, config, last_activity, timeout_streak, bytes_read_total, idle_close_count, .. } => {
                let mut buffer = vec![0u8; 1024];
                let bytes_read = match port.read(buffer.as_mut_slice()) {
                    Ok(n) => n,
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => 0,
                    Err(e) => return Err(CallToolError::from_message(e.to_string()))
                };
                let data_raw = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                if bytes_read > 0 { *last_activity = std::time::Instant::now(); *timeout_streak = 0; *bytes_read_total += bytes_read as u64; }
                else { *timeout_streak += 1; }
                // Check idle auto-close without mut-borrowing st inside the pattern
                let should_close_idle = bytes_read == 0 && config.idle_disconnect_ms.map(|ms| last_activity.elapsed() >= Duration::from_millis(ms)).unwrap_or(false);
                if should_close_idle {
                    if let Some(ms) = config.idle_disconnect_ms { *idle_close_count += 1; }
                    let count = *idle_close_count;
                    let ms = config.idle_disconnect_ms.unwrap_or(0);
                    // Drop match before mutating the overall enum to avoid double borrow
                    let mut structured = serde_json::Map::new();
                    structured.insert("event".into(), json!("auto_close"));
                    structured.insert("reason".into(), json!("idle_timeout"));
                    structured.insert("idle_ms".into(), json!(ms));
                    structured.insert("idle_close_count".into(), json!(count));
                    // Replace outer state now (safe; we still hold the lock but not the inner borrow)
                    *st = PortState::Closed;
                    return Ok(CallToolResult::text_content(vec![TextContent::from("closed (idle timeout)".to_string())]).with_structured_content(structured));
                }
                let data = if let Some(term) = &config.terminator { data_raw.trim_end_matches(term).to_string() } else { data_raw };
                let mut structured = serde_json::Map::new();
                structured.insert("data".into(), serde_json::Value::String(data.clone()));
                structured.insert("bytes_read".into(), serde_json::Value::Number(bytes_read.into()));
                structured.insert("bytes_read_total".into(), serde_json::Value::Number((*bytes_read_total).into()));
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
    fn metrics_impl(&self) -> Result<CallToolResult, CallToolError> {
        use serde_json::Value;
        let st = self.state.lock().map_err(|_| CallToolError::from_message("State lock poisoned"))?;
        let mut structured = serde_json::Map::new();
        match &*st {
            PortState::Closed => {
                structured.insert("state".into(), json!("Closed"));
            }
            PortState::Open { bytes_read_total, bytes_written_total, idle_close_count, open_started, last_activity, timeout_streak, .. } => {
                structured.insert("state".into(), json!("Open"));
                structured.insert("bytes_read_total".into(), json!(bytes_read_total));
                structured.insert("bytes_written_total".into(), json!(bytes_written_total));
                structured.insert("idle_close_count".into(), json!(idle_close_count));
                structured.insert("open_duration_ms".into(), json!(open_started.elapsed().as_millis() as u64));
                structured.insert("last_activity_ms".into(), json!(last_activity.elapsed().as_millis() as u64));
                structured.insert("timeout_streak".into(), json!(timeout_streak));
            }
        }
        Ok(CallToolResult::text_content(vec![TextContent::from("metrics".to_string())]).with_structured_content(structured))
    }
    fn reconfigure_port_impl(&self, tool: ReconfigurePortTool) -> Result<CallToolResult, CallToolError> {
        let mut st = self.state.lock().map_err(|_| CallToolError::from_message("State lock poisoned"))?;
        let target = match (&tool.port_name, &*st) {
            (Some(p), _) => p.clone(),
            (None, PortState::Open { config, .. }) => config.port_name.clone(),
            (None, PortState::Closed) => return Err(CallToolError::from_message("No port open and no port_name provided")),
        };
        let mut builder = serialport::new(&target, tool.baud_rate).timeout(Duration::from_millis(tool.timeout_ms));
        builder = builder
            .data_bits(match tool.data_bits { DataBitsCfg::Five => serialport::DataBits::Five, DataBitsCfg::Six => serialport::DataBits::Six, DataBitsCfg::Seven => serialport::DataBits::Seven, DataBitsCfg::Eight => serialport::DataBits::Eight })
            .parity(match tool.parity { ParityCfg::None => serialport::Parity::None, ParityCfg::Odd => serialport::Parity::Odd, ParityCfg::Even => serialport::Parity::Even })
            .stop_bits(match tool.stop_bits { StopBitsCfg::One => serialport::StopBits::One, StopBitsCfg::Two => serialport::StopBits::Two })
            .flow_control(match tool.flow_control { FlowControlCfg::None => serialport::FlowControl::None, FlowControlCfg::Hardware => serialport::FlowControl::Hardware, FlowControlCfg::Software => serialport::FlowControl::Software });
        let port = builder.open().map_err(|e| CallToolError::from_message(format!("reconfigure failed: {e}")))?;
        *st = PortState::Open {
            port,
            config: PortConfig {
                port_name: target.clone(),
                baud_rate: tool.baud_rate,
                timeout_ms: tool.timeout_ms,
                data_bits: tool.data_bits,
                parity: tool.parity,
                stop_bits: tool.stop_bits,
                flow_control: tool.flow_control,
                terminator: tool.terminator.clone(),
                idle_disconnect_ms: tool.idle_disconnect_ms,
            },
            last_activity: std::time::Instant::now(),
            timeout_streak: 0,
            bytes_read_total: 0,
            bytes_written_total: 0,
            idle_close_count: 0,
            open_started: std::time::Instant::now(),
        };
        let mut structured = serde_json::Map::new();
        structured.insert("port_name".into(), json!(target));
        structured.insert("baud_rate".into(), json!(tool.baud_rate));
        structured.insert("data_bits".into(), json!(format!("{:?}", tool.data_bits).to_lowercase()));
        structured.insert("parity".into(), json!(format!("{:?}", tool.parity).to_lowercase()));
        structured.insert("stop_bits".into(), json!(format!("{:?}", tool.stop_bits).to_lowercase()));
        structured.insert("flow_control".into(), json!(format!("{:?}", tool.flow_control).to_lowercase()));
        if let Some(t) = &tool.terminator { structured.insert("terminator".into(), json!(t)); }
        if let Some(ms) = tool.idle_disconnect_ms { structured.insert("idle_disconnect_ms".into(), json!(ms)); }
        Ok(CallToolResult::text_content(vec![TextContent::from("reconfigured".to_string())]).with_structured_content(structured))
    }
    // --- Session Management ---
    async fn create_session_impl(&self, device_id: String, port_name: Option<String>) -> Result<CallToolResult, CallToolError> {
        let s = self.sessions.create_session(&device_id, port_name.as_deref()).await.map_err(|e| CallToolError::from_message(e.to_string()))?;
        let mut structured = serde_json::Map::new();
        structured.insert("session".into(), serde_json::to_value(s).unwrap_or_default());
        Ok(CallToolResult::text_content(vec![TextContent::from("session created".to_string())]).with_structured_content(structured))
    }
    async fn append_message_impl(&self, session_id: String, role: String, content: String) -> Result<CallToolResult, CallToolError> {
        let (msg_id, created_at) = self.sessions.append_message(&session_id, &role, None, &content, None, None).await.map_err(|e| CallToolError::from_message(e.to_string()))?;
        let mut structured = serde_json::Map::new();
        structured.insert("message_id".into(), serde_json::Value::Number(msg_id.into()));
        structured.insert("session_id".into(), serde_json::Value::String(session_id));
        structured.insert("role".into(), serde_json::Value::String(role));
        structured.insert("created_at".into(), serde_json::Value::String(created_at.to_rfc3339()));
        Ok(CallToolResult::text_content(vec![TextContent::from("message stored".to_string())]).with_structured_content(structured))
    }
    async fn append_message_extended_impl(&self, session_id: String, role: String, direction: Option<String>, content: String, features: Option<String>, latency_ms: Option<i64>) -> Result<CallToolResult, CallToolError> {
        let (msg_id, created_at) = self.sessions.append_message(&session_id, &role, direction.as_deref(), &content, features.as_deref(), latency_ms).await.map_err(|e| CallToolError::from_message(e.to_string()))?;
        let mut structured = serde_json::Map::new();
        structured.insert("message_id".into(), serde_json::Value::Number(msg_id.into()));
        structured.insert("session_id".into(), serde_json::Value::String(session_id));
        structured.insert("role".into(), serde_json::Value::String(role));
        if let Some(d) = direction { structured.insert("direction".into(), serde_json::Value::String(d)); }
        if let Some(f) = features { structured.insert("features".into(), serde_json::Value::String(f)); }
        if let Some(l) = latency_ms { structured.insert("latency_ms".into(), serde_json::Value::Number(l.into())); }
        structured.insert("created_at".into(), serde_json::Value::String(created_at.to_rfc3339()));
        Ok(CallToolResult::text_content(vec![TextContent::from("message stored".to_string())]).with_structured_content(structured))
    }
    async fn list_messages_impl(&self, session_id: String, limit: u64) -> Result<CallToolResult, CallToolError> {
        let msgs = self.sessions.list_messages(&session_id, limit as i64).await.map_err(|e| CallToolError::from_message(e.to_string()))?;
        let mut structured = serde_json::Map::new();
        structured.insert("messages".into(), serde_json::to_value(msgs).unwrap_or_default());
        Ok(CallToolResult::text_content(vec![TextContent::from("messages listed".to_string())]).with_structured_content(structured))
    }
    async fn export_session_impl(&self, session_id: String) -> Result<CallToolResult, CallToolError> {
        let export = self.sessions.export_session_json(&session_id).await.map_err(|e| CallToolError::from_message(e.to_string()))?;
        let mut structured = serde_json::Map::new();
        structured.insert("export".into(), export);
        Ok(CallToolResult::text_content(vec![TextContent::from("session export".to_string())]).with_structured_content(structured))
    }
    async fn filter_messages_impl(&self, session_id: String, role: Option<String>, feature: Option<String>, direction: Option<String>, limit: u64) -> Result<CallToolResult, CallToolError> {
        let msgs = self.sessions.filter_messages(&session_id, role.as_deref(), feature.as_deref(), direction.as_deref(), limit as i64).await.map_err(|e| CallToolError::from_message(e.to_string()))?;
        let mut structured = serde_json::Map::new();
        structured.insert("messages".into(), serde_json::to_value(msgs).unwrap_or_default());
        Ok(CallToolResult::text_content(vec![TextContent::from("messages filtered".to_string())]).with_structured_content(structured))
    }
    async fn feature_index_impl(&self, session_id: String) -> Result<CallToolResult, CallToolError> {
        let idx = self.sessions.export_features_index(&session_id).await.map_err(|e| CallToolError::from_message(e.to_string()))?;
        let mut structured = serde_json::Map::new();
        structured.insert("feature_index".into(), idx);
        Ok(CallToolResult::text_content(vec![TextContent::from("feature index".to_string())]).with_structured_content(structured))
    }
}

#[async_trait]
impl ServerHandler for SerialServerHandler {
    async fn handle_list_tools_request(&self, _req: ListToolsRequest, _rt: Arc<dyn McpServer>) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult { tools: vec![
            ListPortsTool::tool(),
            ListPortsExtendedTool::tool(),
            OpenPortTool::tool(),
            WriteTool::tool(),
            ReadTool::tool(),
            CloseTool::tool(),
            StatusTool::tool(),
            MetricsTool::tool(),
            ReconfigurePortTool::tool(),
            CreateSessionTool::tool(),
            AppendMessageTool::tool(),
            ListMessagesTool::tool(),
            ExportSessionTool::tool(),
            FilterMessagesTool::tool(),
            FeatureIndexTool::tool(),
            SessionStatsTool::tool(),
            // session tools descriptors will be injected dynamically later if needed
        ], meta: None, next_cursor: None })
    }

    async fn handle_call_tool_request(&self, req: CallToolRequest, _rt: Arc<dyn McpServer>) -> Result<CallToolResult, CallToolError> {
        match req.tool_name() {
            n if n == ListPortsTool::tool_name() => self.list_ports_impl(),
            n if n == ListPortsExtendedTool::tool_name() => self.list_ports_extended_impl(),
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
                let terminator = args.get("terminator").and_then(|v| v.as_str()).map(|s| s.to_string());
                let idle_disconnect_ms = args.get("idle_disconnect_ms").and_then(|v| v.as_u64());
                self.open_port_impl(OpenPortTool { port_name, baud_rate, timeout_ms, data_bits, parity, stop_bits, flow_control, terminator, idle_disconnect_ms })
            }
            n if n == WriteTool::tool_name() => {
                let args = req.params.arguments.clone().unwrap_or_default();
                let data = args.get("data").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::invalid_arguments(WriteTool::tool_name(), Some("data missing".into())))?.to_string();
                self.write_impl(WriteTool { data })
            }
            n if n == ReadTool::tool_name() => self.read_impl(),
            n if n == CloseTool::tool_name() => self.close_impl(),
            n if n == StatusTool::tool_name() => self.status_impl(),
            n if n == MetricsTool::tool_name() => self.metrics_impl(),
            n if n == ReconfigurePortTool::tool_name() => {
                let args = req.params.arguments.clone().unwrap_or_default();
                let parse_enum = |key: &str| args.get(key).and_then(|v| v.as_str()).map(|s| s.to_lowercase());
                let data_bits = match parse_enum("data_bits").as_deref() {
                    None => default_data_bits(),
                    Some("5" | "five") => DataBitsCfg::Five,
                    Some("6" | "six") => DataBitsCfg::Six,
                    Some("7" | "seven") => DataBitsCfg::Seven,
                    Some("8" | "eight") => DataBitsCfg::Eight,
                    Some(other) => return Err(CallToolError::invalid_arguments(ReconfigurePortTool::tool_name(), Some(format!("invalid data_bits: {other}"))))
                };
                let parity = match parse_enum("parity").as_deref() {
                    None => default_parity(),
                    Some("none") => ParityCfg::None,
                    Some("odd") => ParityCfg::Odd,
                    Some("even") => ParityCfg::Even,
                    Some(other) => return Err(CallToolError::invalid_arguments(ReconfigurePortTool::tool_name(), Some(format!("invalid parity: {other}"))))
                };
                let stop_bits = match parse_enum("stop_bits").as_deref() {
                    None => default_stop_bits(),
                    Some("1" | "one") => StopBitsCfg::One,
                    Some("2" | "two") => StopBitsCfg::Two,
                    Some(other) => return Err(CallToolError::invalid_arguments(ReconfigurePortTool::tool_name(), Some(format!("invalid stop_bits: {other}"))))
                };
                let flow_control = match parse_enum("flow_control").as_deref() {
                    None => default_flow_control(),
                    Some("none") => FlowControlCfg::None,
                    Some("hardware" | "rtscts") => FlowControlCfg::Hardware,
                    Some("software" | "xonxoff") => FlowControlCfg::Software,
                    Some(other) => return Err(CallToolError::invalid_arguments(ReconfigurePortTool::tool_name(), Some(format!("invalid flow_control: {other}"))))
                };
                let port_name = args.get("port_name").and_then(|v| v.as_str()).map(|s| s.to_string());
                let baud_rate = args.get("baud_rate").and_then(|v| v.as_u64()).unwrap_or(9600) as u32;
                let timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(1000);
                let terminator = args.get("terminator").and_then(|v| v.as_str()).map(|s| s.to_string());
                let idle_disconnect_ms = args.get("idle_disconnect_ms").and_then(|v| v.as_u64());
                self.reconfigure_port_impl(ReconfigurePortTool { port_name, baud_rate, timeout_ms, data_bits, parity, stop_bits, flow_control, terminator, idle_disconnect_ms })
            }
            n if n == CreateSessionTool::tool_name() => {
                let args = req.params.arguments.clone().unwrap_or_default();
                let device_id = args.get("device_id").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::invalid_arguments(CreateSessionTool::tool_name(), Some("device_id missing".into())))?.to_string();
                let port_name = args.get("port_name").and_then(|v| v.as_str()).map(|s| s.to_string());
                // execute async (handler method is async so we can await here by returning future resolved value)
                return self.create_session_impl(device_id, port_name).await;
            }
            n if n == AppendMessageTool::tool_name() => {
                let args = req.params.arguments.clone().unwrap_or_default();
                let session_id = args.get("session_id").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::invalid_arguments(AppendMessageTool::tool_name(), Some("session_id missing".into())))?.to_string();
                let role = args.get("role").and_then(|v| v.as_str()).unwrap_or("tool").to_string();
                let content = args.get("content").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::invalid_arguments(AppendMessageTool::tool_name(), Some("content missing".into())))?.to_string();
                let direction = args.get("direction").and_then(|v| v.as_str()).map(|s| s.to_string());
                let features = args.get("features").and_then(|v| v.as_str()).map(|s| s.to_string());
                let latency_ms = args.get("latency_ms").and_then(|v| v.as_i64());
                return self.append_message_extended_impl(session_id, role, direction, content, features, latency_ms).await;
            }
            n if n == ListMessagesTool::tool_name() => {
                let args = req.params.arguments.clone().unwrap_or_default();
                let session_id = args.get("session_id").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::invalid_arguments(ListMessagesTool::tool_name(), Some("session_id missing".into())))?.to_string();
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100);
                return self.list_messages_impl(session_id, limit).await;
            }
            n if n == ExportSessionTool::tool_name() => {
                let args = req.params.arguments.clone().unwrap_or_default();
                let session_id = args.get("session_id").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::invalid_arguments(ExportSessionTool::tool_name(), Some("session_id missing".into())))?.to_string();
                return self.export_session_impl(session_id).await;
            }
            n if n == FilterMessagesTool::tool_name() => {
                let args = req.params.arguments.clone().unwrap_or_default();
                let session_id = args.get("session_id").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::invalid_arguments(FilterMessagesTool::tool_name(), Some("session_id missing".into())))?.to_string();
                let role = args.get("role").and_then(|v| v.as_str()).map(|s| s.to_string());
                let feature = args.get("feature").and_then(|v| v.as_str()).map(|s| s.to_string());
                let direction = args.get("direction").and_then(|v| v.as_str()).map(|s| s.to_string());
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100);
                return self.filter_messages_impl(session_id, role, feature, direction, limit).await;
            }
            n if n == FeatureIndexTool::tool_name() => {
                let args = req.params.arguments.clone().unwrap_or_default();
                let session_id = args.get("session_id").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::invalid_arguments(FeatureIndexTool::tool_name(), Some("session_id missing".into())))?.to_string();
                return self.feature_index_impl(session_id).await;
            }
            n if n == SessionStatsTool::tool_name() => {
                let args = req.params.arguments.clone().unwrap_or_default();
                let session_id = args.get("session_id").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::invalid_arguments(SessionStatsTool::tool_name(), Some("session_id missing".into())))?.to_string();
                let stats = self.sessions.session_stats(&session_id).await.map_err(|e| CallToolError::from_message(e.to_string()))?;
                let mut structured = serde_json::Map::new();
                if let Some(s) = stats { structured.insert("stats".into(), s); } else { structured.insert("stats".into(), json!({"session_id": session_id, "message_count": 0})); }
                return Ok(CallToolResult::text_content(vec![TextContent::from("session stats".to_string())]).with_structured_content(structured));
            }
            other => Err(CallToolError::unknown_tool(other.to_string()))
        }
    }
}

/// Create and start the MCP server runtime (stdio or http depending on args)
pub async fn start_mcp_server_stdio(state: AppState, session_store: crate::session::SessionStore) -> SdkResult<()> {
    let details = InitializeResult {
        server_info: Implementation { name: "Serial MCP Server".into(), version: env!("CARGO_PKG_VERSION").into(), title: Some("Serial Port MCP Server".into()) },
        capabilities: ServerCapabilities { tools: Some(ServerCapabilitiesTools { list_changed: None }), ..Default::default() },
        meta: None,
        instructions: Some("Use MCP tools to manage a single serial port".into()),
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    };
    let transport = StdioTransport::new(TransportOptions::default())?;
    // Optional: emit a debug frame very early to verify stdout framing in test environments.
    // This helps diagnose situations where the SDK does not appear to respond to initialize.
    if std::env::var("MCP_DEBUG_BOOT").is_ok() {
        let debug_body = serde_json::json!({"debug":"boot_marker"}).to_string();
        let frame = format!("Content-Length: {}\r\nContent-Type: application/json\r\n\r\n{}", debug_body.as_bytes().len(), debug_body);
        if let Err(e) = write!(std::io::stdout(), "{}", frame) { tracing::error!(error = %e, "failed to write debug boot frame"); }
        let _ = std::io::stdout().flush();
    }
    // Use the provided session store (caller is responsible for lifecycle)
    let handler = SerialServerHandler { state, sessions: session_store };
    let server = server_runtime::create_server(details, transport, handler);
    server.start().await
}

