import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { convertFileSrc } from '@tauri-apps/api/core'
import ExternalPluginHostPanel from '../components/ExternalPluginHostPanel.vue'
import {
  getDefaultEnabledBuiltinPluginIds,
  hasStoredEnabledPluginIds,
  BUILTIN_PLUGINS,
  loadStoredEnabledPluginIds,
  saveEnabledPluginIds,
} from '../plugins'
import type { PluginManifest } from '../plugins/types'
import {
  installExternalPlugin,
  listExternalPlugins,
  uninstallExternalPlugin,
  type ExternalPluginInfo,
} from '../utils/tauriApi'

const EXTERNAL_PLUGINS_ENABLED = false

function buildExternalPluginManifest(plugin: ExternalPluginInfo, order: number): PluginManifest {
  return {
    id: plugin.id,
    tabKey: plugin.tab_key,
    menuKey: plugin.menu_key,
    displayName: plugin.name,
    description: plugin.description,
    component: ExternalPluginHostPanel,
    componentProps: {
      pluginName: plugin.name,
      description: plugin.description,
      installPath: plugin.install_path,
      entry: plugin.entry,
      entryUrl: plugin.entry_path ? convertFileSrc(plugin.entry_path) : '',
    },
    source: 'external',
    version: plugin.version,
    defaultEnabled: false,
    order,
    installPath: plugin.install_path,
    entry: plugin.entry,
  }
}

export const usePluginStore = defineStore('plugins', () => {
  const loaded = ref(false)
  const loading = ref(false)
  const enabledPluginIds = ref<string[]>([])
  const externalPlugins = ref<PluginManifest[]>([])

  const plugins = computed(() =>
    [
      ...BUILTIN_PLUGINS,
      ...(EXTERNAL_PLUGINS_ENABLED ? externalPlugins.value : []),
    ].sort((a, b) => (a.order ?? 0) - (b.order ?? 0)),
  )

  const enabledPlugins = computed(() => {
    const set = new Set(enabledPluginIds.value)
    return plugins.value.filter((p) => set.has(p.id))
  })

  function persist() {
    saveEnabledPluginIds(enabledPluginIds.value)
  }

  function normalizeEnabledIds(applyDefaultsWhenEmpty = false) {
    const availableIds = new Set(plugins.value.map((p) => p.id))
    let normalized = enabledPluginIds.value.filter((id) => availableIds.has(id))
    if (!normalized.length && applyDefaultsWhenEmpty) {
      normalized = getDefaultEnabledBuiltinPluginIds().filter((id) => availableIds.has(id))
    }
    enabledPluginIds.value = [...new Set(normalized)]
    persist()
  }

  function applyExternalPlugins(items: ExternalPluginInfo[]) {
    externalPlugins.value = items.map((plugin, index) =>
      buildExternalPluginManifest(plugin, 1000 + index),
    )
    normalizeEnabledIds(false)
  }

  async function refreshExternalPlugins() {
    if (!EXTERNAL_PLUGINS_ENABLED) {
      externalPlugins.value = []
      normalizeEnabledIds(false)
      return
    }
    const items = await listExternalPlugins()
    applyExternalPlugins(items)
  }

  async function load() {
    if (loaded.value || loading.value) return
    loading.value = true
    try {
      const hasStored = hasStoredEnabledPluginIds()
      enabledPluginIds.value = loadStoredEnabledPluginIds()
      await refreshExternalPlugins()
      normalizeEnabledIds(!hasStored)
      loaded.value = true
    } finally {
      loading.value = false
    }
  }

  function isEnabled(pluginId: string): boolean {
    return enabledPluginIds.value.includes(pluginId)
  }

  function enable(pluginId: string) {
    if (isEnabled(pluginId)) return
    enabledPluginIds.value = [...enabledPluginIds.value, pluginId]
    normalizeEnabledIds(false)
  }

  function disable(pluginId: string) {
    if (!isEnabled(pluginId)) return
    enabledPluginIds.value = enabledPluginIds.value.filter((id) => id !== pluginId)
    persist()
  }

  function toggle(pluginId: string) {
    if (isEnabled(pluginId)) {
      disable(pluginId)
    } else {
      enable(pluginId)
    }
  }

  async function installFromPath(sourcePath: string): Promise<ExternalPluginInfo> {
    if (!EXTERNAL_PLUGINS_ENABLED) {
      throw new Error('External plugins are temporarily disabled.')
    }
    const plugin = await installExternalPlugin(sourcePath)
    await refreshExternalPlugins()
    enable(plugin.id)
    return plugin
  }

  async function uninstallById(pluginId: string) {
    if (!EXTERNAL_PLUGINS_ENABLED) {
      throw new Error('External plugins are temporarily disabled.')
    }
    await uninstallExternalPlugin(pluginId)
    disable(pluginId)
    await refreshExternalPlugins()
  }

  return {
    loaded,
    loading,
    externalEnabled: EXTERNAL_PLUGINS_ENABLED,
    enabledPluginIds,
    plugins,
    enabledPlugins,
    load,
    refreshExternalPlugins,
    isEnabled,
    enable,
    disable,
    toggle,
    installFromPath,
    uninstallById,
  }
})
