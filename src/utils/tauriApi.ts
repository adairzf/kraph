import { invoke } from '@tauri-apps/api/core'
import type { Memory, MdRecord } from '../types/memory'
import type { Entity } from '../types/entity'
import type { GraphData } from '../types/graph'
import type { ModelConfig } from '../types/model-config'

export interface ExtractedData {
  entities: { type: string; name: string; attributes?: unknown }[]
  relations: { from: string; to: string; relation: string }[]
}

export interface MemoryLibraryInfo {
  id: string
  name: string
  path: string
  is_current: boolean
  enable_time_normalization?: boolean
}

export interface StoryGenerationRequest {
  key_events: string[]
  genre?: string
  style?: string
  protagonist?: string
  chapter_count?: number
  constraints?: string
  language?: string
}

export interface StoryChapterPlan {
  chapter: number
  title: string
  goal: string
  conflict: string
  twist: string
  hook: string
}

export interface StoryGenerationResult {
  title: string
  premise: string
  outline: string[]
  chapter_plan: StoryChapterPlan[]
  first_chapter: string
  continuity_checks: string[]
}

export interface StoryWrittenChapter {
  chapter: number
  title: string
  content: string
  summary?: string
}

export interface StoryContinuationRequest {
  title: string
  premise: string
  outline: string[]
  chapter_plan: StoryChapterPlan[]
  continuity_checks: string[]
  written_chapters: StoryWrittenChapter[]
  target_chapter?: number
  style?: string
  constraints?: string
  language?: string
}

export interface StoryContinuationResult {
  chapter: number
  title: string
  content: string
  summary: string
}

export interface StoryProjectSaveRequest {
  project_id?: string
  title: string
  premise: string
  outline: string[]
  chapter_plan: StoryChapterPlan[]
  continuity_checks: string[]
  written_chapters: StoryWrittenChapter[]
  style?: string
  constraints?: string
  language?: string
}

export interface StoryProjectSummary {
  id: string
  title: string
  updated_at: string
}

export interface StoryProjectData {
  project_id: string
  title: string
  premise: string
  outline: string[]
  chapter_plan: StoryChapterPlan[]
  continuity_checks: string[]
  written_chapters: StoryWrittenChapter[]
  style?: string
  constraints?: string
  language?: string
  created_at: string
  updated_at: string
}

export interface StoryRewriteChapterRequest {
  title: string
  premise: string
  outline: string[]
  chapter_plan: StoryChapterPlan[]
  continuity_checks: string[]
  written_chapters: StoryWrittenChapter[]
  target_chapter: number
  feedback?: string
  style?: string
  constraints?: string
  language?: string
}

export interface ExternalPluginInfo {
  id: string
  name: string
  version: string
  tab_key?: string
  menu_key?: string
  description?: string
  entry?: string
  entry_path?: string
  install_path: string
}

export async function listMemoriesDir(): Promise<string[]> {
  return invoke('list_memories_dir')
}

export async function listMemoryLibraries(): Promise<MemoryLibraryInfo[]> {
  return invoke('list_memory_libraries')
}

export async function getCurrentMemoryLibrary(): Promise<MemoryLibraryInfo> {
  return invoke('get_current_memory_library')
}

export async function createMemoryLibrary(
  name: string,
  enableTimeNormalization?: boolean,
): Promise<MemoryLibraryInfo> {
  return invoke('create_memory_library', { name, enableTimeNormalization })
}

export async function switchMemoryLibrary(libraryId: string): Promise<MemoryLibraryInfo> {
  return invoke('switch_memory_library', { libraryId })
}

export async function renameMemoryLibrary(
  libraryId: string,
  name: string
): Promise<MemoryLibraryInfo> {
  return invoke('rename_memory_library', { libraryId, name })
}

export async function deleteMemoryLibrary(libraryId: string): Promise<string> {
  return invoke('delete_memory_library', { libraryId })
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
  relations: {
    from_entity_id: number
    from_name: string
    to_entity_id: number
    to_name: string
    relation_type: string
    strength: number
  }[]
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

export async function generateStoryFromEvents(
  request: StoryGenerationRequest
): Promise<StoryGenerationResult> {
  return invoke('generate_story_from_events', { request })
}

export async function continueStoryChapter(
  request: StoryContinuationRequest
): Promise<StoryContinuationResult> {
  return invoke('continue_story_chapter', { request })
}

export async function rewriteStoryChapter(
  request: StoryRewriteChapterRequest
): Promise<StoryContinuationResult> {
  return invoke('rewrite_story_chapter', { request })
}

export async function saveStoryProject(
  request: StoryProjectSaveRequest
): Promise<StoryProjectSummary> {
  return invoke('save_story_project', { request })
}

export async function listStoryProjects(): Promise<StoryProjectSummary[]> {
  return invoke('list_story_projects')
}

export async function loadStoryProject(projectId: string): Promise<StoryProjectData> {
  return invoke('load_story_project', { projectId })
}

export async function listExternalPlugins(): Promise<ExternalPluginInfo[]> {
  return invoke('list_external_plugins')
}

export async function getPluginsFolderPath(): Promise<string> {
  return invoke('get_plugins_folder_path')
}

export async function openPluginsFolder(): Promise<string> {
  return invoke('open_plugins_folder')
}

export async function installExternalPlugin(sourcePath: string): Promise<ExternalPluginInfo> {
  return invoke('install_external_plugin', { sourcePath })
}

export async function uninstallExternalPlugin(pluginId: string): Promise<string> {
  return invoke('uninstall_external_plugin', { pluginId })
}

/** Download and open the Ollama installer (Win/Mac: download package; Linux: open download page). */
export async function downloadOllamaInstaller(): Promise<string> {
  return invoke('download_ollama_installer')
}

/** Check the Ollama service status. */
export async function checkOllama(): Promise<[boolean, string]> {
  return invoke('check_ollama')
}

/**
 * One-click Ollama setup: check install → start service → pull model.
 * Progress is delivered to the frontend via Tauri events "ollama-setup-log" / "ollama-setup-done".
 */
export async function runOllamaSetup(): Promise<void> {
  return invoke('run_ollama_setup')
}

/** Get the current model configuration. */
export async function getModelConfig(): Promise<ModelConfig> {
  return invoke('get_model_config')
}

/** Persist an updated model configuration. */
export async function updateModelConfig(config: ModelConfig): Promise<void> {
  return invoke('update_model_config', { newConfig: config })
}

/** Test whether the current model configuration is reachable. */
export async function testModelConfig(config: ModelConfig): Promise<string> {
  return invoke('test_model_config', { config })
}
