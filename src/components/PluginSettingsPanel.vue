<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { usePluginStore } from '../stores/pluginStore'
import { getPluginsFolderPath, openPluginsFolder } from '../utils/tauriApi'
import type { PluginManifest } from '../plugins/types'

const { t } = useI18n()
const pluginStore = usePluginStore()

const plugins = computed(() =>
  pluginStore.plugins.filter((p) => pluginStore.externalEnabled || p.source === 'builtin'),
)
const externalPlugins = computed(() => plugins.value.filter((p) => p.source === 'external'))
const installPathInput = ref('')
const installing = ref(false)
const removingPluginId = ref('')
const pluginsFolderPath = ref('')

function pluginTitle(plugin: PluginManifest): string {
  if (plugin.menuKey) {
    const translated = t(plugin.menuKey)
    if (translated !== plugin.menuKey) return translated
  }
  return plugin.displayName || plugin.id
}

function pluginDescription(plugin: PluginManifest): string {
  if (plugin.descriptionKey) {
    const translated = t(plugin.descriptionKey)
    if (translated !== plugin.descriptionKey) return translated
  }
  return plugin.description || ''
}

function sourceLabel(source: 'builtin' | 'external') {
  return source === 'builtin'
    ? t('pluginSettings.sourceBuiltin')
    : t('pluginSettings.sourceExternal')
}

function onToggle(pluginId: string) {
  pluginStore.toggle(pluginId)
}

async function onInstall() {
  const sourcePath = installPathInput.value.trim()
  if (!sourcePath || installing.value) return
  installing.value = true
  try {
    await pluginStore.installFromPath(sourcePath)
    installPathInput.value = ''
    ElMessage.success(t('pluginSettings.installSuccess'))
  } catch (e) {
    ElMessage.error(t('pluginSettings.installFailed') + (e instanceof Error ? e.message : String(e)))
  } finally {
    installing.value = false
  }
}

async function onUninstall(plugin: PluginManifest) {
  if (plugin.source !== 'external' || removingPluginId.value) return
  removingPluginId.value = plugin.id
  try {
    await ElMessageBox.confirm(
      t('pluginSettings.uninstallConfirmMessage', { name: pluginTitle(plugin) }),
      t('pluginSettings.uninstallConfirmTitle'),
      {
        confirmButtonText: t('pluginSettings.uninstallConfirm'),
        cancelButtonText: t('pluginSettings.uninstallCancel'),
        type: 'warning',
      },
    )
    await pluginStore.uninstallById(plugin.id)
    ElMessage.success(t('pluginSettings.uninstallSuccess'))
  } catch (e) {
    if (e !== 'cancel') {
      ElMessage.error(t('pluginSettings.uninstallFailed') + (e instanceof Error ? e.message : String(e)))
    }
  } finally {
    removingPluginId.value = ''
  }
}

async function onOpenPluginsFolder() {
  try {
    const path = await openPluginsFolder()
    pluginsFolderPath.value = path
  } catch (e) {
    ElMessage.error(t('pluginSettings.openFolderFailed') + (e instanceof Error ? e.message : String(e)))
  }
}

onMounted(async () => {
  try {
    await pluginStore.load()
  } catch (e) {
    ElMessage.error(t('pluginSettings.loadFailed') + (e instanceof Error ? e.message : String(e)))
  }
  try {
    pluginsFolderPath.value = await getPluginsFolderPath()
  } catch (e) {
    ElMessage.error(t('pluginSettings.openFolderFailed') + (e instanceof Error ? e.message : String(e)))
  }
})
</script>

<template>
  <div class="plugin-settings">
    <h2 class="panel-title">{{ t('pluginSettings.title') }}</h2>
    <p class="hint">{{ t('pluginSettings.hint') }}</p>

    <section v-if="pluginStore.externalEnabled" class="install-box">
      <h3>{{ t('pluginSettings.installTitle') }}</h3>
      <p>{{ t('pluginSettings.installHint') }}</p>
      <div class="install-row">
        <el-input
          v-model="installPathInput"
          :placeholder="t('pluginSettings.installPathPlaceholder')"
          clearable
          size="small"
        />
        <el-button
          type="primary"
          size="small"
          :loading="installing"
          :disabled="!installPathInput.trim()"
          @click="onInstall"
        >
          {{ t('pluginSettings.installBtn') }}
        </el-button>
      </div>
      <div class="folder-row">
        <span class="folder-path">{{ t('pluginSettings.pluginsFolder') }}: <code>{{ pluginsFolderPath }}</code></span>
        <el-button
          size="small"
          text
          @click="onOpenPluginsFolder"
        >
          {{ t('pluginSettings.openFolderBtn') }}
        </el-button>
      </div>
    </section>

    <div class="plugin-list">
      <article
        v-for="plugin in plugins"
        :key="plugin.id"
        class="plugin-card"
      >
        <div class="plugin-main">
          <p class="plugin-name">{{ pluginTitle(plugin) }}</p>
          <p class="plugin-meta">
            {{ sourceLabel(plugin.source) }} · {{ plugin.id }} · v{{ plugin.version || '1.0.0' }}
          </p>
          <p
            v-if="pluginDescription(plugin)"
            class="plugin-desc"
          >
            {{ pluginDescription(plugin) }}
          </p>
          <p
            v-if="plugin.source === 'external' && plugin.installPath"
            class="plugin-path"
          >
            <code>{{ plugin.installPath }}</code>
          </p>
        </div>
        <div class="plugin-actions">
          <el-switch
            :model-value="pluginStore.isEnabled(plugin.id)"
            @change="onToggle(plugin.id)"
          />
          <el-button
            v-if="plugin.source === 'external'"
            text
            type="danger"
            size="small"
            :loading="removingPluginId === plugin.id"
            @click="onUninstall(plugin)"
          >
            {{ t('pluginSettings.uninstallBtn') }}
          </el-button>
        </div>
      </article>
    </div>

    <section v-if="pluginStore.externalEnabled" class="template-box">
      <h3>{{ t('pluginSettings.templateTitle') }}</h3>
      <p>{{ t('pluginSettings.templateHint') }}</p>
      <ul>
        <li><code>plugin-templates/external-plugin-template/plugin.manifest.json</code></li>
        <li><code>plugin-templates/external-plugin-template/src/index.html</code></li>
        <li><code>docs/PLUGIN_SYSTEM.md</code></li>
      </ul>
      <p
        v-if="!externalPlugins.length"
        class="template-empty"
      >
        {{ t('pluginSettings.noExternalYet') }}
      </p>
    </section>
  </div>
</template>

<style scoped>
.plugin-settings { padding: 0; }
.panel-title {
  margin: 0 0 4px 0;
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}
.hint {
  margin: 0 0 10px 0;
  font-size: 13px;
  color: var(--text-muted);
}
.install-box {
  margin-bottom: 12px;
  padding: 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg3);
}
.install-box h3 {
  margin: 0;
  font-size: 13px;
  color: var(--text);
}
.install-box p {
  margin: 6px 0 8px;
  font-size: 12px;
  color: var(--text-muted);
}
.install-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.folder-row {
  margin-top: 8px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.folder-path {
  font-size: 12px;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.plugin-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.plugin-card {
  display: flex;
  gap: 12px;
  justify-content: space-between;
  align-items: flex-start;
  padding: 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg3);
}
.plugin-main { min-width: 0; }
.plugin-name {
  margin: 0;
  font-size: 14px;
  color: var(--text);
  font-weight: 600;
}
.plugin-meta {
  margin: 4px 0 0 0;
  font-size: 11px;
  color: var(--text-dim);
}
.plugin-desc {
  margin: 6px 0 0 0;
  font-size: 12px;
  color: var(--text-muted);
}
.plugin-path {
  margin: 6px 0 0 0;
  font-size: 11px;
  color: var(--text-dim);
}
.plugin-actions {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 6px;
}
.template-box {
  margin-top: 12px;
  padding: 10px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: rgba(59, 130, 246, 0.06);
}
.template-box h3 {
  margin: 0 0 6px;
  font-size: 13px;
  color: var(--text);
}
.template-box p {
  margin: 0 0 8px;
  font-size: 12px;
  color: var(--text-muted);
}
.template-box ul {
  margin: 0;
  padding-left: 18px;
}
.template-box li {
  margin: 4px 0;
  font-size: 12px;
  color: var(--text);
}
.template-empty {
  margin-top: 8px;
}
</style>
