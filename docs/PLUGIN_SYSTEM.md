# Plugin System (Current MVP)

## Architecture

- Built-in plugin registry: `src/plugins/index.ts`
- Plugin manifest type: `src/plugins/types.ts`
- Plugin runtime state and install actions: `src/stores/pluginStore.ts`
- Plugin settings UI: `src/components/PluginSettingsPanel.vue`
- Host tab rendering: `src/App.vue`

## External Plugin Installation

Backend commands (Tauri):

- `list_external_plugins`
- `install_external_plugin`
- `uninstall_external_plugin`
- `get_plugins_folder_path`
- `open_plugins_folder`

Managed install path:

- `<app_data_dir>/plugins/<plugin_id>/`

Manifest file:

- `<plugin_dir>/plugin.manifest.json`

Required fields:

- `id`: lowercase letters, numbers, dashes
- `name`: display name
- `version`: plugin version
- `entry`: plugin UI entry path (MVP expects html for iframe host)

Optional fields:

- `tab_key`, `menu_key` (for i18n key routing)
- `description`

## External Runtime (MVP Scope)

- External plugin tab is hosted by `ExternalPluginHostPanel.vue`.
- If `entry` points to an html file, app loads it in iframe.
- Non-html entry is still installable, but only metadata is shown.

## Development Template

- `plugin-templates/external-plugin-template/plugin.manifest.json`
- `plugin-templates/external-plugin-template/src/index.html`
- `plugin-templates/external-plugin-template/README.md`

This keeps core capability registration decoupled while allowing plugin extension through a stable installation flow.
