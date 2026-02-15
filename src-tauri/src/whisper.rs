use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::Utc;
use reqwest::header::ACCEPT_ENCODING;
use reqwest::blocking::Client;
use std::env;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use std::time::Instant;

fn parse_audio_base64(input: &str) -> Result<Vec<u8>, String> {
    let payload = input
        .split(',')
        .next_back()
        .ok_or("无效音频数据：base64 为空")?;
    BASE64_STANDARD
        .decode(payload.trim())
        .map_err(|e| format!("base64 解码失败: {e}"))
}

fn resolve_model_path(app_data_dir: &Path) -> Option<PathBuf> {
    if let Ok(v) = env::var("WHISPER_MODEL_PATH") {
        let p = PathBuf::from(v);
        if p.exists() {
            return Some(p);
        }
    }

    let candidates = [
        app_data_dir.join("whisper/models/ggml-base.bin"),
        app_data_dir.join("models/ggml-base.bin"),
        PathBuf::from("/opt/homebrew/share/whisper/ggml-base.bin"),
        PathBuf::from("/usr/local/share/whisper/ggml-base.bin"),
    ];
    for c in candidates {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

fn resolve_whisper_bin() -> String {
    if let Ok(v) = env::var("WHISPER_CPP_PATH") {
        if !v.trim().is_empty() {
            return v;
        }
    }
    // 常见二进制名：whisper-cli（新）/main（旧）
    "whisper-cli".to_string()
}

fn default_model_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("whisper/models/ggml-base.bin")
}

fn model_file_looks_valid(path: &Path) -> bool {
    match fs::metadata(path) {
        // base 模型通常远大于 100MB，小于这个值几乎一定是下载不完整
        Ok(m) => m.len() > 100 * 1024 * 1024,
        Err(_) => false,
    }
}

fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--help")
        .output()
        .map(|o| o.status.success() || !o.stdout.is_empty() || !o.stderr.is_empty())
        .unwrap_or(false)
}

fn try_install_whisper_cli() -> Result<String, String> {
    if command_exists("whisper-cli") {
        return Ok("whisper-cli 已可用".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        if !command_exists("brew") {
            return Err(
                "未检测到 Homebrew，无法自动安装 whisper-cpp。请先安装 Homebrew：https://brew.sh".to_string(),
            );
        }
        let output = Command::new("brew")
            .arg("install")
            .arg("whisper-cpp")
            .output()
            .map_err(|e| format!("自动安装 whisper-cpp 失败: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("brew install whisper-cpp 失败：{stderr}"));
        }
        if !command_exists("whisper-cli") {
            return Err("已执行安装，但仍未找到 whisper-cli，请重启终端后重试。".to_string());
        }
        return Ok("已自动安装 whisper-cpp".to_string());
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("当前系统暂未实现自动安装 whisper-cpp，请手动安装 whisper-cli。".to_string())
    }
}

fn ensure_model_downloaded(app_data_dir: &Path) -> Result<PathBuf, String> {
    if let Some(p) = resolve_model_path(app_data_dir) {
        if !model_file_looks_valid(&p) {
            let _ = fs::remove_file(&p);
        } else {
            return Ok(p);
        }
    }
    download_model(app_data_dir)
}

fn force_redownload_model(app_data_dir: &Path) -> Result<PathBuf, String> {
    let p = default_model_path(app_data_dir);
    let _ = fs::remove_file(&p);
    download_model(app_data_dir)
}

fn download_model(app_data_dir: &Path) -> Result<PathBuf, String> {
    if let Some(p) = resolve_model_path(app_data_dir) {
        if model_file_looks_valid(&p) {
            return Ok(p);
        }
        let _ = fs::remove_file(&p);
    }

    let model_path = default_model_path(app_data_dir);
    if let Some(parent) = model_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建模型目录失败: {e}"))?;
    }

    let url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin";
    let mut last_err = String::new();
    for attempt in 1..=3 {
        match download_model_once(url, &model_path) {
            Ok(()) => return Ok(model_path),
            Err(e) => {
                last_err = format!("第 {attempt} 次下载失败: {e}");
                // 网络抖动常见，短暂退避后重试
                thread::sleep(Duration::from_millis(700 * attempt as u64));
            }
        }
    }

    // 某些网络环境 reqwest 可能会报 decoding response body，这里增加 curl 兜底
    if let Err(e) = download_model_with_curl(url, &model_path) {
        return Err(format!(
            "{last_err}\n且 curl 兜底下载失败: {e}\n请检查网络后重试。"
        ));
    }

    if !model_file_looks_valid(&model_path) {
        let _ = fs::remove_file(&model_path);
        return Err("模型下载后校验失败（文件疑似不完整），请重试。".to_string());
    }
    Ok(model_path)
}

fn should_redownload_model(stderr: &str) -> bool {
    let s = stderr.to_lowercase();
    s.contains("not all tensors loaded")
        || s.contains("failed to load model")
        || s.contains("failed to initialize whisper context")
}

fn run_whisper_once(
    whisper_bin: &str,
    model_path: &Path,
    input_path: &Path,
    out_prefix: &Path,
) -> Result<String, String> {
    let out_txt = out_prefix.with_extension("txt");
    let mut child = Command::new(whisper_bin)
        // 关闭 Metal 路径，优先保证稳定性（避免部分机型/驱动触发 SIGABRT）
        .env("GGML_METAL", "0")
        .arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(input_path)
        .arg("-l")
        .arg("zh")
        // 明确要求以简体中文输出，减少繁体结果
        .arg("--prompt")
        .arg("请使用简体中文输出，不要使用繁体字。")
        .arg("-otxt")
        .arg("-of")
        .arg(out_prefix)
        .arg("-nt")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "启动 Whisper 失败，请确认已安装 whisper.cpp 且命令可执行（当前: {whisper_bin}）: {e}"
            )
        })?;

    let timeout = Duration::from_secs(120);
    let start = Instant::now();
    loop {
        match child
            .try_wait()
            .map_err(|e| format!("检查 Whisper 状态失败: {e}"))?
        {
            Some(_) => break,
            None => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("Whisper 转写超时（超过 120 秒）。请缩短录音时长后重试。".to_string());
                }
                thread::sleep(Duration::from_millis(200));
            }
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("读取 Whisper 输出失败: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        return Err(format!(
            "Whisper 转写失败（exit={}）。\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        ));
    }

    let text = fs::read_to_string(&out_txt)
        .map_err(|e| format!("读取 Whisper 输出失败: {e}"))?
        .trim()
        .to_string();
    let _ = fs::remove_file(&out_txt);
    Ok(text)
}

fn extract_stderr(err: &str) -> String {
    if let Some(idx) = err.find("stderr:\n") {
        return err[(idx + "stderr:\n".len())..].to_string();
    }
    err.to_string()
}

fn download_model_once(url: &str, model_path: &Path) -> Result<(), String> {
    let tmp_path = model_path.with_extension("bin.tmp");
    let client = Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| format!("初始化下载客户端失败: {e}"))?;

    let mut resp = client
        .get(url)
        // 避免部分网络/代理在自动解压阶段触发 decode body 错误
        .header(ACCEPT_ENCODING, "identity")
        .send()
        .map_err(|e| format!("请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let mut file = fs::File::create(&tmp_path).map_err(|e| format!("创建临时模型文件失败: {e}"))?;
    io::copy(&mut resp, &mut file).map_err(|e| format!("写入模型文件失败: {e}"))?;
    file.flush().map_err(|e| format!("刷新模型文件失败: {e}"))?;
    file.sync_all().map_err(|e| format!("同步模型文件失败: {e}"))?;

    fs::rename(&tmp_path, model_path).map_err(|e| format!("落盘模型文件失败: {e}"))?;
    Ok(())
}

fn download_model_with_curl(url: &str, model_path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let _ = url;
        let _ = model_path;
        return Err("Windows 暂未实现 curl 兜底下载".to_string());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let status = Command::new("curl")
            .arg("-L")
            .arg("--retry")
            .arg("3")
            .arg("--retry-delay")
            .arg("1")
            .arg(url)
            .arg("-o")
            .arg(model_path)
            .status()
            .map_err(|e| format!("调用 curl 失败: {e}"))?;
        if status.success() {
            return Ok(());
        }
        Err(format!("curl 返回非 0 状态: {status}"))
    }
}

pub fn setup_whisper(app_data_dir: &Path) -> Result<String, String> {
    let install_msg = try_install_whisper_cli()?;
    let model_path = ensure_model_downloaded(app_data_dir)?;
    Ok(format!(
        "{install_msg}；模型已就绪：{}",
        model_path.to_string_lossy()
    ))
}

pub fn transcribe_audio_with_whisper(audio_base64: &str, app_data_dir: &Path) -> Result<String, String> {
    let audio_bytes = parse_audio_base64(audio_base64)?;
    let model_path = resolve_model_path(app_data_dir).ok_or_else(|| {
        "Whisper 模型未就绪，请先点击语音按钮完成初始化后再试。".to_string()
    })?;

    let whisper_bin = resolve_whisper_bin();
    let temp_dir = app_data_dir.join("temp").join("whisper");
    fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败: {e}"))?;

    let stamp = Utc::now().timestamp_millis();
    let input_path = temp_dir.join(format!("input_{stamp}.wav"));
    let out_prefix = temp_dir.join(format!("result_{stamp}"));

    fs::write(&input_path, &audio_bytes).map_err(|e| format!("写入音频失败: {e}"))?;

    let first = run_whisper_once(&whisper_bin, &model_path, &input_path, &out_prefix);
    let text = match first {
        Ok(t) => t,
        Err(e) => {
            let stderr = extract_stderr(&e);
            if should_redownload_model(&stderr) {
                let repaired = force_redownload_model(app_data_dir)?;
                run_whisper_once(&whisper_bin, &repaired, &input_path, &out_prefix)?
            } else {
                let _ = fs::remove_file(&input_path);
                return Err(e);
            }
        }
    };
    let _ = fs::remove_file(&input_path);

    if text.is_empty() {
        return Ok(String::new());
    }
    Ok(text)
}
