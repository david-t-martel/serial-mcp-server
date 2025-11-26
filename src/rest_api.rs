#![allow(clippy::missing_errors_doc)]
//! REST API surface providing HTTP access to serial port and session tools.
//! This mirrors (a subset of) the MCP tool surface for environments where
//! HTTP integration is preferred. Returns JSON responses with a stable shape.

use axum::{
    extract::{Path, Query, State as AxumState},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};

use crate::{
    session::SessionStore,
    state::{
        default_data_bits, default_flow_control, default_parity, default_reconfig_baud,
        default_stop_bits, default_timeout, AppState, DataBitsCfg, FlowControlCfg, ParityCfg,
        StopBitsCfg,
    },
};

#[cfg(feature = "auto-negotiation")]
use crate::state::{PortConfig, PortState};

#[cfg(feature = "auto-negotiation")]
use crate::port::{PortConfiguration, SyncSerialPort};

#[derive(Clone)]
pub struct RestContext {
    pub state: AppState,
    pub sessions: Arc<SessionStore>,
    pub service: crate::service::PortService,
}

// ---------- Serial Port DTOs ----------
#[derive(Deserialize)]
pub struct OpenRequest {
    pub port_name: String,
    pub baud_rate: u32,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_data_bits")]
    pub data_bits: DataBitsCfg,
    #[serde(default = "default_parity")]
    pub parity: ParityCfg,
    #[serde(default = "default_stop_bits")]
    pub stop_bits: StopBitsCfg,
    #[serde(default = "default_flow_control")]
    pub flow_control: FlowControlCfg,
    #[serde(default)]
    pub terminator: Option<String>,
    #[serde(default)]
    pub idle_disconnect_ms: Option<u64>,
}

#[derive(Deserialize)]
pub struct WriteRequest {
    pub data: String,
}

#[derive(Deserialize)]
pub struct ReconfigureRequest {
    pub port_name: Option<String>,
    #[serde(default = "default_reconfig_baud")]
    pub baud_rate: u32,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_data_bits")]
    pub data_bits: DataBitsCfg,
    #[serde(default = "default_parity")]
    pub parity: ParityCfg,
    #[serde(default = "default_stop_bits")]
    pub stop_bits: StopBitsCfg,
    #[serde(default = "default_flow_control")]
    pub flow_control: FlowControlCfg,
    #[serde(default)]
    pub terminator: Option<String>,
    #[serde(default)]
    pub idle_disconnect_ms: Option<u64>,
}

// ---------- Auto-Negotiation DTOs (feature-gated) ----------
#[cfg(feature = "auto-negotiation")]
#[derive(Deserialize)]
pub struct DetectPortRequest {
    pub port_name: String,
    #[serde(default)]
    pub vid: Option<String>,
    #[serde(default)]
    pub pid: Option<String>,
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default)]
    pub suggested_baud_rates: Option<Vec<u32>>,
    #[serde(default = "default_detect_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub preferred_strategy: Option<String>,
}
#[cfg(feature = "auto-negotiation")]
fn default_detect_timeout_ms() -> u64 {
    500
}

#[cfg(feature = "auto-negotiation")]
#[derive(Deserialize)]
pub struct OpenPortAutoRequest {
    pub port_name: String,
    #[serde(default)]
    pub vid: Option<String>,
    #[serde(default)]
    pub pid: Option<String>,
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub terminator: Option<String>,
    #[serde(default)]
    pub idle_disconnect_ms: Option<u64>,
}

// ---------- Session DTOs ----------
#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub device_id: String,
    pub port_name: Option<String>,
}
#[derive(Deserialize)]
pub struct AppendMessageRequest {
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub direction: Option<String>,
    pub features: Option<String>,
    pub latency_ms: Option<i64>,
}

#[derive(Deserialize)]
pub struct ListMessagesParams {
    pub limit: Option<u64>,
}
#[derive(Deserialize)]
pub struct FilterMessagesParams {
    pub role: Option<String>,
    pub feature: Option<String>,
    pub direction: Option<String>,
    pub limit: Option<u64>,
}

// ---------- Router Builder ----------
pub fn build_router(ctx: RestContext) -> Router {
    let mut router = Router::new()
        .route("/health", get(health))
        .route("/ports", get(list_ports))
        .route("/ports/extended", get(list_ports_extended))
        .route("/port/open", post(open_port))
        .route("/port/write", post(write_port))
        .route("/port/read", post(read_port))
        .route("/port/close", post(close_port))
        .route("/port/status", get(status_port))
        .route("/port/metrics", get(metrics_port))
        .route("/port/reconfigure", post(reconfigure_port))
        .route("/sessions", post(create_session))
        .route("/sessions/{id}/messages", get(list_messages))
        .route("/sessions/messages/append", post(append_message))
        .route("/sessions/{id}/export", get(export_session))
        .route("/sessions/{id}/features", get(feature_index))
        .route("/sessions/{id}/stats", get(session_stats))
        .route("/sessions/{id}/filter", get(filter_messages));

    // Add WebSocket route if feature is enabled
    #[cfg(feature = "websocket")]
    {
        router = router.route("/ws/serial", get(crate::websocket::ws_handler));
    }

    // Add auto-negotiation routes if feature is enabled
    #[cfg(feature = "auto-negotiation")]
    {
        router = router
            .route("/port/detect", post(detect_port))
            .route("/port/open_auto", post(open_port_auto))
            .route("/manufacturers", get(list_manufacturer_profiles));
    }

    router.with_state(ctx)
}

// ---------- Handlers ----------
async fn health() -> &'static str {
    "ok"
}

async fn list_ports(AxumState(_ctx): AxumState<RestContext>) -> Json<Value> {
    match serialport::available_ports() {
        Ok(ports) => Json(
            json!({"ports": ports.into_iter().map(|p| json!({"port_name": p.port_name})).collect::<Vec<_>>() }),
        ),
        Err(e) => Json(err_json("ListPortsError", &e.to_string())),
    }
}

async fn list_ports_extended(AxumState(_ctx): AxumState<RestContext>) -> Json<Value> {
    use serialport::SerialPortType;
    match serialport::available_ports() {
        Ok(ports) => {
            let detailed: Vec<_> = ports
                .into_iter()
                .map(|p| {
                    let mut obj = serde_json::Map::new();
                    obj.insert("port_name".into(), json!(p.port_name));
                    match p.port_type {
                        SerialPortType::UsbPort(info) => {
                            obj.insert("transport".into(), json!("usb"));
                            obj.insert("vid".into(), json!(format!("0x{:04x}", info.vid)));
                            obj.insert("pid".into(), json!(format!("0x{:04x}", info.pid)));
                            if let Some(sn) = info.serial_number {
                                obj.insert("serial_number".into(), json!(sn));
                            }
                            if let Some(mf) = info.manufacturer {
                                obj.insert("manufacturer".into(), json!(mf));
                            }
                            if let Some(prod) = info.product {
                                obj.insert("product".into(), json!(prod));
                            }
                        }
                        SerialPortType::BluetoothPort => {
                            obj.insert("transport".into(), json!("bluetooth"));
                        }
                        SerialPortType::PciPort => {
                            obj.insert("transport".into(), json!("pci"));
                        }
                        SerialPortType::Unknown => {
                            obj.insert("transport".into(), json!("unknown"));
                        }
                    }
                    json!(obj)
                })
                .collect();
            Json(json!({"ports": detailed}))
        }
        Err(e) => Json(err_json("ListPortsError", &e.to_string())),
    }
}

async fn open_port(
    AxumState(ctx): AxumState<RestContext>,
    Json(req): Json<OpenRequest>,
) -> Json<Value> {
    use crate::service::OpenConfig;

    let config = OpenConfig {
        port_name: req.port_name,
        baud_rate: req.baud_rate,
        timeout_ms: req.timeout_ms,
        data_bits: req.data_bits,
        parity: req.parity,
        stop_bits: req.stop_bits,
        flow_control: req.flow_control,
        terminator: req.terminator,
        idle_disconnect_ms: req.idle_disconnect_ms,
    };

    match ctx.service.open(config) {
        Ok(_result) => Json(json!({"status":"ok","message":"opened"})),
        Err(e) => {
            let err_type = match e {
                crate::service::ServiceError::PortAlreadyOpen => "PortAlreadyOpen",
                _ => "OpenError",
            };
            Json(err_json(err_type, &e.to_string()))
        }
    }
}

async fn write_port(
    AxumState(ctx): AxumState<RestContext>,
    Json(req): Json<WriteRequest>,
) -> Json<Value> {
    match ctx.service.write(&req.data) {
        Ok(result) => Json(json!({
            "status":"ok",
            "bytes_written": result.bytes_written,
            "bytes_written_total": result.bytes_written_total
        })),
        Err(e) => {
            let err_type = match e {
                crate::service::ServiceError::PortNotOpen => "PortNotOpen",
                _ => "WriteError",
            };
            Json(err_json(err_type, &e.to_string()))
        }
    }
}

async fn read_port(AxumState(ctx): AxumState<RestContext>) -> Json<Value> {
    match ctx.service.read() {
        Ok(result) => {
            if let Some(auto_close) = result.auto_closed {
                Json(json!({
                    "status":"ok",
                    "event":"auto_close",
                    "reason": auto_close.reason,
                    "idle_close_count": auto_close.idle_close_count
                }))
            } else {
                Json(json!({
                    "status":"ok",
                    "data": result.data,
                    "bytes_read": result.bytes_read,
                    "bytes_read_total": result.bytes_read_total
                }))
            }
        }
        Err(e) => {
            let err_type = match e {
                crate::service::ServiceError::PortNotOpen => "PortNotOpen",
                _ => "ReadError",
            };
            Json(err_json(err_type, &e.to_string()))
        }
    }
}

async fn close_port(AxumState(ctx): AxumState<RestContext>) -> Json<Value> {
    match ctx.service.close() {
        Ok(result) => Json(json!({"status":"ok","message": result.message})),
        Err(e) => Json(err_json("CloseError", &e.to_string())),
    }
}

async fn status_port(AxumState(ctx): AxumState<RestContext>) -> Json<Value> {
    match ctx.service.status() {
        Ok(status) => {
            let port_value = serde_json::to_value(&status).unwrap_or(json!({"status":"unknown"}));
            Json(json!({"status":"ok","port": port_value}))
        }
        Err(e) => Json(err_json("StatusError", &e.to_string())),
    }
}

async fn metrics_port(AxumState(ctx): AxumState<RestContext>) -> Json<Value> {
    match ctx.service.metrics() {
        Ok(metrics) => {
            let mut response = json!({"status":"ok","state": metrics.state});
            if let Some(bytes_read) = metrics.bytes_read_total {
                response["bytes_read_total"] = json!(bytes_read);
            }
            if let Some(bytes_written) = metrics.bytes_written_total {
                response["bytes_written_total"] = json!(bytes_written);
            }
            if let Some(idle_count) = metrics.idle_close_count {
                response["idle_close_count"] = json!(idle_count);
            }
            if let Some(duration) = metrics.open_duration_ms {
                response["open_duration_ms"] = json!(duration);
            }
            if let Some(activity) = metrics.last_activity_ms {
                response["last_activity_ms"] = json!(activity);
            }
            if let Some(streak) = metrics.timeout_streak {
                response["timeout_streak"] = json!(streak);
            }
            Json(response)
        }
        Err(e) => Json(err_json("MetricsError", &e.to_string())),
    }
}

// ---------- Session Handlers ----------
async fn create_session(
    AxumState(ctx): AxumState<RestContext>,
    Json(req): Json<CreateSessionRequest>,
) -> Json<Value> {
    match ctx
        .sessions
        .create_session(&req.device_id, req.port_name.as_deref())
        .await
    {
        Ok(s) => Json(json!({"status":"ok","session":s})),
        Err(e) => Json(err_json("CreateSessionError", &e.to_string())),
    }
}

async fn append_message(
    AxumState(ctx): AxumState<RestContext>,
    Json(req): Json<AppendMessageRequest>,
) -> Json<Value> {
    match ctx
        .sessions
        .append_message(
            &req.session_id,
            &req.role,
            req.direction.as_deref(),
            &req.content,
            req.features.as_deref(),
            req.latency_ms,
        )
        .await
    {
        Ok((id, ts)) => Json(json!({"status":"ok","message_id":id,"created_at":ts})),
        Err(e) => Json(err_json("AppendMessageError", &e.to_string())),
    }
}

async fn list_messages(
    Path(id): Path<String>,
    AxumState(ctx): AxumState<RestContext>,
    Query(q): Query<ListMessagesParams>,
) -> Json<Value> {
    let limit = q.limit.unwrap_or(100) as i64;
    match ctx.sessions.list_messages(&id, limit).await {
        Ok(msgs) => Json(json!({"status":"ok","messages":msgs})),
        Err(e) => Json(err_json("ListMessagesError", &e.to_string())),
    }
}

async fn export_session(
    Path(id): Path<String>,
    AxumState(ctx): AxumState<RestContext>,
) -> Json<Value> {
    match ctx.sessions.export_session_json(&id).await {
        Ok(v) => Json(json!({"status":"ok","export":v})),
        Err(e) => Json(err_json("ExportSessionError", &e.to_string())),
    }
}

async fn feature_index(
    Path(id): Path<String>,
    AxumState(ctx): AxumState<RestContext>,
) -> Json<Value> {
    match ctx.sessions.export_features_index(&id).await {
        Ok(idx) => Json(json!({"status":"ok","index":idx})),
        Err(e) => Json(err_json("FeatureIndexError", &e.to_string())),
    }
}

async fn session_stats(
    Path(id): Path<String>,
    AxumState(ctx): AxumState<RestContext>,
) -> Json<Value> {
    match ctx.sessions.session_stats(&id).await {
        Ok(Some(stats)) => Json(json!({"status":"ok","stats":stats})),
        Ok(None) => Json(json!({"status":"ok","stats":null})),
        Err(e) => Json(err_json("SessionStatsError", &e.to_string())),
    }
}

async fn filter_messages(
    Path(id): Path<String>,
    AxumState(ctx): AxumState<RestContext>,
    Query(params): Query<FilterMessagesParams>,
) -> Json<Value> {
    let limit = params.limit.unwrap_or(100) as i64;
    match ctx
        .sessions
        .filter_messages(
            &id,
            params.role.as_deref(),
            params.feature.as_deref(),
            params.direction.as_deref(),
            limit,
        )
        .await
    {
        Ok(msgs) => Json(json!({"status":"ok","messages":msgs})),
        Err(e) => Json(err_json("FilterMessagesError", &e.to_string())),
    }
}

// ---------- Reconfigure Handler ----------
async fn reconfigure_port(
    AxumState(ctx): AxumState<RestContext>,
    Json(req): Json<ReconfigureRequest>,
) -> Json<Value> {
    use crate::service::ReconfigureConfig;

    let config = ReconfigureConfig {
        port_name: req.port_name,
        baud_rate: req.baud_rate,
        timeout_ms: req.timeout_ms,
        data_bits: req.data_bits,
        parity: req.parity,
        stop_bits: req.stop_bits,
        flow_control: req.flow_control,
        terminator: req.terminator,
        idle_disconnect_ms: req.idle_disconnect_ms,
    };

    match ctx.service.reconfigure(config) {
        Ok(result) => Json(json!({
            "status": "ok",
            "message": result.message,
            "port_name": result.port_name,
            "baud_rate": result.baud_rate
        })),
        Err(e) => {
            let err_type = match e {
                crate::service::ServiceError::NoPortSpecified => "InvalidPayload",
                _ => "ReconfigureError",
            };
            Json(err_json(err_type, &e.to_string()))
        }
    }
}

// ---------- Auto-Negotiation Handlers (feature-gated) ----------
#[cfg(feature = "auto-negotiation")]
async fn detect_port(
    AxumState(_ctx): AxumState<RestContext>,
    Json(req): Json<DetectPortRequest>,
) -> Json<Value> {
    use crate::negotiation::{AutoNegotiator, NegotiationHints};

    let mut hints = NegotiationHints {
        timeout_ms: req.timeout_ms,
        ..Default::default()
    };

    // Parse VID/PID from hex strings if provided
    if let Some(vid_str) = &req.vid {
        match u16::from_str_radix(vid_str.trim_start_matches("0x"), 16) {
            Ok(vid) => hints.vid = Some(vid),
            Err(e) => return Json(err_json("InvalidVID", &e.to_string())),
        }
    }
    if let Some(pid_str) = &req.pid {
        match u16::from_str_radix(pid_str.trim_start_matches("0x"), 16) {
            Ok(pid) => hints.pid = Some(pid),
            Err(e) => return Json(err_json("InvalidPID", &e.to_string())),
        }
    }

    hints.manufacturer = req.manufacturer.clone();
    if let Some(rates) = req.suggested_baud_rates {
        hints.suggested_baud_rates = rates;
    }

    let negotiator = AutoNegotiator::new();
    let params = if let Some(strategy) = &req.preferred_strategy {
        negotiator
            .detect_with_preference(&req.port_name, Some(hints), strategy)
            .await
    } else {
        negotiator.detect(&req.port_name, Some(hints)).await
    };

    match params {
        Ok(p) => Json(json!({
            "status": "ok",
            "port_name": req.port_name,
            "baud_rate": p.baud_rate,
            "data_bits": format!("{:?}", p.data_bits).to_lowercase(),
            "parity": format!("{:?}", p.parity).to_lowercase(),
            "stop_bits": format!("{:?}", p.stop_bits).to_lowercase(),
            "flow_control": format!("{:?}", p.flow_control).to_lowercase(),
            "strategy_used": p.strategy_used,
            "confidence": p.confidence
        })),
        Err(e) => Json(err_json("DetectionFailed", &e.to_string())),
    }
}

#[cfg(feature = "auto-negotiation")]
async fn open_port_auto(
    AxumState(ctx): AxumState<RestContext>,
    Json(req): Json<OpenPortAutoRequest>,
) -> Json<Value> {
    use crate::negotiation::{AutoNegotiator, NegotiationHints};

    // Check if port is already open
    {
        let st = ctx.state.lock().unwrap();
        if matches!(&*st, PortState::Open { .. }) {
            return Json(err_json("PortAlreadyOpen", "Port already open"));
        }
    }

    // Build hints for auto-detection
    let mut hints = NegotiationHints {
        timeout_ms: req.timeout_ms,
        ..Default::default()
    };

    if let Some(vid_str) = &req.vid {
        match u16::from_str_radix(vid_str.trim_start_matches("0x"), 16) {
            Ok(vid) => hints.vid = Some(vid),
            Err(e) => return Json(err_json("InvalidVID", &e.to_string())),
        }
    }
    if let Some(pid_str) = &req.pid {
        match u16::from_str_radix(pid_str.trim_start_matches("0x"), 16) {
            Ok(pid) => hints.pid = Some(pid),
            Err(e) => return Json(err_json("InvalidPID", &e.to_string())),
        }
    }
    hints.manufacturer = req.manufacturer.clone();

    // Auto-detect parameters
    let negotiator = AutoNegotiator::new();
    let params = match negotiator.detect(&req.port_name, Some(hints)).await {
        Ok(p) => p,
        Err(e) => return Json(err_json("DetectionFailed", &e.to_string())),
    };

    // Open the port with detected parameters
    let config = PortConfiguration {
        baud_rate: params.baud_rate,
        timeout: Duration::from_millis(req.timeout_ms),
        data_bits: params.data_bits,
        parity: params.parity,
        stop_bits: params.stop_bits,
        flow_control: params.flow_control,
    };

    match SyncSerialPort::open(&req.port_name, config) {
        Ok(port) => {
            let mut st = ctx.state.lock().unwrap();
            *st = PortState::Open {
                port: Box::new(port),
                config: PortConfig {
                    port_name: req.port_name.clone(),
                    baud_rate: params.baud_rate,
                    timeout_ms: req.timeout_ms,
                    data_bits: match params.data_bits {
                        crate::port::DataBits::Five => DataBitsCfg::Five,
                        crate::port::DataBits::Six => DataBitsCfg::Six,
                        crate::port::DataBits::Seven => DataBitsCfg::Seven,
                        crate::port::DataBits::Eight => DataBitsCfg::Eight,
                    },
                    parity: match params.parity {
                        crate::port::Parity::None => ParityCfg::None,
                        crate::port::Parity::Odd => ParityCfg::Odd,
                        crate::port::Parity::Even => ParityCfg::Even,
                    },
                    stop_bits: match params.stop_bits {
                        crate::port::StopBits::One => StopBitsCfg::One,
                        crate::port::StopBits::Two => StopBitsCfg::Two,
                    },
                    flow_control: match params.flow_control {
                        crate::port::FlowControl::None => FlowControlCfg::None,
                        crate::port::FlowControl::Hardware => FlowControlCfg::Hardware,
                        crate::port::FlowControl::Software => FlowControlCfg::Software,
                    },
                    terminator: req.terminator,
                    idle_disconnect_ms: req.idle_disconnect_ms,
                },
                last_activity: std::time::Instant::now(),
                timeout_streak: 0,
                bytes_read_total: 0,
                bytes_written_total: 0,
                idle_close_count: 0,
                open_started: std::time::Instant::now(),
            };
            Json(json!({
                "status": "ok",
                "message": "opened (auto-detected)",
                "port_name": req.port_name,
                "baud_rate": params.baud_rate,
                "strategy_used": params.strategy_used,
                "confidence": params.confidence
            }))
        }
        Err(e) => Json(err_json("OpenError", &e.to_string())),
    }
}

#[cfg(feature = "auto-negotiation")]
async fn list_manufacturer_profiles(AxumState(_ctx): AxumState<RestContext>) -> Json<Value> {
    use crate::negotiation::AutoNegotiator;

    let profiles = AutoNegotiator::all_manufacturer_profiles();
    let profile_list: Vec<_> = profiles
        .iter()
        .map(|p| {
            json!({
                "vid": format!("0x{:04X}", p.vid),
                "name": p.name,
                "default_baud": p.default_baud,
                "common_bauds": p.common_bauds,
            })
        })
        .collect();

    Json(json!({
        "status": "ok",
        "profiles": profile_list,
        "count": profiles.len()
    }))
}

// ---------- Helpers ----------
fn err_json(kind: &str, msg: &str) -> Value {
    json!({"status":"error","error":{"type":kind,"message":msg}})
}
