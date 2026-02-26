mod database;
mod file_manager;
mod model_client;
mod model_config;
mod ollama;
mod ollama_installer;
mod whisper;

use chrono::{Datelike, Duration as ChronoDuration, Local, NaiveDate, Utc};
use database::{
    add_entity_alias, cleanup_database, clear_all_data, clear_memory_entities, delete_memory,
    find_entity_id_by_name_or_alias, get_entity_by_id, get_entity_by_name, get_graph_data,
    get_memories_for_entity, get_memory_by_id, init_db, insert_memory, link_memory_entity,
    list_memories, list_relations, merge_entities, prune_orphan_entities_and_relations,
    update_memory, upsert_entity, upsert_relation, DbState, Entity, GraphData, Memory,
};
use file_manager::{list_memory_files, read_memory, write_memory, MdRecord};
use model_client::{call_model_extract, call_model_fusion, call_model_simple};
use model_config::{ModelConfig, ModelProvider};
use ollama::{
    call_ollama_extract_blocking, check_ollama_status, ensure_model_available,
    ensure_ollama_running, ExtractedData, ExtractedEntity, ExtractedRelation,
    ENTITY_EXTRACT_PROMPT, KNOWLEDGE_FUSION_PROMPT,
};
use ollama_installer::download_and_open_ollama_installer;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};
use whisper::{setup_whisper as setup_whisper_runtime, transcribe_audio_with_whisper};

pub struct AppRootDir(pub PathBuf);
pub struct AppDataDir(pub Mutex<PathBuf>);
pub struct CurrentLibraryId(pub Mutex<String>);
pub struct ModelConfigState(pub Mutex<ModelConfig>);

#[derive(Debug, Clone, Deserialize)]
struct StoryGenerationRequest {
    key_events: Vec<String>,
    #[serde(default)]
    genre: Option<String>,
    #[serde(default)]
    style: Option<String>,
    #[serde(default)]
    protagonist: Option<String>,
    #[serde(default)]
    chapter_count: Option<usize>,
    #[serde(default)]
    constraints: Option<String>,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoryChapterPlan {
    chapter: usize,
    title: String,
    goal: String,
    conflict: String,
    twist: String,
    hook: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoryGenerationResult {
    title: String,
    premise: String,
    outline: Vec<String>,
    chapter_plan: Vec<StoryChapterPlan>,
    first_chapter: String,
    continuity_checks: Vec<String>,
}

#[derive(Debug, Clone)]
struct StoryPromptContext {
    memories: Vec<String>,
    entities: Vec<String>,
    relations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryLibraryInfo {
    id: String,
    name: String,
    path: String,
    is_current: bool,
    #[serde(default)]
    enable_time_normalization: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LibraryMeta {
    name: String,
    created_at: String,
    #[serde(default)]
    enable_time_normalization: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExternalPluginManifestFile {
    id: String,
    name: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    tab_key: Option<String>,
    #[serde(default)]
    menu_key: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExternalPluginInfo {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    tab_key: Option<String>,
    #[serde(default)]
    menu_key: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    entry: Option<String>,
    #[serde(default)]
    entry_path: Option<String>,
    install_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoryWrittenChapter {
    chapter: usize,
    title: String,
    content: String,
    #[serde(default)]
    summary: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct StoryContinuationRequest {
    title: String,
    premise: String,
    #[serde(default)]
    outline: Vec<String>,
    #[serde(default)]
    chapter_plan: Vec<StoryChapterPlan>,
    #[serde(default)]
    continuity_checks: Vec<String>,
    #[serde(default)]
    written_chapters: Vec<StoryWrittenChapter>,
    #[serde(default)]
    target_chapter: Option<usize>,
    #[serde(default)]
    style: Option<String>,
    #[serde(default)]
    constraints: Option<String>,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoryContinuationResult {
    chapter: usize,
    title: String,
    content: String,
    summary: String,
}

#[derive(Debug, Clone, Deserialize)]
struct StoryProjectSaveRequest {
    #[serde(default)]
    project_id: Option<String>,
    title: String,
    premise: String,
    #[serde(default)]
    outline: Vec<String>,
    #[serde(default)]
    chapter_plan: Vec<StoryChapterPlan>,
    #[serde(default)]
    continuity_checks: Vec<String>,
    #[serde(default)]
    written_chapters: Vec<StoryWrittenChapter>,
    #[serde(default)]
    style: Option<String>,
    #[serde(default)]
    constraints: Option<String>,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoryProjectSummary {
    id: String,
    title: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoryProjectData {
    project_id: String,
    title: String,
    premise: String,
    outline: Vec<String>,
    chapter_plan: Vec<StoryChapterPlan>,
    continuity_checks: Vec<String>,
    written_chapters: Vec<StoryWrittenChapter>,
    #[serde(default)]
    style: Option<String>,
    #[serde(default)]
    constraints: Option<String>,
    #[serde(default)]
    language: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct StoryRewriteChapterRequest {
    title: String,
    premise: String,
    #[serde(default)]
    outline: Vec<String>,
    #[serde(default)]
    chapter_plan: Vec<StoryChapterPlan>,
    #[serde(default)]
    continuity_checks: Vec<String>,
    #[serde(default)]
    written_chapters: Vec<StoryWrittenChapter>,
    target_chapter: usize,
    #[serde(default)]
    feedback: Option<String>,
    #[serde(default)]
    style: Option<String>,
    #[serde(default)]
    constraints: Option<String>,
    #[serde(default)]
    language: Option<String>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn get_current_data_dir(data_dir: &State<AppDataDir>) -> Result<PathBuf, String> {
    data_dir
        .0
        .lock()
        .map(|p| p.clone())
        .map_err(|e| e.to_string())
}

fn get_current_library_id(current_library: &State<CurrentLibraryId>) -> Result<String, String> {
    current_library
        .0
        .lock()
        .map(|id| id.clone())
        .map_err(|e| e.to_string())
}

fn libraries_root(app_root: &Path) -> PathBuf {
    app_root.join("libraries")
}

fn current_library_file(app_root: &Path) -> PathBuf {
    app_root.join("current_library.txt")
}

fn library_meta_path(library_dir: &Path) -> PathBuf {
    library_dir.join("library.json")
}

fn library_model_config_path(library_dir: &Path) -> PathBuf {
    library_dir.join("model_config.json")
}

fn story_projects_root(data_dir: &Path) -> PathBuf {
    data_dir.join("novels")
}

fn story_project_file_path(data_dir: &Path, project_id: &str) -> PathBuf {
    story_projects_root(data_dir).join(format!("{project_id}.json"))
}

fn plugins_root(app_root: &Path) -> PathBuf {
    app_root.join("plugins")
}

fn plugin_manifest_path(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join("plugin.manifest.json")
}

fn normalize_library_id(name: &str) -> String {
    let mut id = String::new();
    let mut last_dash = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            id.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !id.is_empty() {
            id.push('-');
            last_dash = true;
        }
    }
    while id.ends_with('-') {
        id.pop();
    }
    if id.is_empty() {
        format!("library-{}", Utc::now().format("%Y%m%d%H%M%S"))
    } else {
        id
    }
}

fn unique_library_id(root: &Path, base_id: &str) -> String {
    if !root.join(base_id).exists() {
        return base_id.to_string();
    }
    for i in 2..10000 {
        let candidate = format!("{base_id}-{i}");
        if !root.join(&candidate).exists() {
            return candidate;
        }
    }
    format!("{base_id}-{}", Utc::now().format("%Y%m%d%H%M%S"))
}

fn normalize_story_project_id(name: &str) -> String {
    let id = normalize_library_id(name);
    if id.is_empty() || id.starts_with("library-") {
        format!("story-{}", Utc::now().format("%Y%m%d%H%M%S"))
    } else {
        id
    }
}

fn normalize_plugin_id(id: &str) -> String {
    normalize_library_id(id)
}

fn validate_plugin_id(id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err("Plugin id cannot be empty.".to_string());
    }
    let normalized = normalize_plugin_id(id);
    if normalized != id {
        return Err("Plugin id must use lowercase letters, numbers, and dashes only.".to_string());
    }
    Ok(())
}

fn read_external_plugin_manifest(plugin_dir: &Path) -> Result<ExternalPluginManifestFile, String> {
    let path = plugin_manifest_path(plugin_dir);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read plugin manifest {:?}: {}", path, e))?;
    let mut manifest: ExternalPluginManifestFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse plugin manifest {:?}: {}", path, e))?;

    manifest.id = manifest.id.trim().to_string();
    manifest.name = manifest.name.trim().to_string();
    manifest.version = manifest
        .version
        .as_ref()
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty());
    manifest.tab_key = manifest
        .tab_key
        .as_ref()
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty());
    manifest.menu_key = manifest
        .menu_key
        .as_ref()
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty());
    manifest.description = manifest
        .description
        .as_ref()
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty());
    manifest.entry = manifest
        .entry
        .as_ref()
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty());

    if manifest.name.is_empty() {
        return Err("Plugin manifest field 'name' cannot be empty.".to_string());
    }
    if let Some(entry) = manifest.entry.as_ref() {
        let entry_path = Path::new(entry);
        if entry_path.is_absolute()
            || entry_path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(
                "Plugin manifest field 'entry' must be a relative path inside plugin directory."
                    .to_string(),
            );
        }
    }
    validate_plugin_id(&manifest.id)?;
    Ok(manifest)
}

fn build_external_plugin_info(plugin_dir: &Path) -> Result<ExternalPluginInfo, String> {
    let manifest = read_external_plugin_manifest(plugin_dir)?;
    let entry_path = manifest
        .entry
        .as_ref()
        .map(|entry| plugin_dir.join(entry))
        .map(|path| path.to_string_lossy().to_string());
    Ok(ExternalPluginInfo {
        id: manifest.id,
        name: manifest.name,
        version: manifest.version.unwrap_or_else(|| "0.1.0".to_string()),
        tab_key: manifest.tab_key,
        menu_key: manifest.menu_key,
        description: manifest.description,
        entry: manifest.entry,
        entry_path,
        install_path: plugin_dir.to_string_lossy().to_string(),
    })
}

fn copy_directory_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() || !src.is_dir() {
        return Err(format!("Source path is not a directory: {:?}", src));
    }
    fs::create_dir_all(dst).map_err(|e| format!("Failed to create directory {:?}: {}", dst, e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read {:?}: {}", src, e))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_directory_recursive(&src_path, &dst_path)?;
        } else if src_path.is_file() {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory {:?}: {}", parent, e))?;
            }
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {:?} -> {:?}: {}", src_path, dst_path, e))?;
        }
    }
    Ok(())
}

fn unique_story_project_id(root: &Path, base_id: &str) -> String {
    let path = root.join(format!("{base_id}.json"));
    if !path.exists() {
        return base_id.to_string();
    }
    for i in 2..10000 {
        let candidate = format!("{base_id}-{i}");
        if !root.join(format!("{candidate}.json")).exists() {
            return candidate;
        }
    }
    format!("{base_id}-{}", Utc::now().format("%Y%m%d%H%M%S"))
}

fn ensure_library_structure(library_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(library_dir)
        .map_err(|e| format!("Failed to create library directory: {e}"))?;
    fs::create_dir_all(library_dir.join("database"))
        .map_err(|e| format!("Failed to create database directory: {e}"))?;
    fs::create_dir_all(library_dir.join("memories"))
        .map_err(|e| format!("Failed to create memories directory: {e}"))?;
    Ok(())
}

fn move_path_if_exists(from: &Path, to: &Path) -> Result<(), String> {
    if !from.exists() || to.exists() {
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {e}"))?;
    }
    fs::rename(from, to).map_err(|e| format!("Failed to move {:?} to {:?}: {}", from, to, e))
}

fn load_library_meta(library_dir: &Path) -> Option<LibraryMeta> {
    let content = fs::read_to_string(library_meta_path(library_dir)).ok()?;
    serde_json::from_str::<LibraryMeta>(&content).ok()
}

fn load_library_name(library_dir: &Path, fallback: &str) -> String {
    let Some(meta) = load_library_meta(library_dir) else {
        return fallback.to_string();
    };
    let trimmed = meta.name.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn load_library_time_normalization(library_dir: &Path) -> bool {
    load_library_meta(library_dir)
        .map(|meta| meta.enable_time_normalization)
        .unwrap_or(false)
}

fn save_library_meta(
    library_dir: &Path,
    name: &str,
    enable_time_normalization: Option<bool>,
) -> Result<(), String> {
    let existing = load_library_meta(library_dir);
    let meta = LibraryMeta {
        name: name.trim().to_string(),
        created_at: existing
            .as_ref()
            .map(|x| x.created_at.clone())
            .filter(|x| !x.trim().is_empty())
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
        enable_time_normalization: enable_time_normalization
            .or_else(|| existing.as_ref().map(|x| x.enable_time_normalization))
            .unwrap_or(false),
    };
    let content = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(library_meta_path(library_dir), content)
        .map_err(|e| format!("Failed to save library metadata: {e}"))?;
    Ok(())
}

fn read_saved_current_library_id(app_root: &Path) -> Option<String> {
    let path = current_library_file(app_root);
    let content = fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn persist_current_library_id(app_root: &Path, library_id: &str) -> Result<(), String> {
    fs::write(current_library_file(app_root), library_id)
        .map_err(|e| format!("Failed to persist current library: {e}"))
}

fn build_library_info(library_dir: &Path, library_id: &str, current_id: &str) -> MemoryLibraryInfo {
    MemoryLibraryInfo {
        id: library_id.to_string(),
        name: load_library_name(library_dir, library_id),
        path: library_dir.to_string_lossy().to_string(),
        is_current: library_id == current_id,
        enable_time_normalization: load_library_time_normalization(library_dir),
    }
}

fn list_library_infos(app_root: &Path, current_id: &str) -> Result<Vec<MemoryLibraryInfo>, String> {
    let root = libraries_root(app_root);
    fs::create_dir_all(&root).map_err(|e| format!("Failed to create libraries root: {e}"))?;

    let mut libraries = Vec::new();
    for entry in fs::read_dir(&root).map_err(|e| format!("Failed to read libraries root: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        libraries.push(build_library_info(&path, &id, current_id));
    }
    libraries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(libraries)
}

fn switch_to_library_internal(
    library_id: String,
    app_root: &Path,
    db: &State<DbState>,
    data_dir: &State<AppDataDir>,
    current_library: &State<CurrentLibraryId>,
    config_state: &State<ModelConfigState>,
) -> Result<MemoryLibraryInfo, String> {
    let libraries_root = libraries_root(app_root);
    let library_dir = libraries_root.join(&library_id);
    if !library_dir.exists() || !library_dir.is_dir() {
        return Err(format!("Library '{}' does not exist.", library_id));
    }
    ensure_library_structure(&library_dir)?;

    let db_path = library_dir.join("database").join("kraph.db");
    let conn = init_db(&db_path).map_err(|e| e.to_string())?;

    {
        let mut db_guard = (&**db)
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        *db_guard = Some(conn);
    }
    {
        let mut path_guard = data_dir.0.lock().map_err(|e| e.to_string())?;
        *path_guard = library_dir.clone();
    }
    {
        let mut id_guard = current_library.0.lock().map_err(|e| e.to_string())?;
        *id_guard = library_id.clone();
    }
    persist_current_library_id(app_root, &library_id)?;

    // Load model config from the active library, defaulting when absent.
    let config_path = library_model_config_path(&library_dir);
    let model_config = ModelConfig::load_from_file(&config_path).unwrap_or_default();
    {
        let mut guard = config_state.0.lock().map_err(|e| e.to_string())?;
        *guard = model_config;
    }

    Ok(build_library_info(&library_dir, &library_id, &library_id))
}

#[tauri::command]
fn list_memories_dir(data_dir: State<AppDataDir>) -> Result<Vec<String>, String> {
    let data_dir = get_current_data_dir(&data_dir)?;
    let memories_dir = data_dir.join("memories");
    let paths = list_memory_files(&memories_dir)?;
    Ok(paths
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
fn list_memory_libraries(
    app_root: State<AppRootDir>,
    current_library: State<CurrentLibraryId>,
) -> Result<Vec<MemoryLibraryInfo>, String> {
    let current_id = get_current_library_id(&current_library)?;
    list_library_infos(&app_root.0, &current_id)
}

#[tauri::command]
fn get_current_memory_library(
    app_root: State<AppRootDir>,
    current_library: State<CurrentLibraryId>,
) -> Result<MemoryLibraryInfo, String> {
    let current_id = get_current_library_id(&current_library)?;
    let library_dir = libraries_root(&app_root.0).join(&current_id);
    if !library_dir.exists() {
        return Err("Current library not found".to_string());
    }
    Ok(build_library_info(&library_dir, &current_id, &current_id))
}

#[tauri::command]
fn create_memory_library(
    name: String,
    enable_time_normalization: Option<bool>,
    app_root: State<AppRootDir>,
    current_library: State<CurrentLibraryId>,
    config_state: State<ModelConfigState>,
) -> Result<MemoryLibraryInfo, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Library name cannot be empty.".to_string());
    }

    let root = libraries_root(&app_root.0);
    fs::create_dir_all(&root).map_err(|e| format!("Failed to create libraries root: {e}"))?;

    let base_id = normalize_library_id(trimmed);
    let library_id = unique_library_id(&root, &base_id);
    let library_dir = root.join(&library_id);
    ensure_library_structure(&library_dir)?;
    save_library_meta(
        &library_dir,
        trimmed,
        Some(enable_time_normalization.unwrap_or(false)),
    )?;

    // Initialize with current model config so the new library is ready immediately.
    let current_config = config_state.0.lock().map_err(|e| e.to_string())?.clone();
    current_config.save_to_file(&library_model_config_path(&library_dir))?;

    let current_id = get_current_library_id(&current_library)?;
    Ok(build_library_info(&library_dir, &library_id, &current_id))
}

#[tauri::command]
fn switch_memory_library(
    library_id: String,
    app_root: State<AppRootDir>,
    db: State<DbState>,
    data_dir: State<AppDataDir>,
    current_library: State<CurrentLibraryId>,
    config_state: State<ModelConfigState>,
) -> Result<MemoryLibraryInfo, String> {
    let library_id = library_id.trim();
    if library_id.is_empty() {
        return Err("Library id cannot be empty.".to_string());
    }
    switch_to_library_internal(
        library_id.to_string(),
        &app_root.0,
        &db,
        &data_dir,
        &current_library,
        &config_state,
    )
}

#[tauri::command]
fn rename_memory_library(
    library_id: String,
    name: String,
    app_root: State<AppRootDir>,
    current_library: State<CurrentLibraryId>,
) -> Result<MemoryLibraryInfo, String> {
    let library_id = library_id.trim();
    let name = name.trim();
    if library_id.is_empty() {
        return Err("Library id cannot be empty.".to_string());
    }
    if name.is_empty() {
        return Err("Library name cannot be empty.".to_string());
    }

    let dir = libraries_root(&app_root.0).join(library_id);
    if !dir.exists() || !dir.is_dir() {
        return Err(format!("Library '{}' does not exist.", library_id));
    }
    save_library_meta(&dir, name, None)?;

    let current_id = get_current_library_id(&current_library)?;
    Ok(build_library_info(&dir, library_id, &current_id))
}

#[tauri::command]
fn delete_memory_library(
    library_id: String,
    app_root: State<AppRootDir>,
    db: State<DbState>,
    data_dir: State<AppDataDir>,
    current_library: State<CurrentLibraryId>,
    config_state: State<ModelConfigState>,
) -> Result<String, String> {
    let library_id = library_id.trim().to_string();
    if library_id.is_empty() {
        return Err("Library id cannot be empty.".to_string());
    }

    let root = libraries_root(&app_root.0);
    let target_dir = root.join(&library_id);
    if !target_dir.exists() || !target_dir.is_dir() {
        return Err(format!("Library '{}' does not exist.", library_id));
    }

    let current_id = get_current_library_id(&current_library)?;
    let libraries = list_library_infos(&app_root.0, &current_id)?;
    if libraries.len() <= 1 {
        return Err("Cannot delete the last remaining library.".to_string());
    }

    if library_id == current_id {
        let fallback = libraries
            .iter()
            .find(|x| x.id != library_id)
            .ok_or("No fallback library available.")?;

        switch_to_library_internal(
            fallback.id.clone(),
            &app_root.0,
            &db,
            &data_dir,
            &current_library,
            &config_state,
        )?;
    }

    fs::remove_dir_all(&target_dir).map_err(|e| format!("Failed to delete library: {e}"))?;
    Ok(library_id)
}

#[tauri::command]
fn save_story_project(
    request: StoryProjectSaveRequest,
    data_dir: State<AppDataDir>,
) -> Result<StoryProjectSummary, String> {
    let data_dir = get_current_data_dir(&data_dir)?;
    let root = story_projects_root(&data_dir);
    fs::create_dir_all(&root).map_err(|e| format!("Failed to create novels directory: {e}"))?;

    let title = request.title.trim();
    if title.is_empty() {
        return Err("Story title cannot be empty.".to_string());
    }

    let mut project_id = request
        .project_id
        .as_ref()
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .map(normalize_story_project_id)
        .unwrap_or_else(|| normalize_story_project_id(title));

    if request
        .project_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        project_id = unique_story_project_id(&root, &project_id);
    }

    let path = story_project_file_path(&data_dir, &project_id);
    let now = Utc::now().to_rfc3339();
    let created_at = if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<StoryProjectData>(&s).ok())
            .map(|p| p.created_at)
            .unwrap_or_else(|| now.clone())
    } else {
        now.clone()
    };

    let payload = StoryProjectData {
        project_id: project_id.clone(),
        title: title.to_string(),
        premise: request.premise.trim().to_string(),
        outline: request.outline,
        chapter_plan: request.chapter_plan,
        continuity_checks: request.continuity_checks,
        written_chapters: request.written_chapters,
        style: request
            .style
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty()),
        constraints: request
            .constraints
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty()),
        language: request
            .language
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty()),
        created_at,
        updated_at: now.clone(),
    };

    let content = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| format!("Failed to save story project: {e}"))?;

    Ok(StoryProjectSummary {
        id: project_id,
        title: payload.title,
        updated_at: now,
    })
}

#[tauri::command]
fn list_story_projects(data_dir: State<AppDataDir>) -> Result<Vec<StoryProjectSummary>, String> {
    let data_dir = get_current_data_dir(&data_dir)?;
    let root = story_projects_root(&data_dir);
    fs::create_dir_all(&root).map_err(|e| format!("Failed to create novels directory: {e}"))?;

    let mut items = Vec::new();
    for entry in fs::read_dir(&root).map_err(|e| format!("Failed to read novels directory: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(project) = serde_json::from_str::<StoryProjectData>(&content) {
                items.push(StoryProjectSummary {
                    id: project.project_id,
                    title: project.title,
                    updated_at: project.updated_at,
                });
            }
        }
    }

    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(items)
}

#[tauri::command]
fn load_story_project(
    project_id: String,
    data_dir: State<AppDataDir>,
) -> Result<StoryProjectData, String> {
    let project_id = project_id.trim();
    if project_id.is_empty() {
        return Err("Project id cannot be empty.".to_string());
    }
    let data_dir = get_current_data_dir(&data_dir)?;
    let path = story_project_file_path(&data_dir, project_id);
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read story project: {e}"))?;
    let project = serde_json::from_str::<StoryProjectData>(&content)
        .map_err(|e| format!("Failed to parse story project: {e}"))?;
    Ok(project)
}

fn open_folder_in_os(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
fn list_external_plugins(app_root: State<AppRootDir>) -> Result<Vec<ExternalPluginInfo>, String> {
    let root = plugins_root(&app_root.0);
    fs::create_dir_all(&root).map_err(|e| format!("Failed to create plugins directory: {e}"))?;

    let mut plugins = Vec::new();
    for entry in
        fs::read_dir(&root).map_err(|e| format!("Failed to read plugins directory: {e}"))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(info) = build_external_plugin_info(&path) {
            plugins.push(info);
        }
    }
    plugins.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(plugins)
}

#[tauri::command]
fn get_plugins_folder_path(app_root: State<AppRootDir>) -> Result<String, String> {
    let root = plugins_root(&app_root.0);
    fs::create_dir_all(&root).map_err(|e| format!("Failed to create plugins directory: {e}"))?;
    Ok(root.to_string_lossy().to_string())
}

#[tauri::command]
fn open_plugins_folder(app_root: State<AppRootDir>) -> Result<String, String> {
    let root = plugins_root(&app_root.0);
    fs::create_dir_all(&root).map_err(|e| format!("Failed to create plugins directory: {e}"))?;
    open_folder_in_os(&root)?;
    Ok(root.to_string_lossy().to_string())
}

#[tauri::command]
fn install_external_plugin(
    source_path: String,
    app_root: State<AppRootDir>,
) -> Result<ExternalPluginInfo, String> {
    let trimmed = source_path.trim();
    if trimmed.is_empty() {
        return Err("Plugin source path cannot be empty.".to_string());
    }
    let source_dir = PathBuf::from(trimmed);
    if !source_dir.exists() || !source_dir.is_dir() {
        return Err(format!(
            "Plugin source directory does not exist: {}",
            trimmed
        ));
    }
    let source_manifest = read_external_plugin_manifest(&source_dir)?;

    let root = plugins_root(&app_root.0);
    fs::create_dir_all(&root).map_err(|e| format!("Failed to create plugins directory: {e}"))?;

    let source_canonical = fs::canonicalize(&source_dir)
        .map_err(|e| format!("Failed to resolve source directory: {e}"))?;
    let root_canonical =
        fs::canonicalize(&root).map_err(|e| format!("Failed to resolve plugins directory: {e}"))?;
    if source_canonical.starts_with(&root_canonical) {
        return Err("Source directory cannot be inside the managed plugins directory.".to_string());
    }

    let target_dir = root.join(&source_manifest.id);
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)
            .map_err(|e| format!("Failed to remove existing plugin directory: {e}"))?;
    }

    copy_directory_recursive(&source_dir, &target_dir)?;
    build_external_plugin_info(&target_dir)
}

#[tauri::command]
fn uninstall_external_plugin(
    plugin_id: String,
    app_root: State<AppRootDir>,
) -> Result<String, String> {
    let plugin_id = plugin_id.trim();
    validate_plugin_id(plugin_id)?;
    let root = plugins_root(&app_root.0);
    let target_dir = root.join(plugin_id);
    if !target_dir.exists() {
        return Err(format!("Plugin '{}' is not installed.", plugin_id));
    }
    fs::remove_dir_all(&target_dir).map_err(|e| format!("Failed to uninstall plugin: {e}"))?;
    Ok(plugin_id.to_string())
}

#[tauri::command]
fn open_memories_folder(data_dir: State<AppDataDir>) -> Result<String, String> {
    let data_dir = get_current_data_dir(&data_dir)?;
    let memories_dir = data_dir.join("memories");
    fs::create_dir_all(&memories_dir)
        .map_err(|e| format!("Failed to create memories directory: {e}"))?;
    open_folder_in_os(&memories_dir)?;
    Ok(memories_dir.to_string_lossy().to_string())
}

#[tauri::command]
fn get_memories_folder_path(data_dir: State<AppDataDir>) -> Result<String, String> {
    let data_dir = get_current_data_dir(&data_dir)?;
    let memories_dir = data_dir.join("memories");
    fs::create_dir_all(&memories_dir)
        .map_err(|e| format!("Failed to create memories directory: {e}"))?;
    Ok(memories_dir.to_string_lossy().to_string())
}

#[tauri::command]
fn read_memory_file(path: String, _data_dir: State<AppDataDir>) -> Result<MdRecord, String> {
    read_memory(&PathBuf::from(path))
}

#[tauri::command]
fn extract_entities(text: String) -> Result<ExtractedData, String> {
    ensure_ollama_running("http://localhost:11434")?;
    ensure_model_available("http://localhost:11434", OLLAMA_MODEL_EXTRACT)?;
    call_ollama_extract_blocking("http://localhost:11434", OLLAMA_MODEL_EXTRACT, &text).or_else(
        |_| {
            let _ = ensure_model_available("http://localhost:11434", OLLAMA_MODEL);
            call_ollama_extract_blocking("http://localhost:11434", OLLAMA_MODEL, &text)
        },
    )
}

/// Blocking core logic for save_memory, executed inside spawn_blocking to ensure real-time event delivery.
fn do_save_memory(
    app: tauri::AppHandle,
    content: String,
    tags: Option<Vec<String>>,
    config: ModelConfig,
    memories_dir: std::path::PathBuf,
) -> Result<Memory, String> {
    // Emit current model info
    match &config.provider {
        ModelProvider::Ollama {
            model_name,
            extract_model_name,
            ..
        } => {
            emit_save_progress(
                &app,
                "saveProgress.modelInfo.ollama",
                "info",
                serde_json::json!({ "model": extract_model_name }),
            );
            println!("📝 [save_memory] Using Ollama model: {}", model_name);
        }
        ModelProvider::DeepSeek { model_name, .. } => {
            emit_save_progress(
                &app,
                "saveProgress.modelInfo.deepseek",
                "info",
                serde_json::json!({ "model": model_name }),
            );
            println!("📝 [save_memory] Using DeepSeek API: {}", model_name);
        }
        ModelProvider::OpenAI { model_name, .. } => {
            emit_save_progress(
                &app,
                "saveProgress.modelInfo.openai",
                "info",
                serde_json::json!({ "model": model_name }),
            );
            println!("📝 [save_memory] Using OpenAI API: {}", model_name);
        }
    }

    // Step 1: Quick entity extraction to find related entities for history lookup
    emit_save_progress(
        &app,
        "saveProgress.step1.extracting",
        "running",
        serde_json::json!({}),
    );
    println!("🔍 [Step 1] Starting entity extraction...");
    let quick_extracted: Option<ExtractedData> = if content.trim().len() > 5 {
        if let ModelProvider::Ollama {
            base_url,
            extract_model_name,
            ..
        } = &config.provider
        {
            let _ = ensure_ollama_running(base_url);
            let _ = ensure_model_available(base_url, extract_model_name);
        }
        match call_model_extract(&config, ENTITY_EXTRACT_PROMPT, &content) {
            Ok(extracted) if extracted.entities.is_empty() => {
                println!("❌ [Step 1] Extraction returned 0 entities, aborting save");
                return Err(
                    serde_json::json!({ "code": "saveProgress.errors.noEntities" }).to_string(),
                );
            }
            Ok(extracted) => {
                emit_save_progress(
                    &app,
                    "saveProgress.step1.extracted",
                    "success",
                    serde_json::json!({ "count": extracted.entities.len() }),
                );
                println!("✅ Extracted {} entities", extracted.entities.len());
                Some(extracted)
            }
            Err(e) => {
                println!("❌ [Step 1] Entity extraction failed, aborting save: {}", e);
                return Err(
                    serde_json::json!({ "code": "saveProgress.errors.extractFailed", "reason": e })
                        .to_string(),
                );
            }
        }
    } else {
        None
    };

    // Step 2: Fetch related historical memories for knowledge fusion
    emit_save_progress(
        &app,
        "saveProgress.step2.lookingUp",
        "running",
        serde_json::json!({}),
    );
    println!("🔍 [Step 2] Looking up related historical memories...");
    let historical_memories = if let Some(ref ex) = quick_extracted {
        let selected = {
            let db = app.state::<DbState>();
            let mut guard =
                db.0.lock()
                    .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
            let conn = guard.as_mut().ok_or("database not initialized")?;
            collect_relevant_historical_memories(conn, ex, &content, None)?
        };
        if selected.is_empty() {
            emit_save_progress(
                &app,
                "saveProgress.step2.noHistory",
                "success",
                serde_json::json!({}),
            );
        } else {
            emit_save_progress(
                &app,
                "saveProgress.step2.found",
                "success",
                serde_json::json!({ "count": selected.len() }),
            );
        }
        println!(
            "✅ Selected {} relevant memories (top_k={}, budget={} chars)",
            selected.len(),
            RAG_TOP_K,
            RAG_MAX_CONTEXT_CHARS
        );
        selected
    } else {
        emit_save_progress(
            &app,
            "saveProgress.step2.noHistory",
            "success",
            serde_json::json!({}),
        );
        Vec::new()
    };

    // Step 3: Knowledge fusion (only when historical memories exist)
    let fused = if !historical_memories.is_empty() && content.trim().len() > 5 {
        emit_save_progress(
            &app,
            "saveProgress.step3.fusing",
            "running",
            serde_json::json!({}),
        );
        println!("🧠 [Step 3] Starting knowledge fusion...");
        if let ModelProvider::Ollama {
            base_url,
            model_name,
            ..
        } = &config.provider
        {
            let _ = ensure_model_available(base_url, model_name);
        }
        call_model_fusion(
            &config,
            KNOWLEDGE_FUSION_PROMPT,
            &historical_memories,
            &content,
        )
        .map_err(|e| {
            emit_save_progress(
                &app,
                "saveProgress.step3.fusionFailed",
                "warning",
                serde_json::json!({}),
            );
            println!("⚠️ Knowledge fusion failed, falling back: {}", e);
            e
        })
        .ok()
    } else {
        emit_save_progress(
            &app,
            "saveProgress.step3.skipped",
            "skipped",
            serde_json::json!({}),
        );
        println!("⏭️ [Step 3] Skipping knowledge fusion (no historical memories)");
        None
    };

    let (mut entities, mut relations, aliases) = if let Some(fused_data) = fused {
        emit_save_progress(
            &app,
            "saveProgress.step3.fusionDone",
            "success",
            serde_json::json!({ "entities": fused_data.entities.len(), "relations": fused_data.relations.len() }),
        );
        println!(
            "✅ Knowledge fusion complete: {} entities, {} relations, {} aliases",
            fused_data.entities.len(),
            fused_data.relations.len(),
            fused_data.aliases.len()
        );
        (
            fused_data.entities,
            fused_data.relations,
            fused_data.aliases,
        )
    } else if let Some(ex) = quick_extracted {
        emit_save_progress(
            &app,
            "saveProgress.step3.extractionDone",
            "success",
            serde_json::json!({ "entities": ex.entities.len(), "relations": ex.relations.len() }),
        );
        println!(
            "✅ Using quick extraction results: {} entities, {} relations",
            ex.entities.len(),
            ex.relations.len()
        );
        (ex.entities, ex.relations, Vec::new())
    } else {
        emit_save_progress(
            &app,
            "saveProgress.step3.noEntities",
            "warning",
            serde_json::json!({}),
        );
        println!("⚠️ No entities extracted");
        (Vec::new(), Vec::new(), Vec::new())
    };

    if is_time_normalization_enabled_for_active_library(&app) {
        normalize_time_entities_in_place(
            &mut entities,
            &mut relations,
            &config,
            Local::now().date_naive(),
        );
    }

    let entity_names: Vec<String> = entities.iter().map(|x| x.name.clone()).collect();

    // Step 4: Persist to database
    emit_save_progress(
        &app,
        "saveProgress.step4.saving",
        "running",
        serde_json::json!({}),
    );
    println!("💾 [Step 4] Saving to database...");
    let path = write_memory(
        &memories_dir,
        &content,
        tags.as_deref(),
        if entity_names.is_empty() {
            None
        } else {
            Some(&entity_names)
        },
    )?;
    let path_str = path.to_string_lossy().to_string();

    let saved_memory = {
        let db = app.state::<DbState>();
        let mut guard =
            db.0.lock()
                .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        let conn = guard.as_mut().ok_or("database not initialized")?;

        let tags_str = tags.as_ref().map(|t| t.join(","));
        let memory_id = insert_memory(conn, &content, Some(&path_str), tags_str.as_deref())
            .map_err(|e| e.to_string())?;

        let mut name_to_id: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        for e in &entities {
            let attrs = e.attributes.as_ref().map(|a| a.to_string());
            let entity_id =
                match find_entity_id_by_name_or_alias(conn, &e.name).map_err(|e| e.to_string())? {
                    Some(id) => id,
                    None => upsert_entity(conn, &e.entity_type, &e.name, attrs.as_deref())
                        .map_err(|e| e.to_string())?,
                };
            link_memory_entity(conn, memory_id, entity_id).map_err(|e| e.to_string())?;
            name_to_id.insert(e.name.clone(), entity_id);
        }
        for alias_info in &aliases {
            let primary_id = name_to_id.get(&alias_info.primary);
            let alias_id = name_to_id.get(&alias_info.alias);
            match (primary_id, alias_id) {
                (Some(&pid), Some(&aid)) if pid != aid => {
                    merge_entities(conn, aid, pid).map_err(|e| e.to_string())?;
                    name_to_id.insert(alias_info.alias.clone(), pid);
                }
                (Some(&pid), None) => {
                    add_entity_alias(conn, pid, &alias_info.alias).map_err(|e| e.to_string())?;
                }
                _ => {}
            }
        }
        for r in &relations {
            if let (Some(&from_id), Some(&to_id)) = (name_to_id.get(&r.from), name_to_id.get(&r.to))
            {
                let _ = upsert_relation(conn, from_id, to_id, &r.relation);
            }
        }

        get_memory_by_id(conn, memory_id).map_err(|e| e.to_string())?
    };

    emit_save_progress(&app, "saveProgress.done", "done", serde_json::json!({}));
    println!("✅ Memory saved successfully!");
    Ok(saved_memory)
}

#[tauri::command]
async fn save_memory(
    app: tauri::AppHandle,
    content: String,
    tags: Option<Vec<String>>,
    config_state: State<'_, ModelConfigState>,
    data_dir: State<'_, AppDataDir>,
) -> Result<Memory, String> {
    let config = config_state.0.lock().map_err(|e| e.to_string())?.clone();
    let memories_dir = get_current_data_dir(&data_dir)?.join("memories");
    tokio::task::spawn_blocking(move || do_save_memory(app, content, tags, config, memories_dir))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn get_memories_list(db: State<DbState>) -> Result<Vec<Memory>, String> {
    let mut guard = (&*db)
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    list_memories(conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_graph(db: State<DbState>) -> Result<GraphData, String> {
    let mut guard = (&*db)
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    get_graph_data(conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn query_entity(name: String, db: State<DbState>) -> Result<Option<Entity>, String> {
    let mut guard = (&*db)
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    get_entity_by_name(conn, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_memories_by_entity(entity_id: i64, db: State<DbState>) -> Result<Vec<Memory>, String> {
    let mut guard = (&*db)
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    get_memories_for_entity(conn, entity_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_character_profile(entity_id: i64, db: State<DbState>) -> Result<serde_json::Value, String> {
    let mut guard = (&*db)
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    let entity = get_entity_by_id(conn, entity_id).map_err(|e| e.to_string())?;
    let memories = get_memories_for_entity(conn, entity_id).map_err(|e| e.to_string())?;
    let relations = list_relations(conn).map_err(|e| e.to_string())?;
    let entity_relations: Vec<_> = relations
        .into_iter()
        .filter(|r| r.from_entity_id == entity_id || r.to_entity_id == entity_id)
        .collect();

    // Collect all entity IDs needed for name resolution
    let mut id_set: std::collections::HashSet<i64> = std::collections::HashSet::new();
    for r in &entity_relations {
        id_set.insert(r.from_entity_id);
        id_set.insert(r.to_entity_id);
    }
    let mut id_to_name: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
    for id in id_set {
        if let Ok(e) = get_entity_by_id(conn, id) {
            id_to_name.insert(id, e.name);
        }
    }

    // Build enriched relation list with entity names
    let enriched_relations: Vec<serde_json::Value> = entity_relations
        .iter()
        .map(|r| {
            serde_json::json!({
                "from_entity_id": r.from_entity_id,
                "from_name": id_to_name.get(&r.from_entity_id).cloned().unwrap_or_default(),
                "to_entity_id": r.to_entity_id,
                "to_name": id_to_name.get(&r.to_entity_id).cloned().unwrap_or_default(),
                "relation_type": r.relation_type,
                "strength": r.strength,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "entity": entity,
        "memories": memories,
        "relations": enriched_relations
    }))
}

#[tauri::command]
fn get_timeline(db: State<DbState>) -> Result<Vec<Memory>, String> {
    let mut guard = (&*db)
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    list_memories(conn).map_err(|e| e.to_string())
}

/// Blocking core logic for update_memory_content, executed inside spawn_blocking to ensure real-time event delivery.
fn do_update_memory(
    app: tauri::AppHandle,
    memory_id: i64,
    content: String,
    tags_str: Option<String>,
    config: ModelConfig,
) -> Result<Memory, String> {
    println!("📝 [update_memory ID:{}]", memory_id);
    match &config.provider {
        ModelProvider::Ollama {
            extract_model_name, ..
        } => {
            emit_save_progress(
                &app,
                "saveProgress.modelInfo.ollama",
                "info",
                serde_json::json!({ "model": extract_model_name }),
            );
        }
        ModelProvider::DeepSeek { model_name, .. } => {
            emit_save_progress(
                &app,
                "saveProgress.modelInfo.deepseek",
                "info",
                serde_json::json!({ "model": model_name }),
            );
        }
        ModelProvider::OpenAI { model_name, .. } => {
            emit_save_progress(
                &app,
                "saveProgress.modelInfo.openai",
                "info",
                serde_json::json!({ "model": model_name }),
            );
        }
    }

    // Step 1: Quick entity extraction
    emit_save_progress(
        &app,
        "saveProgress.step1.extracting",
        "running",
        serde_json::json!({}),
    );
    println!("🔍 Starting entity extraction...");
    let quick_extracted = if content.trim().len() > 5 {
        if let ModelProvider::Ollama {
            base_url,
            extract_model_name,
            ..
        } = &config.provider
        {
            let _ = ensure_ollama_running(base_url);
            let _ = ensure_model_available(base_url, extract_model_name);
        }
        match call_model_extract(&config, ENTITY_EXTRACT_PROMPT, &content) {
            Ok(extracted) if extracted.entities.is_empty() => {
                println!("❌ [Step 1] Extraction returned 0 entities, aborting update");
                return Err(
                    serde_json::json!({ "code": "saveProgress.errors.noEntities" }).to_string(),
                );
            }
            Ok(extracted) => {
                emit_save_progress(
                    &app,
                    "saveProgress.step1.extracted",
                    "success",
                    serde_json::json!({ "count": extracted.entities.len() }),
                );
                println!("✅ Extracted {} entities", extracted.entities.len());
                Some(extracted)
            }
            Err(e) => {
                println!(
                    "❌ [Step 1] Entity extraction failed, aborting update: {}",
                    e
                );
                return Err(
                    serde_json::json!({ "code": "saveProgress.errors.extractFailed", "reason": e })
                        .to_string(),
                );
            }
        }
    } else {
        None
    };

    // Step 2: Fetch related historical memories for knowledge fusion
    emit_save_progress(
        &app,
        "saveProgress.step2.lookingUp",
        "running",
        serde_json::json!({}),
    );
    let historical_memories = if let Some(ref ex) = quick_extracted {
        let selected = {
            let db = app.state::<DbState>();
            let mut guard =
                db.0.lock()
                    .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
            let conn = guard.as_mut().ok_or("database not initialized")?;
            collect_relevant_historical_memories(conn, ex, &content, Some(memory_id))?
        };
        if selected.is_empty() {
            emit_save_progress(
                &app,
                "saveProgress.step2.noHistory",
                "success",
                serde_json::json!({}),
            );
        } else {
            emit_save_progress(
                &app,
                "saveProgress.step2.found",
                "success",
                serde_json::json!({ "count": selected.len() }),
            );
        }
        selected
    } else {
        emit_save_progress(
            &app,
            "saveProgress.step2.noHistory",
            "success",
            serde_json::json!({}),
        );
        Vec::new()
    };

    // Step 3: Knowledge fusion
    let fused = if !historical_memories.is_empty() && content.trim().len() > 5 {
        emit_save_progress(
            &app,
            "saveProgress.step3.fusing",
            "running",
            serde_json::json!({}),
        );
        println!("🧠 Running knowledge fusion...");
        if let ModelProvider::Ollama {
            base_url,
            model_name,
            ..
        } = &config.provider
        {
            let _ = ensure_model_available(base_url, model_name);
        }
        call_model_fusion(
            &config,
            KNOWLEDGE_FUSION_PROMPT,
            &historical_memories,
            &content,
        )
        .ok()
    } else {
        emit_save_progress(
            &app,
            "saveProgress.step3.skipped",
            "skipped",
            serde_json::json!({}),
        );
        None
    };

    let (mut entities, mut relations, aliases) = if let Some(fused_data) = fused {
        emit_save_progress(
            &app,
            "saveProgress.step3.fusionDone",
            "success",
            serde_json::json!({ "entities": fused_data.entities.len(), "relations": fused_data.relations.len() }),
        );
        println!("✅ Knowledge fusion complete");
        (
            fused_data.entities,
            fused_data.relations,
            fused_data.aliases,
        )
    } else if let Some(ex) = quick_extracted {
        emit_save_progress(
            &app,
            "saveProgress.step3.extractionDone",
            "success",
            serde_json::json!({ "entities": ex.entities.len(), "relations": ex.relations.len() }),
        );
        println!("✅ Using quick extraction results");
        (ex.entities, ex.relations, Vec::new())
    } else {
        emit_save_progress(
            &app,
            "saveProgress.step3.noEntities",
            "warning",
            serde_json::json!({}),
        );
        (Vec::new(), Vec::new(), Vec::new())
    };

    if is_time_normalization_enabled_for_active_library(&app) {
        normalize_time_entities_in_place(
            &mut entities,
            &mut relations,
            &config,
            Local::now().date_naive(),
        );
    }

    // Step 4: Persist to database
    emit_save_progress(
        &app,
        "saveProgress.step4.saving",
        "running",
        serde_json::json!({}),
    );
    let updated_memory = {
        let db = app.state::<DbState>();
        let mut guard =
            db.0.lock()
                .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        let conn = guard.as_mut().ok_or("database not initialized")?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        update_memory(&tx, memory_id, &content, tags_str.as_deref()).map_err(|e| e.to_string())?;
        clear_memory_entities(&tx, memory_id).map_err(|e| e.to_string())?;

        let mut name_to_id: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        for e in &entities {
            let attrs = e.attributes.as_ref().map(|a| a.to_string());
            let entity_id =
                match find_entity_id_by_name_or_alias(&tx, &e.name).map_err(|e| e.to_string())? {
                    Some(id) => id,
                    None => upsert_entity(&tx, &e.entity_type, &e.name, attrs.as_deref())
                        .map_err(|e| e.to_string())?,
                };
            link_memory_entity(&tx, memory_id, entity_id).map_err(|e| e.to_string())?;
            name_to_id.insert(e.name.clone(), entity_id);
        }
        for alias_info in &aliases {
            let primary_id = name_to_id.get(&alias_info.primary);
            let alias_id = name_to_id.get(&alias_info.alias);
            match (primary_id, alias_id) {
                (Some(&pid), Some(&aid)) if pid != aid => {
                    merge_entities(&tx, aid, pid).map_err(|e| e.to_string())?;
                    name_to_id.insert(alias_info.alias.clone(), pid);
                }
                (Some(&pid), None) => {
                    add_entity_alias(&tx, pid, &alias_info.alias).map_err(|e| e.to_string())?;
                }
                _ => {}
            }
        }
        for r in &relations {
            if let (Some(&from_id), Some(&to_id)) = (name_to_id.get(&r.from), name_to_id.get(&r.to))
            {
                let _ = upsert_relation(&tx, from_id, to_id, &r.relation);
            }
        }

        prune_orphan_entities_and_relations(&tx).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        get_memory_by_id(conn, memory_id).map_err(|e| e.to_string())?
    };

    emit_save_progress(
        &app,
        "saveProgress.updateDone",
        "done",
        serde_json::json!({}),
    );
    println!("✅ Memory updated successfully!");
    Ok(updated_memory)
}

#[tauri::command]
async fn update_memory_content(
    app: tauri::AppHandle,
    memory_id: i64,
    content: String,
    tags: Option<Vec<String>>,
    config_state: State<'_, ModelConfigState>,
) -> Result<Memory, String> {
    let config = config_state.0.lock().map_err(|e| e.to_string())?.clone();
    let tags_str = tags.map(|t| t.join(","));
    tokio::task::spawn_blocking(move || do_update_memory(app, memory_id, content, tags_str, config))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn delete_memory_by_id(memory_id: i64, db: State<DbState>) -> Result<(), String> {
    let mut guard = (&*db)
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    delete_memory(conn, memory_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn cleanup_db(db: State<DbState>) -> Result<String, String> {
    let mut guard = (&*db)
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    cleanup_database(conn).map_err(|e| e.to_string())?;
    Ok("Database cleanup complete".to_string())
}

/// Clear all data (destructive — use with caution).
#[tauri::command]
fn clear_all_data_cmd(db: State<DbState>, data_dir: State<AppDataDir>) -> Result<String, String> {
    let mut guard = (&*db)
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;

    // Clear database tables
    clear_all_data(conn).map_err(|e| e.to_string())?;

    // Remove and recreate the memories folder
    let memories_dir = get_current_data_dir(&data_dir)?.join("memories");
    if memories_dir.exists() {
        std::fs::remove_dir_all(&memories_dir)
            .map_err(|e| format!("Failed to delete memories folder: {}", e))?;
        std::fs::create_dir_all(&memories_dir)
            .map_err(|e| format!("Failed to recreate memories folder: {}", e))?;
    }

    Ok("All data has been cleared".to_string())
}

/// Transcribe audio: calls the local whisper.cpp (whisper-cli).
#[tauri::command]
fn transcribe_audio(audio_base64: String, app_root: State<AppRootDir>) -> Result<String, String> {
    transcribe_audio_with_whisper(&audio_base64, &app_root.0)
}

/// Set up Whisper: auto-installs whisper-cpp (macOS) and downloads the base model.
#[tauri::command]
fn setup_whisper(app_root: State<AppRootDir>) -> Result<String, String> {
    setup_whisper_runtime(&app_root.0)
}

const OLLAMA_URL: &str = "http://localhost:11434";
/// Model used for Q&A and generation tasks (requires a reasonably capable model).
const OLLAMA_MODEL: &str = "qwen2.5:7b";
/// Model used for entity extraction (person / time / location / event).
const OLLAMA_MODEL_EXTRACT: &str = "qwen2.5:7b";

/// Lightweight RAG parameters for memory fusion.
const RAG_CANDIDATES_PER_ENTITY: usize = 8;
const RAG_TOP_K: usize = 6;
const RAG_MAX_CONTEXT_CHARS: usize = 2800;
const RAG_MAX_MEMORY_CHARS: usize = 520;
const RAG_MIN_RELEVANCE_SCORE: f32 = 0.12;
const TIME_NORMALIZE_AI_MAX_CALLS: usize = 4;

fn is_time_normalization_enabled_for_active_library(app: &tauri::AppHandle) -> bool {
    let data_dir_state = app.state::<AppDataDir>();
    let current_dir = match data_dir_state.0.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => return false,
    };
    load_library_time_normalization(&current_dir)
}

fn parse_chinese_number_token(token: &str) -> Option<i64> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(n) = trimmed.parse::<i64>() {
        return Some(n);
    }

    let normalized = trimmed.replace('两', "二");
    let digit = |ch: char| -> Option<i64> {
        match ch {
            '零' => Some(0),
            '一' => Some(1),
            '二' => Some(2),
            '三' => Some(3),
            '四' => Some(4),
            '五' => Some(5),
            '六' => Some(6),
            '七' => Some(7),
            '八' => Some(8),
            '九' => Some(9),
            _ => None,
        }
    };

    if normalized == "十" {
        return Some(10);
    }

    if normalized.contains('十') {
        let mut parts = normalized.split('十');
        let left = parts.next().unwrap_or_default();
        let right = parts.next().unwrap_or_default();
        let tens = if left.is_empty() {
            1
        } else if left.chars().count() == 1 {
            digit(left.chars().next()?)?
        } else {
            return None;
        };
        let ones = if right.is_empty() {
            0
        } else if right.chars().count() == 1 {
            digit(right.chars().next()?)?
        } else {
            return None;
        };
        return Some(tens * 10 + ones);
    }

    let mut value = 0i64;
    for ch in normalized.chars() {
        value = value.checked_mul(10)?;
        value = value.checked_add(digit(ch)?)?;
    }
    Some(value)
}

fn extract_trailing_number_token(text: &str) -> Option<String> {
    let mut token = String::new();
    let mut started = false;
    for ch in text.chars().rev() {
        let is_num = ch.is_ascii_digit()
            || matches!(
                ch,
                '零' | '一' | '二' | '两' | '三' | '四' | '五' | '六' | '七' | '八' | '九' | '十'
            );
        if is_num {
            token.insert(0, ch);
            started = true;
            continue;
        }
        if ch.is_whitespace() && !started {
            continue;
        }
        break;
    }
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

fn extract_leading_number_token(text: &str) -> Option<String> {
    let mut token = String::new();
    let mut started = false;
    for ch in text.chars() {
        let is_num = ch.is_ascii_digit()
            || matches!(
                ch,
                '零' | '一' | '二' | '两' | '三' | '四' | '五' | '六' | '七' | '八' | '九' | '十'
            );
        if is_num {
            token.push(ch);
            started = true;
            continue;
        }
        if ch.is_whitespace() && !started {
            continue;
        }
        break;
    }
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

fn parse_ymd_from_separated_text(text: &str) -> Option<NaiveDate> {
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() || matches!(ch, '-' | '/' | '.') {
            current.push(ch);
            continue;
        }
        if current.matches('-').count()
            + current.matches('/').count()
            + current.matches('.').count()
            >= 2
        {
            let normalized = current.replace('/', "-").replace('.', "-");
            let parts: Vec<&str> = normalized.split('-').filter(|x| !x.is_empty()).collect();
            if parts.len() >= 3 {
                if let (Ok(y), Ok(m), Ok(d)) = (
                    parts[0].parse::<i32>(),
                    parts[1].parse::<u32>(),
                    parts[2].parse::<u32>(),
                ) {
                    if (1900..=2200).contains(&y) {
                        if let Some(date) = NaiveDate::from_ymd_opt(y, m, d) {
                            return Some(date);
                        }
                    }
                }
            }
        }
        current.clear();
    }
    if current.matches('-').count() + current.matches('/').count() + current.matches('.').count()
        >= 2
    {
        let normalized = current.replace('/', "-").replace('.', "-");
        let parts: Vec<&str> = normalized.split('-').filter(|x| !x.is_empty()).collect();
        if parts.len() >= 3 {
            if let (Ok(y), Ok(m), Ok(d)) = (
                parts[0].parse::<i32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<u32>(),
            ) {
                if (1900..=2200).contains(&y) {
                    return NaiveDate::from_ymd_opt(y, m, d);
                }
            }
        }
    }
    None
}

fn weekday_index_from_char(ch: char) -> Option<i64> {
    match ch {
        '一' => Some(0),
        '二' => Some(1),
        '三' => Some(2),
        '四' => Some(3),
        '五' => Some(4),
        '六' => Some(5),
        '日' | '天' => Some(6),
        _ => None,
    }
}

fn week_start_monday(base: NaiveDate) -> NaiveDate {
    base - ChronoDuration::days(base.weekday().num_days_from_monday() as i64)
}

fn find_weekday_after_marker(text: &str, marker: &str) -> Option<i64> {
    let idx = text.find(marker)?;
    let tail = &text[idx + marker.len()..];
    for ch in tail.chars() {
        if ch.is_whitespace() || ch == '的' {
            continue;
        }
        if ch == '末' {
            return Some(5);
        }
        return weekday_index_from_char(ch);
    }
    None
}

fn parse_date_from_time_text_rule(text: &str, reference: NaiveDate) -> Option<NaiveDate> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.contains("今天") {
        return Some(reference);
    }
    if trimmed.contains("明天") {
        return Some(reference + ChronoDuration::days(1));
    }
    if trimmed.contains("后天") {
        return Some(reference + ChronoDuration::days(2));
    }

    for marker in ["天之后", "日之后", "天后", "日后"] {
        if let Some(pos) = trimmed.find(marker) {
            let token = extract_trailing_number_token(&trimmed[..pos])?;
            let offset = parse_chinese_number_token(&token)?;
            if (0..=365).contains(&offset) {
                return Some(reference + ChronoDuration::days(offset));
            }
        }
    }

    if let Some(date) = parse_ymd_from_separated_text(trimmed) {
        return Some(date);
    }

    if trimmed.contains('年') && trimmed.contains('月') {
        if let (Some(pos_year), Some(pos_month)) = (trimmed.find('年'), trimmed.find('月')) {
            let year_token = extract_trailing_number_token(&trimmed[..pos_year])?;
            let month_token = extract_trailing_number_token(&trimmed[..pos_month])?;
            let day_token = if let Some(pos_day) = trimmed.find('日') {
                extract_trailing_number_token(&trimmed[..pos_day])
            } else {
                extract_leading_number_token(&trimmed[pos_month + '月'.len_utf8()..])
            }?;
            let year = parse_chinese_number_token(&year_token)? as i32;
            let month = parse_chinese_number_token(&month_token)? as u32;
            let day = parse_chinese_number_token(&day_token)? as u32;
            if (1900..=2200).contains(&year) {
                if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                    return Some(date);
                }
            }
        }
    }

    if let Some(pos_month) = trimmed.find('月') {
        let month_token = extract_trailing_number_token(&trimmed[..pos_month])?;
        let day_token = if let Some(pos_day) = trimmed.find('日') {
            extract_trailing_number_token(&trimmed[..pos_day])
        } else {
            extract_leading_number_token(&trimmed[pos_month + '月'.len_utf8()..])
        }?;
        let month = parse_chinese_number_token(&month_token)? as u32;
        let day = parse_chinese_number_token(&day_token)? as u32;
        let mut year = reference.year();
        let mut date = NaiveDate::from_ymd_opt(year, month, day)?;
        if date < reference {
            year += 1;
            date = NaiveDate::from_ymd_opt(year, month, day)?;
        }
        return Some(date);
    }

    if let Some(weekday) = find_weekday_after_marker(trimmed, "下周")
        .or_else(|| find_weekday_after_marker(trimmed, "下星期"))
    {
        let monday = week_start_monday(reference);
        return Some(monday + ChronoDuration::days(7 + weekday));
    }

    if let Some(weekday) = find_weekday_after_marker(trimmed, "本周")
        .or_else(|| find_weekday_after_marker(trimmed, "这周"))
    {
        let monday = week_start_monday(reference);
        let mut date = monday + ChronoDuration::days(weekday);
        if date < reference {
            date += ChronoDuration::days(7);
        }
        return Some(date);
    }

    if let Some(weekday) = find_weekday_after_marker(trimmed, "周") {
        let monday = week_start_monday(reference);
        let mut date = monday + ChronoDuration::days(weekday);
        if date < reference {
            date += ChronoDuration::days(7);
        }
        return Some(date);
    }

    None
}

fn parse_iso_date_from_string(text: &str) -> Option<NaiveDate> {
    let trimmed = text.trim();
    let date_part = if trimmed.len() >= 10 {
        &trimmed[..10]
    } else {
        trimmed
    };
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year = parts[0].parse::<i32>().ok()?;
    let month = parts[1].parse::<u32>().ok()?;
    let day = parts[2].parse::<u32>().ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

fn normalize_time_text_with_model(
    config: &ModelConfig,
    reference: NaiveDate,
    time_text: &str,
) -> Option<NaiveDate> {
    let prompt = format!(
        "你是时间解析器。参考日期是 {ref_date}。\n\
请把输入时间表达转换为公历日期 YYYY-MM-DD。\n\
如果无法确定具体日期，date 必须是 null。\n\
只输出 JSON：{{\"date\":\"YYYY-MM-DD\"|null}}\n\
输入：{input}",
        ref_date = reference.format("%Y-%m-%d"),
        input = time_text.trim()
    );
    let response = call_model_simple(config, &prompt).ok()?;
    let json = extract_json_block(&response)?;
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let date_str = value.get("date").and_then(|x| x.as_str())?;
    parse_iso_date_from_string(date_str)
}

fn normalize_time_entities_in_place(
    entities: &mut Vec<ExtractedEntity>,
    relations: &mut Vec<ExtractedRelation>,
    config: &ModelConfig,
    reference: NaiveDate,
) {
    let mut rename_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut cache: std::collections::HashMap<String, Option<(String, &'static str)>> =
        std::collections::HashMap::new();
    let mut ai_calls = 0usize;

    for entity in entities.iter_mut() {
        if !entity.entity_type.eq_ignore_ascii_case("time") {
            continue;
        }
        let original_name = entity.name.trim().to_string();
        if original_name.is_empty() {
            continue;
        }

        let cached = cache.get(&original_name).cloned();
        let normalized = if let Some(hit) = cached {
            hit
        } else {
            let mut resolved = parse_date_from_time_text_rule(&original_name, reference)
                .map(|d| (d.format("%Y-%m-%d").to_string(), "rule"));
            if resolved.is_none() && ai_calls < TIME_NORMALIZE_AI_MAX_CALLS {
                ai_calls += 1;
                resolved = normalize_time_text_with_model(config, reference, &original_name)
                    .map(|d| (d.format("%Y-%m-%d").to_string(), "model"));
            }
            cache.insert(original_name.clone(), resolved.clone());
            resolved
        };

        let Some((normalized_name, method)) = normalized else {
            continue;
        };
        if normalized_name == original_name {
            continue;
        }

        rename_map.insert(original_name.clone(), normalized_name.clone());

        let mut attrs = serde_json::Map::new();
        if let Some(existing) = entity.attributes.take() {
            if let Some(obj) = existing.as_object() {
                for (k, v) in obj {
                    attrs.insert(k.clone(), v.clone());
                }
            } else {
                attrs.insert("raw_attributes".to_string(), existing);
            }
        }
        attrs
            .entry("original_time_text".to_string())
            .or_insert(serde_json::Value::String(original_name));
        attrs.insert(
            "normalized_date".to_string(),
            serde_json::Value::String(normalized_name.clone()),
        );
        attrs.insert(
            "normalized_by".to_string(),
            serde_json::Value::String(method.to_string()),
        );
        attrs.insert(
            "reference_date".to_string(),
            serde_json::Value::String(reference.format("%Y-%m-%d").to_string()),
        );

        entity.name = normalized_name;
        entity.attributes = Some(serde_json::Value::Object(attrs));
    }

    if rename_map.is_empty() {
        return;
    }

    for relation in relations.iter_mut() {
        if let Some(new_from) = rename_map.get(&relation.from) {
            relation.from = new_from.clone();
        }
        if let Some(new_to) = rename_map.get(&relation.to) {
            relation.to = new_to.clone();
        }
    }
}

fn is_cjk_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x3040..=0x30FF
            | 0xAC00..=0xD7AF
    )
}

fn tokenize_for_rag(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut latin_run = String::new();
    let mut cjk_run: Vec<char> = Vec::new();

    let flush_latin = |run: &mut String, out: &mut Vec<String>| {
        if run.len() >= 2 {
            out.push(run.clone());
        }
        run.clear();
    };

    let flush_cjk = |run: &mut Vec<char>, out: &mut Vec<String>| {
        if run.is_empty() {
            return;
        }
        if run.len() == 1 {
            out.push(run[0].to_string());
            run.clear();
            return;
        }
        for window in run.windows(2) {
            out.push(window.iter().collect::<String>());
        }
        run.clear();
    };

    for raw in text.chars() {
        let ch = raw.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            if !cjk_run.is_empty() {
                flush_cjk(&mut cjk_run, &mut tokens);
            }
            latin_run.push(ch);
            continue;
        }
        if is_cjk_char(raw) {
            if !latin_run.is_empty() {
                flush_latin(&mut latin_run, &mut tokens);
            }
            cjk_run.push(raw);
            continue;
        }
        if !latin_run.is_empty() {
            flush_latin(&mut latin_run, &mut tokens);
        }
        if !cjk_run.is_empty() {
            flush_cjk(&mut cjk_run, &mut tokens);
        }
    }

    if !latin_run.is_empty() {
        flush_latin(&mut latin_run, &mut tokens);
    }
    if !cjk_run.is_empty() {
        flush_cjk(&mut cjk_run, &mut tokens);
    }

    tokens
}

fn memory_relevance_score(
    query_tokens: &std::collections::HashSet<String>,
    entity_terms: &[String],
    candidate_content: &str,
    seed_hits: usize,
) -> (f32, usize, usize) {
    let candidate_tokens: std::collections::HashSet<String> =
        tokenize_for_rag(candidate_content).into_iter().collect();
    let token_overlap = if query_tokens.is_empty() {
        0
    } else {
        query_tokens.intersection(&candidate_tokens).count()
    };
    let overlap_ratio = if query_tokens.is_empty() {
        0.0
    } else {
        token_overlap as f32 / query_tokens.len() as f32
    };

    let candidate_lc = candidate_content.to_lowercase();
    let entity_hits = entity_terms
        .iter()
        .filter(|term| candidate_lc.contains(term.as_str()))
        .count();
    let entity_ratio = if entity_terms.is_empty() {
        0.0
    } else {
        entity_hits as f32 / entity_terms.len() as f32
    };
    let seed_ratio = (seed_hits.min(3) as f32) / 3.0;

    let score = overlap_ratio * 0.65 + entity_ratio * 0.25 + seed_ratio * 0.10;
    (score, token_overlap, entity_hits)
}

fn collect_relevant_historical_memories(
    conn: &rusqlite::Connection,
    extracted: &ExtractedData,
    new_content: &str,
    exclude_memory_id: Option<i64>,
) -> Result<Vec<String>, String> {
    let mut candidate_map: std::collections::HashMap<i64, (String, usize)> =
        std::collections::HashMap::new();
    for entity in &extracted.entities {
        if let Some(existing_entity) =
            get_entity_by_name(conn, &entity.name).map_err(|e| e.to_string())?
        {
            let memories =
                get_memories_for_entity(conn, existing_entity.id).map_err(|e| e.to_string())?;
            for mem in memories.into_iter().take(RAG_CANDIDATES_PER_ENTITY) {
                if exclude_memory_id == Some(mem.id) {
                    continue;
                }
                let entry = candidate_map
                    .entry(mem.id)
                    .or_insert_with(|| (mem.content, 0));
                entry.1 += 1;
            }
        }
    }

    if candidate_map.is_empty() {
        return Ok(Vec::new());
    }

    let query_tokens: std::collections::HashSet<String> =
        tokenize_for_rag(new_content).into_iter().collect();
    let entity_terms: Vec<String> = extracted
        .entities
        .iter()
        .map(|e| e.name.trim().to_lowercase())
        .filter(|s| !s.is_empty() && s.chars().count() >= 2)
        .collect();

    let mut scored: Vec<(f32, usize, usize, String)> = candidate_map
        .into_iter()
        .map(|(_, (content, seed_hits))| {
            let (score, token_overlap, entity_hits) =
                memory_relevance_score(&query_tokens, &entity_terms, &content, seed_hits);
            (score, token_overlap, entity_hits, content)
        })
        .collect();

    scored.sort_by(|a, b| b.0.total_cmp(&a.0));

    let mut selected = Vec::new();
    let mut budget_left = RAG_MAX_CONTEXT_CHARS;

    for (score, token_overlap, entity_hits, content) in scored.into_iter().take(RAG_TOP_K) {
        let is_relevant = score >= RAG_MIN_RELEVANCE_SCORE || entity_hits > 0 || token_overlap >= 2;
        if !is_relevant {
            continue;
        }
        if budget_left == 0 {
            break;
        }

        let trimmed = content.trim();
        if trimmed.is_empty() {
            continue;
        }
        let allow = budget_left.min(RAG_MAX_MEMORY_CHARS);
        let snippet = if trimmed.chars().count() > allow {
            truncate_for_prompt(trimmed, allow)
        } else {
            trimmed.to_string()
        };
        let used = snippet.chars().count();
        if used == 0 {
            continue;
        }
        selected.push(snippet);
        budget_left = budget_left.saturating_sub(used);
    }

    Ok(selected)
}

/// Entity-aware memory retrieval and intelligent Q&A.
fn do_answer_question(
    question: String,
    config: ModelConfig,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let question = question.trim().to_string();
    if question.is_empty() {
        return Ok(String::new());
    }
    if let ModelProvider::Ollama {
        base_url,
        model_name,
        ..
    } = &config.provider
    {
        ensure_ollama_running(base_url)?;
        ensure_model_available(base_url, model_name)?;
    }

    let entity_name = call_model_simple(
        &config,
        &format!("{}{}", ollama::EXTRACT_ENTITY_PROMPT, question),
    )
    .ok()
    .and_then(|s| {
        let s = s.trim();
        if s.is_empty() || s.len() > 50 {
            None
        } else {
            Some(s.to_string())
        }
    });

    // Keep DB lock scope minimal so long model calls don't block other commands.
    let memories = {
        let db = app.state::<DbState>();
        let mut guard = (&*db)
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        let conn = guard.as_mut().ok_or("database not initialized")?;

        if let Some(name) = entity_name {
            if let Some(entity) = get_entity_by_name(conn, &name).map_err(|e| e.to_string())? {
                get_memories_for_entity(conn, entity.id).map_err(|e| e.to_string())?
            } else {
                list_memories(conn)
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .take(10)
                    .collect()
            }
        } else {
            list_memories(conn)
                .map_err(|e| e.to_string())?
                .into_iter()
                .take(10)
                .collect()
        }
    };

    if memories.is_empty() {
        return Ok("No relevant memories found. Please record some content first.".to_string());
    }

    let context: String = memories
        .iter()
        .map(|m| format!("- {}", m.content.trim()))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "{}{}{}{}",
        ollama::ANSWER_PROMPT_PREFIX,
        context,
        ollama::ANSWER_PROMPT_SUFFIX,
        question
    );

    call_model_simple(&config, &prompt)
}

#[tauri::command]
async fn answer_question(
    app: tauri::AppHandle,
    question: String,
    config_state: State<'_, ModelConfigState>,
) -> Result<String, String> {
    let config = config_state.0.lock().map_err(|e| e.to_string())?.clone();
    tokio::task::spawn_blocking(move || do_answer_question(question, config, app))
        .await
        .map_err(|e| e.to_string())?
}

fn truncate_for_prompt(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let mut out: String = input.chars().take(max_chars).collect();
    out.push('…');
    out
}

fn extract_json_block(response: &str) -> Option<&str> {
    let s = response.trim();
    let s = s
        .strip_prefix("```json")
        .or_else(|| s.strip_prefix("```"))
        .unwrap_or(s);
    let s = s.trim_end().strip_suffix("```").unwrap_or(s);
    Some(s.trim())
}

fn read_optional_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = value.get(key).and_then(|v| v.as_str()) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn read_string_array(value: &serde_json::Value, keys: &[&str]) -> Vec<String> {
    for key in keys {
        if let Some(arr) = value.get(key).and_then(|v| v.as_array()) {
            let mut out = Vec::new();
            for item in arr {
                if let Some(s) = item.as_str() {
                    let s = s.trim();
                    if !s.is_empty() {
                        out.push(s.to_string());
                    }
                    continue;
                }
                if let Some(obj) = item.as_object() {
                    for candidate in ["summary", "text", "content", "plot"] {
                        if let Some(s) = obj.get(candidate).and_then(|v| v.as_str()) {
                            let s = s.trim();
                            if !s.is_empty() {
                                out.push(s.to_string());
                                break;
                            }
                        }
                    }
                }
            }
            if !out.is_empty() {
                return out;
            }
        }
    }
    Vec::new()
}

fn parse_story_generation_response(response: &str) -> Result<StoryGenerationResult, String> {
    let json_str =
        extract_json_block(response).ok_or("Could not extract JSON output from model response")?;
    let value: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse story JSON: {e}"))?;

    let title = read_optional_string(&value, &["title"]).unwrap_or_default();
    let premise =
        read_optional_string(&value, &["premise", "summary", "logline"]).unwrap_or_default();
    let outline = read_string_array(&value, &["outline", "story_outline"]);
    let continuity_checks = read_string_array(&value, &["continuity_checks", "checks"]);
    let first_chapter =
        read_optional_string(&value, &["first_chapter", "chapter1", "opening"]).unwrap_or_default();

    let chapter_items = value
        .get("chapter_plan")
        .or_else(|| value.get("chapters"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let chapter_plan = chapter_items
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
            let chapter = item
                .get("chapter")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .unwrap_or(idx + 1);

            StoryChapterPlan {
                chapter,
                title: read_optional_string(&item, &["title"]).unwrap_or_default(),
                goal: read_optional_string(&item, &["goal", "objective"]).unwrap_or_default(),
                conflict: read_optional_string(&item, &["conflict"]).unwrap_or_default(),
                twist: read_optional_string(&item, &["twist", "turn"]).unwrap_or_default(),
                hook: read_optional_string(&item, &["hook", "ending_hook"]).unwrap_or_default(),
            }
        })
        .collect();

    Ok(StoryGenerationResult {
        title,
        premise,
        outline,
        chapter_plan,
        first_chapter,
        continuity_checks,
    })
}

fn collect_story_prompt_context(db: &State<DbState>) -> Result<StoryPromptContext, String> {
    let mut guard = (&**db)
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;

    let memories = list_memories(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .take(12)
        .map(|m| {
            format!(
                "[{}] {}",
                m.created_at,
                truncate_for_prompt(m.content.trim(), 150)
            )
        })
        .collect::<Vec<_>>();

    let graph = get_graph_data(conn).map_err(|e| e.to_string())?;
    let id_to_name: std::collections::HashMap<String, String> = graph
        .nodes
        .iter()
        .map(|n| (n.id.clone(), n.name.clone()))
        .collect();

    let entities = graph
        .nodes
        .iter()
        .take(40)
        .map(|n| format!("{}（{}）", n.name, n.node_type))
        .collect::<Vec<_>>();

    let relations = graph
        .links
        .iter()
        .take(50)
        .filter_map(|l| {
            let source = id_to_name.get(&l.source)?;
            let target = id_to_name.get(&l.target)?;
            Some(format!("{source} -[{}]-> {target}", l.relation))
        })
        .collect::<Vec<_>>();

    Ok(StoryPromptContext {
        memories,
        entities,
        relations,
    })
}

fn clamp_chapter_count(chapter_count: Option<usize>) -> usize {
    chapter_count.unwrap_or(10).clamp(3, 24)
}

fn infer_key_events_from_context(
    context: &StoryPromptContext,
    chapter_count: usize,
) -> Vec<String> {
    let target_len = chapter_count.clamp(3, 12);
    let mut inferred = Vec::new();

    for line in context.relations.iter().take(target_len / 2 + 1) {
        inferred.push(format!("关系线索：{}", truncate_for_prompt(line, 80)));
    }

    for memory in context.memories.iter().take(target_len + 2) {
        let cleaned = memory
            .splitn(2, ']')
            .nth(1)
            .map(|s| s.trim())
            .unwrap_or(memory.as_str());
        let candidate = truncate_for_prompt(cleaned, 100);
        if !candidate.is_empty() {
            inferred.push(format!("记忆事件：{}", candidate));
        }
    }

    inferred.sort();
    inferred.dedup();
    inferred.truncate(target_len);
    inferred
}

fn build_story_prompt(request: &StoryGenerationRequest, context: &StoryPromptContext) -> String {
    let is_zh = request
        .language
        .as_deref()
        .map(|lang| lang.starts_with("zh"))
        .unwrap_or(true);

    let target_language = if is_zh { "中文" } else { "English" };
    let default_genre = if is_zh {
        "不限（可自动判断）"
    } else {
        "Unspecified (infer from events)"
    };
    let default_style = if is_zh {
        "叙事清晰、人物驱动"
    } else {
        "Clear narrative, character-driven"
    };
    let default_protagonist = if is_zh { "未指定" } else { "Not specified" };
    let default_constraints = if is_zh { "无" } else { "None" };

    let key_events = request
        .key_events
        .iter()
        .enumerate()
        .map(|(idx, event)| format!("{}. {}", idx + 1, event))
        .collect::<Vec<_>>()
        .join("\n");

    let memory_context = if context.memories.is_empty() {
        if is_zh {
            "（暂无历史记忆）".to_string()
        } else {
            "(No historical memories)".to_string()
        }
    } else {
        context.memories.join("\n")
    };

    let entity_context = if context.entities.is_empty() {
        if is_zh {
            "（暂无实体）".to_string()
        } else {
            "(No entities)".to_string()
        }
    } else {
        context.entities.join("、")
    };

    let relation_context = if context.relations.is_empty() {
        if is_zh {
            "（暂无关系）".to_string()
        } else {
            "(No relations)".to_string()
        }
    } else {
        context.relations.join("\n")
    };

    format!(
        r#"你是一位专业小说策划与写作助手。请根据用户给出的关键事件，补全为可直接开写的故事方案。

硬性要求：
1. 必须保留并覆盖所有关键事件，不可删除或篡改核心事实。
2. 可以补充过渡情节，但所有新增内容必须服务主线。
3. 人物动机和关系要前后一致，冲突升级要合理。
4. 输出语言必须是：{target_language}
5. 只输出 JSON，不要输出 Markdown、解释或多余文本。

输出 JSON 结构（字段名必须一致）：
{{
  "title": "小说标题",
  "premise": "一句话故事核心",
  "outline": ["三幕/多幕剧情要点1", "剧情要点2"],
  "chapter_plan": [
    {{
      "chapter": 1,
      "title": "章节标题",
      "goal": "本章目标",
      "conflict": "核心冲突",
      "twist": "转折",
      "hook": "章末钩子"
    }}
  ],
  "first_chapter": "第一章正文（可直接阅读，建议 1200-1800 字）",
  "continuity_checks": ["关键一致性检查点1", "检查点2"]
}}

生成参数：
- 目标章节数：{chapter_count}
- 类型：{genre}
- 文风：{style}
- 主角：{protagonist}
- 额外约束：{constraints}

关键事件（必须全部覆盖）：
{key_events}

已有图谱实体（可复用）：
{entity_context}

已有实体关系（可复用）：
{relation_context}

历史记忆摘要（可参考）：
{memory_context}
"#,
        target_language = target_language,
        chapter_count = clamp_chapter_count(request.chapter_count),
        genre = request
            .genre
            .clone()
            .unwrap_or_else(|| default_genre.to_string()),
        style = request
            .style
            .clone()
            .unwrap_or_else(|| default_style.to_string()),
        protagonist = request
            .protagonist
            .clone()
            .unwrap_or_else(|| default_protagonist.to_string()),
        constraints = request
            .constraints
            .clone()
            .unwrap_or_else(|| default_constraints.to_string()),
        key_events = key_events,
        entity_context = entity_context,
        relation_context = relation_context,
        memory_context = memory_context,
    )
}

fn build_story_continue_prompt(
    request: &StoryContinuationRequest,
    target_chapter: usize,
) -> String {
    let is_zh = request
        .language
        .as_deref()
        .map(|lang| lang.starts_with("zh"))
        .unwrap_or(true);

    let target_language = if is_zh { "中文" } else { "English" };
    let style = request.style.clone().unwrap_or_else(|| {
        if is_zh {
            "保持与前文一致"
        } else {
            "Keep style consistent with previous chapters"
        }
        .to_string()
    });
    let constraints = request
        .constraints
        .clone()
        .unwrap_or_else(|| if is_zh { "无" } else { "None" }.to_string());

    let outline_text = if request.outline.is_empty() {
        if is_zh {
            "（未提供）".to_string()
        } else {
            "(Not provided)".to_string()
        }
    } else {
        request
            .outline
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{}. {}", i + 1, item))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let plan_text = if request.chapter_plan.is_empty() {
        if is_zh {
            "（未提供章节计划）".to_string()
        } else {
            "(No chapter plan provided)".to_string()
        }
    } else {
        request
            .chapter_plan
            .iter()
            .map(|c| {
                format!(
                    "第{}章《{}》\n- 目标：{}\n- 冲突：{}\n- 转折：{}\n- 钩子：{}",
                    c.chapter, c.title, c.goal, c.conflict, c.twist, c.hook
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    let checks_text = if request.continuity_checks.is_empty() {
        if is_zh {
            "（无）".to_string()
        } else {
            "(None)".to_string()
        }
    } else {
        request.continuity_checks.join("\n")
    };

    let written_text = if request.written_chapters.is_empty() {
        if is_zh {
            "（尚无正文）".to_string()
        } else {
            "(No written chapters yet)".to_string()
        }
    } else {
        request
            .written_chapters
            .iter()
            .map(|c| {
                format!(
                    "第{}章《{}》\n{}\n",
                    c.chapter,
                    if c.title.trim().is_empty() {
                        if is_zh {
                            "未命名"
                        } else {
                            "Untitled"
                        }
                    } else {
                        c.title.as_str()
                    },
                    truncate_for_prompt(&c.content, 2600)
                )
            })
            .collect::<Vec<_>>()
            .join("\n---\n")
    };

    format!(
        r#"你是长篇小说续写助手。请基于现有设定与已写内容，续写下一章。

硬性要求：
1. 不能推翻已写章节事实。
2. 必须延续人物动机、关系和语气。
3. 如有章节计划，优先遵循目标章节的计划项。
4. 输出语言必须是：{target_language}
5. 只输出 JSON，不要输出解释或 Markdown。

输出 JSON 格式：
{{
  "chapter": {target_chapter},
  "title": "本章标题",
  "content": "本章正文（建议 1200-2200 字）",
  "summary": "本章摘要（80-150 字）"
}}

故事标题：{story_title}
核心 premise：{premise}
文风要求：{style}
额外约束：{constraints}

总纲：
{outline_text}

章节计划：
{plan_text}

一致性检查点：
{checks_text}

已写章节：
{written_text}

请生成第 {target_chapter} 章。
"#,
        target_language = target_language,
        target_chapter = target_chapter,
        story_title = request.title,
        premise = request.premise,
        style = style,
        constraints = constraints,
        outline_text = outline_text,
        plan_text = plan_text,
        checks_text = checks_text,
        written_text = written_text,
    )
}

fn build_story_rewrite_prompt(
    request: &StoryRewriteChapterRequest,
    target: &StoryWrittenChapter,
) -> String {
    let is_zh = request
        .language
        .as_deref()
        .map(|lang| lang.starts_with("zh"))
        .unwrap_or(true);

    let target_language = if is_zh { "中文" } else { "English" };
    let style = request.style.clone().unwrap_or_else(|| {
        if is_zh {
            "保持全书一致风格"
        } else {
            "Keep style consistent across the story"
        }
        .to_string()
    });
    let constraints = request
        .constraints
        .clone()
        .unwrap_or_else(|| if is_zh { "无" } else { "None" }.to_string());
    let feedback = request
        .feedback
        .as_ref()
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .unwrap_or(if is_zh {
            "在不改变主线事实的前提下，优化节奏、冲突和表达。"
        } else {
            "Improve pacing, conflict, and writing quality without changing core facts."
        });

    let outline_text = if request.outline.is_empty() {
        if is_zh {
            "（未提供）".to_string()
        } else {
            "(Not provided)".to_string()
        }
    } else {
        request
            .outline
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{}. {}", i + 1, item))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let plan_text = if request.chapter_plan.is_empty() {
        if is_zh {
            "（未提供章节计划）".to_string()
        } else {
            "(No chapter plan provided)".to_string()
        }
    } else {
        request
            .chapter_plan
            .iter()
            .map(|c| {
                format!(
                    "第{}章《{}》\n- 目标：{}\n- 冲突：{}\n- 转折：{}\n- 钩子：{}",
                    c.chapter, c.title, c.goal, c.conflict, c.twist, c.hook
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    let checks_text = if request.continuity_checks.is_empty() {
        if is_zh {
            "（无）".to_string()
        } else {
            "(None)".to_string()
        }
    } else {
        request.continuity_checks.join("\n")
    };

    let context_chapters = request
        .written_chapters
        .iter()
        .map(|c| {
            format!(
                "第{}章《{}》\n{}",
                c.chapter,
                c.title,
                truncate_for_prompt(
                    &c.content,
                    if c.chapter == target.chapter {
                        3200
                    } else {
                        1200
                    }
                )
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    format!(
        r#"你是小说编辑助手。请根据修改意见，重写指定章节。

硬性要求：
1. 只能重写目标章节，不要改动其他章节事实。
2. 必须保持全书人物关系、时间线和设定一致。
3. 必须严格响应“修改意见”。
4. 输出语言必须是：{target_language}
5. 只输出 JSON。

输出 JSON：
{{
  "chapter": {target_chapter},
  "title": "章节标题",
  "content": "重写后的完整章节正文",
  "summary": "章节摘要（80-150字）"
}}

故事标题：{story_title}
核心 premise：{premise}
风格要求：{style}
额外约束：{constraints}

总纲：
{outline_text}

章节计划：
{plan_text}

一致性检查点：
{checks_text}

全书已写章节（用于保持一致）：
{context_chapters}

目标章节：第 {target_chapter} 章《{target_title}》
原章节文本：
{target_content}

修改意见：
{feedback}
"#,
        target_language = target_language,
        target_chapter = target.chapter,
        story_title = request.title,
        premise = request.premise,
        style = style,
        constraints = constraints,
        outline_text = outline_text,
        plan_text = plan_text,
        checks_text = checks_text,
        context_chapters = context_chapters,
        target_title = target.title,
        target_content = truncate_for_prompt(&target.content, 5000),
        feedback = feedback,
    )
}

fn parse_story_continuation_response(
    response: &str,
    target_chapter: usize,
) -> Result<StoryContinuationResult, String> {
    let json_str =
        extract_json_block(response).ok_or("Could not extract JSON output from model response")?;
    let value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse continuation JSON: {e}"))?;

    let chapter = value
        .get("chapter")
        .and_then(|v| v.as_u64())
        .map(|x| x as usize)
        .unwrap_or(target_chapter);

    Ok(StoryContinuationResult {
        chapter,
        title: read_optional_string(&value, &["title"]).unwrap_or_default(),
        content: read_optional_string(&value, &["content", "chapter_text", "text"])
            .unwrap_or_default(),
        summary: read_optional_string(&value, &["summary", "chapter_summary"]).unwrap_or_default(),
    })
}

#[tauri::command]
fn generate_story_from_events(
    mut request: StoryGenerationRequest,
    db: State<DbState>,
    config_state: State<ModelConfigState>,
) -> Result<StoryGenerationResult, String> {
    let is_zh = request
        .language
        .as_deref()
        .map(|lang| lang.starts_with("zh"))
        .unwrap_or(true);

    request.key_events = request
        .key_events
        .into_iter()
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect();

    let config = config_state.0.lock().map_err(|e| e.to_string())?.clone();

    if let ModelProvider::Ollama {
        base_url,
        model_name,
        ..
    } = &config.provider
    {
        ensure_ollama_running(base_url)?;
        ensure_model_available(base_url, model_name)?;
    }

    let context = collect_story_prompt_context(&db)?;
    if request.key_events.is_empty() {
        request.key_events =
            infer_key_events_from_context(&context, clamp_chapter_count(request.chapter_count));
    }
    if request.key_events.is_empty() {
        return Err(if is_zh {
            "图谱中暂无足够事件，请至少输入一个关键事件或先保存一些人物/事件记忆。".to_string()
        } else {
            "No usable events were found in the graph. Please add at least one key event or save some memories first.".to_string()
        });
    }

    let prompt = build_story_prompt(&request, &context);
    let response = call_model_simple(&config, &prompt)?;

    match parse_story_generation_response(&response) {
        Ok(mut parsed) => {
            if parsed.title.trim().is_empty() {
                parsed.title = if is_zh {
                    "未命名故事".to_string()
                } else {
                    "Untitled Story".to_string()
                };
            }
            if parsed.premise.trim().is_empty() {
                parsed.premise = if is_zh {
                    "基于关键事件生成的故事草案。".to_string()
                } else {
                    "Story draft generated from key events.".to_string()
                };
            }
            if parsed.first_chapter.trim().is_empty() {
                parsed.first_chapter = response.trim().to_string();
            }
            Ok(parsed)
        }
        Err(_) => Ok(StoryGenerationResult {
            title: if is_zh {
                "故事草案（未结构化）".to_string()
            } else {
                "Story Draft (Unstructured)".to_string()
            },
            premise: if is_zh {
                "模型未按 JSON 输出，已返回原始文本。可重试一次。".to_string()
            } else {
                "Model did not return JSON. Raw output is shown. You can retry.".to_string()
            },
            outline: Vec::new(),
            chapter_plan: Vec::new(),
            first_chapter: response.trim().to_string(),
            continuity_checks: vec![if is_zh {
                "建议再次生成以获得结构化章节计划。".to_string()
            } else {
                "Retry generation to obtain a fully structured chapter plan.".to_string()
            }],
        }),
    }
}

#[tauri::command]
fn continue_story_chapter(
    request: StoryContinuationRequest,
    config_state: State<ModelConfigState>,
) -> Result<StoryContinuationResult, String> {
    let config = config_state.0.lock().map_err(|e| e.to_string())?.clone();
    if let ModelProvider::Ollama {
        base_url,
        model_name,
        ..
    } = &config.provider
    {
        ensure_ollama_running(base_url)?;
        ensure_model_available(base_url, model_name)?;
    }

    let next_chapter = request
        .target_chapter
        .unwrap_or_else(|| request.written_chapters.len() + 1)
        .max(1);

    let prompt = build_story_continue_prompt(&request, next_chapter);
    let response = call_model_simple(&config, &prompt)?;

    let is_zh = request
        .language
        .as_deref()
        .map(|lang| lang.starts_with("zh"))
        .unwrap_or(true);

    match parse_story_continuation_response(&response, next_chapter) {
        Ok(mut parsed) => {
            if parsed.title.trim().is_empty() {
                parsed.title = if is_zh {
                    format!("第{}章", parsed.chapter)
                } else {
                    format!("Chapter {}", parsed.chapter)
                };
            }
            if parsed.content.trim().is_empty() {
                parsed.content = response.trim().to_string();
            }
            if parsed.summary.trim().is_empty() {
                parsed.summary = if is_zh {
                    "已生成本章草稿。".to_string()
                } else {
                    "Chapter draft generated.".to_string()
                };
            }
            Ok(parsed)
        }
        Err(_) => Ok(StoryContinuationResult {
            chapter: next_chapter,
            title: if is_zh {
                format!("第{}章（未结构化）", next_chapter)
            } else {
                format!("Chapter {} (Unstructured)", next_chapter)
            },
            content: response.trim().to_string(),
            summary: if is_zh {
                "模型未按 JSON 输出，已回退为原始文本。".to_string()
            } else {
                "Model did not return JSON, falling back to raw output.".to_string()
            },
        }),
    }
}

#[tauri::command]
fn rewrite_story_chapter(
    request: StoryRewriteChapterRequest,
    config_state: State<ModelConfigState>,
) -> Result<StoryContinuationResult, String> {
    let target = request
        .written_chapters
        .iter()
        .find(|x| x.chapter == request.target_chapter)
        .cloned()
        .ok_or_else(|| {
            format!(
                "Chapter {} not found in written chapters.",
                request.target_chapter
            )
        })?;

    let config = config_state.0.lock().map_err(|e| e.to_string())?.clone();
    if let ModelProvider::Ollama {
        base_url,
        model_name,
        ..
    } = &config.provider
    {
        ensure_ollama_running(base_url)?;
        ensure_model_available(base_url, model_name)?;
    }

    let prompt = build_story_rewrite_prompt(&request, &target);
    let response = call_model_simple(&config, &prompt)?;

    let is_zh = request
        .language
        .as_deref()
        .map(|lang| lang.starts_with("zh"))
        .unwrap_or(true);

    match parse_story_continuation_response(&response, request.target_chapter) {
        Ok(mut parsed) => {
            parsed.chapter = request.target_chapter;
            if parsed.title.trim().is_empty() {
                parsed.title = if !target.title.trim().is_empty() {
                    target.title
                } else if is_zh {
                    format!("第{}章", request.target_chapter)
                } else {
                    format!("Chapter {}", request.target_chapter)
                };
            }
            if parsed.content.trim().is_empty() {
                parsed.content = response.trim().to_string();
            }
            if parsed.summary.trim().is_empty() {
                parsed.summary = if is_zh {
                    "已按修改意见完成章节重写。".to_string()
                } else {
                    "Chapter has been rewritten based on the revision notes.".to_string()
                };
            }
            Ok(parsed)
        }
        Err(_) => Ok(StoryContinuationResult {
            chapter: request.target_chapter,
            title: if !target.title.trim().is_empty() {
                target.title
            } else if is_zh {
                format!("第{}章（重写）", request.target_chapter)
            } else {
                format!("Chapter {} (Rewrite)", request.target_chapter)
            },
            content: response.trim().to_string(),
            summary: if is_zh {
                "模型未按 JSON 输出，已回退为原始重写文本。".to_string()
            } else {
                "Model did not return JSON; using raw rewritten text.".to_string()
            },
        }),
    }
}

/// Download and open the Ollama installer (Windows/macOS: download package; Linux: open download page).
#[tauri::command]
fn download_ollama_installer() -> Result<String, String> {
    download_and_open_ollama_installer()
}

/// Get the current model configuration.
#[tauri::command]
fn get_model_config(config_state: State<ModelConfigState>) -> Result<ModelConfig, String> {
    let guard = config_state.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

/// Update and persist the model configuration.
#[tauri::command]
fn update_model_config(
    new_config: ModelConfig,
    config_state: State<ModelConfigState>,
    data_dir: State<AppDataDir>,
) -> Result<(), String> {
    let mut guard = config_state.0.lock().map_err(|e| e.to_string())?;
    *guard = new_config.clone();

    let config_path = get_current_data_dir(&data_dir)?.join("model_config.json");
    new_config.save_to_file(&config_path)?;

    Ok(())
}

/// Test whether the current model configuration is reachable.
#[tauri::command]
fn test_model_config(config: ModelConfig) -> Result<String, String> {
    match &config.provider {
        ModelProvider::Ollama { base_url, .. } => {
            let (is_running, msg) = check_ollama_status(base_url);
            if !is_running {
                return Err(msg);
            }
            call_model_simple(&config, "Hello, please reply: model is working correctly.")
        }
        ModelProvider::DeepSeek { .. } | ModelProvider::OpenAI { .. } => {
            call_model_simple(&config, "Hello, please reply: model is working correctly.")
        }
    }
}

/// Check the Ollama service status.
#[tauri::command]
fn check_ollama() -> Result<(bool, String), String> {
    Ok(check_ollama_status(OLLAMA_URL))
}

/// Helper: emit a setup log event to the frontend.
fn emit_setup_log(app: &tauri::AppHandle, msg: &str, status: &str) {
    let _ = app.emit(
        "ollama-setup-log",
        serde_json::json!({ "message": msg, "status": status }),
    );
}

/// Helper: emit a memory save progress event to the frontend.
/// `code` maps to an i18n key (e.g. "saveProgress.step1.extracting").
/// `params` carries dynamic values for interpolation (e.g. entity count).
fn emit_save_progress(app: &tauri::AppHandle, code: &str, status: &str, params: serde_json::Value) {
    let _ = app.emit(
        "memory-save-progress",
        serde_json::json!({ "code": code, "params": params, "status": status }),
    );
}

/// Helper: emit a setup-done event to the frontend.
fn emit_setup_done(app: &tauri::AppHandle, success: bool) {
    let _ = app.emit(
        "ollama-setup-done",
        serde_json::json!({ "success": success }),
    );
}

/// Blocking body of Ollama one-click setup: check install → start service → pull model.
fn do_ollama_setup(
    app: tauri::AppHandle,
    base_url: String,
    model_name: String,
    extract_model_name: String,
) {
    // Step 1: Check if Ollama is installed
    emit_setup_log(&app, "Checking Ollama installation...", "running");

    if !ollama::check_ollama_installed() {
        emit_setup_log(
            &app,
            "Ollama not found. Downloading installer...",
            "running",
        );
        match ollama_installer::download_and_open_ollama_installer() {
            Ok(msg) => {
                emit_setup_log(&app, &format!("✅ {}", msg), "success");
                emit_setup_log(
                    &app,
                    "⚠️ Please complete the Ollama installation and click [Initialize] again",
                    "warning",
                );
            }
            Err(e) => {
                emit_setup_log(
                    &app,
                    &format!("❌ Failed to download installer: {}", e),
                    "error",
                );
            }
        }
        emit_setup_done(&app, false);
        return;
    }
    emit_setup_log(&app, "✅ Ollama is installed", "success");

    // Step 2: Check and start the Ollama service
    emit_setup_log(&app, "Checking Ollama service status...", "running");
    let (running, _) = ollama::check_ollama_status(&base_url);
    if !running {
        emit_setup_log(
            &app,
            "Ollama service is not running. Starting...",
            "running",
        );
        match ollama::ensure_ollama_running(&base_url) {
            Ok(_) => emit_setup_log(&app, "✅ Ollama service started", "success"),
            Err(e) => {
                emit_setup_log(
                    &app,
                    &format!(
                        "❌ Failed to start service: {}. Please start Ollama manually and retry.",
                        e
                    ),
                    "error",
                );
                emit_setup_done(&app, false);
                return;
            }
        }
    } else {
        emit_setup_log(&app, "✅ Ollama service is running", "success");
    }

    // Step 3: Check and pull required models (skip if already present)
    let mut models: Vec<(String, &str)> = vec![(model_name.clone(), "Q&A")];
    if extract_model_name != model_name {
        models.push((extract_model_name.clone(), "extraction"));
    }

    for (model, label) in &models {
        emit_setup_log(
            &app,
            &format!("Checking {} model {}...", label, model),
            "running",
        );
        if ollama::check_model_exists(&base_url, model) {
            emit_setup_log(&app, &format!("✅ Model {} is ready", model), "success");
        } else {
            emit_setup_log(
                &app,
                &format!(
                    "Downloading {} model {} (this may take a few minutes)...",
                    label, model
                ),
                "running",
            );
            match ollama::pull_model(&base_url, model) {
                Ok(_) => emit_setup_log(&app, &format!("✅ Model {} downloaded", model), "success"),
                Err(e) => {
                    emit_setup_log(
                        &app,
                        &format!("❌ Failed to download model {}: {}", model, e),
                        "error",
                    );
                    emit_setup_done(&app, false);
                    return;
                }
            }
        }
    }

    emit_setup_log(
        &app,
        "🎉 Ollama setup complete! Everything is ready.",
        "success",
    );
    emit_setup_done(&app, true);
}

/// Ollama one-click setup: install check → start service → pull model (already-completed steps are skipped).
#[tauri::command]
async fn run_ollama_setup(
    app: tauri::AppHandle,
    config_state: State<'_, ModelConfigState>,
) -> Result<(), String> {
    let config = {
        let guard = config_state.0.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };

    let (base_url, model_name, extract_model_name) = match &config.provider {
        ModelProvider::Ollama {
            base_url,
            model_name,
            extract_model_name,
        } => (
            base_url.clone(),
            model_name.clone(),
            extract_model_name.clone(),
        ),
        _ => {
            return Err(
                "Ollama provider is not configured. Please select Ollama in Settings first."
                    .to_string(),
            )
        }
    };

    tokio::task::spawn_blocking(move || {
        do_ollama_setup(app, base_url, model_name, extract_model_name);
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;

            let libraries_root_dir = libraries_root(&app_data_dir);
            fs::create_dir_all(&libraries_root_dir).map_err(|e| e.to_string())?;
            let plugins_root_dir = plugins_root(&app_data_dir);
            fs::create_dir_all(&plugins_root_dir).map_err(|e| e.to_string())?;

            let default_library_id = "default".to_string();
            let default_library_dir = libraries_root_dir.join(&default_library_id);
            let legacy_db_path = app_data_dir.join("database").join("kraph.db");
            let legacy_memories_dir = app_data_dir.join("memories");
            let legacy_model_config = app_data_dir.join("model_config.json");

            if !default_library_dir.exists() {
                ensure_library_structure(&default_library_dir)?;
                // Preserve existing single-library data by moving it into the default library.
                move_path_if_exists(
                    &legacy_db_path,
                    &default_library_dir.join("database").join("kraph.db"),
                )?;
                move_path_if_exists(&legacy_memories_dir, &default_library_dir.join("memories"))?;
                move_path_if_exists(
                    &legacy_model_config,
                    &default_library_dir.join("model_config.json"),
                )?;
                let _ = save_library_meta(&default_library_dir, "Default", Some(false));
            }

            let mut current_library_id = read_saved_current_library_id(&app_data_dir)
                .unwrap_or_else(|| default_library_id.clone());
            if !libraries_root_dir.join(&current_library_id).exists() {
                current_library_id = default_library_id.clone();
            }
            let current_library_dir = libraries_root_dir.join(&current_library_id);
            ensure_library_structure(&current_library_dir)?;
            let _ = persist_current_library_id(&app_data_dir, &current_library_id);

            let db_path = current_library_dir.join("database").join("kraph.db");
            let conn = init_db(&db_path).map_err(|e| e.to_string())?;
            app.manage(DbState(Mutex::new(Some(conn))));
            app.manage(AppRootDir(app_data_dir.clone()));
            app.manage(AppDataDir(Mutex::new(current_library_dir.clone())));
            app.manage(CurrentLibraryId(Mutex::new(current_library_id)));

            // Load model configuration from the current library (or use defaults)
            let config_path = library_model_config_path(&current_library_dir);
            let model_config = ModelConfig::load_from_file(&config_path).unwrap_or_default();
            app.manage(ModelConfigState(Mutex::new(model_config)));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            list_memory_libraries,
            get_current_memory_library,
            create_memory_library,
            switch_memory_library,
            rename_memory_library,
            delete_memory_library,
            save_story_project,
            list_story_projects,
            load_story_project,
            list_external_plugins,
            get_plugins_folder_path,
            open_plugins_folder,
            install_external_plugin,
            uninstall_external_plugin,
            open_memories_folder,
            get_memories_folder_path,
            list_memories_dir,
            read_memory_file,
            extract_entities,
            save_memory,
            get_memories_list,
            get_graph,
            query_entity,
            search_memories_by_entity,
            get_character_profile,
            get_timeline,
            update_memory_content,
            delete_memory_by_id,
            cleanup_db,
            clear_all_data_cmd,
            setup_whisper,
            transcribe_audio,
            answer_question,
            generate_story_from_events,
            continue_story_chapter,
            rewrite_story_chapter,
            download_ollama_installer,
            check_ollama,
            run_ollama_setup,
            get_model_config,
            update_model_config,
            test_model_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
