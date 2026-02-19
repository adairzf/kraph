<script setup lang="ts">
import { ref, watch } from 'vue'
import MemoryList from './components/MemoryList.vue'
import InputPanel from './components/InputPanel.vue'
import EditorPanel from './components/EditorPanel.vue'
import GraphPanel from './components/GraphPanel.vue'
import Timeline from './components/Timeline.vue'
import CharacterCard from './components/CharacterCard.vue'
import SearchPanel from './components/SearchPanel.vue'
import ModelSettings from './components/ModelSettings.vue'
import ModelIndicator from './components/ModelIndicator.vue'
import OllamaSetupDialog from './components/OllamaSetupDialog.vue'
import { useGraphStore } from './stores/graphStore'
import { useOllamaStore } from './stores/ollamaStore'
import { checkOllama, getModelConfig, openMemoriesFolder, cleanupDatabase, clearAllData } from './utils/tauriApi'
import { onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'

const editorContent = ref('')
const centerView = ref<'edit' | 'timeline' | 'qa' | 'settings'>('qa')
const graphStore = useGraphStore()
const ollamaStore = useOllamaStore()
const searchName = ref('')
const leftSidebarCollapsed = ref(false)

// Ollama 状态
const ollamaChecking = ref(true)
const ollamaRunning = ref(false)
const isOllamaProvider = ref(false)

// 初始化弹窗 ref
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
    // 忽略初始化检测错误
  } finally {
    ollamaChecking.value = false
  }
})

// 监听其他组件（如 SearchPanel）发出的初始化请求
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
  setupDialogRef.value?.openAndStart()
}

function onSearchEntity() {
  graphStore.setSearchEntity(searchName.value.trim() || null)
}

async function onOpenMemoriesFolder() {
  try {
    await openMemoriesFolder()
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e))
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

</script>

<template>
  <div class="app">
    <header class="header">
      <h1 class="title">记忆 · 知识图谱</h1>
      <div class="header-actions">
        <ModelIndicator />

        <!-- Ollama 一体化状态按钮：仅在使用 Ollama 提供商时显示 -->
        <button
          v-if="isOllamaProvider || ollamaChecking"
          type="button"
          class="btn-ollama-status"
          :class="{
            'status-checking': ollamaChecking,
            'status-ok': !ollamaChecking && ollamaRunning,
            'status-warn': !ollamaChecking && !ollamaRunning,
          }"
          :title="ollamaChecking ? '正在检测 Ollama 状态...' : ollamaRunning ? '点击重新检测/初始化 Ollama' : '点击一键初始化 Ollama'"
          @click="openSetupDialog"
        >
          <span v-if="ollamaChecking">⏳ 检测中…</span>
          <span v-else-if="ollamaRunning">✓ Ollama 已就绪</span>
          <span v-else>⚡ 初始化 Ollama</span>
        </button>

        <!-- <button
          type="button"
          class="btn-cleanup"
          @click="onCleanupDatabase"
          title="清理数据库脏数据"
        >
          🧹 清理数据库
        </button> -->
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
      </div>

      <!-- Ollama 初始化弹窗 -->
      <OllamaSetupDialog ref="setupDialogRef" />
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
/* Ollama 一体化状态按钮 */
.btn-ollama-status {
  padding: 0.35rem 0.75rem;
  font-size: 0.8125rem;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  font-weight: 500;
  border: 1px solid transparent;
}
.btn-ollama-status.status-checking {
  border-color: #ddd;
  background: #f5f5f5;
  color: #999;
  cursor: default;
}
.btn-ollama-status.status-ok {
  border-color: #67c23a;
  background: #f0f9eb;
  color: #67c23a;
}
.btn-ollama-status.status-ok:hover {
  background: #e1f3d8;
}
.btn-ollama-status.status-warn {
  border-color: #e6a23c;
  background: #fdf6ec;
  color: #e6a23c;
}
.btn-ollama-status.status-warn:hover {
  background: #faecd8;
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
  margin-left: 0;
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
