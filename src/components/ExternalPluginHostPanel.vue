<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  pluginName: string
  description?: string
  installPath: string
  entry?: string
  entryUrl?: string
}>()

const { t } = useI18n()

const canRenderIframe = computed(() => {
  if (!props.entry || !props.entryUrl) return false
  const entry = props.entry.toLowerCase()
  return entry.endsWith('.html') || entry.endsWith('.htm')
})
</script>

<template>
  <div class="external-plugin-host">
    <div class="meta">
      <h3 class="name">{{ props.pluginName }}</h3>
      <p v-if="props.description" class="desc">{{ props.description }}</p>
      <p class="path">{{ t('pluginSettings.installPath') }}: <code>{{ props.installPath }}</code></p>
      <p v-if="props.entry" class="path">{{ t('pluginSettings.entryFile') }}: <code>{{ props.entry }}</code></p>
    </div>

    <iframe
      v-if="canRenderIframe"
      class="plugin-iframe"
      :src="props.entryUrl"
      sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
    />

    <div v-else class="unsupported">
      {{ t('pluginSettings.noUiEntry') }}
    </div>
  </div>
</template>

<style scoped>
.external-plugin-host {
  display: flex;
  flex-direction: column;
  gap: 10px;
  height: 100%;
}
.meta {
  padding: 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg3);
}
.name {
  margin: 0;
  font-size: 14px;
  color: var(--text);
}
.desc {
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--text-muted);
}
.path {
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--text-muted);
}
.plugin-iframe {
  width: 100%;
  flex: 1;
  min-height: 360px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: #fff;
}
.unsupported {
  border: 1px dashed var(--border);
  border-radius: 8px;
  padding: 14px;
  color: var(--text-muted);
  font-size: 12px;
}
</style>
