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
use tauri::{Manager, State};

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
    fs::create_dir_all(&memories_dir).map_err(|e| format!("创建记忆目录失败: {e}"))?;

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&memories_dir)
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&memories_dir)
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&memories_dir)
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {e}"))?;
    }

    Ok(memories_dir.to_string_lossy().to_string())
}

#[tauri::command]
fn get_memories_folder_path(data_dir: State<AppDataDir>) -> Result<String, String> {
    let memories_dir = data_dir.0.join("memories");
    fs::create_dir_all(&memories_dir).map_err(|e| format!("创建记忆目录失败: {e}"))?;
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

#[tauri::command]
fn save_memory(
    content: String,
    tags: Option<Vec<String>>,
    db: State<DbState>,
    data_dir: State<AppDataDir>,
    config_state: State<ModelConfigState>,
) -> Result<Memory, String> {
    let memories_dir = data_dir.0.join("memories");
    
    // 获取当前配置
    let config = config_state.0.lock().map_err(|e| e.to_string())?.clone();
    
    // 打印当前使用的模型配置
    match &config.provider {
        ModelProvider::Ollama { base_url, model_name, extract_model_name } => {
            println!("📝 [保存记忆] 使用 Ollama 模型");
            println!("   - 服务地址: {}", base_url);
            println!("   - 问答模型: {}", model_name);
            println!("   - 提取模型: {}", extract_model_name);
        }
        ModelProvider::DeepSeek { model_name, .. } => {
            println!("📝 [保存记忆] 使用 DeepSeek API");
            println!("   - 模型: {}", model_name);
        }
        ModelProvider::OpenAI { model_name, .. } => {
            println!("📝 [保存记忆] 使用 OpenAI API");
            println!("   - 模型: {}", model_name);
        }
    }
    
    // 快速提取获取相关实体名（用于查找历史记忆）
    println!("🔍 [步骤1] 开始快速实体提取...");
    let quick_extracted = if content.trim().len() > 5 {
        // 如果是Ollama，先确保服务运行
        if let ModelProvider::Ollama { base_url, extract_model_name, .. } = &config.provider {
            let _ = ensure_ollama_running(base_url);
            let _ = ensure_model_available(base_url, extract_model_name);
        }
        
        call_model_extract(&config, ENTITY_EXTRACT_PROMPT, &content)
            .map_err(|e| {
                println!("❌ 快速提取失败: {}", e);
                e
            })
            .ok()
    } else {
        None
    };
    
    if let Some(ref ex) = quick_extracted {
        println!("✅ 提取到 {} 个实体", ex.entities.len());
    }
    
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    
    // 获取相关历史记忆（用于知识融合）
    println!("🔍 [步骤2] 查找相关历史记忆...");
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
        println!("✅ 找到 {} 条相关历史记忆", all_memories.len());
        all_memories
    } else {
        Vec::new()
    };
    
    // 使用知识融合进行深度推理（如果有历史记忆）
    let fused = if !historical_memories.is_empty() && content.trim().len() > 5 {
        println!("🧠 [步骤3] 开始知识融合推理...");
        // 如果是Ollama，先确保模型可用
        if let ModelProvider::Ollama { base_url, model_name, .. } = &config.provider {
            let _ = ensure_model_available(base_url, model_name);
        }
        
        call_model_fusion(
            &config,
            KNOWLEDGE_FUSION_PROMPT,
            &historical_memories,
            &content,
        )
        .map_err(|e| {
            println!("⚠️  知识融合失败，回退到快速提取: {}", e);
            e
        })
        .ok()
    } else {
        println!("⏭️  [步骤3] 跳过知识融合（无历史记忆）");
        None
    };
    
    // 如果知识融合失败，回退到快速提取
    let (entities, relations, aliases) = if let Some(fused_data) = fused {
        println!("✅ 知识融合完成: {} 个实体, {} 个关系, {} 个别名", 
                 fused_data.entities.len(), fused_data.relations.len(), fused_data.aliases.len());
        (fused_data.entities, fused_data.relations, fused_data.aliases)
    } else if let Some(ex) = quick_extracted {
        println!("✅ 使用快速提取结果: {} 个实体, {} 个关系", 
                 ex.entities.len(), ex.relations.len());
        (ex.entities, ex.relations, Vec::new())
    } else {
        println!("⚠️  未提取到任何实体");
        (Vec::new(), Vec::new(), Vec::new())
    };
    
    let entity_names: Vec<String> = entities.iter().map(|x| x.name.clone()).collect();
    
    println!("💾 [步骤4] 保存到数据库...");
    // 保存到文件
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
    
    // 保存到数据库
    let tags_str = tags.as_ref().map(|t| t.join(","));
    let memory_id = insert_memory(conn, &content, Some(&path_str), tags_str.as_deref())
        .map_err(|e| e.to_string())?;
    
    // 建立实体和关系（支持别名）
    let mut name_to_id: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    
    // 1. 先创建或获取所有实体
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
    
    // 2. 处理别名关系
    for alias_info in &aliases {
        let primary_id = name_to_id.get(&alias_info.primary);
        let alias_id = name_to_id.get(&alias_info.alias);
        
        match (primary_id, alias_id) {
            (Some(&pid), Some(&aid)) if pid != aid => {
                // 两个实体都存在且不同，需要合并
                merge_entities(conn, aid, pid).map_err(|e| e.to_string())?;
                // 更新name_to_id映射
                name_to_id.insert(alias_info.alias.clone(), pid);
            }
            (Some(&pid), None) => {
                // primary存在，alias不存在，添加别名
                add_entity_alias(conn, pid, &alias_info.alias).map_err(|e| e.to_string())?;
            }
            _ => {}
        }
    }
    
    // 3. 建立关系
    for r in &relations {
        if let (Some(&from_id), Some(&to_id)) = (name_to_id.get(&r.from), name_to_id.get(&r.to)) {
            let _ = upsert_relation(conn, from_id, to_id, &r.relation);
        }
    }
    
    println!("✅ 记忆保存完成！");
    get_memory_by_id(conn, memory_id).map_err(|e| e.to_string())
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
    Ok(serde_json::json!({
        "entity": entity,
        "memories": memories,
        "relations": entity_relations
    }))
}

#[tauri::command]
fn get_timeline(db: State<DbState>) -> Result<Vec<Memory>, String> {
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    list_memories(conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_memory_content(
    memory_id: i64,
    content: String,
    tags: Option<Vec<String>>,
    db: State<DbState>,
    config_state: State<ModelConfigState>,
) -> Result<Memory, String> {
    let tags_str = tags.map(|t| t.join(","));
    
    // 获取当前配置
    let config = config_state.0.lock().map_err(|e| e.to_string())?.clone();
    
    // 打印当前使用的模型配置
    println!("📝 [更新记忆 ID:{}]", memory_id);
    match &config.provider {
        ModelProvider::Ollama { base_url, model_name, extract_model_name } => {
            println!("   使用 Ollama: {}", base_url);
            println!("   提取模型: {}", extract_model_name);
        }
        ModelProvider::DeepSeek { model_name, .. } => {
            println!("   使用 DeepSeek: {}", model_name);
        }
        ModelProvider::OpenAI { model_name, .. } => {
            println!("   使用 OpenAI: {}", model_name);
        }
    }
    
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    
    // 快速提取获取相关实体名
    println!("🔍 开始实体提取...");
    let quick_extracted = if content.trim().len() > 5 {
        // 如果是Ollama，先确保服务运行
        if let ModelProvider::Ollama { base_url, extract_model_name, .. } = &config.provider {
            let _ = ensure_ollama_running(base_url);
            let _ = ensure_model_available(base_url, extract_model_name);
        }
        
        call_model_extract(&config, ENTITY_EXTRACT_PROMPT, &content).ok()
    } else {
        None
    };
    
    if let Some(ref ex) = quick_extracted {
        println!("✅ 提取到 {} 个实体", ex.entities.len());
    }
    
    // 获取相关历史记忆（用于知识融合）
    let historical_memories = if let Some(ref ex) = quick_extracted {
        let mut all_memories = Vec::new();
        for entity in &ex.entities {
            if let Ok(Some(existing_entity)) = get_entity_by_name(conn, &entity.name) {
                if let Ok(memories) = get_memories_for_entity(conn, existing_entity.id) {
                    for mem in memories.into_iter().take(5) {
                        // 排除当前正在编辑的记忆
                        if mem.id != memory_id && !all_memories.contains(&mem.content) {
                            all_memories.push(mem.content);
                        }
                    }
                }
            }
        }
        all_memories
    } else {
        Vec::new()
    };
    
    // 使用知识融合进行深度推理
    let fused = if !historical_memories.is_empty() && content.trim().len() > 5 {
        println!("🧠 进行知识融合...");
        // 如果是Ollama，先确保模型可用
        if let ModelProvider::Ollama { base_url, model_name, .. } = &config.provider {
            let _ = ensure_model_available(base_url, model_name);
        }
        
        call_model_fusion(
            &config,
            KNOWLEDGE_FUSION_PROMPT,
            &historical_memories,
            &content,
        ).ok()
    } else {
        None
    };
    
    // 如果知识融合失败，回退到快速提取
    let (entities, relations, aliases) = if let Some(fused_data) = fused {
        println!("✅ 知识融合完成");
        (fused_data.entities, fused_data.relations, fused_data.aliases)
    } else if let Some(ex) = quick_extracted {
        println!("✅ 使用快速提取结果");
        (ex.entities, ex.relations, Vec::new())
    } else {
        (Vec::new(), Vec::new(), Vec::new())
    };
    
    // 更新记忆内容
    update_memory(conn, memory_id, &content, tags_str.as_deref()).map_err(|e| e.to_string())?;
    
    // 清除旧的实体关联
    clear_memory_entities(conn, memory_id).map_err(|e| e.to_string())?;
    
    // 建立新的实体和关系（支持别名）
    let mut name_to_id: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    
    // 1. 先创建或获取所有实体
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
    
    // 2. 处理别名关系
    for alias_info in &aliases {
        let primary_id = name_to_id.get(&alias_info.primary);
        let alias_id = name_to_id.get(&alias_info.alias);
        
        match (primary_id, alias_id) {
            (Some(&pid), Some(&aid)) if pid != aid => {
                // 两个实体都存在且不同，需要合并
                merge_entities(conn, aid, pid).map_err(|e| e.to_string())?;
                // 更新name_to_id映射
                name_to_id.insert(alias_info.alias.clone(), pid);
            }
            (Some(&pid), None) => {
                // primary存在，alias不存在，添加别名
                add_entity_alias(conn, pid, &alias_info.alias).map_err(|e| e.to_string())?;
            }
            _ => {}
        }
    }
    
    // 3. 建立关系
    for r in &relations {
        if let (Some(&from_id), Some(&to_id)) = (name_to_id.get(&r.from), name_to_id.get(&r.to)) {
            let _ = upsert_relation(conn, from_id, to_id, &r.relation);
        }
    }
    
    // 清理孤立数据
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
    
    println!("✅ 记忆更新完成！");
    get_memory_by_id(conn, memory_id).map_err(|e| e.to_string())
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
    Ok("数据库清理完成".to_string())
}

/// 清空所有数据（危险操作，需谨慎）
#[tauri::command]
fn clear_all_data_cmd(db: State<DbState>, data_dir: State<AppDataDir>) -> Result<String, String> {
    let mut guard = (&*db).0.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    let conn = guard.as_mut().ok_or("database not initialized")?;
    
    // 清空数据库
    clear_all_data(conn).map_err(|e| e.to_string())?;
    
    // 清空记忆文件夹
    let memories_dir = data_dir.0.join("memories");
    if memories_dir.exists() {
        std::fs::remove_dir_all(&memories_dir).map_err(|e| format!("删除记忆文件失败: {}", e))?;
        std::fs::create_dir_all(&memories_dir).map_err(|e| format!("重建记忆文件夹失败: {}", e))?;
    }
    
    Ok("所有数据已清空".to_string())
}

/// 语音转文字：调用本地 whisper.cpp（whisper-cli）。
#[tauri::command]
fn transcribe_audio(audio_base64: String, data_dir: State<AppDataDir>) -> Result<String, String> {
    transcribe_audio_with_whisper(&audio_base64, &data_dir.0)
}

/// 一键准备 Whisper：自动安装 whisper-cpp（macOS）并下载基础模型。
#[tauri::command]
fn setup_whisper(data_dir: State<AppDataDir>) -> Result<String, String> {
    setup_whisper_runtime(&data_dir.0)
}

const OLLAMA_URL: &str = "http://localhost:11434";
/// 问答、从问题中抽实体名等需要「生成」的任务，用稍大模型
const OLLAMA_MODEL: &str = "qwen2.5:7b";
/// 实体拆分（人物/时间/地点/事件）：复杂文本需要7b模型才能准确提取
const OLLAMA_MODEL_EXTRACT: &str = "qwen2.5:7b";

/// 基于实体的记忆检索与智能问答
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
        return Ok("暂无相关记忆。请先记录一些内容。".to_string());
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

/// 下载并打开 Ollama 安装程序（Windows/Mac 直接下载安装包并打开，Linux 打开下载页）
#[tauri::command]
fn download_ollama_installer() -> Result<String, String> {
    download_and_open_ollama_installer()
}

/// 获取当前模型配置
#[tauri::command]
fn get_model_config(config_state: State<ModelConfigState>) -> Result<ModelConfig, String> {
    let guard = config_state.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

/// 更新模型配置
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

/// 测试模型配置是否可用
#[tauri::command]
fn test_model_config(config: ModelConfig) -> Result<String, String> {
    match &config.provider {
        ModelProvider::Ollama { base_url, model_name, .. } => {
            let (is_running, msg) = check_ollama_status(base_url);
            if !is_running {
                return Err(msg);
            }
            // 尝试简单调用
            call_model_simple(&config, "你好，请回复：模型正常工作。")
        }
        ModelProvider::DeepSeek { .. } | ModelProvider::OpenAI { .. } => {
            // 尝试简单调用
            call_model_simple(&config, "你好，请回复：模型正常工作。")
        }
    }
}

/// 检测 Ollama 服务状态
#[tauri::command]
fn check_ollama() -> Result<(bool, String), String> {
    Ok(check_ollama_status(OLLAMA_URL))
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
            
            // 加载模型配置
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
            get_model_config,
            update_model_config,
            test_model_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
