<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useI18n } from 'vue-i18n'
import { useOllamaStore } from '../stores/ollamaStore'
import {
  continueStoryChapter,
  generateStoryFromEvents,
  listStoryProjects,
  loadStoryProject,
  rewriteStoryChapter,
  saveStoryProject,
  type StoryContinuationResult,
  type StoryGenerationResult,
  type StoryProjectSummary,
  type StoryWrittenChapter,
} from '../utils/tauriApi'

const { t, locale } = useI18n()
const ollamaStore = useOllamaStore()

const keyEventsText = ref('')
const genre = ref('')
const style = ref('')
const protagonist = ref('')
const chapterCount = ref(10)
const constraints = ref('')
const loading = ref(false)
const continuing = ref(false)
const saving = ref(false)
const loadingProject = ref(false)
const rewritingChapter = ref<number | null>(null)
const projectsLoading = ref(false)
const error = ref<string | null>(null)

const result = ref<StoryGenerationResult | null>(null)
const writtenChapters = ref<StoryWrittenChapter[]>([])
const projects = ref<StoryProjectSummary[]>([])
const selectedProjectId = ref('')
const projectId = ref('')
const revisionNotes = ref<Record<number, string>>({})

const nextChapterNumber = computed(() => {
  if (!writtenChapters.value.length) return 1
  return Math.max(...writtenChapters.value.map((x) => x.chapter)) + 1
})

const planChapterCount = computed(() => {
  const planned = result.value?.chapter_plan.length ?? 0
  return planned > 0 ? planned : Math.max(3, Math.min(24, Number(chapterCount.value) || 10))
})

const canContinue = computed(() =>
  !!result.value && !continuing.value && nextChapterNumber.value <= planChapterCount.value,
)

const canSave = computed(() =>
  !!result.value && writtenChapters.value.length > 0 && !saving.value,
)

function isOllamaError(msg: string): boolean {
  const keywords = [
    'ollama', 'Ollama',
    'connection refused', 'connection reset',
    '502', 'bad gateway',
  ]
  return keywords.some((k) => msg.includes(k))
}

function getKeyEvents(): string[] {
  return keyEventsText.value
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
}

function sortChapters() {
  writtenChapters.value.sort((a, b) => a.chapter - b.chapter)
}

function upsertChapter(chapter: StoryContinuationResult) {
  const idx = writtenChapters.value.findIndex((x) => x.chapter === chapter.chapter)
  const payload: StoryWrittenChapter = {
    chapter: chapter.chapter,
    title: chapter.title,
    content: chapter.content,
    summary: chapter.summary,
  }
  if (idx >= 0) {
    writtenChapters.value[idx] = payload
  } else {
    writtenChapters.value.push(payload)
    sortChapters()
  }
}

function formatDate(input: string): string {
  const d = new Date(input)
  if (Number.isNaN(d.getTime())) return input
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

async function refreshProjects() {
  projectsLoading.value = true
  try {
    projects.value = await listStoryProjects()
    if (!selectedProjectId.value && projects.value.length) {
      selectedProjectId.value = projects.value[0].id
    }
  } finally {
    projectsLoading.value = false
  }
}

async function handleModelError(errMsg: string) {
  if (!isOllamaError(errMsg)) return
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

async function generate() {
  loading.value = true
  error.value = null

  try {
    const generated = await generateStoryFromEvents({
      key_events: getKeyEvents(),
      genre: genre.value.trim() || undefined,
      style: style.value.trim() || undefined,
      protagonist: protagonist.value.trim() || undefined,
      chapter_count: Math.max(3, Math.min(24, Number(chapterCount.value) || 10)),
      constraints: constraints.value.trim() || undefined,
      language: locale.value,
    })
    result.value = generated
    writtenChapters.value = []
    projectId.value = ''

    if (generated.first_chapter.trim()) {
      const chapterOneTitle = generated.chapter_plan.find((c) => c.chapter === 1)?.title
        || (locale.value.startsWith('zh') ? '第一章' : 'Chapter 1')
      writtenChapters.value.push({
        chapter: 1,
        title: chapterOneTitle,
        content: generated.first_chapter.trim(),
      })
    }
  } catch (e) {
    const errMsg = e instanceof Error ? e.message : String(e)
    error.value = errMsg
    await handleModelError(errMsg)
  } finally {
    loading.value = false
  }
}

async function continueNextChapter() {
  if (!result.value || !canContinue.value) return

  continuing.value = true
  error.value = null
  try {
    const chapter = await continueStoryChapter({
      title: result.value.title,
      premise: result.value.premise,
      outline: result.value.outline,
      chapter_plan: result.value.chapter_plan,
      continuity_checks: result.value.continuity_checks,
      written_chapters: writtenChapters.value,
      target_chapter: nextChapterNumber.value,
      style: style.value.trim() || undefined,
      constraints: constraints.value.trim() || undefined,
      language: locale.value,
    })
    upsertChapter(chapter)
  } catch (e) {
    const errMsg = e instanceof Error ? e.message : String(e)
    error.value = errMsg
    await handleModelError(errMsg)
  } finally {
    continuing.value = false
  }
}

async function saveCurrentStory() {
  if (!result.value || !canSave.value) return
  saving.value = true
  error.value = null
  try {
    const title = result.value.title?.trim() || t('storyGenerator.defaultTitle')
    const summary = await saveStoryProject({
      project_id: projectId.value || undefined,
      title,
      premise: result.value.premise,
      outline: result.value.outline,
      chapter_plan: result.value.chapter_plan,
      continuity_checks: result.value.continuity_checks,
      written_chapters: writtenChapters.value,
      style: style.value.trim() || undefined,
      constraints: constraints.value.trim() || undefined,
      language: locale.value,
    })
    projectId.value = summary.id
    selectedProjectId.value = summary.id
    await refreshProjects()
    ElMessage.success(t('storyGenerator.saved'))
  } catch (e) {
    const errMsg = e instanceof Error ? e.message : String(e)
    error.value = errMsg
  } finally {
    saving.value = false
  }
}

async function loadSelectedStory() {
  if (!selectedProjectId.value || loadingProject.value) return
  loadingProject.value = true
  error.value = null
  try {
    const project = await loadStoryProject(selectedProjectId.value)
    projectId.value = project.project_id
    style.value = project.style || ''
    constraints.value = project.constraints || ''

    result.value = {
      title: project.title,
      premise: project.premise,
      outline: project.outline,
      chapter_plan: project.chapter_plan,
      continuity_checks: project.continuity_checks,
      first_chapter: project.written_chapters.find((x) => x.chapter === 1)?.content || '',
    }
    writtenChapters.value = [...project.written_chapters]
    sortChapters()
    ElMessage.success(t('storyGenerator.loaded'))
  } catch (e) {
    const errMsg = e instanceof Error ? e.message : String(e)
    error.value = errMsg
  } finally {
    loadingProject.value = false
  }
}

function noteForChapter(chapter: number): string {
  return revisionNotes.value[chapter] || ''
}

function updateChapterNote(chapter: number, value: string) {
  revisionNotes.value = {
    ...revisionNotes.value,
    [chapter]: value,
  }
}

async function rewriteChapter(chapterNumber: number) {
  if (!result.value || rewritingChapter.value !== null) return
  rewritingChapter.value = chapterNumber
  error.value = null
  try {
    const chapter = await rewriteStoryChapter({
      title: result.value.title,
      premise: result.value.premise,
      outline: result.value.outline,
      chapter_plan: result.value.chapter_plan,
      continuity_checks: result.value.continuity_checks,
      written_chapters: writtenChapters.value,
      target_chapter: chapterNumber,
      feedback: noteForChapter(chapterNumber).trim() || undefined,
      style: style.value.trim() || undefined,
      constraints: constraints.value.trim() || undefined,
      language: locale.value,
    })
    upsertChapter(chapter)
    ElMessage.success(t('storyGenerator.rewriteDone'))
  } catch (e) {
    const errMsg = e instanceof Error ? e.message : String(e)
    error.value = errMsg
    await handleModelError(errMsg)
  } finally {
    rewritingChapter.value = null
  }
}

onMounted(() => {
  refreshProjects().catch(() => {
    // ignore initial loading errors
  })
})
</script>

<template>
  <div class="story-panel">
    <h2 class="panel-title">{{ t('storyGenerator.title') }}</h2>
    <p class="hint">{{ t('storyGenerator.hint') }}</p>
    <p class="hint subtle">{{ t('storyGenerator.autoFromGraphHint') }}</p>

    <div class="project-row">
      <select
        v-model="selectedProjectId"
        class="project-select"
        :disabled="projectsLoading || !projects.length"
      >
        <option value="" disabled>{{ t('storyGenerator.projectPlaceholder') }}</option>
        <option
          v-for="p in projects"
          :key="p.id"
          :value="p.id"
        >
          {{ p.title }} · {{ formatDate(p.updated_at) }}
        </option>
      </select>
      <button
        type="button"
        class="btn-secondary"
        :disabled="!selectedProjectId || loadingProject"
        @click="loadSelectedStory"
      >
        {{ loadingProject ? t('storyGenerator.loadingProject') : t('storyGenerator.loadProject') }}
      </button>
      <button
        type="button"
        class="btn-secondary"
        :disabled="!canSave"
        @click="saveCurrentStory"
      >
        {{ saving ? t('storyGenerator.saving') : t('storyGenerator.save') }}
      </button>
    </div>

    <textarea
      v-model="keyEventsText"
      class="events-input"
      :placeholder="t('storyGenerator.eventsPlaceholder')"
      rows="6"
    />

    <div class="form-grid">
      <input
        v-model="genre"
        type="text"
        class="text-input"
        :placeholder="t('storyGenerator.genrePlaceholder')"
      />
      <input
        v-model="style"
        type="text"
        class="text-input"
        :placeholder="t('storyGenerator.stylePlaceholder')"
      />
      <input
        v-model="protagonist"
        type="text"
        class="text-input"
        :placeholder="t('storyGenerator.protagonistPlaceholder')"
      />
      <input
        v-model.number="chapterCount"
        type="number"
        min="3"
        max="24"
        class="text-input"
        :placeholder="t('storyGenerator.chapterCountPlaceholder')"
      />
    </div>

    <textarea
      v-model="constraints"
      class="events-input constraints"
      :placeholder="t('storyGenerator.constraintsPlaceholder')"
      rows="2"
    />

    <div class="action-row">
      <button
        type="button"
        class="btn-generate"
        :disabled="loading"
        @click="generate"
      >
        {{ loading ? t('storyGenerator.generating') : t('storyGenerator.generate') }}
      </button>
      <button
        type="button"
        class="btn-continue"
        :disabled="!canContinue"
        @click="continueNextChapter"
      >
        {{ continuing ? t('storyGenerator.continuing') : t('storyGenerator.continueNext') }}
      </button>
    </div>

    <p v-if="error" class="error">{{ error }}</p>
    <p
      v-if="result && nextChapterNumber > planChapterCount"
      class="hint subtle"
    >
      {{ t('storyGenerator.allPlannedDone') }}
    </p>

    <div v-if="result" class="result">
      <h3 class="story-title">{{ result.title }}</h3>
      <p class="premise">{{ result.premise }}</p>

      <section v-if="result.outline.length" class="section">
        <h4>{{ t('storyGenerator.outline') }}</h4>
        <ol class="list">
          <li v-for="(item, i) in result.outline" :key="`outline-${i}`">{{ item }}</li>
        </ol>
      </section>

      <section v-if="result.chapter_plan.length" class="section">
        <h4>{{ t('storyGenerator.chapterPlan') }}</h4>
        <div
          v-for="chapter in result.chapter_plan"
          :key="`chapter-${chapter.chapter}`"
          class="chapter-card"
        >
          <p class="chapter-title">{{ t('storyGenerator.chapterPrefix') }} {{ chapter.chapter }}: {{ chapter.title }}</p>
          <p><strong>{{ t('storyGenerator.goal') }}</strong> {{ chapter.goal }}</p>
          <p><strong>{{ t('storyGenerator.conflict') }}</strong> {{ chapter.conflict }}</p>
          <p><strong>{{ t('storyGenerator.twist') }}</strong> {{ chapter.twist }}</p>
          <p><strong>{{ t('storyGenerator.hook') }}</strong> {{ chapter.hook }}</p>
        </div>
      </section>

      <section v-if="result.continuity_checks.length" class="section">
        <h4>{{ t('storyGenerator.continuityChecks') }}</h4>
        <ul class="list">
          <li
            v-for="(item, i) in result.continuity_checks"
            :key="`check-${i}`"
          >
            {{ item }}
          </li>
        </ul>
      </section>

      <section v-if="writtenChapters.length" class="section">
        <h4>{{ t('storyGenerator.writtenChapters') }}</h4>
        <article
          v-for="chapter in writtenChapters"
          :key="`written-${chapter.chapter}`"
          class="written-chapter"
        >
          <h5 class="written-title">{{ t('storyGenerator.chapterPrefix') }} {{ chapter.chapter }}: {{ chapter.title }}</h5>
          <p v-if="chapter.summary" class="chapter-summary">{{ chapter.summary }}</p>
          <div class="first-chapter">{{ chapter.content }}</div>
          <textarea
            class="revise-input"
            :value="noteForChapter(chapter.chapter)"
            :placeholder="t('storyGenerator.rewritePlaceholder')"
            rows="2"
            @input="updateChapterNote(chapter.chapter, ($event.target as HTMLTextAreaElement).value)"
          />
          <button
            type="button"
            class="btn-secondary revise-btn"
            :disabled="rewritingChapter !== null"
            @click="rewriteChapter(chapter.chapter)"
          >
            {{ rewritingChapter === chapter.chapter ? t('storyGenerator.rewriting') : t('storyGenerator.rewriteChapter') }}
          </button>
        </article>
      </section>
    </div>
  </div>
</template>

<style scoped>
.story-panel { padding: 0; }
.panel-title {
  margin: 0 0 4px 0;
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}
.hint {
  margin: 0 0 8px 0;
  font-size: 13px;
  color: var(--text-muted);
}
.hint.subtle {
  color: var(--text-dim);
  font-size: 12px;
}
.project-row {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 6px;
  margin-bottom: 8px;
}
.project-select {
  width: 100%;
  min-width: 0;
  padding: 7px 10px;
  border: 1px solid var(--border);
  background: var(--bg3);
  color: var(--text);
  border-radius: 6px;
  font-size: 12px;
  font-family: inherit;
}
.events-input {
  width: 100%;
  padding: 9px 10px;
  border: 1px solid var(--border);
  background: var(--bg3);
  color: var(--text);
  border-radius: 6px;
  font-size: 13px;
  font-family: inherit;
  outline: none;
  resize: vertical;
  line-height: 1.55;
}
.events-input:focus { border-color: rgba(124, 92, 252, 0.5); }
.events-input::placeholder { color: var(--text-dim); }
.constraints { margin-top: 8px; }
.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px;
  margin-top: 8px;
}
.text-input {
  width: 100%;
  padding: 7px 10px;
  border: 1px solid var(--border);
  background: var(--bg3);
  color: var(--text);
  border-radius: 6px;
  font-size: 13px;
  font-family: inherit;
  outline: none;
}
.text-input:focus { border-color: rgba(124, 92, 252, 0.5); }
.text-input::placeholder { color: var(--text-dim); }
.action-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
  margin-top: 8px;
}
.btn-generate,
.btn-continue,
.btn-secondary {
  width: 100%;
  padding: 8px 14px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
  font-family: inherit;
  transition: opacity 0.15s;
}
.btn-generate {
  background: var(--grad);
  color: #fff;
}
.btn-continue {
  background: rgba(59, 130, 246, 0.15);
  border: 1px solid rgba(59, 130, 246, 0.35);
  color: #93c5fd;
}
.btn-secondary {
  width: auto;
  padding: 0 12px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border);
  color: var(--text);
}
.btn-generate:hover,
.btn-continue:hover,
.btn-secondary:hover { opacity: 0.9; }
.btn-generate:disabled,
.btn-continue:disabled,
.btn-secondary:disabled { opacity: 0.45; cursor: not-allowed; }
.error {
  color: var(--red);
  font-size: 13px;
  margin: 8px 0 0 0;
  padding: 8px 10px;
  background: rgba(248, 113, 113, 0.08);
  border: 1px solid rgba(248, 113, 113, 0.2);
  border-radius: 6px;
}
.result {
  margin-top: 10px;
  border-top: 1px solid var(--border);
  padding-top: 10px;
}
.story-title {
  margin: 0;
  color: var(--text);
  font-size: 16px;
}
.premise {
  margin: 6px 0 0;
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.6;
}
.section { margin-top: 12px; }
.section h4 {
  margin: 0 0 6px 0;
  color: var(--text);
  font-size: 13px;
}
.list {
  margin: 0;
  padding-left: 20px;
  color: var(--text);
  line-height: 1.6;
}
.chapter-card {
  padding: 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg3);
  margin-bottom: 6px;
}
.chapter-card p {
  margin: 0 0 5px 0;
  font-size: 13px;
  color: var(--text);
  line-height: 1.5;
}
.chapter-card p:last-child { margin-bottom: 0; }
.chapter-title { font-weight: 600; color: var(--cyan); }
.written-chapter {
  margin-bottom: 12px;
}
.written-title {
  margin: 0 0 6px 0;
  color: var(--text);
  font-size: 14px;
}
.chapter-summary {
  margin: 0 0 8px 0;
  color: var(--text-muted);
  font-size: 12px;
}
.first-chapter {
  white-space: pre-wrap;
  line-height: 1.72;
  font-size: 13px;
  color: var(--text);
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg3);
}
.revise-input {
  width: 100%;
  margin-top: 8px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  background: var(--bg3);
  color: var(--text);
  border-radius: 6px;
  font-size: 12px;
  font-family: inherit;
  resize: vertical;
}
.revise-btn {
  margin-top: 6px;
}

@media (max-width: 700px) {
  .form-grid { grid-template-columns: 1fr; }
  .action-row { grid-template-columns: 1fr; }
  .project-row { grid-template-columns: 1fr; }
  .btn-secondary { width: 100%; padding: 8px 14px; }
}
</style>
