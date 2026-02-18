<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getModelConfig, updateModelConfig, testModelConfig } from '../utils/tauriApi'
import type { ModelConfig, ModelProviderType, OllamaProvider, DeepSeekProvider, OpenAIProvider } from '../types/model-config'
import { DEFAULT_OLLAMA_CONFIG, DEFAULT_DEEPSEEK_CONFIG, DEFAULT_OPENAI_CONFIG } from '../types/model-config'

const loading = ref(false)
const testing = ref(false)
const config = ref<ModelConfig>({
  provider: DEFAULT_OLLAMA_CONFIG,
  temperature: 0.2,
  max_tokens: 4096,
})

const providerType = ref<ModelProviderType>('ollama')

// 临时表单数据
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
    
    // 根据类型填充表单
    if (savedConfig.provider.type === 'ollama') {
      ollamaForm.value = { ...savedConfig.provider as OllamaProvider }
    } else if (savedConfig.provider.type === 'deepseek') {
      deepseekForm.value = { ...savedConfig.provider as DeepSeekProvider }
    } else if (savedConfig.provider.type === 'openai') {
      openaiForm.value = { ...savedConfig.provider as OpenAIProvider }
    }
  } catch (error) {
    ElMessage.error('加载配置失败: ' + String(error))
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
  
  // 验证必填字段
  if (providerType.value === 'deepseek' || providerType.value === 'openai') {
    if (!(provider as DeepSeekProvider | OpenAIProvider).api_key) {
      ElMessage.warning('请填写 API Key')
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
    ElMessage.success('配置已保存')
  } catch (error) {
    ElMessage.error('保存失败: ' + String(error))
  } finally {
    loading.value = false
  }
}

async function handleTest() {
  const provider = getCurrentProvider()
  
  if (providerType.value === 'deepseek' || providerType.value === 'openai') {
    if (!(provider as DeepSeekProvider | OpenAIProvider).api_key) {
      ElMessage.warning('请填写 API Key')
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
      `测试成功！模型响应：\n${response}`,
      '测试结果',
      { type: 'success' }
    )
  } catch (error) {
    ElMessage.error('测试失败: ' + String(error))
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
  if (!config.value.provider) return '未配置'
  
  if (config.value.provider.type === 'ollama') {
    return '当前使用：本地 Ollama 模型'
  } else if (config.value.provider.type === 'deepseek') {
    return '当前使用：DeepSeek API'
  } else if (config.value.provider.type === 'openai') {
    return '当前使用：OpenAI API'
  }
  return '未知提供商'
}
</script>

<template>
  <div class="model-settings">
    <h2>模型配置</h2>
    
    <!-- 当前使用的模型提示 -->
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
            <p>🖥️ <strong>本地模型</strong>（数据不会上传云端）</p>
            <p>服务地址: {{ config.provider.base_url }}</p>
            <p>问答模型: {{ config.provider.model_name }}</p>
            <p>提取模型: {{ config.provider.extract_model_name }}</p>
          </div>
          <div v-else-if="config.provider.type === 'deepseek'">
            <p>🌐 <strong>DeepSeek API</strong>（云端处理）</p>
            <p>模型: {{ config.provider.model_name }}</p>
            <p>API地址: {{ config.provider.base_url }}</p>
          </div>
          <div v-else-if="config.provider.type === 'openai'">
            <p>🔥 <strong>OpenAI API</strong>（云端处理）</p>
            <p>模型: {{ config.provider.model_name }}</p>
            <p>API地址: {{ config.provider.base_url }}</p>
          </div>
        </div>
      </template>
    </el-alert>
    
    <el-form v-loading="loading" label-width="120px" class="settings-form">
      <el-form-item label="模型提供商">
        <el-radio-group v-model="providerType">
          <el-radio value="ollama">本地 Ollama</el-radio>
          <el-radio value="deepseek">DeepSeek API</el-radio>
          <el-radio value="openai">OpenAI API</el-radio>
        </el-radio-group>
      </el-form-item>

      <!-- Ollama 配置 -->
      <template v-if="providerType === 'ollama'">
        <el-divider content-position="left">Ollama 配置</el-divider>
        <el-form-item label="服务地址">
          <el-input v-model="ollamaForm.base_url" placeholder="http://localhost:11434" />
        </el-form-item>
        <el-form-item label="问答模型">
          <el-input v-model="ollamaForm.model_name" placeholder="qwen2.5:7b" />
          <span class="form-tip">用于问答、知识融合等任务</span>
        </el-form-item>
        <el-form-item label="提取模型">
          <el-input v-model="ollamaForm.extract_model_name" placeholder="qwen2.5:7b" />
          <span class="form-tip">用于实体提取，推荐使用 7b 或更大的模型</span>
        </el-form-item>
      </template>

      <!-- DeepSeek 配置 -->
      <template v-if="providerType === 'deepseek'">
        <el-divider content-position="left">DeepSeek 配置</el-divider>
        <el-form-item label="API Key" required>
          <el-input v-model="deepseekForm.api_key" type="password" show-password placeholder="sk-..." />
          <span class="form-tip">
            在 <a href="https://platform.deepseek.com" target="_blank">DeepSeek 平台</a> 获取 API Key
          </span>
        </el-form-item>
        <el-form-item label="API 地址">
          <el-input v-model="deepseekForm.base_url" placeholder="https://api.deepseek.com/v1" />
        </el-form-item>
        <el-form-item label="模型名称">
          <el-select v-model="deepseekForm.model_name" placeholder="选择模型">
            <el-option label="deepseek-chat" value="deepseek-chat" />
            <el-option label="deepseek-reasoner" value="deepseek-reasoner" />
          </el-select>
        </el-form-item>
      </template>

      <!-- OpenAI 配置 -->
      <template v-if="providerType === 'openai'">
        <el-divider content-position="left">OpenAI 配置</el-divider>
        <el-form-item label="API Key" required>
          <el-input v-model="openaiForm.api_key" type="password" show-password placeholder="sk-..." />
        </el-form-item>
        <el-form-item label="API 地址">
          <el-input v-model="openaiForm.base_url" placeholder="https://api.openai.com/v1" />
          <span class="form-tip">可配置兼容 OpenAI 格式的其他 API</span>
        </el-form-item>
        <el-form-item label="模型名称">
          <el-input v-model="openaiForm.model_name" placeholder="gpt-4" />
        </el-form-item>
      </template>

      <!-- 通用参数 -->
      <el-divider content-position="left">通用参数</el-divider>
      <el-form-item label="Temperature">
        <el-slider v-model="config.temperature" :min="0" :max="1" :step="0.1" show-input />
        <span class="form-tip">较低值使输出更确定，较高值使输出更随机</span>
      </el-form-item>
      <el-form-item label="最大 Tokens">
        <el-input-number v-model="config.max_tokens" :min="512" :max="32768" :step="512" />
      </el-form-item>

      <!-- 操作按钮 -->
      <el-form-item>
        <el-button type="primary" @click="handleSave" :loading="loading">
          保存配置
        </el-button>
        <el-button @click="handleTest" :loading="testing">
          测试连接
        </el-button>
        <el-button @click="handleReset">
          重置为默认
        </el-button>
      </el-form-item>
    </el-form>

    <!-- 说明文档 -->
    <el-card class="info-card">
      <template #header>
        <span>使用说明</span>
      </template>
      <div class="info-content">
        <h4>🤖 本地 Ollama</h4>
        <ul>
          <li>完全免费，数据本地化</li>
          <li>需要安装 Ollama 并下载模型</li>
          <li>推荐模型：qwen2.5:7b（快速）、qwen2.5:14b（准确）</li>
          <li>安装：<code>ollama pull qwen2.5:7b</code></li>
        </ul>

        <h4>🌐 DeepSeek API</h4>
        <ul>
          <li>国内可直接访问，速度快</li>
          <li>价格便宜：1M tokens 约 ¥1</li>
          <li>推荐模型：deepseek-chat（通用）、deepseek-reasoner（推理）</li>
          <li>注册地址：<a href="https://platform.deepseek.com" target="_blank">platform.deepseek.com</a></li>
        </ul>

        <h4>🔥 OpenAI API</h4>
        <ul>
          <li>效果最好，但需要科学上网</li>
          <li>价格较高：gpt-4 约 $30/1M tokens</li>
          <li>也可配置兼容 OpenAI 格式的其他 API（如 Azure OpenAI）</li>
        </ul>

        <h4>💡 推荐配置</h4>
        <ul>
          <li><strong>新手/测试</strong>：使用本地 Ollama + qwen2.5:7b（免费）</li>
          <li><strong>日常使用</strong>：DeepSeek API（便宜快速）</li>
          <li><strong>追求极致</strong>：本地 Ollama + qwen2.5:14b 或 DeepSeek Reasoner</li>
        </ul>
      </div>
    </el-card>
  </div>
</template>

<style scoped>
.model-settings {
  padding: 20px;
  max-width: 800px;
  margin: 0 auto;
}

h2 {
  margin-bottom: 20px;
  font-size: 24px;
  font-weight: 600;
}

.current-model-alert {
  margin-bottom: 20px;
}

.current-model-details {
  font-size: 14px;
  line-height: 1.8;
}

.current-model-details p {
  margin: 5px 0;
}

.current-model-details strong {
  color: #409eff;
}

.settings-form {
  margin-bottom: 30px;
}

.form-tip {
  display: block;
  margin-top: 5px;
  font-size: 12px;
  color: #909399;
}

.form-tip a {
  color: #409eff;
  text-decoration: none;
}

.form-tip a:hover {
  text-decoration: underline;
}

.info-card {
  margin-top: 30px;
}

.info-content h4 {
  margin: 15px 0 10px 0;
  font-size: 16px;
  font-weight: 600;
}

.info-content h4:first-child {
  margin-top: 0;
}

.info-content ul {
  margin: 0 0 10px 20px;
  padding: 0;
  list-style: disc;
}

.info-content li {
  margin: 5px 0;
  line-height: 1.6;
}

.info-content code {
  padding: 2px 6px;
  background: #f5f7fa;
  border-radius: 3px;
  font-family: 'Monaco', 'Courier New', monospace;
  font-size: 13px;
}
</style>
