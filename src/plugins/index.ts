import { storyGeneratorPlugin } from './builtin/storyGenerator'
import { relationshipManagerPlugin } from './builtin/relationshipManager'
import type { PluginManifest } from './types'

const ENABLED_PLUGINS_KEY = 'enabled-plugin-ids'
const LEGACY_ENABLED_BUILTIN_PLUGINS_KEY = 'enabled-builtin-plugins'

export const BUILTIN_PLUGINS: PluginManifest[] = [
  storyGeneratorPlugin,
  relationshipManagerPlugin,
]

export function loadStoredEnabledPluginIds(): string[] {
  const raw = localStorage.getItem(ENABLED_PLUGINS_KEY)
    ?? localStorage.getItem(LEGACY_ENABLED_BUILTIN_PLUGINS_KEY)
  if (!raw) return []

  try {
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    const normalized = parsed
      .map((x) => String(x))
    return [...new Set(normalized)]
  } catch {
    return []
  }
}

export function hasStoredEnabledPluginIds(): boolean {
  return localStorage.getItem(ENABLED_PLUGINS_KEY) !== null
    || localStorage.getItem(LEGACY_ENABLED_BUILTIN_PLUGINS_KEY) !== null
}

export function saveEnabledPluginIds(ids: string[]) {
  const normalized = [...new Set(ids)]
  localStorage.setItem(ENABLED_PLUGINS_KEY, JSON.stringify(normalized))
}

export function getDefaultEnabledBuiltinPluginIds(): string[] {
  return BUILTIN_PLUGINS
    .filter((p) => p.defaultEnabled !== false)
    .map((p) => p.id)
}
