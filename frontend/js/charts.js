/**
 * 图表管理器 - 使用 Chart.js 绘制实时带宽图表
 */

class ChartManager {
    constructor() {
        this.bandwidthChart = null;
        this.chartData = {
            labels: [],
            speeds: [],
            retransmits: [],
        };
        this.maxDataPoints = 100;
    }

    /**
     * 初始化带宽图表
     */
    initBandwidthChart(canvasId = 'bandwidthChart') {
        const canvas = document.getElementById(canvasId);
        if (!canvas) return;

        const ctx = canvas.getContext('2d');

        // 创建渐变
        const gradient = ctx.createLinearGradient(0, 0, 0, 280);
        gradient.addColorStop(0, 'rgba(79, 70, 229, 0.3)');
        gradient.addColorStop(1, 'rgba(79, 70, 229, 0.02)');

        this.bandwidthChart = new Chart(ctx, {
            type: 'line',
            data: {
                labels: this.chartData.labels,
                datasets: [
                    {
                        label: '带宽 (Mbps)',
                        data: this.chartData.speeds,
                        borderColor: '#4f46e5',
                        backgroundColor: gradient,
                        borderWidth: 2,
                        fill: true,
                        tension: 0.3,
                        pointRadius: 0,
                        pointHoverRadius: 5,
                        pointHoverBackgroundColor: '#4f46e5',
                        yAxisID: 'y',
                    },
                ],
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                animation: { duration: 200 },
                plugins: {
                    legend: {
                        display: true,
                        position: 'top',
                        labels: { boxWidth: 12, padding: 12, font: { size: 12 } },
                    },
                    tooltip: {
                        mode: 'index',
                        intersect: false,
                        callbacks: {
                            label: (ctx) => {
                                const val = ctx.parsed.y;
                                return `${ctx.dataset.label}: ${val.toFixed(2)} Mbps`;
                            },
                        },
                    },
                },
                scales: {
                    x: {
                        title: { display: true, text: '时间 (秒)', font: { size: 11 } },
                        grid: { display: false },
                        ticks: { font: { size: 10 } },
                    },
                    y: {
                        title: { display: true, text: '带宽 (Mbps)', font: { size: 11 } },
                        beginAtZero: true,
                        grid: { color: 'rgba(0,0,0,0.05)' },
                        ticks: {
                            font: { size: 10 },
                            callback: (val) => val.toFixed(0),
                        },
                    },
                },
                interaction: {
                    mode: 'nearest',
                    axis: 'x',
                    intersect: false,
                },
            },
        });

        return this.bandwidthChart;
    }

    /**
     * 添加数据点
     */
    addDataPoint(time, speed, retransmits = 0) {
        this.chartData.labels.push(time.toFixed(1));
        this.chartData.speeds.push(speed);
        this.chartData.retransmits.push(retransmits);

        // 限制数据点数量
        if (this.chartData.labels.length > this.maxDataPoints) {
            this.chartData.labels.shift();
            this.chartData.speeds.shift();
            this.chartData.retransmits.shift();
        }

        if (this.bandwidthChart) {
            this.bandwidthChart.update('none');
        }
    }

    /**
     * 加载历史数据到图表
     */
    loadHistoryData(intervals) {
        this.clearData();
        intervals.forEach((item) => {
            const timeInfo = item.time || {};
            const time = timeInfo.seconds || timeInfo.end || 0;
            const speed = (item.bits_per_second || 0) / 1_000_000;
            this.addDataPoint(time, speed, item.retransmits || 0);
        });
    }

    /**
     * 清空图表数据
     */
    clearData() {
        this.chartData.labels = [];
        this.chartData.speeds = [];
        this.chartData.retransmits = [];
        if (this.bandwidthChart) {
            this.bandwidthChart.update();
        }
    }

    /**
     * 销毁图表
     */
    destroy() {
        if (this.bandwidthChart) {
            this.bandwidthChart.destroy();
            this.bandwidthChart = null;
        }
    }
}
