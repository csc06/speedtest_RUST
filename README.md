# SpeedTest Rust 重写版

基于 iperf3 的局域网链路质量检测工具，由 Python 版重写为 Rust。

## 特点

- 🦀 **单一可执行文件**：编译后生成独立二进制，无需 Python 环境
- 🚀 **高性能**：Rust 编译，启动迅速
- 🌐 **Web 管理界面**：内置 Web 服务器，浏览器操作
- 📊 **实时图表**：Chart.js 可视化带宽趋势
- 📄 **HTML 报告**：每次测试自动生成详细报告
- 🔒 **网络安全**：支持自动检测/禁用无线网卡
- 🖥️ **跨平台**：支持 macOS / Windows / Linux

## 编译

```bash
# 确保已安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 编译
cd speedtest_RUST
cargo build --release

# 编译产物位于 target/release/speedtest
```

## 使用

### 前置条件

需要安装 iperf3：
- **macOS**: `brew install iperf3`
- **Windows**: 下载 iperf3.exe 放入 `bin/` 目录
- **Linux**: `sudo apt install iperf3`

### 客户端模式（默认）

```bash
# 启动客户端 Web 界面
./speedtest

# 或明确指定
./speedtest client

# 指定端口
./speedtest client -p 5001

# 不自动打开浏览器
./speedtest --no-browser
```

然后浏览器访问 http://127.0.0.1:5001

### 服务端模式

```bash
# 在 Windows 服务端运行
./speedtest server

# 指定 Web 端口和 iperf3 端口
./speedtest server -p 5002 -P 5201
```

## 部署

编译后，将以下内容复制到目标机器：

1. `target/release/speedtest` — 主程序
2. `frontend/` — 前端静态文件目录
3. `bin/` — iperf3 可执行文件（可选）

目录结构：
```
your-app/
├── speedtest          # 主程序
├── frontend/
│   ├── index.html
│   ├── css/style.css
│   ├── js/api.js
│   ├── js/app.js
│   ├── js/charts.js
│   └── lib/chart.js
└── bin/
    └── iperf3.exe     # 可选
```

## API 接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET  | /api/status | 系统状态 |
| GET  | /api/config | 获取配置 |
| POST | /api/test/start | 启动测试 |
| POST | /api/test/stop | 停止测试 |
| GET  | /api/test/result | 获取结果 |
| GET  | /api/network/wifi | Wi-Fi 状态 |
| POST | /api/network/wifi/disable | 禁用 Wi-Fi |
| POST | /api/network/wifi/enable | 启用 Wi-Fi |
| GET  | /api/history | 历史记录 |
| GET  | /api/history/statistics | 历史统计 |
| POST | /api/report/generate | 生成报告 |

## 与原版对比

| 特性 | Python 版 | Rust 版 |
|------|-----------|---------|
| 运行时依赖 | Python 3 + Flask | 无（单一二进制） |
| 性能 | 一般 | 高 |
| 前端 | Chart.js | Chart.js（相同） |
| API | Flask REST | Axum REST |
| 历史存储 | JSON 文件 | JSON 文件 |
| 报告生成 | Python 字符串模板 | Rust 字符串模板 |
