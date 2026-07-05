use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

use crate::config::AppConfig;
use crate::iperf_controller::{IperfController, TestResult};
use crate::network_manager::NetworkManager;
use crate::report_generator::ReportGenerator;
use crate::test_history::TestHistory;

/// 应用共享状态
pub struct AppState {
    pub config: AppConfig,
    pub iperf: IperfController,
    pub history: TestHistory,
    pub report_gen: ReportGenerator,
    pub test_result: Mutex<Option<TestResult>>,
    pub client_addr: Mutex<Option<String>>,
}

/// 测试启动参数
#[derive(Debug, Deserialize)]
pub struct StartTestParams {
    pub server_host: Option<String>,
    pub server_port: Option<u16>,
    pub duration: Option<u32>,
    pub parallel: Option<u32>,
    pub protocol: Option<String>,
    pub reverse: Option<bool>,
    pub bidirectional: Option<bool>,
    pub udp_bandwidth: Option<String>,
    pub client_addr: Option<String>,
}

/// 服务端启动参数
#[derive(Debug, Deserialize)]
pub struct ServerStartParams {
    pub port: Option<u16>,
}

/// 报告生成参数
#[derive(Debug, Deserialize)]
pub struct ReportGenerateParams {
    pub record_id: Option<String>,
}

/// 通用 API 响应
#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

/// 创建 API 路由
pub fn create_routes(state: Arc<AppState>) -> Router {
    Router::new()
        // 系统状态
        .route("/api/status", get(get_status_handler))
        .route("/api/config", get(get_config_handler))
        .route("/api/config/client-addr", post(set_client_addr_handler))
        .route("/api/check/server", get(check_server_handler))

        // 服务端管理
        .route("/api/server/start", post(start_server_handler))
        .route("/api/server/stop", post(stop_server_handler))
        .route("/api/server/status", get(get_server_status_handler))

        // 测试控制
        .route("/api/test/start", post(start_test_handler))
        .route("/api/test/stop", post(stop_test_handler))
        .route("/api/test/result", get(get_test_result_handler))

        // 网络管理
        .route("/api/network/wifi", get(get_wifi_status_handler))
        .route("/api/network/wifi/disable", post(disable_wifi_handler))
        .route("/api/network/wifi/enable", post(enable_wifi_handler))

        // 历史记录
        .route("/api/history", get(get_history_handler))
        .route("/api/history/statistics", get(get_history_statistics_handler))
        .route("/api/history/clear", post(clear_history_handler))
        .route("/api/history/:id", get(get_history_detail_handler).delete(delete_history_handler))

        // 报告管理
        .route("/api/report/generate", post(generate_report_handler))
        .route("/api/report/list", get(list_reports_handler))
        .route("/api/report/download/:filename", get(download_report_handler))

        .layer(CorsLayer::permissive())
        .with_state(state)
}

// ==================== 处理器 ====================

/// 获取系统状态
async fn get_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let wifi = NetworkManager::get_wifi_status();
    let client_addr = state.client_addr.lock().await.clone();
    Json(serde_json::json!({
        "system": std::env::consts::OS,
        "wifi": wifi,
        "iperf_running": state.iperf.is_server_running(),
        "test_in_progress": false,
        "client_addr": client_addr,
    }))
}

/// 获取配置
async fn get_config_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let client_addr = state.client_addr.lock().await.clone();
    Json(serde_json::json!({
        "default_duration": state.config.default_duration,
        "default_parallel": state.config.default_parallel,
        "default_protocol": state.config.default_protocol,
        "default_port": state.config.server_port,
        "client_addr": client_addr,
    }))
}

/// 设置客户端绑定地址
#[derive(Debug, Deserialize)]
pub struct SetClientAddrParams {
    pub addr: Option<String>,
}

async fn set_client_addr_handler(
    State(state): State<Arc<AppState>>,
    Json(params): Json<SetClientAddrParams>,
) -> Json<serde_json::Value> {
    let mut guard = state.client_addr.lock().await;
    *guard = params.addr.clone();
    Json(serde_json::json!({
        "success": true,
        "client_addr": *guard,
    }))
}

/// 检查服务端可达性
#[derive(Debug, Deserialize)]
pub struct CheckServerQuery {
    pub host: Option<String>,
    pub port: Option<u16>,
}

async fn check_server_handler(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<CheckServerQuery>,
) -> Json<serde_json::Value> {
    let host = query.host.unwrap_or_else(|| "127.0.0.1".to_string());
    let port = query.port.unwrap_or(5201);

    let online = std::net::TcpStream::connect_timeout(
        &format!("{}:{}", host, port).parse().unwrap_or(std::net::SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, 5201).into()),
        std::time::Duration::from_secs(3),
    )
    .is_ok();

    Json(serde_json::json!({
        "online": online,
        "host": host,
        "port": port,
        "latency_ms": null,
    }))
}

/// 启动服务端
async fn start_server_handler(
    State(state): State<Arc<AppState>>,
    Json(params): Json<ServerStartParams>,
) -> Json<serde_json::Value> {
    if state.iperf.is_server_running() {
        return Json(serde_json::json!({
            "success": false,
            "message": "服务端已在运行"
        }));
    }

    let port = params.port.unwrap_or(state.config.server_port);
    match state.iperf.start_server(port) {
        Ok(()) => Json(serde_json::json!({
            "success": true,
            "message": format!("服务端已启动 (端口: {})", port)
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "message": format!("启动失败: {}", e)
        })),
    }
}

/// 停止服务端
async fn stop_server_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    state.iperf.stop_server();
    Json(serde_json::json!({
        "success": true,
        "message": "服务端已停止"
    }))
}

/// 获取服务端状态
async fn get_server_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "running": state.iperf.is_server_running(),
    }))
}

/// 启动测试
async fn start_test_handler(
    State(state): State<Arc<AppState>>,
    Json(params): Json<StartTestParams>,
) -> Json<serde_json::Value> {
    let server_host = params.server_host.unwrap_or_else(|| "127.0.0.1".to_string());
    let server_port = params.server_port.unwrap_or(state.config.server_port);
    let duration = params.duration.unwrap_or(state.config.default_duration);
    let parallel = params.parallel.unwrap_or(state.config.default_parallel);
    let protocol = params.protocol.unwrap_or_else(|| state.config.default_protocol.clone());
    let reverse = params.reverse.unwrap_or(false);
    let bidirectional = params.bidirectional.unwrap_or(false);
    let udp_bandwidth = params.udp_bandwidth.unwrap_or_else(|| state.config.default_udp_bandwidth.clone());
    // 客户端绑定地址：优先用请求参数，其次用服务端保存的配置
    let saved_addr = state.client_addr.lock().await.clone();
    let client_addr = params.client_addr.clone().or(saved_addr);

    // 清空上次测试结果，避免前端读到旧数据
    {
        let mut last_result = state.test_result.lock().await;
        *last_result = None;
    }

    // 在新线程中运行测试
    let state_clone = state.clone();
    tokio::spawn(async move {
        let result = if bidirectional {
            state_clone.iperf.run_bidirectional_test(
                &server_host, server_port, duration, parallel,
                &protocol, &udp_bandwidth, client_addr.as_deref(),
            )
        } else {
            state_clone.iperf.run_test(
                &server_host, server_port, duration, parallel,
                &protocol, reverse, &udp_bandwidth, client_addr.as_deref(),
            )
        };

        // 保存结果
        {
            let mut test_result = state_clone.test_result.lock().await;
            *test_result = Some(result.clone());
        }

        // 保存历史记录
        if result.status == "completed" || result.status == "stopped" {
            state_clone.history.add_record(result);
        }
    });

    Json(serde_json::json!({
        "success": true,
        "message": "测试已启动"
    }))
}

/// 停止测试
async fn stop_test_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    state.iperf.stop_test();
    Json(serde_json::json!({
        "success": true,
        "message": "测试已停止"
    }))
}

/// 获取测试结果
async fn get_test_result_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let test_result = state.test_result.lock().await;
    if let Some(ref result) = *test_result {
        Json(serde_json::to_value(result).unwrap_or(serde_json::json!({"status": "error"})))
    } else {
        Json(serde_json::json!({"status": "no_result"}))
    }
}

/// 获取 Wi-Fi 状态
async fn get_wifi_status_handler() -> Json<serde_json::Value> {
    let wifi = NetworkManager::get_wifi_status();
    Json(serde_json::to_value(wifi).unwrap_or_default())
}

/// 禁用 Wi-Fi
async fn disable_wifi_handler() -> Json<serde_json::Value> {
    let success = NetworkManager::disable_wifi();
    Json(serde_json::json!({
        "success": success,
        "message": if success { "无线网卡已禁用" } else { "禁用失败" }
    }))
}

/// 启用 Wi-Fi
async fn enable_wifi_handler() -> Json<serde_json::Value> {
    let success = NetworkManager::enable_wifi();
    Json(serde_json::json!({
        "success": success,
        "message": if success { "无线网卡已启用" } else { "启用失败" }
    }))
}

/// 获取历史记录
async fn get_history_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let records = state.history.get_all(50);
    Json(serde_json::to_value(records).unwrap_or_default())
}

/// 获取历史详情
async fn get_history_detail_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.history.get_by_id(&id) {
        Some(record) => Ok(Json(serde_json::to_value(record).unwrap_or_default())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// 删除历史记录
async fn delete_history_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let success = state.history.delete_record(&id);
    Json(serde_json::json!({ "success": success }))
}

/// 清空历史记录
async fn clear_history_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    state.history.clear_all();
    Json(serde_json::json!({ "success": true }))
}

/// 获取历史统计
async fn get_history_statistics_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let stats = state.history.get_statistics();
    Json(serde_json::to_value(stats).unwrap_or_default())
}

/// 生成报告
async fn generate_report_handler(
    State(state): State<Arc<AppState>>,
    Json(params): Json<ReportGenerateParams>,
) -> Json<serde_json::Value> {
    let result = if let Some(ref record_id) = params.record_id {
        match state.history.get_by_id(record_id) {
            Some(record) => record.result,
            None => {
                return Json(serde_json::json!({
                    "success": false,
                    "message": "记录不存在"
                }));
            }
        }
    } else {
        let test_result = state.test_result.lock().await;
        match test_result.as_ref() {
            Some(result) => result.clone(),
            None => {
                return Json(serde_json::json!({
                    "success": false,
                    "message": "无测试结果"
                }));
            }
        }
    };

    match state.report_gen.generate_html_report(&result, params.record_id.as_deref()) {
        Ok(filepath) => {
            let filename = std::path::Path::new(&filepath)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            Json(serde_json::json!({
                "success": true,
                "message": "报告已生成",
                "filename": filename,
                "path": format!("/api/report/download/{}", filename),
            }))
        }
        Err(e) => Json(serde_json::json!({
            "success": false,
            "message": format!("报告生成失败: {}", e)
        })),
    }
}

/// 列出报告
async fn list_reports_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let reports = state.report_gen.list_reports();
    Json(serde_json::to_value(reports).unwrap_or_default())
}

/// 下载报告
async fn download_report_handler(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> Response {
    // 安全检查：防止路径穿越
    if filename.contains("..") || filename.contains("/") {
        return (StatusCode::BAD_REQUEST, "Invalid filename").into_response();
    }

    match state.report_gen.get_report_path(&filename) {
        Some(filepath) => {
            match std::fs::read_to_string(&filepath) {
                Ok(content) => {
                    Response::builder()
                        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                        .body(axum::body::Body::from(content))
                        .unwrap()
                        .into_response()
                }
                Err(_) => (StatusCode::NOT_FOUND, "Report file not found").into_response(),
            }
        }
        None => (StatusCode::NOT_FOUND, "Report not found").into_response(),
    }
}
