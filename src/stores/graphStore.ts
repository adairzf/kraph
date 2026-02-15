import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { GraphData } from '../types/graph'
import { getGraph } from '../utils/tauriApi'

export const useGraphStore = defineStore('graph', () => {
  const graphData = ref<GraphData | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const selectedEntityId = ref<number | null>(null)
  const searchEntityName = ref<string | null>(null)

  async function fetchGraph() {
    loading.value = true
    error.value = null
    try {
      graphData.value = await getGraph()
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  function setSelectedEntity(id: number | null) {
    selectedEntityId.value = id
    searchEntityName.value = null
  }

  function setSearchEntity(name: string | null) {
    searchEntityName.value = name
    selectedEntityId.value = null
  }

  return {
    graphData,
    loading,
    error,
    selectedEntityId,
    searchEntityName,
    fetchGraph,
    setSelectedEntity,
    setSearchEntity,
  }
})
