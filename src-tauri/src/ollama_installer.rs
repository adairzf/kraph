//! Download and open the Ollama installer via an in-app button.

use std::io::Write;
use std::path::PathBuf;

const OLLAMA_DOWNLOAD_PAGE: &str = "https://ollama.com/download";
const WINDOWS_INSTALLER_URL: &str = "https://ollama.com/download/OllamaSetup.exe";
const MACOS_INSTALLER_URL: &str = "https://ollama.com/download/Ollama.dmg";

/// Return the installer download URL for the current OS, or `None` for Linux.
fn get_installer_url() -> Option<&'static str> {
    match std::env::consts::OS {
        "windows" => Some(WINDOWS_INSTALLER_URL),
        "macos" => Some(MACOS_INSTALLER_URL),
        _ => None,
    }
}

/// Download a URL to a temporary file and return the path.
fn download_to_temp(url: &str, filename: &str) -> Result<PathBuf, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(url)
        .send()
        .map_err(|e| format!("Download failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Download returned error: {}", resp.status()));
    }
    let bytes = resp.bytes().map_err(|e| e.to_string())?;
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join(filename);
    let mut f = std::fs::File::create(&path).map_err(|e| format!("Failed to create temp file: {}", e))?;
    f.write_all(&bytes).map_err(|e| format!("Write failed: {}", e))?;
    f.sync_all().map_err(|e| e.to_string())?;
    Ok(path)
}

/// Open a file using the system default handler (i.e. run the installer).
fn open_installer(path: &std::path::Path) -> Result<(), String> {
    let _path_str = path.to_string_lossy();
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", _path_str.as_ref()])
            .spawn()
            .map_err(|e| format!("Failed to open installer: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open installer: {}", e))?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        return Err("Please open the download page in your browser for this platform.".to_string());
    }
    Ok(())
}

/// Download and launch the Ollama installer.
/// On Linux, opens the download page in the default browser instead.
pub fn download_and_open_ollama_installer() -> Result<String, String> {
    let url = match get_installer_url() {
        Some(u) => u,
        None => {
            open_download_page_in_browser()?;
            return Ok("Opened the Ollama download page in your browser. Please select the version for your system.".to_string());
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
            Ok("Ollama installer downloaded and opened. Please follow the prompts to complete installation.".to_string())
        }
        Err(e) => {
            let _ = open_download_page_in_browser();
            Err(format!(
                "{} — opened the download page in your browser as a fallback. Please install manually.",
                e
            ))
        }
    }
}

/// Open the Ollama download page in the system default browser.
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
        return Err("Cannot open browser on this platform.".to_string());
    }
    Ok(())
}
