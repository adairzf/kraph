import { invoke } from '@tauri-apps/api/core'
import type { Memory, MdRecord } from '../types/memory'
import type { Entity } from '../types/entity'
import type { GraphData } from '../types/graph'
import type { ModelConfig } from '../types/model-config'

export interface ExtractedData {
  entities: { type: string; name: string; attributes?: unknown }[]
  relations: { from: string; to: string; relation: string }[]
}

export async function listMemoriesDir(): Promise<string[]> {
  return invoke('list_memories_dir')
}

export async function openMemoriesFolder(): Promise<string> {
  return invoke('open_memories_folder')
}

export async function getMemoriesFolderPath(): Promise<string> {
  return invoke('get_memories_folder_path')
}

export async function readMemoryFile(path: string): Promise<MdRecord> {
  return invoke('read_memory_file', { path })
}

export async function extractEntities(text: string): Promise<ExtractedData> {
  return invoke('extract_entities', { text })
}

export async function saveMemory(
  content: string,
  tags?: string[]
): Promise<Memory> {
  return invoke('save_memory', { content, tags })
}

export async function getMemoriesList(): Promise<Memory[]> {
  return invoke('get_memories_list')
}

export async function getGraph(): Promise<GraphData> {
  return invoke('get_graph')
}

export async function queryEntity(name: string): Promise<Entity | null> {
  return invoke('query_entity', { name })
}

export async function searchMemoriesByEntity(
  entityId: number
): Promise<Memory[]> {
  return invoke('search_memories_by_entity', { entityId })
}

export async function getCharacterProfile(entityId: number): Promise<{
  entity: Entity
  memories: Memory[]
  relations: { from_entity_id: number; to_entity_id: number; relation_type: string }[]
}> {
  return invoke('get_character_profile', { entityId })
}

export async function getTimeline(): Promise<Memory[]> {
  return invoke('get_timeline')
}

export async function updateMemory(
  memoryId: number,
  content: string,
  tags?: string[]
): Promise<Memory> {
  return invoke('update_memory_content', { memoryId, content, tags })
}

export async function deleteMemory(memoryId: number): Promise<void> {
  return invoke('delete_memory_by_id', { memoryId })
}

export async function cleanupDatabase(): Promise<string> {
  return invoke('cleanup_db')
}

export async function clearAllData(): Promise<string> {
  return invoke('clear_all_data_cmd')
}

export async function transcribeAudio(audioBase64: string): Promise<string> {
  return invoke('transcribe_audio', { audioBase64 })
}

export async function setupWhisper(): Promise<string> {
  return invoke('setup_whisper')
}

export async function answerQuestion(question: string): Promise<string> {
  return invoke('answer_question', { question })
}

/** 下载并打开 Ollama 安装程序（Win/Mac 下载安装包并打开，Linux 打开下载页） */
export async function downloadOllamaInstaller(): Promise<string> {
  return invoke('download_ollama_installer')
}

/** 检测 Ollama 服务状态 */
export async function checkOllama(): Promise<[boolean, string]> {
  return invoke('check_ollama')
}

/** 获取当前模型配置 */
export async function getModelConfig(): Promise<ModelConfig> {
  return invoke('get_model_config')
}

/** 更新模型配置 */
export async function updateModelConfig(config: ModelConfig): Promise<void> {
  return invoke('update_model_config', { newConfig: config })
}

/** 测试模型配置是否可用 */
export async function testModelConfig(config: ModelConfig): Promise<string> {
  return invoke('test_model_config', { config })
}
