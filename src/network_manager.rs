use serde::{Deserialize, Serialize};
use std::process::Command;

/// 网络管理器
pub struct NetworkManager;

/// Wi-Fi 状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiStatus {
    pub available: bool,
    pub interface: Option<String>,
    pub power_on: bool,
    pub connected: bool,
    pub ssid: Option<String>,
    pub ip_address: Option<String>,
}

impl NetworkManager {
    /// 获取 Wi-Fi 状态
    pub fn get_wifi_status() -> WifiStatus {
        let os = std::env::consts::OS;
        let interface = Self::detect_wifi_interface();
        let available = interface.is_some();

        let mut status = WifiStatus {
            available,
            interface: interface.clone(),
            power_on: false,
            connected: false,
            ssid: None,
            ip_address: None,
        };

        if let Some(ref iface) = interface {
            match os {
                "macos" => {
                    status = Self::get_macos_wifi_status(iface, status);
                }
                "windows" => {
                    status = Self::get_windows_wifi_status(iface, status);
                }
                "linux" => {
                    status = Self::get_linux_wifi_status(iface, status);
                }
                _ => {}
            }

            // 获取 IP 地址
            status.ip_address = Self::get_interface_ip(iface);
        }

        status
    }

    fn detect_wifi_interface() -> Option<String> {
        let os = std::env::consts::OS;
        match os {
            "macos" => {
                // 使用 networksetup 检测 Wi-Fi 接口
                if let Ok(output) = Command::new("networksetup")
                    .args(["-listallhardwareports"])
                    .output()
                {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let mut current_port = String::new();
                    for line in stdout.lines() {
                        if line.contains("Hardware Port") {
                            current_port = line.split(':').last().unwrap_or("").trim().to_string();
                        }
                        if line.contains("Device") && (current_port.contains("Wi-Fi") || current_port.contains("AirPort")) {
                            return line.split(':').last().map(|s| s.trim().to_string());
                        }
                    }
                }
                Some("en0".to_string())
            }
            "windows" => {
                if let Ok(output) = Command::new("netsh")
                    .args(["interface", "show", "interface"])
                    .output()
                {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        if line.contains("Wi-Fi") || line.contains("Wireless") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 4 {
                                return Some(parts[parts.len() - 1].to_string());
                            }
                        }
                    }
                }
                Some("Wi-Fi".to_string())
            }
            "linux" => {
                if let Ok(output) = Command::new("iwconfig").output() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        if let Some(name) = line.split_whitespace().next() {
                            return Some(name.to_string());
                        }
                    }
                }
                Some("wlan0".to_string())
            }
            _ => None,
        }
    }

    fn get_macos_wifi_status(iface: &str, mut status: WifiStatus) -> WifiStatus {
        // 检查电源状态
        if let Ok(output) = Command::new("networksetup")
            .args(["-getairportpower", iface])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            status.power_on = stdout.contains("On");
        }

        if status.power_on {
            // 获取 SSID
            if let Ok(output) = Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
                .args(["-I"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("SSID") && !line.to_lowercase().contains("none") {
                        status.ssid = line.split(':').last().map(|s| s.trim().to_string());
                    }
                    if line.contains("link auth") && status.ssid.is_some() {
                        status.connected = true;
                    }
                }
            }
            if status.ssid.is_some() {
                status.connected = true;
            }
        }
        status
    }

    fn get_windows_wifi_status(iface: &str, mut status: WifiStatus) -> WifiStatus {
        if let Ok(output) = Command::new("netsh")
            .args(["interface", "show", "interface", &format!("name={}", iface)])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            status.power_on = stdout.contains("Connected") || stdout.contains("Enabled");
            status.connected = stdout.contains("Connected");
        }

        if status.connected {
            if let Ok(output) = Command::new("netsh")
                .args(["wlan", "show", "interfaces"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("SSID") && !line.contains("BSSID") {
                        status.ssid = line.split(':').last().map(|s| s.trim().to_string());
                        break;
                    }
                }
            }
        }
        status
    }

    fn get_linux_wifi_status(iface: &str, mut status: WifiStatus) -> WifiStatus {
        if let Ok(output) = Command::new("ip")
            .args(["link", "show", iface])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            status.power_on = stdout.contains("UP");
            status.connected = status.power_on;
        }
        status
    }

    fn get_interface_ip(iface: &str) -> Option<String> {
        let os = std::env::consts::OS;
        match os {
            "macos" => {
                if let Ok(output) = Command::new("ipconfig")
                    .args(["getifaddr", iface])
                    .output()
                {
                    let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !ip.is_empty() {
                        return Some(ip);
                    }
                }
            }
            "windows" | "linux" => {
                // 尝试用系统命令获取 IP
                if let Ok(output) = Command::new("ip")
                    .args(["addr", "show", iface])
                    .output()
                {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("inet ") {
                            if let Some(ip) = trimmed.split_whitespace().nth(1) {
                                if let Some(addr) = ip.split('/').next() {
                                    return Some(addr.to_string());
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// 禁用无线网卡
    pub fn disable_wifi() -> bool {
        let os = std::env::consts::OS;
        let iface = Self::detect_wifi_interface().unwrap_or_default();

        match os {
            "macos" => {
                Command::new("networksetup")
                    .args(["-setairportpower", &iface, "off"])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            }
            "windows" => {
                Command::new("netsh")
                    .args(["interface", "set", "interface", &format!("name={}", iface), "admin=disable"])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            }
            "linux" => {
                Command::new("ip")
                    .args(["link", "set", &iface, "down"])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    /// 启用无线网卡
    pub fn enable_wifi() -> bool {
        let os = std::env::consts::OS;
        let iface = Self::detect_wifi_interface().unwrap_or_default();

        match os {
            "macos" => {
                Command::new("networksetup")
                    .args(["-setairportpower", &iface, "on"])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            }
            "windows" => {
                Command::new("netsh")
                    .args(["interface", "set", "interface", &format!("name={}", iface), "admin=enable"])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            }
            "linux" => {
                Command::new("ip")
                    .args(["link", "set", &iface, "up"])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            }
            _ => false,
        }
    }
}
