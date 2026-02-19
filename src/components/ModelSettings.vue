<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getModelConfig, updateModelConfig, testModelConfig } from '../utils/tauriApi'
import type { ModelConfig, ModelProviderType, OllamaProvider, DeepSeekProvider, OpenAIProvider } from '../types/model-config'
import { DEFAULT_OLLAMA_CONFIG, DEFAULT_DEEPSEEK_CONFIG, DEFAULT_OPENAI_CONFIG } from '../types/model-config'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const loading = ref(false)
const testing = ref(false)
const config = ref<ModelConfig>({
  provider: DEFAULT_OLLAMA_CONFIG,
  temperature: 0.2,
  max_tokens: 4096,
})

const providerType = ref<ModelProviderType>('ollama')

// Local form state (not yet persisted)
const ollamaForm = ref<OllamaProvider>({ ...DEFAULT_OLLAMA_CONFIG })
const deepseekForm = ref<DeepSeekProvider>({ ...DEFAULT_DEEPSEEK_CONFIG })
const openaiForm = ref<OpenAIProvider>({ ...DEFAULT_OPENAI_CONFIG })

onMounted(async () => {
  await loadConfig()
})

async function loadConfig() {
  loading.value = true
  try {
    const savedConfig = await getModelConfig()
    config.value = savedConfig
    providerType.value = savedConfig.provider.type
    
    if (savedConfig.provider.type === 'ollama') {
      ollamaForm.value = { ...savedConfig.provider as OllamaProvider }
    } else if (savedConfig.provider.type === 'deepseek') {
      deepseekForm.value = { ...savedConfig.provider as DeepSeekProvider }
    } else if (savedConfig.provider.type === 'openai') {
      openaiForm.value = { ...savedConfig.provider as OpenAIProvider }
    }
  } catch (error) {
    ElMessage.error(t('modelSettings.messages.loadFailed') + String(error))
  } finally {
    loading.value = false
  }
}

function getCurrentProvider(): OllamaProvider | DeepSeekProvider | OpenAIProvider {
  if (providerType.value === 'ollama') {
    return ollamaForm.value
  } else if (providerType.value === 'deepseek') {
    return deepseekForm.value
  } else {
    return openaiForm.value
  }
}

async function handleSave() {
  const provider = getCurrentProvider()
  
  if (providerType.value === 'deepseek' || providerType.value === 'openai') {
    if (!(provider as DeepSeekProvider | OpenAIProvider).api_key) {
      ElMessage.warning(t('modelSettings.messages.fillApiKey'))
      return
    }
  }
  
  loading.value = true
  try {
    const newConfig: ModelConfig = {
      provider,
      temperature: config.value.temperature,
      max_tokens: config.value.max_tokens,
    }
    
    await updateModelConfig(newConfig)
    config.value = newConfig
    ElMessage.success(t('modelSettings.messages.saved'))
  } catch (error) {
    ElMessage.error(t('modelSettings.messages.saveFailed') + String(error))
  } finally {
    loading.value = false
  }
}

async function handleTest() {
  const provider = getCurrentProvider()
  
  if (providerType.value === 'deepseek' || providerType.value === 'openai') {
    if (!(provider as DeepSeekProvider | OpenAIProvider).api_key) {
      ElMessage.warning(t('modelSettings.messages.fillApiKey'))
      return
    }
  }
  
  testing.value = true
  try {
    const testConfig: ModelConfig = {
      provider,
      temperature: config.value.temperature,
      max_tokens: config.value.max_tokens,
    }
    
    const response = await testModelConfig(testConfig)
    await ElMessageBox.alert(
      t('modelSettings.messages.testSuccess') + response,
      t('modelSettings.messages.testResultTitle'),
      { type: 'success' }
    )
  } catch (error) {
    ElMessage.error(t('modelSettings.messages.testFailed') + String(error))
  } finally {
    testing.value = false
  }
}

function handleReset() {
  if (providerType.value === 'ollama') {
    ollamaForm.value = { ...DEFAULT_OLLAMA_CONFIG }
  } else if (providerType.value === 'deepseek') {
    deepseekForm.value = { ...DEFAULT_DEEPSEEK_CONFIG }
  } else {
    openaiForm.value = { ...DEFAULT_OPENAI_CONFIG }
  }
  config.value.temperature = 0.2
  config.value.max_tokens = 4096
}

function getCurrentProviderInfo(): string {
  if (!config.value.provider) return t('modelSettings.currentModel.notConfigured')
  
  if (config.value.provider.type === 'ollama') {
    return t('modelSettings.currentModel.localOllama')
  } else if (config.value.provider.type === 'deepseek') {
    return t('modelSettings.currentModel.deepseek')
  } else if (config.value.provider.type === 'openai') {
    return t('modelSettings.currentModel.openai')
  }
  return t('modelSettings.currentModel.unknown')
}
</script>

<template>
  <div class="model-settings">
    <h2>{{ t('modelSettings.title') }}</h2>
    
    <el-alert
      v-if="config.provider"
      :title="getCurrentProviderInfo()"
      type="info"
      :closable="false"
      class="current-model-alert"
    >
      <template #default>
        <div class="current-model-details">
          <div v-if="config.provider.type === 'ollama'">
            <p>{{ t('modelSettings.localModelNote') }}</p>
            <p>{{ t('modelSettings.serviceUrl') }} {{ config.provider.base_url }}</p>
            <p>{{ t('modelSettings.qaModel') }} {{ config.provider.model_name }}</p>
            <p>{{ t('modelSettings.extractModel') }} {{ config.provider.extract_model_name }}</p>
          </div>
          <div v-else-if="config.provider.type === 'deepseek'">
            <p>🌐 <strong>DeepSeek API</strong>（{{ t('modelSettings.cloudProcessing') }}）</p>
            <p>{{ t('modelSettings.model') }} {{ config.provider.model_name }}</p>
            <p>{{ t('modelSettings.apiUrl') }} {{ config.provider.base_url }}</p>
          </div>
          <div v-else-if="config.provider.type === 'openai'">
            <p>🔥 <strong>OpenAI API</strong>（{{ t('modelSettings.cloudProcessing') }}）</p>
            <p>{{ t('modelSettings.model') }} {{ config.provider.model_name }}</p>
            <p>{{ t('modelSettings.apiUrl') }} {{ config.provider.base_url }}</p>
          </div>
        </div>
      </template>
    </el-alert>
    
    <el-form v-loading="loading" label-width="120px" class="settings-form">
      <el-form-item :label="t('modelSettings.form.provider')">
        <el-radio-group v-model="providerType">
          <el-radio value="ollama">{{ t('modelSettings.form.localOllama') }}</el-radio>
          <el-radio value="deepseek">{{ t('modelSettings.form.deepseek') }}</el-radio>
          <el-radio value="openai">{{ t('modelSettings.form.openai') }}</el-radio>
        </el-radio-group>
      </el-form-item>

      <!-- Ollama settings -->
      <template v-if="providerType === 'ollama'">
        <el-divider content-position="left">{{ t('modelSettings.form.ollamaConfig') }}</el-divider>
        <el-form-item :label="t('modelSettings.form.serviceUrl')">
          <el-input v-model="ollamaForm.base_url" placeholder="http://localhost:11434" />
        </el-form-item>
        <el-form-item :label="t('modelSettings.form.qaModel')">
          <el-input v-model="ollamaForm.model_name" placeholder="qwen2.5:7b" />
          <span class="form-tip">{{ t('modelSettings.form.qaModelTip') }}</span>
        </el-form-item>
        <el-form-item :label="t('modelSettings.form.extractModel')">
          <el-input v-model="ollamaForm.extract_model_name" placeholder="qwen2.5:7b" />
          <span class="form-tip">{{ t('modelSettings.form.extractModelTip') }}</span>
        </el-form-item>
      </template>

      <!-- DeepSeek settings -->
      <template v-if="providerType === 'deepseek'">
        <el-divider content-position="left">{{ t('modelSettings.form.deepseekConfig') }}</el-divider>
        <el-form-item :label="t('modelSettings.form.apiKey')" required>
          <el-input v-model="deepseekForm.api_key" type="password" show-password placeholder="sk-..." />
          <span class="form-tip">
            {{ t('modelSettings.form.deepseekApiKeyTip') }}
            <a href="https://platform.deepseek.com" target="_blank">{{ t('modelSettings.form.deepseekPlatform') }}</a>
          </span>
        </el-form-item>
        <el-form-item :label="t('modelSettings.form.apiUrl')">
          <el-input v-model="deepseekForm.base_url" placeholder="https://api.deepseek.com/v1" />
        </el-form-item>
        <el-form-item :label="t('modelSettings.form.modelName')">
          <el-select v-model="deepseekForm.model_name" :placeholder="t('modelSettings.form.selectModel')">
            <el-option label="deepseek-chat" value="deepseek-chat" />
            <el-option label="deepseek-reasoner" value="deepseek-reasoner" />
          </el-select>
        </el-form-item>
      </template>

      <!-- OpenAI settings -->
      <template v-if="providerType === 'openai'">
        <el-divider content-position="left">{{ t('modelSettings.form.openaiConfig') }}</el-divider>
        <el-form-item :label="t('modelSettings.form.apiKey')" required>
          <el-input v-model="openaiForm.api_key" type="password" show-password placeholder="sk-..." />
        </el-form-item>
        <el-form-item :label="t('modelSettings.form.apiUrl')">
          <el-input v-model="openaiForm.base_url" placeholder="https://api.openai.com/v1" />
          <span class="form-tip">{{ t('modelSettings.form.openaiApiUrlTip') }}</span>
        </el-form-item>
        <el-form-item :label="t('modelSettings.form.modelName')">
          <el-input v-model="openaiForm.model_name" placeholder="gpt-4" />
        </el-form-item>
      </template>

      <!-- General parameters -->
      <el-divider content-position="left">{{ t('modelSettings.form.generalParams') }}</el-divider>
      <el-form-item label="Temperature">
        <el-slider v-model="config.temperature" :min="0" :max="1" :step="0.1" show-input />
        <span class="form-tip">{{ t('modelSettings.form.temperatureTip') }}</span>
      </el-form-item>
      <el-form-item :label="t('modelSettings.form.maxTokens')">
        <el-input-number v-model="config.max_tokens" :min="512" :max="32768" :step="512" />
      </el-form-item>

      <el-form-item>
        <el-button type="primary" @click="handleSave" :loading="loading">
          {{ t('modelSettings.form.saveConfig') }}
        </el-button>
        <el-button @click="handleTest" :loading="testing">
          {{ t('modelSettings.form.testConnection') }}
        </el-button>
        <el-button @click="handleReset">
          {{ t('modelSettings.form.resetDefault') }}
        </el-button>
      </el-form-item>
    </el-form>

    <el-card class="info-card">
      <template #header>
        <span>{{ t('modelSettings.guide.title') }}</span>
      </template>
      <div class="info-content">
        <h4>{{ t('modelSettings.guide.localOllama.title') }}</h4>
        <ul>
          <li>{{ t('modelSettings.guide.localOllama.free') }}</li>
          <li>{{ t('modelSettings.guide.localOllama.requirement') }}</li>
          <li>{{ t('modelSettings.guide.localOllama.recommend') }}</li>
          <li>{{ t('modelSettings.guide.localOllama.install') }}<code>ollama pull qwen2.5:7b</code></li>
        </ul>

        <h4>{{ t('modelSettings.guide.deepseek.title') }}</h4>
        <ul>
          <li>{{ t('modelSettings.guide.deepseek.accessible') }}</li>
          <li>{{ t('modelSettings.guide.deepseek.price') }}</li>
          <li>{{ t('modelSettings.guide.deepseek.recommend') }}</li>
          <li>{{ t('modelSettings.guide.deepseek.register') }}<a href="https://platform.deepseek.com" target="_blank">platform.deepseek.com</a></li>
        </ul>

        <h4>{{ t('modelSettings.guide.openai.title') }}</h4>
        <ul>
          <li>{{ t('modelSettings.guide.openai.best') }}</li>
          <li>{{ t('modelSettings.guide.openai.price') }}</li>
          <li>{{ t('modelSettings.guide.openai.compatible') }}</li>
        </ul>

        <h4>{{ t('modelSettings.guide.recommend.title') }}</h4>
        <ul>
          <li><strong>{{ t('modelSettings.guide.recommend.beginner') }}</strong>：{{ t('modelSettings.guide.recommend.beginnerValue') }}</li>
          <li><strong>{{ t('modelSettings.guide.recommend.daily') }}</strong>：{{ t('modelSettings.guide.recommend.dailyValue') }}</li>
          <li><strong>{{ t('modelSettings.guide.recommend.best') }}</strong>：{{ t('modelSettings.guide.recommend.bestValue') }}</li>
        </ul>
      </div>
    </el-card>
  </div>
</template>

<style scoped>
.model-settings {
  padding: 0;
}

h2 {
  margin-bottom: 14px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.current-model-alert {
  margin-bottom: 14px;
}

.current-model-details {
  font-size: 13px;
  line-height: 1.8;
  color: var(--text-muted);
}

.current-model-details p {
  margin: 4px 0;
}

.current-model-details strong {
  color: #a78bfa;
}

.settings-form {
  margin-bottom: 20px;
}

.form-tip {
  display: block;
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-dim);
}

.form-tip a { color: #a78bfa; text-decoration: none; }
.form-tip a:hover { text-decoration: underline; }

.info-card { margin-top: 20px; }

.info-content h4 {
  margin: 10px 0 6px 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}
.info-content h4:first-child { margin-top: 0; }

.info-content ul {
  margin: 0 0 8px 18px;
  padding: 0;
  list-style: disc;
}

.info-content li {
  margin: 4px 0;
  line-height: 1.6;
  font-size: 13px;
  color: var(--text-muted);
}

.info-content a { color: #a78bfa; text-decoration: none; }
.info-content a:hover { text-decoration: underline; }

.info-content code {
  padding: 2px 6px;
  background: var(--bg4);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: 'Monaco', 'Courier New', monospace;
  font-size: 12px;
  color: var(--cyan);
}
</style>
