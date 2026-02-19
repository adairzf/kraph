import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useOllamaStore = defineStore('ollama', () => {
  // 是否请求打开初始化弹窗（由其他组件触发，App.vue 监听并响应）
  const setupRequested = ref(false)

  function requestSetup() {
    setupRequested.value = true
  }

  function consumeSetupRequest() {
    setupRequested.value = false
  }

  return { setupRequested, requestSetup, consumeSetupRequest }
})
