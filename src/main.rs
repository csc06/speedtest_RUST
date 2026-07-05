mod api_routes;
mod config;
mod embedded;
mod iperf_controller;
mod network_manager;
mod report_generator;
mod test_history;

use std::sync::Arc;
use std::path::PathBuf;

use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use clap::Parser;
use tracing_subscriber;

use api_routes::{AppState, create_routes};
use config::AppConfig;
use iperf_controller::IperfController;
use report_generator::ReportGenerator;
use test_history::TestHistory;

/// SpeedTest 网络链路速度测试工具 (Rust 重写版)
#[derive(Parser, Debug)]
#[command(name = "speedtest")]
#[command(about = "基于 iperf3 的局域网链路质量检测工具", long_about = None)]
struct Cli {
    /// 运行模式: client 或 server
    #[arg(default_value = "client")]
    mode: String,

    /// Web 服务端口
    #[arg(short, long, default_value = "0")]
    port: u16,

    /// iperf3 服务端口
    #[arg(short = 'P', long = "iperf-port", default_value = "0")]
    iperf_port: u16,

    /// 是否禁用自动打开浏览器
    #[arg(long, default_value = "false")]
    no_browser: bool,

    /// 调试模式
    #[arg(short, long, default_value = "false")]
    debug: bool,

    /// 客户端绑定地址（多网卡时指定用哪个 IP 发起 iperf3 连接）
    #[arg(long = "client-addr")]
    client_addr: Option<String>,
}

/// 获取可执行文件所在目录
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 获取前端文件目录
fn frontend_dir() -> PathBuf {
    // 优先使用可执行文件同目录下的 frontend
    let path = exe_dir().join("frontend");
    if path.exists() {
        return path;
    }
    // 回退到项目源码目录
    let src_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("frontend");
    if src_path.exists() {
        return src_path;
    }
    path
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // 初始化配置
    let mut app_config = AppConfig::new();
    if let Some(ref addr) = cli.client_addr {
        app_config.client_addr = Some(addr.clone());
        println!("  📍 客户端绑定地址: {}", addr);
    }
    let iperf3_path = app_config.iperf3_path.clone();
    let history_file = app_config.history_file.clone();
    let reports_dir = app_config.reports_dir.clone();

    // 创建共享状态
    let state = Arc::new(AppState {
        config: app_config,
        iperf: IperfController::new(&iperf3_path),
        history: TestHistory::new(history_file),
        report_gen: ReportGenerator::new(reports_dir),
        test_result: tokio::sync::Mutex::new(None),
        client_addr: tokio::sync::Mutex::new(cli.client_addr.clone()),
    });

    match cli.mode.as_str() {
        "server" | "s" => run_server_mode(state, cli).await,
        "client" | "c" | _ => run_client_mode(state, cli).await,
    }
}

/// 服务端模式
async fn run_server_mode(state: Arc<AppState>, cli: Cli) {
    let web_port = if cli.port > 0 { cli.port } else { state.config.server_web_port };
    let iperf_port = if cli.iperf_port > 0 { cli.iperf_port } else { state.config.server_port };

    println!("{}", "=".repeat(56));
    println!("  🖥️  SpeedTest 网络链路速度测试 - 服务端");
    println!("  ========================================");
    println!("  📡 iperf3 端口: {}", iperf_port);
    println!("  🌐 Web 管理: http://0.0.0.0:{}", web_port);
    println!("  ⚠️  请确保防火墙允许端口 {} 和 {}", iperf_port, web_port);
    println!("{}", "=".repeat(56));
    println!();

    // 启动 iperf3 服务端
    println!("  ⏳ 正在启动 iperf3 服务端 (端口: {})...", iperf_port);
    match state.iperf.start_server(iperf_port) {
        Ok(()) => {
            println!("  ✅ iperf3 服务端已启动，监听端口: {}", iperf_port);
            println!("  📡 等待客户端连接...");
        }
        Err(e) => {
            eprintln!("  ❌ 启动 iperf3 服务端失败: {}", e);
            eprintln!("  ⚠️  请确保已安装 iperf3 并可在 PATH 中访问");
        }
    }

    // 启动 Web 服务
    start_web_server(state, web_port).await;
}

/// 客户端模式
async fn run_client_mode(state: Arc<AppState>, cli: Cli) {
    let web_port = if cli.port > 0 { cli.port } else { state.config.client_web_port };
    let iperf_port = if cli.iperf_port > 0 { cli.iperf_port } else { state.config.server_port };

    println!("{}", "=".repeat(56));
    println!("  🚀 SpeedTest 网络链路速度测试");
    println!("  ========================================");
    println!("  📡 本机 iperf3 服务端: 端口 {}", iperf_port);
    println!("  🌐 Web 界面: http://127.0.0.1:{}", web_port);
    println!("  ⚠️  启动后将自动检测无线网卡状态");
    println!("  🔒 禁止互联网连接环境专用");
    println!("  💡 在 Web 界面修改「服务端地址」可连接其他机器");
    println!("{}", "=".repeat(56));
    println!();

    // 检查 iperf3 是否可用
    let version = state.iperf.iperf3_version();
    if version == "未知" {
        println!("  ⚠️  未检测到 iperf3，请确保已安装并可在 PATH 中访问");
    } else {
        println!("  ✅ iperf3 版本: {}", version);
    }

    // 启动本机 iperf3 服务端（方便直接测试或作为被测试端）
    println!("  ⏳ 正在启动 iperf3 服务端 (端口: {})...", iperf_port);
    match state.iperf.start_server(iperf_port) {
        Ok(()) => {
            println!("  ✅ 本机 iperf3 服务端已启动，监听端口: {}", iperf_port);
        }
        Err(e) => {
            eprintln!("  ⚠️  启动本机 iperf3 服务端失败: {}", e);
            eprintln!("  ⚠️  但仍可作为客户端连接其他服务端");
        }
    }

    // 自动打开浏览器（如果不是 --no-browser）
    if !cli.no_browser {
        let url = format!("http://127.0.0.1:{}", web_port);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            println!("  🌐 正在打开浏览器: {}", url);
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(&url).spawn();
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("cmd").args(["/c", "start", &url]).spawn();
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
        });
    }

    // 启动 Web 服务
    start_web_server(state, web_port).await;
}

/// 启动 Web 服务器（含优雅关闭，双击 Dock 退出时清理 iperf3）
async fn start_web_server(state: Arc<AppState>, port: u16) {
    // 创建 API 路由
    let api_routes = create_routes(state.clone());

    // 前端文件目录（若有则优先文件系统）
    let frontend = frontend_dir();

    // 构建应用
    let app = Router::new()
        // API 路由优先
        .merge(api_routes)
        // 首页
        .route("/", get(index_handler))
        // 静态文件（使用完整路径，/*path 捕获后缀）
        .route("/css/*path", get(serve_css_handler))
        .route("/js/*path", get(serve_js_handler))
        .route("/lib/*path", get(serve_lib_handler));

    let addr = format!("0.0.0.0:{}", port);
    println!("  🌐 SpeedTest Web 界面已启动: http://127.0.0.1:{}", port);
    println!("  📡 API 基础路径: http://127.0.0.1:{}/api/", port);
    if embedded::has_embedded_frontend() {
        println!("  📦 前端和二进制文件已嵌入，单一文件开箱即用");
    } else {
        println!("  📁 使用外部前端文件: {}", frontend.display());
    }
    println!();

    // 优雅关闭：捕获 SIGTERM（Dock Quit）和 SIGINT（Ctrl+C）
    let state_for_shutdown = state.clone();
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            // 等待退出信号
            wait_for_exit_signal().await;
            // 清理 iperf3 服务端
            state_for_shutdown.iperf.stop_server();
            println!("  👋 SpeedTest 已关闭");
        })
        .await
        .unwrap();
}

#[cfg(unix)]
async fn wait_for_exit_signal() {
    use tokio::signal::unix;
    let mut sigterm = unix::signal(unix::SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = sigterm.recv() => {},
    }
}

#[cfg(not(unix))]
async fn wait_for_exit_signal() {
    tokio::signal::ctrl_c().await.unwrap();
}

/// 首页处理器
async fn index_handler() -> Response {
    serve_static_file("index.html", true).await
}

/// CSS 静态文件
async fn serve_css_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    serve_static_file(&format!("css/{}", path), true).await
}

/// JS 静态文件
async fn serve_js_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    serve_static_file(&format!("js/{}", path), true).await
}

/// Lib 静态文件
async fn serve_lib_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    serve_static_file(&format!("lib/{}", path), true).await
}

/// 统一静态文件服务逻辑
/// prefix_required: 为 true 时路径必须包含前缀（如 css/），false 时为 index.html 等
async fn serve_static_file(relative_path: &str, _prefix_required: bool) -> Response {
    let frontend = frontend_dir();

    // 1. 尝试文件系统
    let fs_path = frontend.join(relative_path);
    if fs_path.exists() && fs_path.is_file() {
        match std::fs::read(&fs_path) {
            Ok(data) => {
                let mime = mime_type(relative_path);
                return Response::builder()
                    .header(header::CONTENT_TYPE, mime)
                    .body(Body::from(data))
                    .unwrap()
                    .into_response();
            }
            Err(_) => {}
        }
    }

    // 2. 尝试嵌入的数据
    match embedded::get_frontend_file(relative_path) {
        Some(file) => {
            let mime = mime_type(relative_path);
            Response::builder()
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(file.data.to_vec()))
                .unwrap()
                .into_response()
        }
        None => {
            (StatusCode::NOT_FOUND, "File not found").into_response()
        }
    }
}

/// 获取 MIME 类型
fn mime_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}
