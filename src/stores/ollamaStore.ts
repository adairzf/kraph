import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useOllamaStore = defineStore('ollama', () => {
  // Whether a setup dialog open has been requested (triggered by other components; App.vue listens and responds)
  const setupRequested = ref(false)

  function requestSetup() {
    setupRequested.value = true
  }

  function consumeSetupRequest() {
    setupRequested.value = false
  }

  return { setupRequested, requestSetup, consumeSetupRequest }
})
