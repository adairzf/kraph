<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import MemoryList from './components/MemoryList.vue'
import InputPanel from './components/InputPanel.vue'
import EditorPanel from './components/EditorPanel.vue'
import GraphPanel from './components/GraphPanel.vue'
import CharacterCard from './components/CharacterCard.vue'
import SearchPanel from './components/SearchPanel.vue'
import ModelSettings from './components/ModelSettings.vue'
import ModelIndicator from './components/ModelIndicator.vue'
import OllamaSetupDialog from './components/OllamaSetupDialog.vue'
import { useGraphStore } from './stores/graphStore'
import { useOllamaStore } from './stores/ollamaStore'
import { checkOllama, getModelConfig, openMemoriesFolder, clearAllData } from './utils/tauriApi'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useI18n } from 'vue-i18n'

const { t, locale } = useI18n()

function toggleLocale() {
  const next = locale.value === 'zh-CN' ? 'en-US' : 'zh-CN'
  locale.value = next
  localStorage.setItem('app-locale', next)
}

const editorContent = ref('')
const rightView = ref<'entity' | 'qa' | 'edit' | 'settings'>('entity')
const graphStore = useGraphStore()
const ollamaStore = useOllamaStore()
const searchName = ref('')
const leftSidebarCollapsed = ref(false)
const menuOpen = ref(false)

// Ollama status
const ollamaChecking = ref(true)
const ollamaRunning = ref(false)
const isOllamaProvider = ref(false)

// Ref to the setup dialog component
const setupDialogRef = ref<InstanceType<typeof OllamaSetupDialog> | null>(null)

onMounted(async () => {
  try {
    const config = await getModelConfig()
    isOllamaProvider.value = config.provider.type === 'ollama'
    if (isOllamaProvider.value) {
      const [running] = await checkOllama()
      ollamaRunning.value = running
    }
  } catch {
    // Ignore startup check errors silently
  } finally {
    ollamaChecking.value = false
  }

  document.addEventListener('click', handleOutsideClick)
})

onUnmounted(() => {
  document.removeEventListener('click', handleOutsideClick)
})

function handleOutsideClick(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (!target.closest('.menu-wrapper')) {
    menuOpen.value = false
  }
}

// Listen for setup requests emitted by child components (e.g. SearchPanel)
watch(
  () => ollamaStore.setupRequested,
  (val: boolean) => {
    if (val) {
      ollamaStore.consumeSetupRequest()
      openSetupDialog()
    }
  },
)

function openSetupDialog() {
  setupDialogRef.value?.open()
}

function onSearchEntity() {
  graphStore.setSearchEntity(searchName.value.trim() || null)
}

async function onOpenMemoriesFolder() {
  menuOpen.value = false
  try {
    await openMemoriesFolder()
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e))
  }
}

async function onClearAllData() {
  menuOpen.value = false
  try {
    await ElMessageBox.confirm(
      t('app.clearAllConfirm.message'),
      t('app.clearAllConfirm.title'),
      {
        confirmButtonText: t('app.clearAllConfirm.confirm'),
        cancelButtonText: t('app.clearAllConfirm.cancel'),
        type: 'warning',
        confirmButtonClass: 'el-button--danger'
      }
    )
    
    const msg = await clearAllData()
    await graphStore.fetchGraph()
    ElMessage.success(msg)
    
    window.location.reload()
  } catch (e) {
    if (e !== 'cancel') {
      ElMessage.error(t('app.errors.clearAllFailed') + (e instanceof Error ? e.message : String(e)))
    }
  }
}

</script>

<template>
  <div class="app">
    <header class="header">
      <div class="header-left">
        <button
          class="sidebar-toggle"
          @click="leftSidebarCollapsed = !leftSidebarCollapsed"
          :title="leftSidebarCollapsed ? t('app.header.expandMemoryList') : t('app.header.collapseMemoryList')"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <rect x="1" y="3" width="14" height="1.5" rx="0.75"/>
            <rect x="1" y="7.25" width="14" height="1.5" rx="0.75"/>
            <rect x="1" y="11.5" width="14" height="1.5" rx="0.75"/>
          </svg>
        </button>
        <h1 class="title">{{ t('app.title') }}</h1>
      </div>
      <div class="header-actions">
        <ModelIndicator />

        <button
          v-if="isOllamaProvider || ollamaChecking"
          type="button"
          class="btn-ollama-status"
          :class="{
            'status-checking': ollamaChecking,
            'status-ok': !ollamaChecking && ollamaRunning,
            'status-warn': !ollamaChecking && !ollamaRunning,
          }"
          :title="ollamaChecking ? t('app.header.ollamaChecking') : ollamaRunning ? t('app.header.ollamaReadyTitle') : t('app.header.ollamaNotReadyTitle')"
          @click="openSetupDialog"
        >
          <span class="status-dot" />
          <span v-if="ollamaChecking">{{ t('app.header.ollamaStatusChecking') }}</span>
          <span v-else-if="ollamaRunning">Ollama</span>
          <span v-else>Ollama</span>
        </button>

        <!-- More actions dropdown -->
        <div class="menu-wrapper">
          <button class="btn-menu" @click.stop="menuOpen = !menuOpen" title="更多操作">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <circle cx="8" cy="2.5" r="1.5"/>
              <circle cx="8" cy="8" r="1.5"/>
              <circle cx="8" cy="13.5" r="1.5"/>
            </svg>
          </button>
          <div v-if="menuOpen" class="dropdown-menu">
            <button class="menu-item" @click="toggleLocale">
              <span class="menu-icon">🌐</span>
              {{ t('app.langSwitch') }}
            </button>
            <button class="menu-item" @click="onOpenMemoriesFolder">
              <span class="menu-icon">📂</span>
              {{ t('app.header.openMemoriesFolder') }}
            </button>
            <div class="menu-divider" />
            <button class="menu-item danger" @click="onClearAllData">
              <span class="menu-icon">🗑️</span>
              {{ t('app.header.clearAllData') }}
            </button>
          </div>
        </div>
      </div>

      <OllamaSetupDialog ref="setupDialogRef" />
    </header>

    <div class="main">
      <!-- ─── LEFT: Memory list + input ─── -->
      <aside class="sidebar-left" :class="{ collapsed: leftSidebarCollapsed }">
        <div class="sidebar-left-inner">
          <div class="memory-area">
            <MemoryList />
          </div>
          <div class="input-area">
            <InputPanel v-model="editorContent" />
          </div>
        </div>
      </aside>

      <!-- ─── CENTER: Knowledge Graph (main) ─── -->
      <section class="graph-center">
        <div class="graph-search-bar">
          <input
            v-model="searchName"
            type="text"
            :placeholder="t('app.search.placeholder')"
            class="search-input"
            @keyup.enter="onSearchEntity"
          />
          <button type="button" class="btn-search" @click="onSearchEntity">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="7" cy="7" r="5"/>
              <line x1="11" y1="11" x2="15" y2="15"/>
            </svg>
          </button>
        </div>
        <div class="graph-wrap">
          <GraphPanel />
        </div>
      </section>

      <!-- ─── RIGHT: Entity card + functional panel ─── -->
      <aside class="sidebar-right">
        <!-- Tabs -->
        <div class="right-tabs">
          <button class="rtab" :class="{ active: rightView === 'entity' }" @click="rightView = 'entity'">
            {{ t('characterCard.title') }}
          </button>
          <button class="rtab" :class="{ active: rightView === 'qa' }" @click="rightView = 'qa'">
            {{ t('app.tabs.qa') }}
          </button>
          <button class="rtab" :class="{ active: rightView === 'edit' }" @click="rightView = 'edit'">
            {{ t('app.tabs.edit') }}
          </button>
          <button class="rtab" :class="{ active: rightView === 'settings' }" @click="rightView = 'settings'">
            {{ t('app.tabs.settings') }}
          </button>
        </div>
        <!-- Tab content -->
        <div class="right-content">
          <CharacterCard
            v-show="rightView === 'entity'"
            :entity-id="graphStore.selectedEntityId"
            :entity-name="graphStore.searchEntityName"
          />
          <SearchPanel v-show="rightView === 'qa'" />
          <EditorPanel v-show="rightView === 'edit'" />
          <ModelSettings v-show="rightView === 'settings'" />
        </div>
      </aside>
    </div>
  </div>
</template>

<style>
:root {
  --bg: #0d0d14;
  --bg2: #111118;
  --bg3: #16161f;
  --bg4: #1c1c28;
  --border: rgba(255, 255, 255, 0.07);
  --border-hover: rgba(255, 255, 255, 0.12);
  --text: #e2e2ee;
  --text-muted: #7a7a8e;
  --text-dim: #4a4a5e;
  --accent: #7c5cfc;
  --accent-2: #3b82f6;
  --accent-glow: rgba(124, 92, 252, 0.25);
  --cyan: #22d3ee;
  --green: #34d399;
  --red: #f87171;
  --orange: #fb923c;
  --radius: 8px;
  --grad: linear-gradient(135deg, #7c5cfc, #3b82f6);

  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  font-size: 14px;
  line-height: 1.5;
  font-weight: 400;
  color: var(--text);
  background-color: var(--bg);
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
* { box-sizing: border-box; margin: 0; padding: 0; }
html, body, #app { height: 100%; }

/* ─── Element Plus dark theme token overrides ─── */
html.dark {
  --el-bg-color: var(--bg3);
  --el-bg-color-page: var(--bg);
  --el-bg-color-overlay: var(--bg3);
  --el-text-color-primary: var(--text);
  --el-text-color-regular: var(--text-muted);
  --el-text-color-secondary: var(--text-dim);
  --el-border-color: rgba(255,255,255,0.12);
  --el-border-color-light: rgba(255,255,255,0.08);
  --el-border-color-lighter: rgba(255,255,255,0.06);
  --el-fill-color-blank: var(--bg3);
  --el-fill-color: var(--bg4);
  --el-fill-color-light: var(--bg4);
  --el-fill-color-lighter: var(--bg3);
  --el-color-primary: var(--accent);
  --el-color-primary-light-9: rgba(124,92,252,0.1);
  --el-mask-color: rgba(0,0,0,0.6);
  --el-box-shadow: 0 8px 32px rgba(0,0,0,0.5);
  --el-box-shadow-light: 0 4px 16px rgba(0,0,0,0.4);
  --el-disabled-bg-color: var(--bg4);
  --el-disabled-border-color: rgba(255,255,255,0.06);
  --el-disabled-text-color: var(--text-dim);
}

/* Extra polish for Element Plus components */
html.dark .el-card {
  background: var(--bg3) !important;
  border-color: var(--border) !important;
}
html.dark .el-card__header {
  border-color: var(--border) !important;
  color: var(--text-muted) !important;
  font-size: 13px !important;
}
html.dark .el-input__wrapper {
  background: var(--bg4) !important;
  box-shadow: 0 0 0 1px rgba(255,255,255,0.1) inset !important;
}
html.dark .el-input__wrapper.is-focus {
  box-shadow: 0 0 0 1px rgba(124,92,252,0.5) inset !important;
}
html.dark .el-input__inner { color: var(--text) !important; }
html.dark .el-input__inner::placeholder { color: var(--text-dim) !important; }
html.dark .el-select__wrapper {
  background: var(--bg4) !important;
  box-shadow: 0 0 0 1px rgba(255,255,255,0.1) inset !important;
}
html.dark .el-select-dropdown {
  background: var(--bg3) !important;
  border-color: var(--border) !important;
}
html.dark .el-select-dropdown__item { color: var(--text-muted) !important; }
html.dark .el-select-dropdown__item:hover,
html.dark .el-select-dropdown__item.is-hovering { background: var(--bg4) !important; }
html.dark .el-radio__label { color: var(--text-muted) !important; }
html.dark .el-radio.is-checked .el-radio__label { color: var(--text) !important; }
html.dark .el-divider { border-color: var(--border) !important; }
html.dark .el-divider__text {
  background: var(--bg) !important;
  color: var(--text-dim) !important;
  font-size: 12px !important;
}
html.dark .el-alert { border: 1px solid var(--border) !important; }
html.dark .el-alert--info { background: var(--bg4) !important; }
html.dark .el-slider__runway { background: var(--bg4) !important; }
html.dark .el-input-number { background: var(--bg4) !important; }
html.dark .el-input-number .el-input__wrapper { background: transparent !important; }
html.dark .el-button { font-family: inherit !important; }
html.dark .el-form-item__label { color: var(--text-muted) !important; font-size: 13px !important; }
html.dark .el-message-box {
  background: var(--bg3) !important;
  border: 1px solid var(--border) !important;
}
html.dark .el-overlay { background: rgba(0,0,0,0.6) !important; }
</style>

<style scoped>
.app {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg);
}

/* ─── HEADER ─── */
.header {
  flex-shrink: 0;
  height: 46px;
  padding: 0 14px;
  border-bottom: 1px solid var(--border);
  background: var(--bg2);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}
.sidebar-toggle {
  width: 28px;
  height: 28px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-muted);
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  flex-shrink: 0;
}
.sidebar-toggle:hover {
  background: var(--bg4);
  color: var(--text);
  border-color: var(--border-hover);
}
.title {
  margin: 0;
  font-size: 15px;
  font-weight: 700;
  background: var(--grad);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  letter-spacing: -0.01em;
}
.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* Ollama status */
.btn-ollama-status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  font-size: 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
  font-weight: 500;
  border: 1px solid transparent;
}
.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}
.btn-ollama-status.status-checking {
  border-color: var(--border);
  background: var(--bg3);
  color: var(--text-dim);
  cursor: default;
}
.btn-ollama-status.status-checking .status-dot { background: var(--text-dim); }
.btn-ollama-status.status-ok {
  border-color: rgba(52, 211, 153, 0.3);
  background: rgba(52, 211, 153, 0.08);
  color: var(--green);
}
.btn-ollama-status.status-ok .status-dot {
  background: var(--green);
  box-shadow: 0 0 6px var(--green);
  animation: pulse-dot 2s infinite;
}
.btn-ollama-status.status-ok:hover { background: rgba(52, 211, 153, 0.14); }
.btn-ollama-status.status-warn {
  border-color: rgba(251, 146, 60, 0.3);
  background: rgba(251, 146, 60, 0.08);
  color: var(--orange);
}
.btn-ollama-status.status-warn .status-dot { background: var(--orange); }
.btn-ollama-status.status-warn:hover { background: rgba(251, 146, 60, 0.14); }
@keyframes pulse-dot {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* Menu dropdown */
.menu-wrapper { position: relative; }
.btn-menu {
  width: 28px;
  height: 28px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-muted);
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}
.btn-menu:hover {
  background: var(--bg4);
  color: var(--text);
  border-color: var(--border-hover);
}
.dropdown-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  min-width: 178px;
  background: var(--bg3);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 4px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.45);
  z-index: 200;
}
.menu-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-size: 13px;
  font-family: inherit;
  border-radius: 6px;
  cursor: pointer;
  text-align: left;
  transition: all 0.15s;
}
.menu-item:hover { background: var(--bg4); color: var(--text); }
.menu-item.danger { color: var(--red); }
.menu-item.danger:hover { background: rgba(248, 113, 113, 0.1); color: var(--red); }
.menu-icon { font-size: 14px; }
.menu-divider { height: 1px; background: var(--border); margin: 4px 0; }

/* ─── MAIN LAYOUT ─── */
.main {
  flex: 1;
  display: flex;
  min-height: 0;
}

/* ─── LEFT SIDEBAR: memory list + input ─── */
.sidebar-left {
  flex: 0 0 220px;
  background: var(--bg2);
  border-right: 1px solid var(--border);
  overflow: hidden;
  transition: flex-basis 0.25s ease, opacity 0.25s ease;
  display: flex;
  flex-direction: column;
}
.sidebar-left.collapsed {
  flex: 0 0 0;
  opacity: 0;
  pointer-events: none;
}
.sidebar-left-inner {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
}
.memory-area {
  flex: 1;
  overflow: hidden;
  min-height: 0;
  padding: 12px 10px 0;
}
.input-area {
  flex-shrink: 0;
  border-top: 1px solid var(--border);
}

/* ─── CENTER: Graph (main) ─── */
.graph-center {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: var(--bg);
}
.graph-search-bar {
  flex-shrink: 0;
  display: flex;
  gap: 6px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg2);
}
.search-input {
  flex: 1;
  padding: 6px 10px;
  border: 1px solid var(--border);
  background: var(--bg3);
  color: var(--text);
  border-radius: 6px;
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.15s;
}
.search-input::placeholder { color: var(--text-dim); }
.search-input:focus { border-color: rgba(124, 92, 252, 0.5); }
.btn-search {
  padding: 0 10px;
  height: 30px;
  border: 1px solid rgba(124, 92, 252, 0.4);
  background: rgba(124, 92, 252, 0.12);
  color: #a78bfa;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}
.btn-search:hover {
  background: rgba(124, 92, 252, 0.22);
  border-color: rgba(124, 92, 252, 0.6);
}
.graph-wrap {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

/* ─── RIGHT SIDEBAR: entity + panels ─── */
.sidebar-right {
  flex: 0 0 300px;
  border-left: 1px solid var(--border);
  background: var(--bg2);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.right-tabs {
  flex-shrink: 0;
  display: flex;
  gap: 2px;
  padding: 8px 10px 0;
  border-bottom: 1px solid var(--border);
  background: var(--bg2);
}
.rtab {
  padding: 6px 12px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  border-radius: 6px 6px 0 0;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  font-family: inherit;
  transition: all 0.15s;
  position: relative;
  white-space: nowrap;
}
.rtab:hover { color: var(--text); background: var(--bg4); }
.rtab.active {
  color: var(--text);
  background: var(--bg);
}
.rtab.active::after {
  content: '';
  position: absolute;
  bottom: -1px;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--grad);
  border-radius: 1px;
}
.right-content {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 12px;
  min-height: 0;
}
.right-content::-webkit-scrollbar { width: 3px; }
.right-content::-webkit-scrollbar-track { background: transparent; }
.right-content::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }
</style>
