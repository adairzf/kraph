import { defineAsyncComponent } from 'vue'
import type { PluginManifest } from '../types'

export const relationshipManagerPlugin: PluginManifest = {
  id: 'relationship-manager',
  tabKey: 'app.tabs.relationship',
  menuKey: 'app.plugins.relationshipManager',
  descriptionKey: 'app.plugins.relationshipManagerDesc',
  component: defineAsyncComponent(() => import('../../components/RelationshipManagerPanel.vue')),
  source: 'builtin',
  version: '1.0.0',
  defaultEnabled: true,
  order: 12,
}
