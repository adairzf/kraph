import { defineAsyncComponent } from 'vue'
import type { PluginManifest } from '../types'

export const storyGeneratorPlugin: PluginManifest = {
  id: 'story-generator',
  tabKey: 'app.tabs.story',
  menuKey: 'app.plugins.storyGenerator',
  descriptionKey: 'app.plugins.storyGeneratorDesc',
  component: defineAsyncComponent(() => import('../../components/StoryGeneratorPanel.vue')),
  source: 'builtin',
  version: '1.0.0',
  defaultEnabled: true,
  order: 10,
}
