# External Plugin Template

This folder is a starter template for external plugin development.

## Structure

- `plugin.manifest.json` - plugin metadata
- `src/index.html` - plugin UI entry (loaded in an iframe)

## Required Manifest Fields

- `id`: lowercase letters/numbers/dashes only
- `name`: display name
- `version`: plugin version
- `entry`: html entry file path, e.g. `./src/index.html`

## Installation Concept (Recommended)

1. Build or prepare plugin directory with `plugin.manifest.json`
2. In app Plugin Settings, paste plugin directory path and click Install
3. App copies plugin to managed `plugins/` directory
4. Enable it in Plugin Settings
5. Open plugin tab from the right panel

## Notes

- Keep plugin logic self-contained.
- Avoid direct imports from app internals.
- Current MVP external runtime supports html iframe entry.
