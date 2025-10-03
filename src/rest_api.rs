#![allow(clippy::missing_errors_doc)]
//! REST API surface providing HTTP access to serial port and session tools.
//! This mirrors (a subset of) the MCP tool surface for environments where
//! HTTP integration is preferred. Returns JSON responses with a stable shape.

use axum::{
    extract::{Path, Query, State as AxumState},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::{Arc, Mutex}, time::Duration};

use crate::{
    error::{AppError, AppResult},
    state::{AppState, PortState, PortConfig, DataBitsCfg, ParityCfg, StopBitsCfg, FlowControlCfg},
    session::SessionStore,
};

#[derive(Clone)]
pub struct RestContext {
    pub state: AppState,
    pub sessions: Arc<SessionStore>,
}

// ---------- Serial Port DTOs ----------
#[derive(Deserialize)]
pub struct OpenRequest {
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

#[derive(Deserialize)]
pub struct WriteRequest { pub data: String }

// ---------- Session DTOs ----------
#[derive(Deserialize)]
pub struct CreateSessionRequest { pub device_id: String, pub port_name: Option<String> }
#[derive(Deserialize)]
pub struct AppendMessageRequest { pub session_id: String, pub role: String, pub content: String, pub direction: Option<String>, pub features: Option<String>, pub latency_ms: Option<i64> }

#[derive(Deserialize)]
pub struct ListMessagesParams { pub limit: Option<u64> }
#[derive(Deserialize)]
pub struct FilterMessagesParams { pub role: Option<String>, pub feature: Option<String>, pub direction: Option<String>, pub limit: Option<u64> }

// ---------- Router Builder ----------
pub fn build_router(ctx: RestContext) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ports", get(list_ports))
        .route("/ports/extended", get(list_ports_extended))
        .route("/port/open", post(open_port))
        .route("/port/write", post(write_port))
        .route("/port/read", post(read_port))
        .route("/port/close", post(close_port))
        .route("/port/status", get(status_port))
        .route("/port/metrics", get(metrics_port))
        .route("/sessions", post(create_session))
        .route("/sessions/:id/messages", get(list_messages))
        .route("/sessions/messages/append", post(append_message))
        .route("/sessions/:id/export", get(export_session))
        .route("/sessions/:id/features", get(feature_index))
        .route("/sessions/:id/stats", get(session_stats))
        .route("/sessions/:id/filter", get(filter_messages))
        .with_state(ctx)
}

// ---------- Handlers ----------
async fn health() -> &'static str { "ok" }

async fn list_ports(AxumState(ctx): AxumState<RestContext>) -> Json<Value> {
    match serialport::available_ports() {
        Ok(ports) => Json(json!({"ports": ports.into_iter().map(|p| json!({"port_name": p.port_name})).collect::<Vec<_>>() })),
        Err(e) => Json(err_json("ListPortsError", &e.to_string())),
    }
}

async fn list_ports_extended(AxumState(_ctx): AxumState<RestContext>) -> Json<Value> {
    use serialport::SerialPortType;
    match serialport::available_ports() {
        Ok(ports) => {
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
            Json(json!({"ports": detailed}))
        }
        Err(e) => Json(err_json("ListPortsError", &e.to_string())),
    }
}

async fn open_port(AxumState(ctx): AxumState<RestContext>, Json(req): Json<OpenRequest>) -> Json<Value> {
    let mut st = ctx.state.lock().map_err(|_| AppError::InvalidPayload("state lock".into())).unwrap();
    if matches!(&*st, PortState::Open { .. }) { return Json(err_json("PortAlreadyOpen", "Port already open")); }
    let mut builder = serialport::new(&req.port_name, req.baud_rate)
        .timeout(Duration::from_millis(req.timeout_ms));
    builder = builder
        .data_bits(match req.data_bits { DataBitsCfg::Five => serialport::DataBits::Five, DataBitsCfg::Six => serialport::DataBits::Six, DataBitsCfg::Seven => serialport::DataBits::Seven, DataBitsCfg::Eight => serialport::DataBits::Eight })
        .parity(match req.parity { ParityCfg::None => serialport::Parity::None, ParityCfg::Odd => serialport::Parity::Odd, ParityCfg::Even => serialport::Parity::Even })
        .stop_bits(match req.stop_bits { StopBitsCfg::One => serialport::StopBits::One, StopBitsCfg::Two => serialport::StopBits::Two })
        .flow_control(match req.flow_control { FlowControlCfg::None => serialport::FlowControl::None, FlowControlCfg::Hardware => serialport::FlowControl::Hardware, FlowControlCfg::Software => serialport::FlowControl::Software });
    match builder.open() {
        Ok(port) => {
            *st = PortState::Open {
                port,
                config: PortConfig { port_name: req.port_name.clone(), baud_rate: req.baud_rate, timeout_ms: req.timeout_ms, data_bits: req.data_bits, parity: req.parity, stop_bits: req.stop_bits, flow_control: req.flow_control, terminator: req.terminator, idle_disconnect_ms: req.idle_disconnect_ms },
                last_activity: std::time::Instant::now(),
                timeout_streak: 0,
                bytes_read_total: 0,
                bytes_written_total: 0,
                idle_close_count: 0,
                open_started: std::time::Instant::now(),
            };
            Json(json!({"status":"ok","message":"opened"}))
        }
        Err(e) => Json(err_json("OpenError", &e.to_string())),
    }
}

async fn write_port(AxumState(ctx): AxumState<RestContext>, Json(req): Json<WriteRequest>) -> Json<Value> {
    let mut st = ctx.state.lock().unwrap();
    match &mut *st {
        PortState::Open { port, config, last_activity, bytes_written_total, .. } => {
            let mut data = req.data;
            if let Some(term) = &config.terminator { if !data.ends_with(term) { data.push_str(term); } }
            match port.write(data.as_bytes()) {
                Ok(bytes) => { *bytes_written_total += bytes as u64; *last_activity = std::time::Instant::now(); Json(json!({"status":"ok","bytes_written":bytes,"bytes_written_total":*bytes_written_total})) }
                Err(e) => Json(err_json("WriteError", &e.to_string()))
            }
        }
        _ => Json(err_json("PortNotOpen", "Port not open"))
    }
}

async fn read_port(AxumState(ctx): AxumState<RestContext>) -> Json<Value> {
    let mut st = ctx.state.lock().unwrap();
    match &mut *st {
        PortState::Open { port, config, last_activity, timeout_streak, bytes_read_total, idle_close_count, .. } => {
            let mut buffer = vec![0u8; 1024];
            let bytes_read = match port.read(buffer.as_mut_slice()) { Ok(n) => n, Err(e) if e.kind() == std::io::ErrorKind::TimedOut => 0, Err(e) => return Json(err_json("ReadError", &e.to_string())) };
            let raw = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
            if bytes_read > 0 { *last_activity = std::time::Instant::now(); *timeout_streak = 0; *bytes_read_total += bytes_read as u64; } else { *timeout_streak += 1; }
            let idle_expired = bytes_read == 0 && config.idle_disconnect_ms.map(|ms| last_activity.elapsed() >= Duration::from_millis(ms)).unwrap_or(false);
            if idle_expired { *idle_close_count += 1; *st = PortState::Closed; return Json(json!({"status":"ok","event":"auto_close","reason":"idle_timeout","idle_close_count":*idle_close_count})); }
            let data = if let Some(term) = &config.terminator { raw.trim_end_matches(term).to_string() } else { raw };
            Json(json!({"status":"ok","data":data,"bytes_read":bytes_read,"bytes_read_total":*bytes_read_total}))
        }
        _ => Json(err_json("PortNotOpen", "Port not open"))
    }
}

async fn close_port(AxumState(ctx): AxumState<RestContext>) -> Json<Value> {
    let mut st = ctx.state.lock().unwrap();
    match &*st {
        PortState::Closed => Json(json!({"status":"ok","message":"already closed"})),
        _ => { *st = PortState::Closed; Json(json!({"status":"ok","message":"closed"})) }
    }
}

async fn status_port(AxumState(ctx): AxumState<RestContext>) -> Json<Value> {
    let st = ctx.state.lock().unwrap();
    let status = serde_json::to_value(&*st).unwrap_or(json!({"status":"unknown"}));
    Json(json!({"status":"ok","port":status}))
}

async fn metrics_port(AxumState(ctx): AxumState<RestContext>) -> Json<Value> {
    let st = ctx.state.lock().unwrap();
    match &*st {
        PortState::Closed => Json(json!({"status":"ok","state":"Closed"})),
        PortState::Open { bytes_read_total, bytes_written_total, idle_close_count, open_started, last_activity, timeout_streak, .. } => {
            Json(json!({
                "status":"ok",
                "state":"Open",
                "bytes_read_total":bytes_read_total,
                "bytes_written_total":bytes_written_total,
                "idle_close_count":idle_close_count,
                "open_duration_ms": open_started.elapsed().as_millis() as u64,
                "last_activity_ms": last_activity.elapsed().as_millis() as u64,
                "timeout_streak": timeout_streak,
            }))
        }
    }
}

// ---------- Session Handlers ----------
async fn create_session(AxumState(ctx): AxumState<RestContext>, Json(req): Json<CreateSessionRequest>) -> Json<Value> {
    match ctx.sessions.create_session(&req.device_id, req.port_name.as_deref()).await {
        Ok(s) => Json(json!({"status":"ok","session":s})),
        Err(e) => Json(err_json("CreateSessionError", &e.to_string())),
    }
}

async fn append_message(AxumState(ctx): AxumState<RestContext>, Json(req): Json<AppendMessageRequest>) -> Json<Value> {
    match ctx.sessions.append_message(&req.session_id, &req.role, req.direction.as_deref(), &req.content, req.features.as_deref(), req.latency_ms).await {
        Ok((id, ts)) => Json(json!({"status":"ok","message_id":id,"created_at":ts})),
        Err(e) => Json(err_json("AppendMessageError", &e.to_string())),
    }
}

async fn list_messages(Path(id): Path<String>, AxumState(ctx): AxumState<RestContext>, Query(q): Query<ListMessagesParams>) -> Json<Value> {
    let limit = q.limit.unwrap_or(100) as i64;
    match ctx.sessions.list_messages(&id, limit).await {
        Ok(msgs) => Json(json!({"status":"ok","messages":msgs})),
        Err(e) => Json(err_json("ListMessagesError", &e.to_string())),
    }
}

async fn export_session(Path(id): Path<String>, AxumState(ctx): AxumState<RestContext>) -> Json<Value> {
    match ctx.sessions.export_session_json(&id).await {
        Ok(v) => Json(json!({"status":"ok","export":v})),
        Err(e) => Json(err_json("ExportSessionError", &e.to_string())),
    }
}

async fn feature_index(Path(id): Path<String>, AxumState(ctx): AxumState<RestContext>) -> Json<Value> {
    match ctx.sessions.export_features_index(&id).await {
        Ok(idx) => Json(json!({"status":"ok","index":idx})),
        Err(e) => Json(err_json("FeatureIndexError", &e.to_string())),
    }
}

async fn session_stats(Path(id): Path<String>, AxumState(ctx): AxumState<RestContext>) -> Json<Value> {
    match ctx.sessions.session_stats(&id).await {
        Ok(Some(stats)) => Json(json!({"status":"ok","stats":stats})),
        Ok(None) => Json(json!({"status":"ok","stats":null})),
        Err(e) => Json(err_json("SessionStatsError", &e.to_string())),
    }
}

async fn filter_messages(Path(id): Path<String>, AxumState(ctx): AxumState<RestContext>, Query(params): Query<FilterMessagesParams>) -> Json<Value> {
    let limit = params.limit.unwrap_or(100) as i64;
    match ctx.sessions.filter_messages(&id, params.role.as_deref(), params.feature.as_deref(), params.direction.as_deref(), limit).await {
        Ok(msgs) => Json(json!({"status":"ok","messages":msgs})),
        Err(e) => Json(err_json("FilterMessagesError", &e.to_string())),
    }
}

// ---------- Helpers ----------
fn err_json(kind: &str, msg: &str) -> Value { json!({"status":"error","error":{"type":kind,"message":msg}}) }
