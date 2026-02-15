//! Markdown 文件管理器：按日期组织，支持 YAML frontmatter 元数据

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdFrontmatter {
    pub created: String,
    pub tags: Option<Vec<String>>,
    pub entities: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdRecord {
    pub frontmatter: MdFrontmatter,
    pub content: String,
    pub file_path: String,
}

/// 从内容中取标题：第一行或前 30 字符，并清理用于文件名
fn slug_from_content(content: &str) -> String {
    let first_line = content.lines().next().unwrap_or("").trim();
    let title: String = if first_line.is_empty() {
        "未命名".to_string()
    } else if first_line.len() > 30 {
        first_line.chars().take(30).collect()
    } else {
        first_line.to_string()
    };
    title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' || c == ' ' { c } else { '_' })
        .collect::<String>()
        .trim()
        .replace(' ', "_")
        .chars()
        .take(40)
        .collect::<String>()
}

/// 生成新记忆的 Markdown 文件路径：memories/YYYY/MM/YYYYMMDD_HHMMSS_标题.md
pub fn memory_file_path(memories_dir: &Path, content: &str) -> PathBuf {
    let now = Utc::now();
    let slug = slug_from_content(content);
    let filename = format!(
        "{}_{}_{}.md",
        now.format("%Y%m%d"),
        now.format("%H%M%S"),
        if slug.is_empty() { "未命名" } else { slug.as_str() }
    );
    memories_dir
        .join(now.format("%Y").to_string())
        .join(now.format("%m").to_string())
        .join(filename)
}

/// 写入一条记忆到 Markdown 文件
pub fn write_memory(
    memories_dir: &Path,
    content: &str,
    tags: Option<&[String]>,
    entities: Option<&[String]>,
) -> Result<PathBuf, String> {
    let path = memory_file_path(memories_dir, content);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let created = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let tags_str = tags.map(|t| serde_json::to_string(t).unwrap_or_default());
    let entities_str = entities.map(|e| serde_json::to_string(e).unwrap_or_default());

    let mut front = String::from("---\n");
    front.push_str(&format!("created: {}\n", created));
    if let Some(ref t) = tags_str {
        front.push_str(&format!("tags: {}\n", t));
    }
    if let Some(ref e) = entities_str {
        front.push_str(&format!("entities: {}\n", e));
    }
    front.push_str("---\n\n");
    let body = content.trim();
    let full = format!("{}{}", front, body);

    fs::write(&path, full).map_err(|e| e.to_string())?;
    Ok(path)
}

/// 更新已存在文件的内容与元数据（路径不变）
pub fn update_memory_file(
    path: &Path,
    content: &str,
    tags: Option<&[String]>,
    entities: Option<&[String]>,
) -> Result<(), String> {
    let created = if let Ok(rec) = read_memory(path) {
        rec.frontmatter.created
    } else {
        Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
    };
    let tags_str = tags.map(|t| serde_json::to_string(t).unwrap_or_default());
    let entities_str = entities.map(|e| serde_json::to_string(e).unwrap_or_default());

    let mut front = String::from("---\n");
    front.push_str(&format!("created: {}\n", created));
    if let Some(ref t) = tags_str {
        front.push_str(&format!("tags: {}\n", t));
    }
    if let Some(ref e) = entities_str {
        front.push_str(&format!("entities: {}\n", e));
    }
    front.push_str("---\n\n");
    let full = format!("{}{}", front, content.trim());
    fs::write(path, full).map_err(|e| e.to_string())?;
    Ok(())
}

/// 解析 frontmatter：简单逐行解析（不依赖 yaml crate）
fn parse_frontmatter(s: &str) -> Option<MdFrontmatter> {
    let s = s.trim();
    if !s.starts_with("---") {
        return None;
    }
    let rest = s.strip_prefix("---")?.trim();
    let end = rest.find("\n---")?;
    let block = rest.get(..end)?;
    let mut created = String::new();
    let mut tags: Option<Vec<String>> = None;
    let mut entities: Option<Vec<String>> = None;
    for line in block.lines() {
        let line = line.trim();
        if let Some(stripped) = line.strip_prefix("created:") {
            created = stripped.trim().to_string();
        } else if let Some(stripped) = line.strip_prefix("tags:") {
            let t = stripped.trim();
            if let Ok(v) = serde_json::from_str(t) {
                tags = Some(v);
            }
        } else if let Some(stripped) = line.strip_prefix("entities:") {
            let e = stripped.trim();
            if let Ok(v) = serde_json::from_str(e) {
                entities = Some(v);
            }
        }
    }
    if created.is_empty() {
        created = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    }
    Some(MdFrontmatter {
        created,
        tags,
        entities,
    })
}

/// 读取一个 Markdown 记忆文件
pub fn read_memory(path: &Path) -> Result<MdRecord, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let (frontmatter, content) = if raw.trim_start().starts_with("---") {
        let rest = raw.trim_start().strip_prefix("---").ok_or("invalid frontmatter")?;
        let end = rest.find("\n---").ok_or("invalid frontmatter")?;
        let block = rest.get(..end).ok_or("invalid frontmatter")?;
        let fm = parse_frontmatter(&format!("---\n{}\n---", block)).ok_or("parse frontmatter")?;
        let body = rest.get((end + 4)..).unwrap_or("").trim();
        (fm, body.to_string())
    } else {
        let fm = MdFrontmatter {
            created: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            tags: None,
            entities: None,
        };
        (fm, raw)
    };
    Ok(MdRecord {
        frontmatter,
        content,
        file_path: path.to_string_lossy().to_string(),
    })
}

/// 列出 memories 目录下所有 .md 文件（按修改时间倒序）
pub fn list_memory_files(memories_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !memories_dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = WalkDir::new(memories_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "md"))
        .map(|e| e.path().to_path_buf())
        .collect();
    paths.sort_by(|a, b| {
        let ta = fs::metadata(a).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let tb = fs::metadata(b).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        tb.cmp(&ta)
    });
    Ok(paths)
}
