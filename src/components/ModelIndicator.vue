<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getModelConfig } from '../utils/tauriApi'

const loading = ref(false)
const currentModel = ref('')

onMounted(async () => {
  await loadCurrentModel()
})

async function loadCurrentModel() {
  loading.value = true
  try {
    const config = await getModelConfig()
    if (config.provider.type === 'ollama') {
      const provider = config.provider as any
      currentModel.value = `本地 Ollama (${provider.extract_model_name})`
    } else if (config.provider.type === 'deepseek') {
      const provider = config.provider as any
      currentModel.value = `DeepSeek (${provider.model_name})`
    } else if (config.provider.type === 'openai') {
      const provider = config.provider as any
      currentModel.value = `OpenAI (${provider.model_name})`
    }
  } catch (error) {
    currentModel.value = '未配置'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="model-indicator">
    <span class="label">当前模型:</span>
    <span v-if="loading" class="value loading">加载中...</span>
    <span v-else class="value">{{ currentModel }}</span>
  </div>
</template>

<style scoped>
.model-indicator {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.3rem 0.6rem;
  background: #e8f4f8;
  border-radius: 4px;
  font-size: 0.8125rem;
}

.label {
  color: #666;
  font-weight: 500;
}

.value {
  color: #24c8db;
  font-weight: 600;
}

.value.loading {
  color: #999;
}
</style>
