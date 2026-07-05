import Cocoa

/// SpeedTest 启动器 — 提供真正的 Dock 图标支持，避免 CLI 二进制弹跳
class SpeedTestLauncher: NSObject, NSApplicationDelegate {
    var serverTask: Process?
    var statusItem: NSStatusItem?

    func applicationDidFinishLaunching(_ notification: Notification) {
        // 设置 Dock 图标（自动从 icon.icns 读取，macOS 自动加圆角）
        NSApp.setActivationPolicy(.regular)

        // 启动 Rust 后端
        startServer()

        // 创建菜单栏（确保有退出选项）
        let menu = NSMenu()
        menu.addItem(NSMenuItem(title: "关于 SpeedTest", action: #selector(showAbout), keyEquivalent: ""))
        menu.addItem(NSMenuItem.separator())
        menu.addItem(NSMenuItem(title: "退出 SpeedTest", action: #selector(quitApp), keyEquivalent: "q"))
        NSApp.mainMenu = menu

        // 菜单栏图标
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        if let button = statusItem?.button {
            button.title = "🚀"
            button.action = #selector(showMenu)
        }
        statusItem?.menu = menu

        // 打开浏览器
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.0) {
            if let url = URL(string: "http://127.0.0.1:5001") {
                NSWorkspace.shared.open(url)
            }
        }
    }

    func startServer() {
        let resourcePath = Bundle.main.resourcePath ?? ""
        let serverPath = resourcePath + "/speedtest_server"

        guard FileManager.default.fileExists(atPath: serverPath) else {
            print("speedtest_server not found at \(serverPath)")
            return
        }

        let task = Process()
        task.executableURL = URL(fileURLWithPath: serverPath)
        task.arguments = ["--no-browser"]
        task.currentDirectoryURL = URL(fileURLWithPath: resourcePath)

        // 不显示终端输出
        task.standardOutput = FileHandle.nullDevice
        task.standardError = FileHandle.nullDevice

        do {
            try task.run()
            serverTask = task
            print("SpeedTest server started")
        } catch {
            print("Failed to start server: \(error)")
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        // 1. 优雅关闭 Rust 后端（会触发 iperf3 清理）
        if let task = serverTask {
            if task.isRunning {
                task.terminate()  // SIGTERM
                // 等 3 秒让后端自己清理
                let deadline = DispatchTime.now() + .seconds(3)
                _ = DispatchQueue.main.sync { /* small delay */ }
                if task.isRunning {
                    task.terminate()  // 还没退出就再发一次
                }
                task.waitUntilExit()
            }
        }

        // 2. 兜底：直接杀掉残留的 iperf3 进程
        let killTask = Process()
        killTask.executableURL = URL(fileURLWithPath: "/usr/bin/pkill")
        killTask.arguments = ["-f", "iperf3 -s"]
        try? killTask.run()
        killTask.waitUntilExit()
    }

    @objc func quitApp() {
        NSApp.terminate(nil)
    }

    @objc func showAbout() {
        let alert = NSAlert()
        alert.messageText = "SpeedTest 网络链路速度测试"
        alert.informativeText = "基于 iperf3 的局域网链路质量检测工具\nRust 重写版 v1.0.0"
        alert.alertStyle = .informational
        alert.runModal()
    }

    @objc func showMenu() {
        statusItem?.button?.performClick(nil)
    }
}

let app = NSApplication.shared
let delegate = SpeedTestLauncher()
app.delegate = delegate
app.run()
