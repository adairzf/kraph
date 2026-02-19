mod database;
mod file_manager;
mod model_client;
mod model_config;
mod ollama;
mod ollama_installer;
mod whisper;

use database::{
    get_entity_by_id, get_entity_by_name, get_graph_data, get_memories_for_entity,
    get_memory_by_id, init_db, insert_memory, link_memory_entity, list_memories,
    list_relations, upsert_entity, upsert_relation, update_memory, delete_memory,
    clear_memory_entities, cleanup_database, clear_all_data, add_entity_alias,
    get_entity_aliases, find_entity_id_by_name_or_alias, merge_entities,
    DbState, Entity, GraphData, Memory,
};
use file_manager::{list_memory_files, read_memory, write_memory, MdRecord};
use model_client::{call_model_extract, call_model_fusion, call_model_simple};
use model_config::{ModelConfig, ModelProvider};
use ollama::{
    call_ollama_extract_blocking, call_ollama_simple, call_ollama_knowledge_fusion,
    check_ollama_status, ensure_model_available, ensure_ollama_running,
    ExtractedData, FusedKnowledge, ENTITY_EXTRACT_PROMPT, KNOWLEDGE_FUSION_PROMPT,
};
use ollama_installer::download_and_open_ollama_installer;
use whisper::{setup_whisper as setup_whisper_runtime, transcribe_audio_with_whisper};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};

pub struct AppDataDir(pub PathBuf);
pub struct ModelConfigState(pub Mutex<ModelConfig>);

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn list_memories_dir(data_dir: State<AppDataDir>) -> Result<Vec<String>, String> {
    let memories_dir = data_dir.0.join("memories");
    let paths = list_memory_files(&memories_dir)?;
    Ok(paths
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
fn open_memories_folder(data_dir: State<AppDataDir>) -> Result<String, String> {
    let memories_dir = data_dir.0.join("memories");
    fs::create_dir_all(&memories_dir).map_err(|e| format!("Failed to create memories directory: {e}"))?;

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&memories_dir)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&memories_dir)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&memories_dir)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
    }

    Ok(memories_dir.to_string_lossy().to_string())
}

#[tauri::command]
fn get_memories_folder_path(data_dir: State<AppDataDir>) -> Result<String, String> {
    let memories_dir = data_dir.0.join("memories");
    fs::create_dir_all(&memories_dir).map_err(|e| format!("Failed to create memories directory: {e}"))?;
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
    call_ollama_extract_blocking("http://localhost:11434", OLLAMA_MODEL_EXTRACT, &text)
        .or_else(|_| {
            let _ = ensure_model_available("http://localhost:11434", OLLAMA_MODEL);
            call_ollama_extract_blocking("http://localhost:11434", OLLAMA_MODEL, &text)
        })
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
        ModelProvider::Ollama { model_name, extract_model_name, .. } => {
            emit_save_progress(&app, &format!("📝 Using Ollama (extract model: {})", extract_model_name), "info");
            println!("📝 [save_memory] Using Ollama model: {}", model_name);
        }
        ModelProvider::DeepSeek { model_name, .. } => {
            emit_save_progress(&app, &format!("📝 Using DeepSeek API ({})", model_name), "info");
            println!("📝 [save_memory] Using DeepSeek API: {}", model_name);
        }
        ModelProvider::OpenAI { model_name, .. } => {
            emit_save_progress(&app, &format!("📝 Using OpenAI API ({})", model_name), "info");
            println!("📝 [save_memory] Using OpenAI API: {}", model_name);
        }
    }

    // Step 1: Quick entity extraction to find related entities for history lookup
    emit_save_progress(&app, "🔍 Step 1/4: Extracting entities...", "running");
    println!("🔍 [Step 1] Starting entity extraction...");
    let quick_extracted: Option<ExtractedData> = if content.trim().len() > 5 {
        if let ModelProvider::Ollama { base_url, extract_model_name, .. } = &config.provider {
            let _ = ensure_ollama_running(base_url);
            let _ = ensure_model_available(base_url, extract_model_name);
        }
        let extracted = call_model_extract(&config, ENTITY_EXTRACT_PROMPT, &content)
            .map_err(|e| {
                emit_save_progress(&app, &format!("❌ Entity extraction failed: {}", e), "error");
                println!("❌ Extraction failed: {}", e);
                e
            })?;
        Some(extracted)
    } else {
        None
    };

    if let Some(ref ex) = quick_extracted {
        emit_save_progress(&app, &format!("✅ Extracted {} entities", ex.entities.len()), "success");
        println!("✅ Extracted {} entities", ex.entities.len());
    }

    let db = app.state::<DbState>();
    let mut guard = db.0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;

    // Step 2: Fetch related historical memories for knowledge fusion
    emit_save_progress(&app, "🔍 Step 2/4: Looking up related memories...", "running");
    println!("🔍 [Step 2] Looking up related historical memories...");
    let historical_memories = if let Some(ref ex) = quick_extracted {
        let mut all_memories = Vec::new();
        for entity in &ex.entities {
            if let Ok(Some(existing_entity)) = get_entity_by_name(conn, &entity.name) {
                if let Ok(memories) = get_memories_for_entity(conn, existing_entity.id) {
                    for mem in memories.into_iter().take(5) {
                        if !all_memories.contains(&mem.content) {
                            all_memories.push(mem.content);
                        }
                    }
                }
            }
        }
        emit_save_progress(&app, &format!("✅ Found {} related memories", all_memories.len()), "success");
        println!("✅ Found {} related historical memories", all_memories.len());
        all_memories
    } else {
        emit_save_progress(&app, "✅ No history lookup needed", "success");
        Vec::new()
    };

    // Step 3: Knowledge fusion (only when historical memories exist)
    let fused = if !historical_memories.is_empty() && content.trim().len() > 5 {
        emit_save_progress(&app, "🧠 Step 3/4: Running knowledge fusion...", "running");
        println!("🧠 [Step 3] Starting knowledge fusion...");
        if let ModelProvider::Ollama { base_url, model_name, .. } = &config.provider {
            let _ = ensure_model_available(base_url, model_name);
        }
        call_model_fusion(&config, KNOWLEDGE_FUSION_PROMPT, &historical_memories, &content)
            .map_err(|e| {
                emit_save_progress(&app, "⚠️ Knowledge fusion failed, falling back to quick extraction", "warning");
                println!("⚠️ Knowledge fusion failed, falling back: {}", e);
                e
            })
            .ok()
    } else {
        emit_save_progress(&app, "⏭️ Step 3/4: Skipping knowledge fusion (no history)", "skipped");
        println!("⏭️ [Step 3] Skipping knowledge fusion (no historical memories)");
        None
    };

    let (entities, relations, aliases) = if let Some(fused_data) = fused {
        emit_save_progress(&app, &format!("✅ Knowledge fusion done: {} entities, {} relations",
                 fused_data.entities.len(), fused_data.relations.len()), "success");
        println!("✅ Knowledge fusion complete: {} entities, {} relations, {} aliases",
                 fused_data.entities.len(), fused_data.relations.len(), fused_data.aliases.len());
        (fused_data.entities, fused_data.relations, fused_data.aliases)
    } else if let Some(ex) = quick_extracted {
        emit_save_progress(&app, &format!("✅ Extraction done: {} entities, {} relations",
                 ex.entities.len(), ex.relations.len()), "success");
        println!("✅ Using quick extraction results: {} entities, {} relations",
                 ex.entities.len(), ex.relations.len());
        (ex.entities, ex.relations, Vec::new())
    } else {
        emit_save_progress(&app, "⚠️ No entities extracted", "warning");
        println!("⚠️ No entities extracted");
        (Vec::new(), Vec::new(), Vec::new())
    };

    let entity_names: Vec<String> = entities.iter().map(|x| x.name.clone()).collect();

    // Step 4: Persist to database
    emit_save_progress(&app, "💾 Step 4/4: Saving to database...", "running");
    println!("💾 [Step 4] Saving to database...");
    let path = write_memory(
        &memories_dir,
        &content,
        tags.as_deref(),
        if entity_names.is_empty() { None } else { Some(&entity_names) },
    )?;
    let path_str = path.to_string_lossy().to_string();

    let tags_str = tags.as_ref().map(|t| t.join(","));
    let memory_id = insert_memory(conn, &content, Some(&path_str), tags_str.as_deref())
        .map_err(|e| e.to_string())?;

    let mut name_to_id: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for e in &entities {
        let attrs = e.attributes.as_ref().map(|a| a.to_string());
        let entity_id = match find_entity_id_by_name_or_alias(conn, &e.name).map_err(|e| e.to_string())? {
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
        if let (Some(&from_id), Some(&to_id)) = (name_to_id.get(&r.from), name_to_id.get(&r.to)) {
            let _ = upsert_relation(conn, from_id, to_id, &r.relation);
        }
    }

    emit_save_progress(&app, "✅ Memory saved successfully!", "done");
    println!("✅ Memory saved successfully!");
    get_memory_by_id(conn, memory_id).map_err(|e| e.to_string())
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
    let memories_dir = data_dir.0.join("memories");
    tokio::task::spawn_blocking(move || {
        do_save_memory(app, content, tags, config, memories_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn get_memories_list(db: State<DbState>) -> Result<Vec<Memory>, String> {
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    list_memories(conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_graph(db: State<DbState>) -> Result<GraphData, String> {
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    get_graph_data(conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn query_entity(name: String, db: State<DbState>) -> Result<Option<Entity>, String> {
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    get_entity_by_name(conn, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_memories_by_entity(entity_id: i64, db: State<DbState>) -> Result<Vec<Memory>, String> {
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    get_memories_for_entity(conn, entity_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_character_profile(entity_id: i64, db: State<DbState>) -> Result<serde_json::Value, String> {
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
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
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
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
        ModelProvider::Ollama { extract_model_name, .. } => {
            emit_save_progress(&app, &format!("📝 Updating memory using Ollama (extract: {})", extract_model_name), "info");
        }
        ModelProvider::DeepSeek { model_name, .. } => {
            emit_save_progress(&app, &format!("📝 Updating memory using DeepSeek ({})", model_name), "info");
        }
        ModelProvider::OpenAI { model_name, .. } => {
            emit_save_progress(&app, &format!("📝 Updating memory using OpenAI ({})", model_name), "info");
        }
    }

    // Step 1: Quick entity extraction
    emit_save_progress(&app, "🔍 Step 1/4: Extracting entities...", "running");
    println!("🔍 Starting entity extraction...");
    let quick_extracted = if content.trim().len() > 5 {
        if let ModelProvider::Ollama { base_url, extract_model_name, .. } = &config.provider {
            let _ = ensure_ollama_running(base_url);
            let _ = ensure_model_available(base_url, extract_model_name);
        }
        call_model_extract(&config, ENTITY_EXTRACT_PROMPT, &content).ok()
    } else {
        None
    };

    if let Some(ref ex) = quick_extracted {
        emit_save_progress(&app, &format!("✅ Extracted {} entities", ex.entities.len()), "success");
        println!("✅ Extracted {} entities", ex.entities.len());
    }

    let db = app.state::<DbState>();
    let mut guard = db.0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;

    // Step 2: Fetch related historical memories for knowledge fusion
    emit_save_progress(&app, "🔍 Step 2/4: Looking up related memories...", "running");
    let historical_memories = if let Some(ref ex) = quick_extracted {
        let mut all_memories = Vec::new();
        for entity in &ex.entities {
            if let Ok(Some(existing_entity)) = get_entity_by_name(conn, &entity.name) {
                if let Ok(memories) = get_memories_for_entity(conn, existing_entity.id) {
                    for mem in memories.into_iter().take(5) {
                        if mem.id != memory_id && !all_memories.contains(&mem.content) {
                            all_memories.push(mem.content);
                        }
                    }
                }
            }
        }
        emit_save_progress(&app, &format!("✅ Found {} related memories", all_memories.len()), "success");
        all_memories
    } else {
        emit_save_progress(&app, "✅ No history lookup needed", "success");
        Vec::new()
    };

    // Step 3: Knowledge fusion
    let fused = if !historical_memories.is_empty() && content.trim().len() > 5 {
        emit_save_progress(&app, "🧠 Step 3/4: Running knowledge fusion...", "running");
        println!("🧠 Running knowledge fusion...");
        if let ModelProvider::Ollama { base_url, model_name, .. } = &config.provider {
            let _ = ensure_model_available(base_url, model_name);
        }
        call_model_fusion(&config, KNOWLEDGE_FUSION_PROMPT, &historical_memories, &content).ok()
    } else {
        emit_save_progress(&app, "⏭️ Step 3/4: Skipping knowledge fusion (no history)", "skipped");
        None
    };

    let (entities, relations, aliases) = if let Some(fused_data) = fused {
        emit_save_progress(&app, &format!("✅ Knowledge fusion done: {} entities, {} relations",
                 fused_data.entities.len(), fused_data.relations.len()), "success");
        println!("✅ Knowledge fusion complete");
        (fused_data.entities, fused_data.relations, fused_data.aliases)
    } else if let Some(ex) = quick_extracted {
        emit_save_progress(&app, &format!("✅ Extraction done: {} entities, {} relations",
                 ex.entities.len(), ex.relations.len()), "success");
        println!("✅ Using quick extraction results");
        (ex.entities, ex.relations, Vec::new())
    } else {
        emit_save_progress(&app, "⚠️ No entities extracted", "warning");
        (Vec::new(), Vec::new(), Vec::new())
    };

    // Step 4: Persist to database
    emit_save_progress(&app, "💾 Step 4/4: Saving to database...", "running");

    update_memory(conn, memory_id, &content, tags_str.as_deref()).map_err(|e| e.to_string())?;
    clear_memory_entities(conn, memory_id).map_err(|e| e.to_string())?;

    let mut name_to_id: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for e in &entities {
        let attrs = e.attributes.as_ref().map(|a| a.to_string());
        let entity_id = match find_entity_id_by_name_or_alias(conn, &e.name).map_err(|e| e.to_string())? {
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
        if let (Some(&from_id), Some(&to_id)) = (name_to_id.get(&r.from), name_to_id.get(&r.to)) {
            let _ = upsert_relation(conn, from_id, to_id, &r.relation);
        }
    }

    conn.execute(
        r#"DELETE FROM relations
           WHERE from_entity_id NOT IN (SELECT id FROM entities)
              OR to_entity_id NOT IN (SELECT id FROM entities)"#,
        [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM entities WHERE id NOT IN (SELECT DISTINCT entity_id FROM memory_entities)",
        [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
        r#"DELETE FROM relations
           WHERE from_entity_id NOT IN (SELECT id FROM entities)
              OR to_entity_id NOT IN (SELECT id FROM entities)"#,
        [],
    ).map_err(|e| e.to_string())?;

    emit_save_progress(&app, "✅ Memory updated successfully!", "done");
    println!("✅ Memory updated successfully!");
    get_memory_by_id(conn, memory_id).map_err(|e| e.to_string())
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
    tokio::task::spawn_blocking(move || {
        do_update_memory(app, memory_id, content, tags_str, config)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn delete_memory_by_id(memory_id: i64, db: State<DbState>) -> Result<(), String> {
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    delete_memory(conn, memory_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn cleanup_db(db: State<DbState>) -> Result<String, String> {
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    cleanup_database(conn).map_err(|e| e.to_string())?;
    Ok("Database cleanup complete".to_string())
}

/// Clear all data (destructive — use with caution).
#[tauri::command]
fn clear_all_data_cmd(db: State<DbState>, data_dir: State<AppDataDir>) -> Result<String, String> {
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;

    // Clear database tables
    clear_all_data(conn).map_err(|e| e.to_string())?;

    // Remove and recreate the memories folder
    let memories_dir = data_dir.0.join("memories");
    if memories_dir.exists() {
        std::fs::remove_dir_all(&memories_dir).map_err(|e| format!("Failed to delete memories folder: {}", e))?;
        std::fs::create_dir_all(&memories_dir).map_err(|e| format!("Failed to recreate memories folder: {}", e))?;
    }

    Ok("All data has been cleared".to_string())
}

/// Transcribe audio: calls the local whisper.cpp (whisper-cli).
#[tauri::command]
fn transcribe_audio(audio_base64: String, data_dir: State<AppDataDir>) -> Result<String, String> {
    transcribe_audio_with_whisper(&audio_base64, &data_dir.0)
}

/// Set up Whisper: auto-installs whisper-cpp (macOS) and downloads the base model.
#[tauri::command]
fn setup_whisper(data_dir: State<AppDataDir>) -> Result<String, String> {
    setup_whisper_runtime(&data_dir.0)
}

const OLLAMA_URL: &str = "http://localhost:11434";
/// Model used for Q&A and generation tasks (requires a reasonably capable model).
const OLLAMA_MODEL: &str = "qwen2.5:7b";
/// Model used for entity extraction (person / time / location / event).
const OLLAMA_MODEL_EXTRACT: &str = "qwen2.5:7b";

/// Entity-aware memory retrieval and intelligent Q&A.
#[tauri::command]
fn answer_question(question: String, db: State<DbState>) -> Result<String, String> {
    if question.trim().is_empty() {
        return Ok(String::new());
    }
    ensure_ollama_running(OLLAMA_URL)?;
    ensure_model_available(OLLAMA_URL, OLLAMA_MODEL)?;
    let entity_name = call_ollama_simple(
        OLLAMA_URL,
        OLLAMA_MODEL,
        &format!("{}{}", ollama::EXTRACT_ENTITY_PROMPT, question.trim()),
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

    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;

    let memories = if let Some(name) = entity_name {
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
        question.trim()
    );

    call_ollama_simple(OLLAMA_URL, OLLAMA_MODEL, &prompt)
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

    let config_path = data_dir.0.join("model_config.json");
    new_config.save_to_file(&config_path)?;

    Ok(())
}

/// Test whether the current model configuration is reachable.
#[tauri::command]
fn test_model_config(config: ModelConfig) -> Result<String, String> {
    match &config.provider {
        ModelProvider::Ollama { base_url, model_name, .. } => {
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
fn emit_save_progress(app: &tauri::AppHandle, msg: &str, status: &str) {
    let _ = app.emit(
        "memory-save-progress",
        serde_json::json!({ "message": msg, "status": status }),
    );
}

/// Helper: emit a setup-done event to the frontend.
fn emit_setup_done(app: &tauri::AppHandle, success: bool) {
    let _ = app.emit("ollama-setup-done", serde_json::json!({ "success": success }));
}

/// Blocking body of Ollama one-click setup: check install → start service → pull model.
fn do_ollama_setup(app: tauri::AppHandle, base_url: String, model_name: String, extract_model_name: String) {
    // Step 1: Check if Ollama is installed
    emit_setup_log(&app, "Checking Ollama installation...", "running");

    if !ollama::check_ollama_installed() {
        emit_setup_log(&app, "Ollama not found. Downloading installer...", "running");
        match ollama_installer::download_and_open_ollama_installer() {
            Ok(msg) => {
                emit_setup_log(&app, &format!("✅ {}", msg), "success");
                emit_setup_log(&app, "⚠️ Please complete the Ollama installation and click [Initialize] again", "warning");
            }
            Err(e) => {
                emit_setup_log(&app, &format!("❌ Failed to download installer: {}", e), "error");
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
        emit_setup_log(&app, "Ollama service is not running. Starting...", "running");
        match ollama::ensure_ollama_running(&base_url) {
            Ok(_) => emit_setup_log(&app, "✅ Ollama service started", "success"),
            Err(e) => {
                emit_setup_log(&app, &format!("❌ Failed to start service: {}. Please start Ollama manually and retry.", e), "error");
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
        emit_setup_log(&app, &format!("Checking {} model {}...", label, model), "running");
        if ollama::check_model_exists(&base_url, model) {
            emit_setup_log(&app, &format!("✅ Model {} is ready", model), "success");
        } else {
            emit_setup_log(
                &app,
                &format!("Downloading {} model {} (this may take a few minutes)...", label, model),
                "running",
            );
            match ollama::pull_model(&base_url, model) {
                Ok(_) => emit_setup_log(&app, &format!("✅ Model {} downloaded", model), "success"),
                Err(e) => {
                    emit_setup_log(&app, &format!("❌ Failed to download model {}: {}", model, e), "error");
                    emit_setup_done(&app, false);
                    return;
                }
            }
        }
    }

    emit_setup_log(&app, "🎉 Ollama setup complete! Everything is ready.", "success");
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
        ModelProvider::Ollama { base_url, model_name, extract_model_name } => {
            (base_url.clone(), model_name.clone(), extract_model_name.clone())
        }
        _ => return Err("Ollama provider is not configured. Please select Ollama in Settings first.".to_string()),
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
            let db_dir = app_data_dir.join("database");
            let db_path = db_dir.join("memoryai.db");
            let conn = init_db(&db_path).map_err(|e| e.to_string())?;
            app.manage(DbState(Mutex::new(Some(conn))));
            app.manage(AppDataDir(app_data_dir.clone()));

            // Load model configuration from disk (or use defaults)
            let config_path = app_data_dir.join("model_config.json");
            let model_config = ModelConfig::load_from_file(&config_path).unwrap_or_default();
            app.manage(ModelConfigState(Mutex::new(model_config)));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
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
