<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useMemoryStore } from '../stores/memoryStore'
import { useGraphStore } from '../stores/graphStore'
import { setupWhisper, transcribeAudio } from '../utils/tauriApi'

const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>()
const props = defineProps<{ modelValue: string }>()
const memoryStore = useMemoryStore()
const graphStore = useGraphStore()
const saving = ref(false)
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
      voiceError.value = '当前环境不支持麦克风权限请求'
      return false
    }
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
    stream.getTracks().forEach((t) => t.stop())
    return true
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    voiceError.value = `麦克风权限未授权（请到系统设置中允许本应用访问麦克风）: ${msg}`
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
      reject(new Error('转写超时，请重试（可缩短录音时长）'))
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
    throw new Error('当前环境不支持 AudioContext')
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
  whisperProgress.value = '正在进行语音转文字，请稍候'
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
  whisperProgress.value = '正在初始化 Whisper（首次会自动安装与下载模型）'
  let dot = 0
  whisperProgressTimer = window.setInterval(() => {
    dot = (dot + 1) % 4
    whisperProgress.value = `正在初始化 Whisper${'.'.repeat(dot)}`
  }, 500)
  try {
    await setupWhisper()
    whisperReady.value = true
    whisperProgress.value = 'Whisper 已就绪'
    window.setTimeout(() => {
      if (!recording.value && !transcribing.value) whisperProgress.value = ''
    }, 1200)
    return true
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    voiceError.value = `Whisper 初始化失败：${msg}`
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
      voiceError.value = `语音转写失败：${msg}`
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
      voiceError.value = e instanceof Error ? e.message : '启动语音识别失败'
      recording.value = false
      whisperProgress.value = ''
    }
  }
}

async function handleSave() {
  const content = text.value.trim()
  if (!content) return
  saving.value = true
  try {
    await memoryStore.saveMemory(content)
    await graphStore.fetchGraph()
    text.value = ''
    emit('update:modelValue', '')
  } catch {
    // error in store
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="input-panel">
    <textarea
      v-model="text"
      class="textarea"
      placeholder="输入要记录的内容，或使用语音输入…"
      rows="4"
      @input="onInput"
    />
    <div class="actions">
      <button
        v-if="speechSupported"
        type="button"
        class="btn voice"
        :class="{ active: recording }"
        :title="recording ? '停止录音并转写' : '开始录音（Whisper）'"
        :disabled="transcribing || preparingWhisper"
        @click="toggleVoice"
      >
        {{ preparingWhisper ? '⚙️ 初始化…' : (transcribing ? '⏳ 转写中…' : (recording ? '⏹ 停止' : '🎤 语音')) }}
      </button>
      <button class="btn primary" :disabled="saving || !text.trim()" @click="handleSave">
        {{ saving ? '保存中…' : '保存记忆' }}
      </button>
    </div>
    <p v-if="whisperProgress" class="voice-progress">{{ whisperProgress }}</p>
    <p v-if="voiceError" class="voice-error">{{ voiceError }}</p>
  </div>
</template>

<style scoped>
.input-panel {
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
  padding: 0.75rem;
}
.textarea {
  width: 100%;
  box-sizing: border-box;
  padding: 0.5rem;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 0.9375rem;
  resize: vertical;
  min-height: 80px;
}
.actions {
  margin-top: 0.5rem;
  display: flex;
  gap: 0.5rem;
  align-items: center;
}
.btn.voice {
  background: #f0f0f0;
}
.btn.voice.active {
  background: #24c8db;
  color: #fff;
  border-color: #24c8db;
}
.btn {
  padding: 0.4rem 1rem;
  border-radius: 6px;
  border: 1px solid #ccc;
  background: #f5f5f5;
  cursor: pointer;
  font-size: 0.875rem;
}
.btn.primary {
  background: #24c8db;
  color: #fff;
  border-color: #24c8db;
}
.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.voice-error {
  margin: 0.5rem 0 0;
  font-size: 0.8125rem;
  color: #c00;
}
.voice-progress {
  margin: 0.5rem 0 0;
  font-size: 0.8125rem;
  color: #0a7f8c;
}
</style>
