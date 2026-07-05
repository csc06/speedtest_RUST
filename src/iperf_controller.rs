use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
// use std::time::{Duration, Instant};

/// iperf3 测试控制器
pub struct IperfController {
    pub iperf3_path: String,
    server_process: Arc<Mutex<Option<std::process::Child>>>,
    pub is_running: Arc<AtomicBool>,
    test_results: Arc<Mutex<Vec<IntervalResult>>>,
    stop_flag: Arc<AtomicBool>,
    iperf3_version: String,
}

/// 间隔测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalResult {
    pub time_start: f64,
    pub time_end: f64,
    pub seconds: f64,
    pub bits_per_second: f64,
    pub bytes: u64,
    pub retransmits: u32,
    pub lost_packets: u32,
    pub lost_percent: f64,
    pub jitter_ms: f64,
}

/// 测试摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    pub avg_bits_per_second: f64,
    pub max_bits_per_second: f64,
    pub min_bits_per_second: f64,
    pub avg_mbps: f64,
    pub max_mbps: f64,
    pub min_mbps: f64,
}

/// iperf3 最终输出解析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IperfJsonOutput {
    pub start: Option<serde_json::Value>,
    pub intervals: Vec<IperfInterval>,
    pub end: Option<IperfEnd>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IperfInterval {
    pub sum: Option<IperfSum>,
    pub streams: Option<Vec<IperfStream>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IperfSum {
    pub start: f64,
    pub end: f64,
    pub seconds: f64,
    pub bits_per_second: f64,
    pub bytes: u64,
    pub retransmits: Option<u32>,
    pub lost_packets: Option<u32>,
    pub lost_percent: Option<f64>,
    pub jitter_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IperfStream {
    pub socket: Option<i32>,
    pub start: Option<f64>,
    pub end: Option<f64>,
    pub seconds: Option<f64>,
    pub bits_per_second: Option<f64>,
    pub bytes: Option<u64>,
    pub retransmits: Option<u32>,
    pub snd_cwnd: Option<u32>,
    pub rtt: Option<u32>,
    pub lost_packets: Option<u32>,
    pub lost_percent: Option<f64>,
    pub jitter_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IperfEnd {
    pub sum_sent: Option<IperfEndpoint>,
    pub sum_received: Option<IperfEndpoint>,
    pub sum: Option<IperfEndpoint>,
    pub streams: Option<Vec<IperfStreamEnd>>,
    pub cpu_utilization_percent: Option<CpuUtil>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IperfEndpoint {
    pub bits_per_second: f64,
    pub bytes: u64,
    pub retransmits: Option<u32>,
    pub jitter_ms: Option<f64>,
    pub lost_packets: Option<u32>,
    pub lost_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IperfStreamEnd {
    pub sender: Option<IperfEndpoint>,
    pub receiver: Option<IperfEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuUtil {
    pub host_total: f64,
    pub host_user: f64,
    pub host_system: f64,
    pub remote_total: f64,
}

/// 完整测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub status: String,
    pub start_time: String,
    pub end_time: String,
    pub config: TestConfig,
    pub command: String,
    pub iperf3_version: String,
    pub intervals: Vec<IntervalResult>,
    pub summary: TestSummary,
    pub bidirectional: Option<bool>,
    pub upload: Option<Box<TestResult>>,
    pub download: Option<Box<TestResult>>,
    pub command_down: Option<String>,
    pub download_summary: Option<TestSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub client_host: String,
    pub server_host: String,
    pub server_port: u16,
    pub duration: u32,
    pub parallel: u32,
    pub protocol: String,
    pub reverse: bool,
    pub client_addr: Option<String>,
}

impl IperfController {
    pub fn new(iperf3_path: &str) -> Self {
        let version = Self::get_version(iperf3_path);
        IperfController {
            iperf3_path: iperf3_path.to_string(),
            server_process: Arc::new(Mutex::new(None)),
            is_running: Arc::new(AtomicBool::new(false)),
            test_results: Arc::new(Mutex::new(Vec::new())),
            stop_flag: Arc::new(AtomicBool::new(false)),
            iperf3_version: version,
        }
    }

    pub fn iperf3_version(&self) -> &str {
        &self.iperf3_version
    }

    fn get_version(iperf3_path: &str) -> String {
        match Command::new(iperf3_path)
            .arg("--version")
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines().next().unwrap_or("未知").to_string()
            }
            Err(_) => "未知".to_string(),
        }
    }

    /// 启动 iperf3 服务端
    pub fn start_server(&self, port: u16) -> Result<(), String> {
        let proc = Command::new(&self.iperf3_path)
            .args(["-s", "-p", &port.to_string(), "--json"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("启动 iperf3 服务端失败: {}", e))?;

        let mut server = self.server_process.lock().unwrap();
        *server = Some(proc);
        self.is_running.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// 停止 iperf3 服务端
    pub fn stop_server(&self) {
        let mut server = self.server_process.lock().unwrap();
        if let Some(mut proc) = server.take() {
            let _ = proc.kill();
            let _ = proc.wait();
        }
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// 获取服务端是否在运行
    pub fn is_server_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// 运行测试
    pub fn run_test(
        &self,
        server_host: &str,
        server_port: u16,
        duration: u32,
        parallel: u32,
        protocol: &str,
        reverse: bool,
        udp_bandwidth: &str,
        client_addr: Option<&str>,
    ) -> TestResult {
        self.stop_flag.store(false, Ordering::SeqCst);

        // 获取本机 IP（优先用指定的绑定地址，否则自动检测）
        let custom_addr = client_addr.map(|s| s.to_string());
        let client_host = custom_addr.clone()
            .unwrap_or_else(Self::get_local_ip);

        let mut cmd = vec![
            "-c".to_string(),
            server_host.to_string(),
            "-p".to_string(),
            server_port.to_string(),
            "-t".to_string(),
            duration.to_string(),
            "-P".to_string(),
            parallel.to_string(),
            "-i".to_string(),
            "1".to_string(),
            "--json".to_string(),
        ];

        // 指定客户端绑定地址（多网卡时选择用哪个 IP 发起连接）
        if let Some(addr) = client_addr {
            cmd.push("-B".to_string());
            cmd.push(addr.to_string());
        }

        if protocol == "udp" {
            cmd.push("-u".to_string());
            cmd.push("-b".to_string());
            cmd.push(udp_bandwidth.to_string());
        }

        if reverse {
            cmd.push("-R".to_string());
        }

        let command_str = format!("{} {}", self.iperf3_path, cmd.join(" "));
        let start_time = chrono::Local::now().to_rfc3339();

        // 清空之前的结果
        {
            let mut results = self.test_results.lock().unwrap();
            results.clear();
        }

        // 启动 iperf3
        let output = Command::new(&self.iperf3_path)
            .args(&cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        let end_time = chrono::Local::now().to_rfc3339();

        match output {
            Ok(output) => {
                // 检查 iperf3 进程退出码 —— 非零表示连接失败或出错
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let error_msg = if stderr.is_empty() {
                        "iperf3 连接失败（可能是对方未放行端口或服务端未运行）".to_string()
                    } else {
                        // 从 stderr 提取关键错误信息
                        stderr.lines()
                            .find(|l| l.contains("error") || l.contains("Error"))
                            .unwrap_or(&stderr)
                            .trim()
                            .to_string()
                    };
                    return TestResult {
                        status: "failed".to_string(),
                        start_time,
                        end_time,
                        config: TestConfig {
                            client_host,
                            server_host: server_host.to_string(),
                            server_port,
                            duration,
                            parallel,
                            protocol: protocol.to_string(),
                            reverse,
                            client_addr: custom_addr,
                        },
                        command: command_str,
                        iperf3_version: self.iperf3_version.clone(),
                        intervals: vec![],
                        summary: TestSummary {
                            avg_bits_per_second: 0.0,
                            max_bits_per_second: 0.0,
                            min_bits_per_second: 0.0,
                            avg_mbps: 0.0,
                            max_mbps: 0.0,
                            min_mbps: 0.0,
                        },
                        bidirectional: None,
                        upload: None,
                        download: None,
                        command_down: None,
                        download_summary: None,
                    };
                }

                let stdout = String::from_utf8_lossy(&output.stdout);

                // 解析 JSON 输出
                if let Ok(json_output) = serde_json::from_str::<IperfJsonOutput>(&stdout) {
                    self.parse_iperf_output(&json_output);
                }

                let intervals = self.test_results.lock().unwrap().clone();
                let summary = self.calculate_summary(&intervals);

                TestResult {
                    status: if self.stop_flag.load(Ordering::SeqCst) {
                        "stopped".to_string()
                    } else {
                        "completed".to_string()
                    },
                    start_time,
                    end_time,
                    config: TestConfig {
                        client_host,
                        server_host: server_host.to_string(),
                        server_port,
                        duration,
                        parallel,
                        protocol: protocol.to_string(),
                        reverse,
                        client_addr: custom_addr,
                    },
                    command: command_str,
                    iperf3_version: self.iperf3_version.clone(),
                    intervals,
                    summary,
                    bidirectional: None,
                    upload: None,
                    download: None,
                    command_down: None,
                    download_summary: None,
                }
            }
            Err(_e) => TestResult {
                status: "failed".to_string(),
                start_time,
                end_time,
                config: TestConfig {
                    client_host,
                    server_host: server_host.to_string(),
                    server_port,
                    duration,
                    parallel,
                    protocol: protocol.to_string(),
                    reverse,
                    client_addr: custom_addr,
                },
                command: command_str,
                iperf3_version: self.iperf3_version.clone(),
                intervals: vec![],
                summary: TestSummary {
                    avg_bits_per_second: 0.0,
                    max_bits_per_second: 0.0,
                    min_bits_per_second: 0.0,
                    avg_mbps: 0.0,
                    max_mbps: 0.0,
                    min_mbps: 0.0,
                },
                bidirectional: None,
                upload: None,
                download: None,
                command_down: None,
                download_summary: None,
            },
        }
    }

    /// 运行双向测试（先上传后下载）
    pub fn run_bidirectional_test(
        &self,
        server_host: &str,
        server_port: u16,
        duration: u32,
        parallel: u32,
        protocol: &str,
        udp_bandwidth: &str,
        client_addr: Option<&str>,
    ) -> TestResult {
        // 上传测试
        let up_result = self.run_test(
            server_host, server_port, duration, parallel,
            protocol, false, udp_bandwidth, client_addr,
        );

        // 下载测试
        let down_result = self.run_test(
            server_host, server_port, duration, parallel,
            protocol, true, udp_bandwidth, client_addr,
        );

        let up_summary = up_result.summary.clone();
        let down_summary = down_result.summary.clone();

        // 合并摘要：上传 + 下载的带宽合并统计
        let combined_bits = {
            let mut all = Vec::new();
            for item in &up_result.intervals {
                if item.bits_per_second > 0.0 {
                    all.push(item.bits_per_second);
                }
            }
            for item in &down_result.intervals {
                if item.bits_per_second > 0.0 {
                    all.push(item.bits_per_second);
                }
            }
            all
        };

        let combined_summary = if combined_bits.is_empty() {
            up_summary.clone()
        } else {
            let avg = combined_bits.iter().sum::<f64>() / combined_bits.len() as f64;
            let max = combined_bits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let min = combined_bits.iter().cloned().fold(f64::INFINITY, f64::min);
            TestSummary {
                avg_bits_per_second: avg,
                max_bits_per_second: max,
                min_bits_per_second: min,
                avg_mbps: (avg / 1_000_000.0 * 100.0).round() / 100.0,
                max_mbps: (max / 1_000_000.0 * 100.0).round() / 100.0,
                min_mbps: (min / 1_000_000.0 * 100.0).round() / 100.0,
            }
        };

        // 合并结果
        TestResult {
            status: if up_result.status == "completed" || down_result.status == "completed" {
                "completed".to_string()
            } else {
                "failed".to_string()
            },
            start_time: up_result.start_time.clone(),
            end_time: down_result.end_time.clone(),
            config: up_result.config.clone(),
            command: up_result.command.clone(),
            command_down: Some(down_result.command.clone()),
            iperf3_version: self.iperf3_version.clone(),
            intervals: up_result.intervals.clone(),
            summary: combined_summary,
            bidirectional: Some(true),
            upload: Some(Box::new(up_result)),
            download: Some(Box::new(down_result)),
            download_summary: Some(down_summary),
        }
    }

    /// 停止测试
    pub fn stop_test(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }

    fn get_local_ip() -> String {
        match std::process::Command::new("hostname")
            .output()
        {
            Ok(output) => {
                let hostname = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // 尝试 DNS 解析
                if let Ok(ip) = std::net::ToSocketAddrs::to_socket_addrs(
                    &(hostname + ":0"),
                ) {
                    if let Some(addr) = ip.filter(|a| a.is_ipv4()).next() {
                        return addr.ip().to_string();
                    }
                }
                "127.0.0.1".to_string()
            }
            Err(_) => "127.0.0.1".to_string(),
        }
    }

    fn parse_iperf_output(&self, data: &IperfJsonOutput) {
        let mut results = self.test_results.lock().unwrap();
        results.clear();

        for interval in &data.intervals {
            if let Some(ref sum) = interval.sum {
                results.push(IntervalResult {
                    time_start: sum.start,
                    time_end: sum.end,
                    seconds: sum.seconds,
                    bits_per_second: sum.bits_per_second,
                    bytes: sum.bytes,
                    retransmits: sum.retransmits.unwrap_or(0),
                    lost_packets: sum.lost_packets.unwrap_or(0),
                    lost_percent: sum.lost_percent.unwrap_or(0.0),
                    jitter_ms: sum.jitter_ms.unwrap_or(0.0),
                });
            } else if let Some(ref streams) = interval.streams {
                for stream in streams {
                    results.push(IntervalResult {
                        time_start: stream.start.unwrap_or(0.0),
                        time_end: stream.end.unwrap_or(0.0),
                        seconds: stream.seconds.unwrap_or(0.0),
                        bits_per_second: stream.bits_per_second.unwrap_or(0.0),
                        bytes: stream.bytes.unwrap_or(0),
                        retransmits: stream.retransmits.unwrap_or(0),
                        lost_packets: stream.lost_packets.unwrap_or(0),
                        lost_percent: stream.lost_percent.unwrap_or(0.0),
                        jitter_ms: stream.jitter_ms.unwrap_or(0.0),
                    });
                }
            }
        }
    }

    fn calculate_summary(&self, intervals: &[IntervalResult]) -> TestSummary {
        let bits: Vec<f64> = intervals
            .iter()
            .map(|r| r.bits_per_second)
            .filter(|&b| b > 0.0)
            .collect();

        if bits.is_empty() {
            return TestSummary {
                avg_bits_per_second: 0.0,
                max_bits_per_second: 0.0,
                min_bits_per_second: 0.0,
                avg_mbps: 0.0,
                max_mbps: 0.0,
                min_mbps: 0.0,
            };
        }

        let sum: f64 = bits.iter().sum();
        let avg = sum / bits.len() as f64;
        let max = bits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = bits.iter().cloned().fold(f64::INFINITY, f64::min);

        TestSummary {
            avg_bits_per_second: avg,
            max_bits_per_second: max,
            min_bits_per_second: min,
            avg_mbps: (avg / 1_000_000.0 * 100.0).round() / 100.0,
            max_mbps: (max / 1_000_000.0 * 100.0).round() / 100.0,
            min_mbps: (min / 1_000_000.0 * 100.0).round() / 100.0,
        }
    }

    pub fn get_test_status(&self) -> serde_json::Value {
        let intervals_count = {
            let results = self.test_results.lock().unwrap();
            results.len()
        };

        let latest_bps = {
            let results = self.test_results.lock().unwrap();
            results.last().map(|r| r.bits_per_second).unwrap_or(0.0)
        };

        serde_json::json!({
            "running": self.stop_flag.load(Ordering::SeqCst),
            "intervals_count": intervals_count,
            "latest_bits_per_second": latest_bps,
            "latest_speed_mbps": (latest_bps / 1_000_000.0 * 100.0).round() / 100.0,
        })
    }
}
