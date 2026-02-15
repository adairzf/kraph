import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useSettingsStore = defineStore('settings', () => {
  const ollamaBaseUrl = ref('http://localhost:11434')
  const ollamaModel = ref('qwen2.5:7b')
  const theme = ref<'light' | 'dark' | 'auto'>('auto')

  return {
    ollamaBaseUrl,
    ollamaModel,
    theme,
  }
})
