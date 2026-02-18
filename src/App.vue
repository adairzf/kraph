<script setup lang="ts">
import { ref } from 'vue'
import MemoryList from './components/MemoryList.vue'
import InputPanel from './components/InputPanel.vue'
import EditorPanel from './components/EditorPanel.vue'
import GraphPanel from './components/GraphPanel.vue'
import Timeline from './components/Timeline.vue'
import CharacterCard from './components/CharacterCard.vue'
import SearchPanel from './components/SearchPanel.vue'
import ModelSettings from './components/ModelSettings.vue'
import ModelIndicator from './components/ModelIndicator.vue'
import { useGraphStore } from './stores/graphStore'
import { downloadOllamaInstaller, checkOllama, openMemoriesFolder, cleanupDatabase, clearAllData } from './utils/tauriApi'
import { onMounted, onUnmounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'

const editorContent = ref('')
const centerView = ref<'edit' | 'timeline' | 'qa' | 'settings'>('qa')
const graphStore = useGraphStore()
const searchName = ref('')
const ollamaDownloading = ref(false)
const ollamaMessage = ref('')
const ollamaStatus = ref<string>('')
const ollamaProgress = ref('')
const leftSidebarCollapsed = ref(false)
let ollamaProgressTimer: number | null = null

onMounted(async () => {
  const [running, msg] = await checkOllama()
  ollamaStatus.value = running ? '✓ Ollama 已运行' : msg
})

function onSearchEntity() {
  graphStore.setSearchEntity(searchName.value.trim() || null)
}

async function onOpenMemoriesFolder() {
  try {
    await openMemoriesFolder()
  } catch (e) {
    ollamaMessage.value = e instanceof Error ? e.message : String(e)
  }
}

async function onCleanupDatabase() {
  try {
    const msg = await cleanupDatabase()
    await graphStore.fetchGraph()
    ElMessage.success(msg)
  } catch (e) {
    ElMessage.error('清理失败: ' + (e instanceof Error ? e.message : String(e)))
  }
}

async function onClearAllData() {
  try {
    await ElMessageBox.confirm(
      '此操作将永久删除所有记忆、实体、关系数据，是否继续？',
      '⚠️ 危险操作',
      {
        confirmButtonText: '确定清空',
        cancelButtonText: '取消',
        type: 'warning',
        confirmButtonClass: 'el-button--danger'
      }
    )
    
    const msg = await clearAllData()
    await graphStore.fetchGraph()
    ElMessage.success(msg)
    
    // 刷新页面
    window.location.reload()
  } catch (e) {
    if (e !== 'cancel') {
      ElMessage.error('清空失败: ' + (e instanceof Error ? e.message : String(e)))
    }
  }
}

async function onDownloadOllama() {
  ollamaDownloading.value = true
  ollamaMessage.value = ''
  ollamaProgress.value = '正在准备下载安装包，请稍候'
  let dot = 0
  ollamaProgressTimer = window.setInterval(() => {
    dot = (dot + 1) % 4
    ollamaProgress.value = `正在下载并打开安装器${'.'.repeat(dot)}`
  }, 500)
  try {
    const msg = await downloadOllamaInstaller()
    ollamaMessage.value = msg
    setTimeout(async () => {
      const [running, status] = await checkOllama()
      ollamaStatus.value = running ? '✓ Ollama 已运行' : status
    }, 2000)
  } catch (e) {
    ollamaMessage.value = e instanceof Error ? e.message : String(e)
  } finally {
    if (ollamaProgressTimer) {
      window.clearInterval(ollamaProgressTimer)
      ollamaProgressTimer = null
    }
    ollamaProgress.value = ''
    ollamaDownloading.value = false
  }
}

onUnmounted(() => {
  if (ollamaProgressTimer) {
    window.clearInterval(ollamaProgressTimer)
    ollamaProgressTimer = null
  }
})
</script>

<template>
  <div class="app">
    <header class="header">
      <h1 class="title">记忆 · 知识图谱</h1>
      <div class="header-actions">
        <ModelIndicator />
        <span v-if="ollamaStatus" class="ollama-status">{{ ollamaStatus }}</span>
        <button
          type="button"
          class="btn-cleanup"
          @click="onCleanupDatabase"
          title="清理数据库脏数据"
        >
          🧹 清理数据库
        </button>
        <button
          type="button"
          class="btn-clear-all"
          @click="onClearAllData"
          title="清空所有数据（危险操作）"
        >
          ⚠️ 清空数据
        </button>
        <button
          type="button"
          class="btn-open-memories"
          @click="onOpenMemoriesFolder"
        >
          打开记忆文件夹
        </button>
        <button
          type="button"
          class="btn-download-ollama"
          :disabled="ollamaDownloading"
          @click="onDownloadOllama"
        >
          {{ ollamaDownloading ? '正在下载…' : '下载并安装 Ollama' }}
        </button>
        <p v-if="ollamaProgress" class="header-progress">{{ ollamaProgress }}</p>
        <p v-if="ollamaMessage" class="header-message">{{ ollamaMessage }}</p>
      </div>
    </header>
    <div class="main">
      <aside class="sidebar left" :class="{ collapsed: leftSidebarCollapsed }">
        <button 
          class="collapse-btn" 
          @click="leftSidebarCollapsed = !leftSidebarCollapsed"
          :title="leftSidebarCollapsed ? '展开记忆列表' : '收起记忆列表'"
        >
          {{ leftSidebarCollapsed ? '→' : '←' }}
        </button>
        <div v-show="!leftSidebarCollapsed" class="sidebar-content">
          <MemoryList />
        </div>
      </aside>
      <section class="center" :class="{ 'left-collapsed': leftSidebarCollapsed }">
        <InputPanel v-model="editorContent" />
        <div class="center-tabs">
        <button
            type="button"
            class="tab"
            :class="{ active: centerView === 'qa' }"
            @click="centerView = 'qa'"
        >
            问答
        </button>
          <button
            type="button"
            class="tab"
            :class="{ active: centerView === 'edit' }"
            @click="centerView = 'edit'"
          >
            编辑
          </button>
          <button
            type="button"
            class="tab"
            :class="{ active: centerView === 'settings' }"
            @click="centerView = 'settings'"
          >
            ⚙️ 设置
          </button>
          <!-- <button
            type="button"
            class="tab"
            :class="{ active: centerView === 'timeline' }"
            @click="centerView = 'timeline'"
          >
            时间轴
          </button> -->
          
        </div>
        <div class="center-content">
          <EditorPanel v-show="centerView === 'edit'" />
          <Timeline v-show="centerView === 'timeline'" />
          <SearchPanel v-show="centerView === 'qa'" />
          <ModelSettings v-show="centerView === 'settings'" />
        </div>
      </section>
      <aside class="sidebar right">
        <div class="right-search">
          <input
            v-model="searchName"
            type="text"
            placeholder="搜索人物/实体…"
            class="search-input"
            @keyup.enter="onSearchEntity"
          />
          <button type="button" class="btn-search" @click="onSearchEntity">搜索</button>
        </div>
        <div class="graph-container">
          <GraphPanel />
        </div>
        <div class="character-card-container">
          <CharacterCard
            :entity-id="graphStore.selectedEntityId"
            :entity-name="graphStore.searchEntityName"
          />
        </div>
      </aside>
    </div>
  </div>
</template>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;
  color: #0f0f0f;
  background-color: #f6f6f6;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
* {
  box-sizing: border-box;
}
</style>

<style scoped>
.app {
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.header {
  flex-shrink: 0;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
  background: #fff;
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 0.5rem;
}
.title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 600;
}
.header-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}
.ollama-status {
  font-size: 0.8125rem;
  padding: 0.3rem 0.6rem;
  border-radius: 4px;
  background: #f0f0f0;
  color: #666;
}
.btn-download-ollama {
  padding: 0.4rem 0.75rem;
  font-size: 0.8125rem;
  border: 1px solid #24c8db;
  background: #fff;
  color: #24c8db;
  border-radius: 6px;
  cursor: pointer;
}
.btn-open-memories {
  padding: 0.4rem 0.75rem;
  font-size: 0.8125rem;
  border: 1px solid #aaa;
  background: #fff;
  color: #333;
  border-radius: 6px;
  cursor: pointer;
}
.btn-open-memories:hover {
  background: #f6f6f6;
}
.btn-cleanup {
  padding: 0.4rem 0.75rem;
  font-size: 0.8125rem;
  border: 1px solid #ff9800;
  background: #fff;
  color: #ff9800;
  border-radius: 6px;
  cursor: pointer;
}
.btn-cleanup:hover {
  background: #fff3e0;
}
.btn-clear-all {
  padding: 0.4rem 0.75rem;
  font-size: 0.8125rem;
  border: 1px solid #f44336;
  background: #fff;
  color: #f44336;
  border-radius: 6px;
  cursor: pointer;
}
.btn-clear-all:hover {
  background: #ffebee;
}
.btn-download-ollama:hover:not(:disabled) {
  background: #e8f9fb;
}
.btn-download-ollama:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}
.header-message {
  margin: 0;
  font-size: 0.8125rem;
  color: #666;
  max-width: 320px;
}
.header-progress {
  margin: 0;
  font-size: 0.8125rem;
  color: #0a7f8c;
}
.main {
  flex: 1;
  display: flex;
  min-height: 0;
}
.sidebar {
  flex: 0 0 260px;
  padding: 1rem;
  background: #fff;
  border-right: 1px solid rgba(0, 0, 0, 0.08);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  position: relative;
  transition: flex-basis 0.3s ease, padding 0.3s ease;
}
.sidebar.collapsed {
  flex: 0 0 40px;
  padding: 1rem 0.5rem;
}
.sidebar-content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.collapse-btn {
  position: absolute;
  top: 1rem;
  right: 0.5rem;
  width: 24px;
  height: 24px;
  border: 1px solid #ddd;
  background: #fff;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
  transition: all 0.2s;
}
.collapse-btn:hover {
  background: #f5f5f5;
  border-color: #24c8db;
  color: #24c8db;
}
.sidebar.right {
  border-right: none;
  border-left: 1px solid rgba(0, 0, 0, 0.08);
  flex: 0 0 50%;
  max-width: 800px;
}
.right-search {
  display: flex;
  gap: 0.35rem;
  margin-bottom: 0.5rem;
  flex-shrink: 0;
}
.graph-container {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  margin-bottom: 0.5rem;
}
.character-card-container {
  flex-shrink: 0;
  max-height: 40%;
  overflow-y: auto;
}
.search-input {
  flex: 1;
  padding: 0.35rem 0.5rem;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 0.875rem;
}
.btn-search {
  padding: 0.35rem 0.6rem;
  border: 1px solid #24c8db;
  background: #24c8db;
  color: #fff;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.875rem;
}
.center {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: #fafafa;
  transition: margin-left 0.3s ease;
}
.center.left-collapsed {
  /* 当左侧收起时，可以有更多空间 */
}
.center-tabs {
  display: flex;
  gap: 4px;
  padding: 0.5rem 0.75rem 0;
}
.center-tabs .tab {
  padding: 0.35rem 0.75rem;
  border: 1px solid #ddd;
  background: #f9f9f9;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.875rem;
}
.center-tabs .tab.active {
  background: #fff;
  border-color: #24c8db;
  color: #24c8db;
}
.center-content {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 0 0.75rem 0.75rem;
}
</style>
