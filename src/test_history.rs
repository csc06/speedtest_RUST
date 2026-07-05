use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::iperf_controller::TestResult;

/// 历史记录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub id: String,
    pub timestamp: String,
    pub result: TestResult,
}

/// 历史统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryStatistics {
    pub total_tests: usize,
    pub avg_speed: f64,
    pub max_speed: f64,
    pub min_speed: f64,
}

/// 测试历史管理器
pub struct TestHistory {
    history_file: PathBuf,
    records: Mutex<Vec<HistoryRecord>>,
}

impl TestHistory {
    pub fn new(history_file: PathBuf) -> Self {
        let history = TestHistory {
            history_file,
            records: Mutex::new(Vec::new()),
        };
        history.load();
        history
    }

    fn load(&self) {
        if self.history_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.history_file) {
                if let Ok(records) = serde_json::from_str::<Vec<HistoryRecord>>(&content) {
                    let mut data = self.records.lock().unwrap();
                    *data = records;
                }
            }
        }
    }

    fn save(&self) {
        if let Some(dir) = self.history_file.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        let data = self.records.lock().unwrap();
        if let Ok(content) = serde_json::to_string_pretty(&*data) {
            std::fs::write(&self.history_file, content).ok();
        }
    }

    /// 添加记录
    pub fn add_record(&self, result: TestResult) -> String {
        let id = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let record = HistoryRecord {
            id: id.clone(),
            timestamp: chrono::Local::now().to_rfc3339(),
            result,
        };

        let mut records = self.records.lock().unwrap();
        records.insert(0, record);
        drop(records);

        self.save();
        id
    }

    /// 获取所有记录
    pub fn get_all(&self, limit: usize) -> Vec<HistoryRecord> {
        let records = self.records.lock().unwrap();
        records.iter().take(limit).cloned().collect()
    }

    /// 根据 ID 获取记录
    pub fn get_by_id(&self, record_id: &str) -> Option<HistoryRecord> {
        let records = self.records.lock().unwrap();
        records.iter().find(|r| r.id == record_id).cloned()
    }

    /// 删除记录
    pub fn delete_record(&self, record_id: &str) -> bool {
        let mut records = self.records.lock().unwrap();
        let len = records.len();
        records.retain(|r| r.id != record_id);
        let deleted = records.len() < len;
        drop(records);
        if deleted {
            self.save();
        }
        deleted
    }

    /// 清空所有记录
    pub fn clear_all(&self) {
        let mut records = self.records.lock().unwrap();
        records.clear();
        drop(records);
        self.save();
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> HistoryStatistics {
        let records = self.records.lock().unwrap();

        if records.is_empty() {
            return HistoryStatistics {
                total_tests: 0,
                avg_speed: 0.0,
                max_speed: 0.0,
                min_speed: 0.0,
            };
        }

        let speeds: Vec<f64> = records
            .iter()
            .map(|r| r.result.summary.avg_mbps)
            .filter(|&s| s > 0.0)
            .collect();

        if speeds.is_empty() {
            return HistoryStatistics {
                total_tests: records.len(),
                avg_speed: 0.0,
                max_speed: 0.0,
                min_speed: 0.0,
            };
        }

        let sum: f64 = speeds.iter().sum();
        let max = speeds.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = speeds.iter().cloned().fold(f64::INFINITY, f64::min);

        HistoryStatistics {
            total_tests: records.len(),
            avg_speed: (sum / speeds.len() as f64 * 100.0).round() / 100.0,
            max_speed: (max * 100.0).round() / 100.0,
            min_speed: (min * 100.0).round() / 100.0,
        }
    }
}
