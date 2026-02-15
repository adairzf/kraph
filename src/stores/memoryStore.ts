import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Memory } from '../types/memory'
import {
  getMemoriesList,
  saveMemory as apiSaveMemory,
  updateMemory as apiUpdateMemory,
  deleteMemory as apiDeleteMemory,
  getTimeline,
} from '../utils/tauriApi'

export const useMemoryStore = defineStore('memory', () => {
  const memories = ref<Memory[]>([])
  const currentMemory = ref<Memory | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchMemories() {
    loading.value = true
    error.value = null
    try {
      memories.value = await getMemoriesList()
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  async function saveMemory(content: string, tags?: string[]) {
    loading.value = true
    error.value = null
    try {
      const m = await apiSaveMemory(content, tags)
      memories.value = [m, ...memories.value]
      currentMemory.value = m
      return m
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function fetchTimeline() {
    loading.value = true
    error.value = null
    try {
      return await getTimeline()
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      return []
    } finally {
      loading.value = false
    }
  }

  async function updateMemoryContent(memoryId: number, content: string, tags?: string[]) {
    loading.value = true
    error.value = null
    try {
      const updated = await apiUpdateMemory(memoryId, content, tags)
      const index = memories.value.findIndex(m => m.id === memoryId)
      if (index >= 0) {
        memories.value[index] = updated
      }
      if (currentMemory.value?.id === memoryId) {
        currentMemory.value = updated
      }
      return updated
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteMemoryById(memoryId: number) {
    loading.value = true
    error.value = null
    try {
      console.log('Store: 调用 API 删除记忆', memoryId)
      await apiDeleteMemory(memoryId)
      console.log('Store: API 删除成功，更新本地状态')
      memories.value = memories.value.filter(m => m.id !== memoryId)
      if (currentMemory.value?.id === memoryId) {
        currentMemory.value = null
      }
      console.log('Store: 本地状态更新完成')
    } catch (e) {
      console.error('Store: 删除失败', e)
      error.value = e instanceof Error ? e.message : String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  function setCurrentMemory(m: Memory | null) {
    currentMemory.value = m
  }

  return {
    memories,
    currentMemory,
    loading,
    error,
    fetchMemories,
    saveMemory,
    fetchTimeline,
    updateMemoryContent,
    deleteMemoryById,
    setCurrentMemory,
  }
})
