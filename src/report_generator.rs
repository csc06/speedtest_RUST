use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::iperf_controller::TestResult;

/// 格式化字节数为人类可读形式
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.2} GBytes", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.2} MBytes", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.2} KBytes", bytes as f64 / 1_000.0)
    } else {
        format!("{} Bytes", bytes)
    }
}

/// 格式化比特率为人类可读形式
fn format_bitrate(bps: f64) -> String {
    if bps >= 1_000_000_000.0 {
        format!("{:.2} Gbps", bps / 1_000_000_000.0)
    } else if bps >= 1_000_000.0 {
        format!("{:.2} Mbps", bps / 1_000_000.0)
    } else if bps >= 1_000.0 {
        format!("{:.2} Kbps", bps / 1_000.0)
    } else {
        format!("{:.2} bps", bps)
    }
}

/// 报告生成器
pub struct ReportGenerator {
    reports_dir: PathBuf,
}

/// 报告信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportInfo {
    pub filename: String,
    pub path: String,
    pub size: u64,
    pub created: String,
}

impl ReportGenerator {
    pub fn new(reports_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&reports_dir).ok();
        ReportGenerator { reports_dir }
    }

    /// 生成 HTML 测试报告
    pub fn generate_html_report(&self, result: &TestResult, record_id: Option<&str>) -> Result<String, String> {
        let summary = &result.summary;
        let config = &result.config;
        let intervals = &result.intervals;
        let is_bidirectional = result.bidirectional.unwrap_or(false);
        let upload = result.upload.as_ref();
        let download = result.download.as_ref();

        // 上下行数据
        let up_summary = upload.map(|u| &u.summary);
        let down_summary = download.map(|d| &d.summary);
        let up_avg = up_summary.map(|s| s.avg_mbps).unwrap_or(0.0);
        let down_avg = down_summary.map(|s| s.avg_mbps).unwrap_or(0.0);
        let up_intervals = upload.map(|u| &u.intervals).cloned().unwrap_or_default();
        let down_intervals = download.map(|d| &d.intervals).cloned().unwrap_or_default();

        let avg_mbps = summary.avg_mbps;
        let max_mbps = summary.max_mbps;
        let min_mbps = summary.min_mbps;

        // 格式化速度
        let format_speed = |bps: f64| -> String {
            if bps >= 1_000_000_000.0 {
                format!("{:.2} Gbps", bps / 1_000_000_000.0)
            } else if bps >= 1_000_000.0 {
                format!("{:.2} Mbps", bps / 1_000_000.0)
            } else if bps >= 1_000.0 {
                format!("{:.2} Kbps", bps / 1_000.0)
            } else {
                format!("{:.2} bps", bps)
            }
        };

        // 间隔数据 JSON
        let interval_data: Vec<serde_json::Value> = intervals.iter().map(|item| {
            serde_json::json!({
                "time": item.time_end,
                "bits_per_second": item.bits_per_second,
                "retransmits": item.retransmits,
                "jitter_ms": item.jitter_ms,
                "lost_percent": item.lost_percent,
            })
        }).collect();

        let up_interval_data: Vec<serde_json::Value> = up_intervals.iter().map(|item| {
            serde_json::json!({
                "time": item.time_end,
                "bits_per_second": item.bits_per_second,
            })
        }).collect();

        let down_interval_data: Vec<serde_json::Value> = down_intervals.iter().map(|item| {
            serde_json::json!({
                "time": item.time_end,
                "bits_per_second": item.bits_per_second,
            })
        }).collect();

        let intervals_json = serde_json::to_string(&interval_data).unwrap_or_default();
        let up_intervals_json = serde_json::to_string(&up_interval_data).unwrap_or_default();
        let down_intervals_json = serde_json::to_string(&down_interval_data).unwrap_or_default();

        let protocol = config.protocol.to_uppercase();
        let test_type = if is_bidirectional {
            "上下行测试"
        } else if config.reverse {
            "下载测试"
        } else {
            "上传测试"
        };

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let summary_cards = if is_bidirectional {
            format!(r#"
            <div class="card">
                <div class="label">上传平均带宽</div>
                <div class="value good">{:.2}<span class="unit">Mbps</span></div>
            </div>
            <div class="card">
                <div class="label">下载平均带宽</div>
                <div class="value good">{:.2}<span class="unit">Mbps</span></div>
            </div>
            <div class="card">
                <div class="label">上传最高</div>
                <div class="value">{:.2}<span class="unit">Mbps</span></div>
            </div>
            <div class="card">
                <div class="label">下载最高</div>
                <div class="value">{:.2}<span class="unit">Mbps</span></div>
            </div>"#,
                up_avg, down_avg,
                up_summary.map(|s| s.max_mbps).unwrap_or(0.0),
                down_summary.map(|s| s.max_mbps).unwrap_or(0.0),
            )
        } else {
            let total_bits = summary.avg_bits_per_second * config.duration as f64;
            format!(r#"
            <div class="card">
                <div class="label">平均带宽</div>
                <div class="value good">{:.2}<span class="unit">Mbps</span></div>
            </div>
            <div class="card">
                <div class="label">最高带宽</div>
                <div class="value">{:.2}<span class="unit">Mbps</span></div>
            </div>
            <div class="card">
                <div class="label">最低带宽</div>
                <div class="value">{:.2}<span class="unit">Mbps</span></div>
            </div>
            <div class="card">
                <div class="label">传输总量</div>
                <div class="value">{}</div>
            </div>"#,
                avg_mbps, max_mbps, min_mbps,
                format_speed(total_bits),
            )
        };

        let command_section = if is_bidirectional {
            let cmd_up = &result.command;
            let cmd_down = result.command_down.as_deref().unwrap_or("无");
            format!(r#"
            <div class="info-item" style="grid-column:1/-1;justify-content:flex-start;gap:20px;">
                <span class="info-label" style="flex-shrink:0;">上传命令</span>
                <span class="info-value" style="font-family:monospace;font-size:12px;word-break:break-all;text-align:left;">{}</span>
            </div>
            <div class="info-item" style="grid-column:1/-1;justify-content:flex-start;gap:20px;">
                <span class="info-label" style="flex-shrink:0;">下载命令</span>
                <span class="info-value" style="font-family:monospace;font-size:12px;word-break:break-all;text-align:left;">{}</span>
            </div>"#, cmd_up, cmd_down)
        } else {
            format!(r#"
            <div class="info-item" style="grid-column:1/-1;justify-content:flex-start;gap:20px;">
                <span class="info-label" style="flex-shrink:0;">执行命令</span>
                <span class="info-value" style="font-family:monospace;font-size:12px;word-break:break-all;text-align:left;">{}</span>
            </div>"#, result.command)
        };

        let bidirectional_chart = if is_bidirectional {
            r#"
        <div class="section">
            <h2>📊 上下行对比</h2>
            <div class="chart-container" style="height:350px;">
                <canvas id="bidirectionalChart"></canvas>
            </div>
        </div>"#.to_string()
        } else {
            String::new()
        };

        // 构建终端回显内容
        let mut terminal_lines = Vec::new();

        // 辅助：生成一组间隔数据的文本
        let mut append_intervals = |lines: &mut Vec<String>, label: &str, items: &[crate::iperf_controller::IntervalResult], cmd: &str| {
            if items.is_empty() {
                return;
            }
            if !lines.is_empty() {
                lines.push(String::new()); // 空行分隔
            }
            // 命令行
            lines.push(format!("$ {}", cmd));
            // 方向标签
            lines.push(format!("--- {} ({}) ---", label, config.protocol.to_uppercase()));
            // 表头
            lines.push(format!(
                "{:>5}  {:<15}  {:<12}  {:<15}  {:<10}",
                "[ID]", "Interval", "Transfer", "Bitrate", "Retr"
            ));
            for item in items {
                let interval_str = format!("{:.2}-{:.2}  sec", item.time_start, item.time_end);
                let transfer_str = format_bytes(item.bytes);
                let bitrate_str = format_bitrate(item.bits_per_second);
                let retr_str = if item.retransmits > 0 {
                    format!("{}", item.retransmits)
                } else {
                    "-".to_string()
                };
                lines.push(format!(
                    "{:>5}  {:<15}  {:<12}  {:<15}  {:<10}",
                    "[  1]", interval_str, transfer_str, bitrate_str, retr_str,
                ));
            }
            // 摘要
            let avg_bps = items.iter().map(|i| i.bits_per_second).filter(|&b| b > 0.0);
            let count = avg_bps.clone().count();
            let avg = if count > 0 { avg_bps.sum::<f64>() / count as f64 } else { 0.0 };
            let total_bytes: u64 = items.iter().map(|i| i.bytes).sum();
            lines.push(format!("[  1]  {:.2}-{:.2}  sec  {}  {}  ✔",
                items.first().map(|i| i.time_start).unwrap_or(0.0),
                items.last().map(|i| i.time_end).unwrap_or(0.0),
                format_bytes(total_bytes),
                format_bitrate(avg),
            ));
        };

        if is_bidirectional {
            append_intervals(&mut terminal_lines, "上传", &up_intervals, &result.command);
            if let Some(ref down_cmd) = result.command_down {
                append_intervals(&mut terminal_lines, "下载", &down_intervals, down_cmd);
            } else {
                append_intervals(&mut terminal_lines, "下载", &down_intervals, "");
            }
        } else {
            let label = if config.reverse { "下载" } else { "上传" };
            append_intervals(&mut terminal_lines, label, &intervals, &result.command);
        }

        let terminal_text = terminal_lines.join("\n");
        // 转义HTML特殊字符
        let terminal_escaped = terminal_text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let inner_html = format!(
            "<pre style=\"margin:0;color:#a6adc8;font-size:13px;line-height:1.5;white-space:pre-wrap;word-break:break-all;\">{}</pre>\n<div style=\"color:#6c7086;font-size:11px;margin-top:8px;user-select:none;\">$</div>",
            terminal_escaped,
        );
        let terminal_html = format!(
            "<div class=\"section\" style=\"background:#1e1e2e;color:#cdd6f4;font-family:'Cascadia Code','Fira Code','JetBrains Mono','Consolas',monospace;font-size:13px;padding:20px;border-radius:10px;overflow-x:auto;\">\n    <h2 style=\"color:#89b4fa;border-bottom-color:#45475a;display:flex;align-items:center;gap:8px;margin-bottom:12px;border-bottom:2px solid #45475a;padding-bottom:8px;\">💻 终端回显</h2>\n    <div style=\"background:#181825;border-radius:8px;padding:16px;\">\n        {}\n    </div>\n</div>",
            inner_html,
        );

        let html = format!(r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>网络链路速度测试报告</title>
    <script src="/lib/chart.js"></script>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            background: #f0f2f5;
            color: #333;
            padding: 20px;
        }}
        .container {{ max-width: 1000px; margin: 0 auto; }}
        .header {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 30px;
            border-radius: 12px;
            margin-bottom: 24px;
        }}
        .header h1 {{ font-size: 24px; margin-bottom: 8px; }}
        .header .meta {{ opacity: 0.9; font-size: 14px; }}
        .summary-cards {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px; margin-bottom: 24px; }}
        .card {{
            background: white;
            border-radius: 10px;
            padding: 20px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.08);
        }}
        .card .label {{ font-size: 12px; color: #888; margin-bottom: 4px; }}
        .card .value {{ font-size: 24px; font-weight: 600; color: #333; }}
        .card .value.good {{ color: #22c55e; }}
        .card .unit {{ font-size: 14px; color: #888; margin-left: 4px; }}
        .section {{ background: white; border-radius: 10px; padding: 20px; margin-bottom: 24px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }}
        .section h2 {{ font-size: 18px; margin-bottom: 16px; color: #444; border-bottom: 2px solid #f0f2f5; padding-bottom: 8px; }}
        .chart-container {{ position: relative; height: 300px; }}
        .info-grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }}
        .info-item {{ display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid #f5f5f5; }}
        .info-item .info-label {{ color: #888; }}
        .info-item .info-value {{ font-weight: 500; }}
        .footer {{ text-align: center; color: #888; font-size: 12px; padding: 20px; }}
        @media print {{
            body {{ background: white; padding: 0; }}
            .header {{ border-radius: 0; }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>📊 网络链路速度测试报告</h1>
            <div class="meta">
                测试时间: {} |
                协议: {} | 类型: {}
            </div>
            <div class="meta" style="margin-top:4px;">
                服务端: {}:{} |
                并行流: {} | 时长: {}s
            </div>
        </div>

        <div class="summary-cards">
            {}
        </div>

        <div class="section">
            <h2>📈 实时带宽趋势</h2>
            <div class="chart-container">
                <canvas id="bandwidthChart"></canvas>
            </div>
        </div>

        {}
        
        <div class="section">
            <h2>📋 详细信息</h2>
            <div class="info-grid">
                <div class="info-item">
                    <span class="info-label">测试协议</span>
                    <span class="info-value">{}</span>
                </div>
                <div class="info-item">
                    <span class="info-label">测试方向</span>
                    <span class="info-value">{}</span>
                </div>
                <div class="info-item">
                    <span class="info-label">客户端地址</span>
                    <span class="info-value">{}</span>
                </div>
                <div class="info-item">
                    <span class="info-label">服务端地址</span>
                    <span class="info-value">{}</span>
                </div>
                <div class="info-item">
                    <span class="info-label">服务端端口</span>
                    <span class="info-value">{}</span>
                </div>
                <div class="info-item">
                    <span class="info-label">并行流数</span>
                    <span class="info-value">{}</span>
                </div>
                <div class="info-item">
                    <span class="info-label">测试时长</span>
                    <span class="info-value">{} 秒</span>
                </div>
            </div>
        </div>

        <div class="section" style="background:#f8f9fa;">
            <h2>🔧 测试引擎</h2>
            <div class="info-grid">
                <div class="info-item" style="justify-content:flex-start;gap:20px;">
                    <span class="info-label" style="flex-shrink:0;">引擎来源</span>
                    <span class="info-value">iperf3 ({})</span>
                </div>
                {}
            </div>
        </div>

        TERMINAL_SECTION_PLACEHOLDER

        <div class="footer">
            <p>由 SpeedTest 网络链路速度测试工具生成 | {}</p>
        </div>
    </div>

    <script>
        const intervalData = {};
        const upIntervalData = {};
        const downIntervalData = {};
        const isBidirectional = {};

        const ctx = document.getElementById('bandwidthChart').getContext('2d');
        const times = intervalData.map(d => d.time.toFixed(1));
        const speeds = intervalData.map(d => (d.bits_per_second / 1_000_000).toFixed(2));

        new Chart(ctx, {{
            type: 'line',
            data: {{
                labels: times,
                datasets: [{{
                    label: '带宽 (Mbps)',
                    data: speeds,
                    borderColor: '#667eea',
                    backgroundColor: 'rgba(102, 126, 234, 0.1)',
                    fill: true,
                    tension: 0.3,
                    pointRadius: 2,
                    pointHoverRadius: 6,
                }}]
            }},
            options: {{
                responsive: true,
                maintainAspectRatio: false,
                plugins: {{
                    legend: {{ display: false }},
                    tooltip: {{
                        callbacks: {{
                            label: ctx => ctx.parsed.y + ' Mbps'
                        }}
                    }}
                }},
                scales: {{
                    x: {{
                        title: {{ display: true, text: '时间 (秒)' }},
                        grid: {{ display: false }}
                    }},
                    y: {{
                        title: {{ display: true, text: '带宽 (Mbps)' }},
                        beginAtZero: true,
                        grid: {{ color: 'rgba(0,0,0,0.05)' }}
                    }}
                }}
            }}
        }});

        if (isBidirectional) {{
            const ctx2 = document.getElementById('bidirectionalChart').getContext('2d');
            const upTimes = upIntervalData.map(d => d.time.toFixed(1));
            const upSpeeds = upIntervalData.map(d => (d.bits_per_second / 1_000_000).toFixed(2));
            const downSpeeds = downIntervalData.map(d => (d.bits_per_second / 1_000_000).toFixed(2));

            new Chart(ctx2, {{
                type: 'line',
                data: {{
                    labels: upTimes,
                    datasets: [{{
                        label: '上传 (Mbps)',
                        data: upSpeeds,
                        borderColor: '#22c55e',
                        backgroundColor: 'rgba(34, 197, 94, 0.1)',
                        fill: true,
                        tension: 0.3,
                        pointRadius: 2,
                    }}, {{
                        label: '下载 (Mbps)',
                        data: downSpeeds,
                        borderColor: '#3b82f6',
                        backgroundColor: 'rgba(59, 130, 246, 0.1)',
                        fill: true,
                        tension: 0.3,
                        pointRadius: 2,
                    }}]
                }},
                options: {{
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {{
                        legend: {{ position: 'top' }},
                        tooltip: {{
                            callbacks: {{
                                label: ctx => ctx.dataset.label + ': ' + ctx.parsed.y + ' Mbps'
                            }}
                        }}
                    }},
                    scales: {{
                        x: {{ title: {{ display: true, text: '时间 (秒)' }}, grid: {{ display: false }} }},
                        y: {{ title: {{ display: true, text: '带宽 (Mbps)' }}, beginAtZero: true }}
                    }}
                }}
            }});
        }}
    </script>
</body>
</html>"#,
            result.start_time,
            protocol,
            test_type,
            config.server_host,
            config.server_port,
            config.parallel,
            config.duration,
            summary_cards,
            bidirectional_chart,
            protocol,
            test_type,
            config.client_host,
            config.server_host,
            config.server_port,
            config.parallel,
            config.duration,
            result.iperf3_version,
            command_section,
            now,
            intervals_json,
            up_intervals_json,
            down_intervals_json,
            if is_bidirectional { "true" } else { "false" },
        );

        // 保存文件
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let report_id = record_id.unwrap_or(&timestamp);
        let filename = format!("speedtest_report_{}.html", report_id);
        let filepath = self.reports_dir.join(&filename);

        // 替换终端回显占位符
        let html = html.replace("TERMINAL_SECTION_PLACEHOLDER", &terminal_html);
        std::fs::write(&filepath, html).map_err(|e| format!("写入报告失败: {}", e))?;

        Ok(filepath.to_string_lossy().to_string())
    }

    /// 列出所有报告
    pub fn list_reports(&self) -> Vec<ReportInfo> {
        let mut reports = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.reports_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("html") {
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        let created = metadata
                            .created()
                            .ok()
                            .and_then(|t| {
                                let dt: chrono::DateTime<chrono::Local> = t.into();
                                Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string())
                            })
                            .unwrap_or_default();

                        reports.push(ReportInfo {
                            filename: path.file_name().unwrap().to_string_lossy().to_string(),
                            path: path.to_string_lossy().to_string(),
                            size: metadata.len(),
                            created,
                        });
                    }
                }
            }
        }
        reports.sort_by(|a, b| b.filename.cmp(&a.filename));
        reports
    }

    /// 获取报告文件路径
    pub fn get_report_path(&self, filename: &str) -> Option<PathBuf> {
        let filepath = self.reports_dir.join(filename);
        if filepath.exists() {
            Some(filepath)
        } else {
            None
        }
    }
}
