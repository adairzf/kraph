<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { ElMessage } from 'element-plus'
import { useMemoryStore } from '../stores/memoryStore'
import { useGraphStore } from '../stores/graphStore'
import { setupWhisper, transcribeAudio } from '../utils/tauriApi'
import { useI18n } from 'vue-i18n'

// Parse a JSON-encoded backend error string into a user-facing message.
// Rust returns errors as: JSON.stringify({ code: "saveProgress.errors.xxx", reason?: "..." })
// Falls back to raw string if parsing fails or code is not found in i18n.
function parseBackendError(raw: string, te: (key: string) => boolean, t: (key: string, params?: Record<string, unknown>) => string): string {
  try {
    const parsed = JSON.parse(raw)
    if (parsed.code && te(parsed.code)) {
      return t(parsed.code, parsed as Record<string, unknown>)
    }
  } catch { /* not JSON, fall through */ }
  return raw
}

interface SaveProgressStep {
  message: string
  status: 'info' | 'running' | 'success' | 'warning' | 'error' | 'skipped' | 'done'
}

const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>()
const props = defineProps<{ modelValue: string }>()
const memoryStore = useMemoryStore()
const graphStore = useGraphStore()
const { t, te } = useI18n()
const saving = ref(false)
const saveProgress = ref<SaveProgressStep[]>([])
const recording = ref(false)
const transcribing = ref(false)
const preparingWhisper = ref(false)
const speechSupported = ref(false)
const voiceError = ref<string | null>(null)
const whisperReady = ref(false)
const whisperProgress = ref('')
let whisperProgressTimer: number | null = null

const text = ref(props.modelValue)
watch(() => props.modelValue, (v: string) => { text.value = v })
function onInput() {
  emit('update:modelValue', text.value)
}

let mediaStream: MediaStream | null = null
let audioContext: AudioContext | null = null
let sourceNode: MediaStreamAudioSourceNode | null = null
let processorNode: ScriptProcessorNode | null = null
const pcmChunks: Float32Array[] = []

async function requestMicPermission(): Promise<boolean> {
  try {
    if (!navigator.mediaDevices?.getUserMedia) {
      voiceError.value = t('inputPanel.errors.micNotSupported')
      return false
    }
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
    stream.getTracks().forEach((track) => track.stop())
    return true
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    voiceError.value = t('inputPanel.errors.micPermissionDenied', { msg })
    return false
  }
}

function interleavePcm(chunks: Float32Array[]): Float32Array {
  const total = chunks.reduce((n, c) => n + c.length, 0)
  const out = new Float32Array(total)
  let offset = 0
  for (const chunk of chunks) {
    out.set(chunk, offset)
    offset += chunk.length
  }
  return out
}

function encodeWavFromFloat32(samples: Float32Array, sampleRate: number): Uint8Array {
  const bytesPerSample = 2
  const dataLength = samples.length * bytesPerSample
  const buffer = new ArrayBuffer(44 + dataLength)
  const view = new DataView(buffer)

  const writeString = (offset: number, s: string) => {
    for (let i = 0; i < s.length; i++) view.setUint8(offset + i, s.charCodeAt(i))
  }

  writeString(0, 'RIFF')
  view.setUint32(4, 36 + dataLength, true)
  writeString(8, 'WAVE')
  writeString(12, 'fmt ')
  view.setUint32(16, 16, true) // PCM header size
  view.setUint16(20, 1, true) // PCM format
  view.setUint16(22, 1, true) // mono
  view.setUint32(24, sampleRate, true)
  view.setUint32(28, sampleRate * bytesPerSample, true)
  view.setUint16(32, bytesPerSample, true)
  view.setUint16(34, 16, true)
  writeString(36, 'data')
  view.setUint32(40, dataLength, true)

  let offset = 44
  for (let i = 0; i < samples.length; i++) {
    const s = Math.max(-1, Math.min(1, samples[i]))
    const pcm = s < 0 ? s * 0x8000 : s * 0x7fff
    view.setInt16(offset, pcm, true)
    offset += 2
  }
  return new Uint8Array(buffer)
}

function uint8ToBase64(bytes: Uint8Array): string {
  let binary = ''
  const chunkSize = 0x8000
  for (let i = 0; i < bytes.length; i += chunkSize) {
    const chunk = bytes.subarray(i, i + chunkSize)
    binary += String.fromCharCode(...chunk)
  }
  return btoa(binary)
}

async function transcribeWithTimeout(audioBase64: string, timeoutMs = 130000): Promise<string> {
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => {
      reject(new Error(t('inputPanel.errors.transcribeTimeout')))
    }, timeoutMs)
    transcribeAudio(audioBase64)
      .then((res) => {
        window.clearTimeout(timer)
        resolve(res)
      })
      .catch((err) => {
        window.clearTimeout(timer)
        reject(err)
      })
  })
}

async function startPcmRecording() {
  mediaStream = await navigator.mediaDevices.getUserMedia({ audio: true })
  const Ctx = (window.AudioContext || (window as any).webkitAudioContext) as typeof AudioContext
  if (!Ctx) {
    throw new Error(t('inputPanel.errors.audioContextNotSupported'))
  }

  audioContext = new Ctx()
  sourceNode = audioContext.createMediaStreamSource(mediaStream)
  processorNode = audioContext.createScriptProcessor(4096, 1, 1)
  pcmChunks.length = 0

  processorNode.onaudioprocess = (e: AudioProcessingEvent) => {
    const input = e.inputBuffer.getChannelData(0)
    pcmChunks.push(new Float32Array(input))
  }

  sourceNode.connect(processorNode)
  processorNode.connect(audioContext.destination)
}

async function stopPcmRecordingAndTranscribe() {
  processorNode?.disconnect()
  sourceNode?.disconnect()
  mediaStream?.getTracks().forEach((t) => t.stop())

  const sr = audioContext?.sampleRate ?? 44100
  if (audioContext && audioContext.state !== 'closed') {
    await audioContext.close()
  }

  processorNode = null
  sourceNode = null
  mediaStream = null
  audioContext = null

  const pcm = interleavePcm(pcmChunks)
  pcmChunks.length = 0
  if (!pcm.length) return

  transcribing.value = true
  whisperProgress.value = t('inputPanel.whisper.transcribing')
  try {
    const wavBytes = encodeWavFromFloat32(pcm, sr)
    const audioBase64 = uint8ToBase64(wavBytes)
    const textPart = await transcribeWithTimeout(audioBase64)
    const finalText = textPart?.trim()
    if (finalText) {
      text.value += (text.value ? ' ' : '') + finalText
      onInput()
    }
  } finally {
    transcribing.value = false
    whisperProgress.value = ''
  }
}

async function ensureWhisperReady(): Promise<boolean> {
  if (whisperReady.value) return true
  preparingWhisper.value = true
  whisperProgress.value = t('inputPanel.whisper.initializing')
  let dot = 0
  whisperProgressTimer = window.setInterval(() => {
    dot = (dot + 1) % 4
    whisperProgress.value = t('inputPanel.whisper.initializing').split('（')[0] + '.'.repeat(dot)
  }, 500)
  try {
    await setupWhisper()
    whisperReady.value = true
    whisperProgress.value = t('inputPanel.whisper.ready')
    window.setTimeout(() => {
      if (!recording.value && !transcribing.value) whisperProgress.value = ''
    }, 1200)
    return true
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    voiceError.value = t('inputPanel.whisper.initFailed', { msg })
    whisperProgress.value = ''
    return false
  } finally {
    if (whisperProgressTimer) {
      window.clearInterval(whisperProgressTimer)
      whisperProgressTimer = null
    }
    preparingWhisper.value = false
  }
}

onMounted(() => {
  speechSupported.value = !!navigator.mediaDevices?.getUserMedia
})

async function toggleVoice() {
  if (!speechSupported.value) return
  voiceError.value = null
  if (recording.value) {
    try {
      await stopPcmRecordingAndTranscribe()
      recording.value = false
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      voiceError.value = t('inputPanel.errors.transcribeFailed', { msg })
      recording.value = false
      whisperProgress.value = ''
    }
  } else {
    try {
      const ready = await ensureWhisperReady()
      if (!ready) return
      const granted = await requestMicPermission()
      if (!granted) return
      await startPcmRecording()
      recording.value = true
    } catch (e) {
      voiceError.value = e instanceof Error ? e.message : t('inputPanel.errors.voiceStartFailed')
      recording.value = false
      whisperProgress.value = ''
    }
  }
}

async function handleSave() {
  const content = text.value.trim()
  if (!content) return
  saving.value = true
  saveProgress.value = []

  const unlisten = await listen<{ code: string; params: Record<string, unknown>; status: string }>('memory-save-progress', (event) => {
    // Errors are shown via ElMessage in catch; only show other steps in the inline panel
    if (event.payload.status !== 'error') {
      const { code, params } = event.payload
      const message = te(code) ? t(code, params) : code
      saveProgress.value.push({
        message,
        status: event.payload.status as SaveProgressStep['status'],
      })
    }
  })

  try {
    await memoryStore.saveMemory(content)
    await graphStore.fetchGraph()
    text.value = ''
    emit('update:modelValue', '')
    // Briefly show the completed progress before fading out
    await new Promise((resolve) => setTimeout(resolve, 2500))
    saveProgress.value = []
  } catch (e) {
    const raw = e instanceof Error ? e.message : String(e)
    const msg = parseBackendError(raw, te, t)
    ElMessage({ type: 'error', message: msg, duration: 6000, showClose: true })
    saveProgress.value = []
  } finally {
    unlisten()
    saving.value = false
  }
}
</script>

<template>
  <div class="input-panel">
    <textarea
      v-model="text"
      class="textarea"
      :placeholder="t('inputPanel.placeholder')"
      rows="4"
      @input="onInput"
    />
    <div class="actions">
      <button
        v-if="speechSupported"
        type="button"
        class="btn voice"
        :class="{ active: recording }"
        :title="recording ? t('inputPanel.voiceStopTitle') : t('inputPanel.voiceStartTitle')"
        :disabled="transcribing || preparingWhisper"
        @click="toggleVoice"
      >
        {{ preparingWhisper ? t('inputPanel.voiceInit') : (transcribing ? t('inputPanel.voiceTranscribing') : (recording ? t('inputPanel.voiceStop') : t('inputPanel.voiceStart'))) }}
      </button>
      <button class="btn primary" :disabled="saving || !text.trim()" @click="handleSave">
        {{ saving ? t('inputPanel.saving') : t('inputPanel.save') }}
      </button>
    </div>
    <p v-if="whisperProgress" class="voice-progress">{{ whisperProgress }}</p>
    <p v-if="voiceError" class="voice-error">{{ voiceError }}</p>

    <!-- Save progress panel -->
    <div v-if="saveProgress.length > 0" class="save-progress">
      <div
        v-for="(step, i) in saveProgress"
        :key="i"
        class="progress-step"
        :class="`step-${step.status}`"
      >
        <span class="step-icon">
          <span v-if="step.status === 'running'" class="spinner">⏳</span>
          <span v-else-if="step.status === 'success'">✅</span>
          <span v-else-if="step.status === 'done'">🎉</span>
          <span v-else-if="step.status === 'warning'">⚠️</span>
          <span v-else-if="step.status === 'error'">❌</span>
          <span v-else-if="step.status === 'skipped'">⏭️</span>
          <span v-else>ℹ️</span>
        </span>
        <span class="step-message">{{ step.message }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.input-panel {
  padding: 10px;
  background: var(--bg2);
  flex-shrink: 0;
}
.textarea {
  width: 100%;
  box-sizing: border-box;
  padding: 8px 10px;
  border: 1px solid var(--border);
  background: var(--bg3);
  color: var(--text);
  border-radius: 7px;
  font-size: 13px;
  resize: none;
  min-height: 70px;
  max-height: 120px;
  outline: none;
  font-family: inherit;
  line-height: 1.5;
  transition: border-color 0.15s;
}
.textarea::placeholder { color: var(--text-dim); }
.textarea:focus { border-color: rgba(124, 92, 252, 0.5); }
.actions {
  margin-top: 7px;
  display: flex;
  gap: 6px;
  align-items: center;
}
.btn {
  padding: 5px 12px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg4);
  color: var(--text-muted);
  cursor: pointer;
  font-size: 12px;
  font-family: inherit;
  transition: all 0.15s;
}
.btn:hover { background: var(--bg3); color: var(--text); border-color: var(--border-hover); }
.btn.voice { background: var(--bg4); }
.btn.voice.active {
  background: rgba(124, 92, 252, 0.15);
  color: #a78bfa;
  border-color: rgba(124, 92, 252, 0.4);
}
.btn.primary {
  background: var(--grad);
  color: #fff;
  border-color: transparent;
  font-weight: 500;
}
.btn.primary:hover { opacity: 0.88; }
.btn:disabled { opacity: 0.45; cursor: not-allowed; }
.voice-error {
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--red);
}
.voice-progress {
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--cyan);
}

/* Save progress panel */
.save-progress {
  margin-top: 8px;
  padding: 8px 10px;
  background: var(--bg3);
  border: 1px solid var(--border);
  border-radius: 6px;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.progress-step {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  font-size: 12px;
  line-height: 1.4;
  padding: 1px 0;
}
.step-icon { flex-shrink: 0; }
.step-message { color: var(--text-muted); }
.step-running .step-message { color: var(--cyan); font-style: italic; }
.step-success .step-message { color: var(--green); }
.step-done .step-message { color: var(--green); font-weight: 600; }
.step-warning .step-message { color: var(--orange); }
.step-error .step-message { color: var(--red); }
.step-skipped .step-message { color: var(--text-dim); }
.step-info .step-message { color: var(--text-muted); }

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
.spinner {
  display: inline-block;
  animation: spin 1.5s linear infinite;
}
</style>
