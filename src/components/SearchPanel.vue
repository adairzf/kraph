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
.search-panel { padding: 0; }
.panel-title {
  margin: 0 0 4px 0;
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}
.hint {
  margin: 0 0 10px 0;
  font-size: 13px;
  color: var(--text-muted);
}
.qa-form {
  display: flex;
  gap: 6px;
  margin-bottom: 10px;
}
.question-input {
  flex: 1;
  padding: 7px 10px;
  border: 1px solid var(--border);
  background: var(--bg3);
  color: var(--text);
  border-radius: 6px;
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.15s;
}
.question-input::placeholder { color: var(--text-dim); }
.question-input:focus { border-color: rgba(124, 92, 252, 0.5); }
.btn-ask {
  padding: 7px 14px;
  border: none;
  background: var(--grad);
  color: #fff;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  font-family: inherit;
  transition: opacity 0.15s;
  white-space: nowrap;
}
.btn-ask:hover { opacity: 0.88; }
.btn-ask:disabled { opacity: 0.4; cursor: not-allowed; }
.error {
  color: var(--red);
  font-size: 13px;
  margin: 8px 0;
  padding: 8px 10px;
  background: rgba(248, 113, 113, 0.08);
  border: 1px solid rgba(248, 113, 113, 0.2);
  border-radius: 6px;
}
.answer-box {
  padding: 12px;
  background: var(--bg3);
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 14px;
  line-height: 1.65;
  white-space: pre-wrap;
  color: var(--text);
}
</style>
