import type { Component } from 'vue'

export interface PluginManifest {
  id: string
  tabKey?: string
  menuKey?: string
  displayName?: string
  description?: string
  descriptionKey?: string
  component: Component
  source: 'builtin' | 'external'
  version?: string
  defaultEnabled?: boolean
  order?: number
  componentProps?: Record<string, unknown>
  installPath?: string
  entry?: string
}
