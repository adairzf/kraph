// Type definitions for model configuration

export type ModelProviderType = 'ollama' | 'deepseek' | 'openai'

export interface OllamaProvider {
  type: 'ollama'
  base_url: string
  model_name: string
  extract_model_name: string
}

export interface DeepSeekProvider {
  type: 'deepseek'
  api_key: string
  base_url: string
  model_name: string
}

export interface OpenAIProvider {
  type: 'openai'
  api_key: string
  base_url: string
  model_name: string
}

export type ModelProvider = OllamaProvider | DeepSeekProvider | OpenAIProvider

export interface ModelConfig {
  provider: ModelProvider
  temperature: number
  max_tokens: number
}

export const DEFAULT_OLLAMA_CONFIG: OllamaProvider = {
  type: 'ollama',
  base_url: 'http://localhost:11434',
  model_name: 'qwen2.5:7b',
  extract_model_name: 'qwen2.5:7b',
}

export const DEFAULT_DEEPSEEK_CONFIG: DeepSeekProvider = {
  type: 'deepseek',
  api_key: '',
  base_url: 'https://api.deepseek.com/v1',
  model_name: 'deepseek-chat',
}

export const DEFAULT_OPENAI_CONFIG: OpenAIProvider = {
  type: 'openai',
  api_key: '',
  base_url: 'https://api.openai.com/v1',
  model_name: 'gpt-4',
}
