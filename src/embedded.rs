use rust_embed::Embed;
use std::path::PathBuf;

/// 嵌入的 iperf3 二进制文件（从 bin/ 目录编译时嵌入）
/// 注意：编译时 bin/ 目录中应包含 iperf3.exe 及其依赖的 DLL
#[derive(Embed)]
#[folder = "bin/"]
struct EmbeddedBinaries;

/// 嵌入的前端文件（从 frontend/ 目录编译时嵌入）
#[derive(Embed)]
#[folder = "frontend/"]
struct EmbeddedFrontend;

/// 提取嵌入的二进制文件到指定目录
/// 已知的二进制文件名列表（编译时 bin/ 中的文件，在此列出以便提取）
/// 如果依赖的 DLL 变了，更新此列表即可
const BINARY_FILES: &[&str] = &[
    "iperf3.exe",
    "iperf3",
    "cygwin1.dll",
    "cygcrypto-3.dll",
    "cygssl-3.dll",
    "cygz.dll",
    "libcrypto-3.dll",
    "libssl-3.dll",
    "msvcr100.dll",
    "msvcp100.dll",
    "vcruntime140.dll",
    "vcruntime140_1.dll",
    "msvcp140.dll",
];

pub fn extract_binaries(target_dir: &PathBuf) {
    let mut extracted_count = 0;

    for &name in BINARY_FILES {
        try_extract::<EmbeddedBinaries>(name, target_dir, &mut extracted_count);
    }

    if extracted_count > 0 {
        println!("  📦 已提取 {} 个嵌入的二进制文件到 {}", extracted_count, target_dir.display());
    }
}

/// 尝试从嵌入中提取单个文件
fn try_extract<T: rust_embed::Embed>(name: &str, target_dir: &PathBuf, count: &mut u32) {
    if let Some(file) = T::get(name) {
        let target_path = target_dir.join(name);
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if std::fs::write(&target_path, &file.data).is_ok() {
            // 设置可执行权限（Unix）
            #[cfg(not(target_os = "windows"))]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(0o755)).ok();
            }
            *count += 1;
        }
    }
}

/// 获取嵌入的前端文件
pub fn get_frontend_file(path: &str) -> Option<rust_embed::EmbeddedFile> {
    // 标准化路径：去除开头的 /
    let clean_path = path.trim_start_matches('/');
    EmbeddedFrontend::get(clean_path)
}

/// 检查前端是否有嵌入的文件
pub fn has_embedded_frontend() -> bool {
    EmbeddedFrontend::get("index.html").is_some()
}
