/**
 * API 客户端 - 封装所有后端接口调用
 */
class ApiClient {
    constructor(baseURL = '') {
        this.baseURL = baseURL;
    }

    async request(method, path, data = null) {
        const options = {
            method,
            headers: { 'Content-Type': 'application/json' },
        };
        if (data && (method === 'POST' || method === 'PUT')) {
            options.body = JSON.stringify(data);
        }
        try {
            const response = await fetch(`${this.baseURL}${path}`, options);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }
            return await response.json();
        } catch (err) {
            console.error(`API Error [${method} ${path}]:`, err);
            throw err;
        }
    }

    // 系统状态
    getStatus() { return this.request('GET', '/api/status'); }

    // Wi-Fi 管理
    getWifiStatus() { return this.request('GET', '/api/network/wifi'); }
    disableWifi() { return this.request('POST', '/api/network/wifi/disable'); }
    enableWifi() { return this.request('POST', '/api/network/wifi/enable'); }

    // iperf3 服务端
    startServer(port) { return this.request('POST', '/api/server/start', { port }); }
    stopServer() { return this.request('POST', '/api/server/stop'); }
    getServerStatus() { return this.request('GET', '/api/server/status'); }

    // 测试控制
    startTest(params) { return this.request('POST', '/api/test/start', params); }
    stopTest() { return this.request('POST', '/api/test/stop'); }
    getTestResult() { return this.request('GET', '/api/test/result'); }

    // 历史记录
    getHistory() { return this.request('GET', '/api/history'); }
    getHistoryDetail(id) { return this.request('GET', `/api/history/${id}`); }
    deleteHistory(id) { return this.request('DELETE', `/api/history/${id}`); }
    clearHistory() { return this.request('POST', '/api/history/clear'); }
    getStatistics() { return this.request('GET', '/api/history/statistics'); }

    // 报告
    generateReport(recordId) { return this.request('POST', '/api/report/generate', { record_id: recordId }); }
    listReports() { return this.request('GET', '/api/report/list'); }

    // 配置
    getConfig() { return this.request('GET', '/api/config'); }
    setClientAddr(addr) { return this.request('POST', '/api/config/client-addr', { addr }); }

    // 检查服务端可达性
    checkServer(host, port) {
        return this.request('GET', `/api/check/server?host=${encodeURIComponent(host)}&port=${port}`);
    }
}

// 创建全局实例
const api = new ApiClient();
