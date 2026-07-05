use std::path::PathBuf;

use crate::embedded;

/// 应用配置
pub struct AppConfig {
    /// iperf3 可执行文件路径
    pub iperf3_path: String,
    /// 服务端监听端口 (iperf3)
    pub server_port: u16,
    /// 服务端 Web 管理端口
    pub server_web_port: u16,
    /// 客户端 Web 管理端口
    pub client_web_port: u16,
    /// 数据目录
    pub data_dir: PathBuf,
    /// 报告目录
    pub reports_dir: PathBuf,
    /// 历史记录文件
    pub history_file: PathBuf,
    /// 默认测试时长 (秒)
    pub default_duration: u32,
    /// 默认并行流数
    pub default_parallel: u32,
    /// 默认协议
    pub default_protocol: String,
    /// 默认 UDP 带宽
    pub default_udp_bandwidth: String,
    /// 客户端绑定地址（多网卡时指定用哪个 IP 发起连接）
    pub client_addr: Option<String>,
}

impl AppConfig {
    pub fn new() -> Self {
        let base_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        let data_dir = base_dir.join("data");
        let reports_dir = base_dir.join("static").join("reports");
        let history_file = data_dir.join("history.json");

        // 确保目录存在
        std::fs::create_dir_all(&data_dir).ok();
        std::fs::create_dir_all(&reports_dir).ok();

        // 提取嵌入的二进制文件到 data/bin/
        let bin_dir = data_dir.join("bin");
        embedded::extract_binaries(&bin_dir);

        // 解析 iperf3 路径
        let iperf3_path = Self::resolve_iperf3_path(&base_dir, &bin_dir);

        AppConfig {
            iperf3_path,
            server_port: 5201,
            server_web_port: 5002,
            client_web_port: 5001,
            data_dir,
            reports_dir,
            history_file,
            default_duration: 10,
            default_parallel: 4,
            default_protocol: "tcp".to_string(),
            default_udp_bandwidth: "100M".to_string(),
            client_addr: None,
        }
    }

    fn resolve_iperf3_path(base_dir: &PathBuf, bin_dir: &PathBuf) -> String {
        let exe_suffix = if cfg!(target_os = "windows") { ".exe" } else { "" };
        let cmd = format!("iperf3{}", exe_suffix);

        // 1. 优先使用提取的嵌入二进制
        let extracted = bin_dir.join(&cmd);
        if extracted.exists() {
            return extracted.to_string_lossy().to_string();
        }

        // 2. 检查系统 PATH
        if let Ok(path) = std::process::Command::new(&cmd).arg("--version").output() {
            if path.status.success() {
                return cmd;
            }
        }

        // 3. 检查项目内置目录（编译时未嵌入，但用户手动放置）
        let bundled = base_dir.join("bin").join(&cmd);
        if bundled.exists() {
            return bundled.to_string_lossy().to_string();
        }

        // 4. 返回默认值（让调用者给出错误提示）
        cmd
    }
}

/// 无线网卡名称（按系统）
pub fn wifi_interface_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "en0",
        "windows" => "Wi-Fi",
        "linux" => "wlan0",
        _ => "en0",
    }
}
