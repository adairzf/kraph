//! Markdown file manager: date-based directory layout with YAML frontmatter metadata.

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

/// Derive a URL-safe slug from the content's first line (max 30 chars).
fn slug_from_content(content: &str) -> String {
    let first_line = content.lines().next().unwrap_or("").trim();
    let title: String = if first_line.is_empty() {
        "untitled".to_string()
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

/// Build the file path for a new memory: `memories/YYYY/MM/YYYYMMDD_HHMMSS_<slug>.md`
pub fn memory_file_path(memories_dir: &Path, content: &str) -> PathBuf {
    let now = Utc::now();
    let slug = slug_from_content(content);
    let filename = format!(
        "{}_{}_{}.md",
        now.format("%Y%m%d"),
        now.format("%H%M%S"),
        if slug.is_empty() { "untitled" } else { slug.as_str() }
    );
    memories_dir
        .join(now.format("%Y").to_string())
        .join(now.format("%m").to_string())
        .join(filename)
}

/// Write a memory to a new Markdown file.
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

/// Parse YAML frontmatter using a simple line-by-line parser (no external YAML crate needed).
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

/// Read and parse a Markdown memory file.
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

/// List all `.md` files under the memories directory, sorted by modification time (newest first).
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
