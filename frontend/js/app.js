/**
 * SpeedTest 主应用逻辑
 * 管理状态、事件和 UI 更新
 */

class SpeedTestApp {
    constructor() {
        this.charts = new ChartManager();
        this.isTestRunning = false;
        this.eventSource = null;
        this.testStartTime = null;
        this.wifiDisabledByApp = false;

        // 初始化
        this.init();
    }

    async init() {
        try {
            // 获取初始状态
            const status = await api.getStatus();
            this.updateWifiUI(status.wifi);
            this.updateSystemInfo(status.system);

            // 更新初始连接状态
            this.updateConnectionStatus(status);

            // 检查是否首次打开 - 自动禁用 Wi-Fi
            if (status.wifi && status.wifi.power_on) {
                this.showRiskDialog();
            } else {
                this.wifiDisabledByApp = true;
            }

            // 加载历史记录
            this.loadHistory();

            // 加载统计数据
            this.loadStatistics();

            // 绑定事件
            this.bindEvents();

            // 恢复保存的颜色主题
            this.restoreTheme();

            // 更新状态指示器
            this.startStatusPolling();

            // 加载客户端地址配置
            await this.loadClientAddr();

            // 立即检查远程服务端状态
            this.checkRemoteServer();

        } catch (err) {
            console.error('初始化失败:', err);
            this.showToast('连接服务器失败，请确保后端已启动', 'error');
        }
    }

    async loadClientAddr() {
        try {
            const config = await api.getConfig();
            const input = document.getElementById('clientAddr');
            if (input && config.client_addr) {
                input.value = config.client_addr;
            }
        } catch (_) {}
    }

    updateConnectionStatus(status) {
        const clientText = document.getElementById('clientStatusText');
        const clientIcon = document.getElementById('clientIcon');
        const serverText = document.getElementById('serverStatusText');
        const serverIcon = document.getElementById('serverIcon');

        if (status.client_addr) {
            if (clientText) clientText.textContent = `绑定地址: ${status.client_addr}`;
        } else if (status.wifi && status.wifi.ip_address) {
            if (clientText) clientText.textContent = `本机 IP: ${status.wifi.ip_address}`;
        } else {
            if (clientText) clientText.textContent = '本地客户端运行正常';
        }
        if (clientIcon) clientIcon.textContent = '✅';
    }

    // ==================== 事件绑定 ====================

    bindEvents() {
        // 启动测试
        document.getElementById('btnStartTest')?.addEventListener('click', () => this.startTest());

        // 停止测试
        document.getElementById('btnStopTest')?.addEventListener('click', () => this.stopTest());

        // 启用 Wi-Fi
        document.getElementById('btnEnableWifi')?.addEventListener('click', () => this.enableWifi());

        // 禁用 Wi-Fi
        document.getElementById('btnDisableWifi')?.addEventListener('click', () => this.disableWifi());

        // 风险确认：已断开外联，无需禁用
        document.getElementById('btnSafeNoDisable')?.addEventListener('click', () => this.confirmSafe());

        // 风险确认：禁用无线网卡
        document.getElementById('btnDisableWifiRisk')?.addEventListener('click', () => this.disableWifiAndClose());

        // 生成报告
        document.getElementById('btnGenerateReport')?.addEventListener('click', () => this.generateReport());

        // 清空历史
        document.getElementById('btnClearHistory')?.addEventListener('click', () => this.clearHistory());

        // 颜色主题切换
        document.getElementById('themeToggle')?.addEventListener('click', () => this.toggleTheme());

        // 设置客户端绑定地址
        document.getElementById('btnSetClientAddr')?.addEventListener('click', () => this.saveClientAddr());
    }

    async saveClientAddr() {
        const input = document.getElementById('clientAddr');
        if (!input) return;
        const addr = input.value.trim();
        try {
            const result = await api.setClientAddr(addr || null);
            if (result.success) {
                this.showToast(addr ? `客户端地址已设为 ${addr}` : '已清除客户端地址，使用自动检测', 'success');
                // 刷新状态显示
                const status = await api.getStatus();
                this.updateConnectionStatus(status);
            }
        } catch (err) {
            this.showToast('保存失败: ' + err.message, 'error');
        }
    }

    // ==================== Wi-Fi 管理 ====================

    showRiskDialog() {
        const modal = document.getElementById('riskModal');
        if (modal) {
            modal.classList.add('active');
        }
    }

    confirmSafe() {
        // 用户确认已断开外联，无需操作 Wi-Fi
        document.getElementById('riskModal')?.classList.remove('active');
        this.showToast('已确认安全，继续测试', 'success');
    }

    async disableWifiAndClose() {
        // 禁用无线网卡
        const result = await this.disableWifi();
        document.getElementById('riskModal')?.classList.remove('active');
        if (result.success) {
            this.showToast('无线网卡已禁用，可进行安全测试', 'success');
        }
    }

    async disableWifi() {
        try {
            const result = await api.disableWifi();
            if (result.success) {
                this.wifiDisabledByApp = true;
                await this.refreshWifiStatus();
                this.showToast('无线网卡已禁用 ✓', 'success');
            } else {
                this.showToast('禁用无线网卡失败', 'error');
            }
            return result;
        } catch (err) {
            this.showToast('操作失败: ' + err.message, 'error');
            return { success: false };
        }
    }

    async enableWifi() {
        try {
            const result = await api.enableWifi();
            if (result.success) {
                this.wifiDisabledByApp = false;
                await this.refreshWifiStatus();
                this.showToast('无线网卡已启用 ✓', 'success');
            } else {
                this.showToast('启用无线网卡失败', 'error');
            }
            return result;
        } catch (err) {
            this.showToast('操作失败: ' + err.message, 'error');
            return { success: false };
        }
    }

    async refreshWifiStatus() {
        const wifiStatus = await api.getWifiStatus();
        this.updateWifiUI(wifiStatus);
    }

    updateWifiUI(wifi) {
        const container = document.getElementById('wifiStatus');
        if (!container) return;

        const powerOn = wifi && wifi.power_on;
        const ssid = (wifi && wifi.ssid) || '';
        const ip = (wifi && wifi.ip_address) || '';

        container.className = `wifi-status ${powerOn ? 'wifi-on' : 'wifi-off'}`;

        container.innerHTML = `
            <div class="wifi-icon">${powerOn ? '📶' : '📡'}</div>
            <div class="wifi-info">
                <div class="status-label">
                    ${powerOn ? '⚠️ 无线网卡已连接' : '✅ 无线网卡已禁用'}
                </div>
                <div class="status-detail">
                    ${powerOn
                        ? `已连接到 ${ssid}${ip ? ` | IP: ${ip}` : ''}`
                        : '网络隔离环境，可进行安全测试'
                    }
                </div>
            </div>
            <div class="wifi-action">
                ${powerOn
                    ? `<button class="btn btn-danger btn-sm" id="btnDisableWifi">禁用 Wi-Fi</button>`
                    : `<button class="btn btn-success btn-sm" id="btnEnableWifi">启用 Wi-Fi</button>`
                }
            </div>
        `;

        // 重新绑定按钮事件
        document.getElementById('btnDisableWifi')?.addEventListener('click', () => this.disableWifi());
        document.getElementById('btnEnableWifi')?.addEventListener('click', () => this.enableWifi());
    }

    // ==================== 测试控制 ====================

    async startTest() {
        if (this.isTestRunning) return;

        // 获取测试参数
        const serverHost = document.getElementById('serverHost')?.value || '127.0.0.1';
        const serverPort = parseInt(document.getElementById('serverPort')?.value || '5201');
        const duration = parseInt(document.getElementById('testDuration')?.value || '10');
        const parallel = parseInt(document.getElementById('parallelStreams')?.value || '4');
        const protocol = document.getElementById('protocol')?.value || 'tcp';
        const dirValue = document.getElementById('testDirection')?.value || 'upload';
        const reverse = dirValue === 'download';
        const bidirectional = dirValue === 'bidirectional';
        const clientAddr = document.getElementById('clientAddr')?.value.trim() || null;

        // 清空图表和历史数据
        this.charts.clearData();
        this.updateResultUI(null);

        try {
            const result = await api.startTest({
                server_host: serverHost,
                server_port: serverPort,
                duration,
                parallel,
                protocol,
                reverse,
                bidirectional,
                client_addr: clientAddr,
            });

            if (!result.success) {
                this.showToast(result.message || '启动测试失败', 'error');
                return;
            }

            this.isTestRunning = true;
            this.testStartTime = Date.now();
            this.updateTestUI(true);

            // 计算总时长（双向测试翻倍）
            const totalDuration = bidirectional ? duration * 2 : duration;
            let remaining = totalDuration;
            const countdownEl = document.getElementById('countdown');
            if (countdownEl) countdownEl.textContent = remaining;

            this._countdownTimer = setInterval(() => {
                remaining--;
                if (countdownEl) countdownEl.textContent = Math.max(0, remaining);
                if (remaining <= 0 && countdownEl) countdownEl.textContent = '0';
            }, 1000);

            // 轮询检测测试完成
            this._resultPollTimer = setInterval(async () => {
                if (!this.isTestRunning) {
                    clearInterval(this._resultPollTimer);
                    return;
                }
                try {
                    const res = await api.getTestResult();
                    if (res && res.status === 'completed') {
                        clearInterval(this._resultPollTimer);
                        clearInterval(this._countdownTimer);
                        this.isTestRunning = false;
                        this.updateTestUI(false);
                        this.closeProgressListener();
                        this.stopPolling();
                        this.stopCountdown();
                        this.updateResultUI(res);
                        this.addToHistory(res);
                        this.loadStatistics();
                        this.showToast('测试完成 ✓', 'success');
                        await this.generateReport();
                        setTimeout(() => location.reload(), 500);
                    } else if (res && res.status === 'failed') {
                        // 测试失败（连接被拒、超时等），停止轮询并提示
                        clearInterval(this._resultPollTimer);
                        clearInterval(this._countdownTimer);
                        this.isTestRunning = false;
                        this.updateTestUI(false);
                        this.stopPolling();
                        this.stopCountdown();
                        this.showToast('❌ 测速失败：无法连接到服务端，请检查地址和防火墙设置', 'error');
                    }
                } catch (_) {}
            }, 1000);

            this.showToast('测试开始...', 'info');

        } catch (err) {
            this.showToast('启动测试失败: ' + err.message, 'error');
        }
    }

    async stopTest() {
        try {
            await api.stopTest();
            this.isTestRunning = false;
            this.updateTestUI(false);
            this.closeProgressListener();
            this.stopPolling();
            this.stopCountdown();
            this.showToast('测试已停止', 'warning');

            // 获取部分结果
            const result = await api.getTestResult();
            if (result && result.status) {
                this.updateResultUI(result);
            }
        } catch (err) {
            this.showToast('停止测试失败', 'error');
        }
    }

    startProgressListener() {
        // Rust 版本暂时不支持 SSE，使用轮询
    }

    stopPolling() {
        if (this._resultPollTimer) {
            clearInterval(this._resultPollTimer);
            this._resultPollTimer = null;
        }
    }

    stopCountdown() {
        if (this._countdownTimer) {
            clearInterval(this._countdownTimer);
            this._countdownTimer = null;
        }
    }

    closeProgressListener() {
        if (this.eventSource) {
            this.eventSource.close();
            this.eventSource = null;
        }
    }

    // ==================== UI 更新 ====================

    updateTestUI(running) {
        const btnStart = document.getElementById('btnStartTest');
        const btnStop = document.getElementById('btnStopTest');
        const testProgress = document.getElementById('testProgress');

        if (btnStart) btnStart.disabled = running;
        if (btnStop) btnStop.style.display = running ? 'inline-flex' : 'none';

        if (testProgress) {
            testProgress.style.display = running ? 'block' : 'none';
        }

        // 禁用表单
        const formElements = document.querySelectorAll('.test-config-form input, .test-config-form select');
        formElements.forEach(el => el.disabled = running);
    }

    updateLiveStats(result) {
        const summary = result.summary || {};
        const currentSpeed = summary.avg_mbps || 0;

        const el = document.getElementById('liveSpeed');
        if (el) {
            el.textContent = currentSpeed.toFixed(2);
        }
    }

    updateResultUI(result) {
        if (!result || !result.summary) {
            // 重置结果区域
            const resultPanel = document.getElementById('testResult');
            if (resultPanel) resultPanel.style.display = 'none';
            return;
        }

        const summary = result.summary;
        const config = result.config || {};
        const isBidirectional = result.bidirectional;
        const resultPanel = document.getElementById('testResult');
        if (resultPanel) resultPanel.style.display = 'block';

        // 更新统计值（使用 innerHTML 保留 <span> 标签）
        const unitHtml = ' <span class="stat-unit">Mbps</span>';
        if (isBidirectional) {
            const upSummary = result.upload?.summary || {};
            const downSummary = result.download?.summary || {};
            document.getElementById('resultAvgSpeed').innerHTML = (upSummary.avg_mbps || 0).toFixed(2) + unitHtml;
            document.getElementById('resultMaxSpeed').innerHTML = (downSummary.avg_mbps || 0).toFixed(2) + unitHtml;
            document.querySelector('#resultStats .stat-card:nth-child(1) .stat-label').textContent = '上传平均';
            document.querySelector('#resultStats .stat-card:nth-child(2) .stat-label').textContent = '下载平均';
            document.querySelector('#resultStats .stat-card:nth-child(3) .stat-label').textContent = '上传最高';
            document.getElementById('resultMinSpeed').innerHTML = (upSummary.max_mbps || 0).toFixed(2) + unitHtml;
            document.querySelector('#resultStats .stat-card:nth-child(4) .stat-label').textContent = '下载最高';
            document.querySelector('#resultStats .stat-card:nth-child(4) .stat-value').innerHTML =
                (downSummary.max_mbps || 0).toFixed(2) + unitHtml;
        } else {
            document.getElementById('resultAvgSpeed').innerHTML = (summary.avg_mbps || 0).toFixed(2) + unitHtml;
            document.getElementById('resultMaxSpeed').innerHTML = (summary.max_mbps || 0).toFixed(2) + unitHtml;
            document.getElementById('resultMinSpeed').innerHTML = (summary.min_mbps || 0).toFixed(2) + unitHtml;
            document.querySelector('#resultStats .stat-card:nth-child(1) .stat-label').textContent = '平均带宽';
            document.querySelector('#resultStats .stat-card:nth-child(2) .stat-label').textContent = '最高带宽';
            document.querySelector('#resultStats .stat-card:nth-child(3) .stat-label').textContent = '最低带宽';
            document.querySelector('#resultStats .stat-card:nth-child(4) .stat-label').textContent = '测试时长';
        }

        // 更新协议和方向
        document.getElementById('resultProtocol').textContent = (config.protocol || 'tcp').toUpperCase();
        document.getElementById('resultDirection').textContent = isBidirectional ? '上下行' : (config.reverse ? '下载 (反向)' : '上传');

        // 更新状态
        const statusEl = document.getElementById('resultStatus');
        if (statusEl) {
            const status = result.status;
            if (status === 'completed') {
                statusEl.textContent = '✅ 完成';
                statusEl.style.color = '#22c55e';
            } else if (status === 'stopped') {
                statusEl.textContent = '⏹️ 已停止';
                statusEl.style.color = '#f59e0b';
            } else {
                statusEl.textContent = '❌ 失败';
                statusEl.style.color = '#ef4444';
            }
        }

        // 更新服务端
        if (config.server_host) {
            document.getElementById('resultServer').textContent = `${config.server_host}:${config.server_port}`;
        }

        // 更新持续时间（双向模式下第4个卡片用于显示下载最高，不覆盖）
        if (!isBidirectional) {
            const startTime = result.start_time;
            const endTime = result.end_time;
            if (startTime && endTime) {
                const duration = (new Date(endTime) - new Date(startTime)) / 1000;
                document.getElementById('resultDuration').innerHTML = duration.toFixed(1) + ' <span class="stat-unit">秒</span>';
            }
        }

        // 加载间隔数据到图表
        const intervals = isBidirectional ? (result.upload?.intervals || []) : (result.intervals || []);
        if (intervals.length > 0) {
            this.charts.loadHistoryData(intervals);
        }

        // 启用报告生成按钮
        const btnReport = document.getElementById('btnGenerateReport');
        if (btnReport) btnReport.disabled = false;
    }

    updateSystemInfo(system) {
        const badge = document.getElementById('systemBadge');
        if (badge) {
            const systemNames = {
                'macos': '🍎 macOS',
                'windows': '🪟 Windows',
                'linux': '🐧 Linux',
            };
            badge.textContent = systemNames[system] || system;
        }
    }

    // ==================== 历史管理 ====================

    async loadHistory() {
        try {
            const history = await api.getHistory();
            this.renderHistoryTable(history);
        } catch (err) {
            console.error('加载历史失败:', err);
        }
    }

    renderHistoryTable(history) {
        const tbody = document.getElementById('historyBody');
        if (!tbody) return;

        if (!history || history.length === 0) {
            tbody.innerHTML = `
                <tr>
                    <td colspan="8">
                        <div class="empty-state">
                            <div class="empty-icon">📋</div>
                            <p>暂无测试记录，运行一次测试后历史将显示在此处</p>
                        </div>
                    </td>
                </tr>
            `;
            return;
        }

        let html = '';
        history.forEach((record, index) => {
            const result = record.result || {};
            const config = result.config || {};
            const summary = result.summary || {};
            const avgSpeed = summary.avg_mbps || 0;
            const maxSpeed = summary.max_mbps || 0;
            const protocol = (config.protocol || 'tcp').toUpperCase();
            const ts = record.timestamp ? new Date(record.timestamp).toLocaleString('zh-CN') : '-';

            const clientHost = config.client_host || '-';
            const serverHost = config.server_host || '-';
            const serverPort = config.server_port || '';
            const serverAddr = serverPort ? `${serverHost}:${serverPort}` : serverHost;

            html += `
                <tr>
                    <td>${index + 1}</td>
                    <td>${ts}</td>
                    <td>${clientHost}</td>
                    <td>${serverAddr}</td>
                    <td class="speed-cell">${avgSpeed.toFixed(2)} Mbps</td>
                    <td>${maxSpeed.toFixed(2)} Mbps</td>
                    <td>${protocol}</td>
                    <td class="actions-cell">
                        <button class="btn btn-primary btn-sm" onclick="app.viewHistoryDetail('${record.id}')">📄 报告</button>
                        <button class="btn btn-danger btn-sm" onclick="app.deleteHistoryRecord('${record.id}')">🗑️</button>
                    </td>
                </tr>
            `;
        });

        tbody.innerHTML = html;
    }

    async viewHistoryDetail(recordId) {
        try {
            const record = await api.getHistoryDetail(recordId);
            if (record && record.result) {
                this.updateResultUI(record.result);
                // 自动生成报告
                const reportResult = await api.generateReport(recordId);
                if (reportResult.success) {
                    this.showToast('报告已生成', 'success');
                    // 下载报告
                    window.open(reportResult.path, '_blank');
                }
            }
        } catch (err) {
            this.showToast('获取记录失败', 'error');
        }
    }

    async deleteHistoryRecord(recordId) {
        if (!confirm('确定要删除这条记录吗？')) return;
        try {
            await api.deleteHistory(recordId);
            this.loadHistory();
            this.showToast('记录已删除', 'success');
        } catch (err) {
            this.showToast('删除失败', 'error');
        }
    }

    async addToHistory(result) {
        // 历史已由后端自动保存，只需刷新列表
        await this.loadHistory();
    }

    async loadStatistics() {
        try {
            const stats = await api.getStatistics();
            document.getElementById('statTotalTests').textContent = stats.total_tests || 0;
            document.getElementById('statAvgSpeed').innerHTML = (stats.avg_speed || 0).toFixed(2) + ' <span class="stat-unit">Mbps</span>';
            document.getElementById('statMaxSpeed').innerHTML = (stats.max_speed || 0).toFixed(2) + ' <span class="stat-unit">Mbps</span>';
        } catch (err) {
            console.error('加载统计失败:', err);
        }
    }

    async clearHistory() {
        if (!confirm('确定要清空所有测试历史吗？此操作不可恢复。')) return;
        try {
            await api.clearHistory();
            this.loadHistory();
            this.loadStatistics();
            this.showToast('历史记录已清空', 'success');
        } catch (err) {
            this.showToast('清空失败', 'error');
        }
    }

    // ==================== 报告 ====================

    async generateReport(recordId) {
        try {
            const result = await api.generateReport(recordId || null);
            if (result.success) {
                this.showToast('报告已生成', 'success');
                // 在新窗口打开报告
                window.open(result.path, '_blank');
                return result;
            } else {
                this.showToast(result.message || '报告生成失败', 'error');
            }
        } catch (err) {
            this.showToast('报告生成失败: ' + err.message, 'error');
        }
        return null;
    }

    // ==================== 状态轮询 ====================

    startStatusPolling() {
        setInterval(async () => {
            try {
                const status = await api.getStatus();
                if (status.wifi) {
                    this.updateWifiUI(status.wifi);
                }

                // 更新本地客户端状态
                const clientText = document.getElementById('clientStatusText');
                const clientIcon = document.getElementById('clientIcon');
                if (clientText) {
                    if (status.client_addr) {
                        clientText.textContent = `绑定地址: ${status.client_addr}`;
                    } else if (status.wifi && status.wifi.ip_address) {
                        clientText.textContent = `本机 IP: ${status.wifi.ip_address}`;
                    } else {
                        clientText.textContent = '运行正常 ✅';
                    }
                }
                if (clientIcon) clientIcon.textContent = '✅';

                // 检查远程 iperf3 服务端是否可达
                this.checkRemoteServer();

            } catch (err) {
                // 忽略轮询错误
            }
        }, 5000);
    }

    async checkRemoteServer() {
        try {
            const serverHost = document.getElementById('serverHost')?.value || '127.0.0.1';
            const serverPort = document.getElementById('serverPort')?.value || '5201';
            const result = await api.checkServer(serverHost, parseInt(serverPort));
            
            const serverText = document.getElementById('serverStatusText');
            const serverIcon = document.getElementById('serverIcon');
            const serverBox = document.getElementById('serverStatusBox');
            
            if (serverText && serverIcon && serverBox) {
                if (result.online) {
                    serverText.textContent = `服务端 ${result.host}:${result.port} 在线 ✅`;
                    serverIcon.textContent = '✅';
                    serverBox.style.background = '#f0fdf4';
                    serverBox.style.borderColor = '#bbf7d0';
                } else {
                    serverText.textContent = `服务端 ${result.host}:${result.port} 不可达 ❌`;
                    serverIcon.textContent = '❌';
                    serverBox.style.background = '#fef2f2';
                    serverBox.style.borderColor = '#fecaca';
                }
            }
        } catch (err) {
            console.error('检查远程服务端失败:', err);
        }
    }

    // ==================== 颜色主题 ====================

    restoreTheme() {
        const saved = localStorage.getItem('speedtest-theme');
        const btn = document.getElementById('themeToggle');
        if (saved === 'dark') {
            document.documentElement.setAttribute('data-theme', 'dark');
            if (btn) btn.textContent = '☀️';
        } else {
            if (btn) btn.textContent = '🌙';
        }
    }

    toggleTheme() {
        const btn = document.getElementById('themeToggle');
        const isDark = document.documentElement.getAttribute('data-theme') === 'dark';
        if (isDark) {
            document.documentElement.removeAttribute('data-theme');
            if (btn) btn.textContent = '🌙';
            localStorage.setItem('speedtest-theme', 'light');
        } else {
            document.documentElement.setAttribute('data-theme', 'dark');
            if (btn) btn.textContent = '☀️';
            localStorage.setItem('speedtest-theme', 'dark');
        }
    }

    // ==================== Toast 通知 ====================

    showToast(message, type = 'info') {
        const container = document.getElementById('toastContainer');
        if (!container) return;

        const toast = document.createElement('div');
        toast.className = `toast toast-${type}`;
        toast.textContent = message;

        container.appendChild(toast);

        // 3秒后自动移除
        setTimeout(() => {
            toast.style.opacity = '0';
            toast.style.transform = 'translateX(100%)';
            toast.style.transition = 'all 0.3s ease';
            setTimeout(() => toast.remove(), 300);
        }, 3000);
    }
}

// 创建全局实例
const app = new SpeedTestApp();
