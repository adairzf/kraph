<script setup lang="ts">
import { ref } from 'vue'
import { ElMessageBox } from 'element-plus'
import { answerQuestion } from '../utils/tauriApi'
import { useOllamaStore } from '../stores/ollamaStore'

const question = ref('')
const answer = ref('')
const loading = ref(false)
const error = ref<string | null>(null)

const ollamaStore = useOllamaStore()

/** 判断错误信息是否为 Ollama 服务相关的异常 */
function isOllamaError(msg: string): boolean {
  const keywords = [
    'ollama', 'Ollama',
    '连接失败', 'connection refused', 'connection reset',
    '502', 'bad gateway', '未响应', '无法连接',
  ]
  return keywords.some((k) => msg.includes(k))
}

async function ask() {
  const q = question.value.trim()
  if (!q) return
  loading.value = true
  error.value = null
  answer.value = ''
  try {
    answer.value = await answerQuestion(q)
  } catch (e) {
    const errMsg = e instanceof Error ? e.message : String(e)
    error.value = errMsg

    // 检测到 Ollama 服务异常，询问用户是否运行一键初始化
    if (isOllamaError(errMsg)) {
      try {
        await ElMessageBox.confirm(
          '检测到 Ollama 服务异常，是否立即运行一键初始化？\n（将自动完成安装检测、服务启动、模型下载）',
          'Ollama 服务异常',
          {
            confirmButtonText: '立即初始化',
            cancelButtonText: '忽略',
            type: 'warning',
          },
        )
        ollamaStore.requestSetup()
      } catch {
        // 用户点击了忽略，不做任何处理
      }
    }
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="search-panel">
    <h2 class="panel-title">智能问答</h2>
    <p class="hint">基于已记录的记忆回答，例如：「李明是谁？」</p>
    <div class="qa-form">
      <input
        v-model="question"
        type="text"
        class="question-input"
        placeholder="输入问题…"
        @keyup.enter="ask"
      />
      <button
        type="button"
        class="btn-ask"
        :disabled="loading || !question.trim()"
        @click="ask"
      >
        {{ loading ? '思考中…' : '提问' }}
      </button>
    </div>
    <p v-if="error" class="error">{{ error }}</p>
    <div v-else-if="answer" class="answer-box">
      {{ answer }}
    </div>
  </div>
</template>

<style scoped>
.search-panel {
  padding: 0.5rem 0;
}
.panel-title {
  margin: 0 0 0.35rem 0;
  font-size: 1rem;
  font-weight: 600;
}
.hint {
  margin: 0 0 0.75rem 0;
  font-size: 0.8125rem;
  color: #666;
}
.qa-form {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}
.question-input {
  flex: 1;
  padding: 0.5rem 0.6rem;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 0.9375rem;
}
.btn-ask {
  padding: 0.5rem 1rem;
  border: 1px solid #24c8db;
  background: #24c8db;
  color: #fff;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.875rem;
}
.btn-ask:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.error {
  color: var(--color-error, #c00);
  font-size: 0.875rem;
  margin: 0.5rem 0;
}
.answer-box {
  padding: 0.75rem;
  background: #f0f9fa;
  border-radius: 6px;
  font-size: 0.9375rem;
  line-height: 1.5;
  white-space: pre-wrap;
}
</style>
