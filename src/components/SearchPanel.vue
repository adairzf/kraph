<script setup lang="ts">
import { ref } from 'vue'
import { ElMessageBox } from 'element-plus'
import { answerQuestion } from '../utils/tauriApi'
import { useOllamaStore } from '../stores/ollamaStore'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const question = ref('')
const answer = ref('')
const loading = ref(false)
const error = ref<string | null>(null)

const ollamaStore = useOllamaStore()

/** Returns true when the error message indicates an Ollama service failure. */
function isOllamaError(msg: string): boolean {
  const keywords = [
    'ollama', 'Ollama',
    'connection refused', 'connection reset',
    '502', 'bad gateway',
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

    if (isOllamaError(errMsg)) {
      try {
        await ElMessageBox.confirm(
          t('searchPanel.ollamaError.message'),
          t('searchPanel.ollamaError.title'),
          {
            confirmButtonText: t('searchPanel.ollamaError.confirm'),
            cancelButtonText: t('searchPanel.ollamaError.cancel'),
            type: 'warning',
          },
        )
        ollamaStore.requestSetup()
      } catch {
        // user dismissed the dialog
      }
    }
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="search-panel">
    <h2 class="panel-title">{{ t('searchPanel.title') }}</h2>
    <p class="hint">{{ t('searchPanel.hint') }}</p>
    <div class="qa-form">
      <input
        v-model="question"
        type="text"
        class="question-input"
        :placeholder="t('searchPanel.placeholder')"
        @keyup.enter="ask"
      />
      <button
        type="button"
        class="btn-ask"
        :disabled="loading || !question.trim()"
        @click="ask"
      >
        {{ loading ? t('searchPanel.thinking') : t('searchPanel.ask') }}
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
