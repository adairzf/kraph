//! 通过应用内按钮下载并打开 Ollama 安装程序

use std::io::Write;
use std::path::PathBuf;

const OLLAMA_DOWNLOAD_PAGE: &str = "https://ollama.com/download";
const WINDOWS_INSTALLER_URL: &str = "https://ollama.com/download/OllamaSetup.exe";
const MACOS_INSTALLER_URL: &str = "https://ollama.com/download/Ollama.dmg";

/// 获取当前系统对应的 Ollama 安装包下载 URL
fn get_installer_url() -> Option<&'static str> {
    match std::env::consts::OS {
        "windows" => Some(WINDOWS_INSTALLER_URL),
        "macos" => Some(MACOS_INSTALLER_URL),
        _ => None,
    }
}

/// 下载到临时文件并返回路径
fn download_to_temp(url: &str, filename: &str) -> Result<PathBuf, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(url)
        .send()
        .map_err(|e| format!("下载失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("下载返回错误: {}", resp.status()));
    }
    let bytes = resp.bytes().map_err(|e| e.to_string())?;
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join(filename);
    let mut f = std::fs::File::create(&path).map_err(|e| format!("创建临时文件失败: {}", e))?;
    f.write_all(&bytes).map_err(|e| format!("写入失败: {}", e))?;
    f.sync_all().map_err(|e| e.to_string())?;
    Ok(path)
}

/// 用系统默认方式打开文件（安装程序）
fn open_installer(path: &std::path::Path) -> Result<(), String> {
    let path_str = path.to_string_lossy();
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", path_str.as_ref()])
            .spawn()
            .map_err(|e| format!("打开安装程序失败: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("打开安装程序失败: {}", e))?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = path_str;
        return Err("当前系统请使用浏览器打开下载页".to_string());
    }
    Ok(())
}

/// 下载并打开 Ollama 安装程序；Linux 则只在浏览器打开下载页
pub fn download_and_open_ollama_installer() -> Result<String, String> {
    let url = match get_installer_url() {
        Some(u) => u,
        None => {
            open_download_page_in_browser()?;
            return Ok("已在浏览器打开 Ollama 下载页，请选择适合您系统的版本下载安装。".to_string());
        }
    };

    let filename = if std::env::consts::OS == "windows" {
        "OllamaSetup.exe"
    } else {
        "Ollama.dmg"
    };

    match download_to_temp(url, filename) {
        Ok(path) => {
            open_installer(&path)?;
            Ok("Ollama 安装包已下载并打开，请按提示完成安装。安装完成后可回到本应用使用。".to_string())
        }
        Err(e) => {
            let _ = open_download_page_in_browser();
            Err(format!(
                "{} 已改为在浏览器打开下载页，请手动下载安装。",
                e
            ))
        }
    }
}

/// 在默认浏览器中打开 Ollama 下载页
pub fn open_download_page_in_browser() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", OLLAMA_DOWNLOAD_PAGE])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(OLLAMA_DOWNLOAD_PAGE)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(OLLAMA_DOWNLOAD_PAGE)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        return Err("无法打开浏览器".to_string());
    }
    Ok(())
}
