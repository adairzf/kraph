# Kraph — Personal Knowledge Graph

> A local-first desktop app that turns your notes into a living knowledge graph, powered by LLMs.

Kraph extracts entities (people, places, events, time) and their relationships from the text you write. It builds a visual knowledge graph you can explore, search, and query in natural language — all running on your machine with your data staying private.

---

## ✨ Features

- **Automatic entity extraction** — paste or type any text and the app extracts people, locations, events, and more
- **Knowledge fusion** — when you add new memories, the app merges them with related historical records to keep the graph coherent
- **Interactive knowledge graph** — force-directed graph with clickable nodes and relationship labels
- **Entity profiles** — click any node to see a full profile: attributes, relations, and all linked memories
- **Natural language Q&A** — ask questions like "Who is Alice?" and get context-aware answers drawn from your memories
- **Markdown storage** — every memory is saved as a `.md` file with YAML frontmatter, so your data is always human-readable
- **Voice input** — dictate memories using local Whisper speech-to-text (macOS)
- **Multi-language UI** — switch between English and Chinese at runtime
- **Multiple AI backends** — works with local Ollama models, DeepSeek API, or any OpenAI-compatible endpoint

---

## 🖥️ Tech Stack

| Layer | Technology |
|---|---|
| Desktop runtime | [Tauri 2](https://tauri.app) (Rust + WebView) |
| Frontend | [Vue 3](https://vuejs.org) + TypeScript + [Vite](https://vitejs.dev) |
| UI components | [Element Plus](https://element-plus.org) |
| Charts | [Apache ECharts](https://echarts.apache.org) via vue-echarts |
| State management | [Pinia](https://pinia.vuejs.org) |
| i18n | [Vue I18n](https://vue-i18n.intlify.dev) |
| Database | SQLite (via [rusqlite](https://github.com/rusqlite/rusqlite)) |
| AI (local) | [Ollama](https://ollama.com) |
| AI (cloud) | DeepSeek API / OpenAI API |
| Speech-to-text | [whisper.cpp](https://github.com/ggerganov/whisper.cpp) |

---

## 🚀 Getting Started

### Prerequisites

- [Node.js](https://nodejs.org) ≥ 18 and [pnpm](https://pnpm.io) (or npm)
- [Rust toolchain](https://rustup.rs) (stable)
- [Tauri CLI prerequisites](https://tauri.app/start/prerequisites/) for your OS

For AI features, you need **at least one** of:
- [Ollama](https://ollama.com) installed locally *(recommended for privacy)*
- A [DeepSeek](https://platform.deepseek.com) or [OpenAI](https://platform.openai.com) API key

### Installation

```bash
# 1. Clone the repository
git clone https://github.com/feng138168/kraph.git
cd kraph

# 2. Install frontend dependencies
pnpm install

# 3. Run in development mode
pnpm tauri dev

# 4. Build a release binary
pnpm tauri build
```

---

## ⚙️ Configuration

On first launch, open **Settings** (⚙️ tab) to choose your AI provider:

### Option A — Local Ollama *(free, private)*

1. [Install Ollama](https://ollama.com/download) or use the in-app one-click setup button
2. Pull the recommended model:
   ```bash
   ollama pull qwen2.5:7b
   ```
3. In Settings, select **Local Ollama** and save

### Option B — DeepSeek API

1. Create an account at [platform.deepseek.com](https://platform.deepseek.com) and copy your API key
2. In Settings, select **DeepSeek**, paste the key, and save
3. Recommended model: `deepseek-chat`

### Option C — OpenAI or compatible API

1. Set your API key and base URL (supports Azure OpenAI, local proxies, etc.)
2. In Settings, select **OpenAI**, fill in the details, and save

---

## 📁 Data Storage

All data is stored locally in your OS app-data directory:

| Platform | Path |
|---|---|
| macOS | `~/Library/Application Support/me.kraph.app/` |
| Windows | `%APPDATA%\me.kraph.app\` |
| Linux | `~/.local/share/me.kraph.app/` |

Inside that directory:
- `database/kraph.db` — SQLite database (entities, relations, memories)
- `memories/YYYY/MM/` — one Markdown file per memory with YAML frontmatter
- `model_config.json` — saved model configuration

The Markdown files are plain text and portable — you can open them in any editor or sync them with your notes tool.

---

## 🏗️ Project Structure

```
kraph/
├── src/                        # Vue 3 frontend
│   ├── components/             # UI components
│   │   ├── InputPanel.vue      # Memory input + voice recording
│   │   ├── EditorPanel.vue     # Memory editor (markdown)
│   │   ├── GraphPanel.vue      # ECharts knowledge graph
│   │   ├── MemoryList.vue      # Sidebar memory list
│   │   ├── SearchPanel.vue     # Entity search + Q&A
│   │   ├── CharacterCard.vue   # Entity profile view
│   │   ├── ModelSettings.vue   # AI provider configuration
│   │   └── OllamaSetupDialog.vue  # One-click Ollama setup
│   ├── stores/                 # Pinia stores
│   ├── types/                  # TypeScript type definitions
│   ├── utils/tauriApi.ts       # Tauri command bindings
│   └── i18n/                   # Locale files (en-US, zh-CN)
└── src-tauri/                  # Rust backend
    └── src/
        ├── lib.rs              # Tauri command handlers
        ├── database.rs         # SQLite data layer
        ├── model_client.rs     # Generic LLM client (Ollama / OpenAI-compatible)
        ├── model_config.rs     # Config persistence
        ├── ollama.rs           # Ollama-specific integration + prompts
        ├── ollama_installer.rs # Auto-download Ollama installer
        ├── file_manager.rs     # Markdown file read/write
        └── whisper.rs          # Whisper speech-to-text
```

---

## 🤝 Contributing

Contributions, issues, and feature requests are welcome! Please open an issue first to discuss major changes.

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Commit your changes: `git commit -m 'feat: add my feature'`
4. Push to the branch: `git push origin feature/my-feature`
5. Open a Pull Request

---

## 📄 License

[MIT](LICENSE)

---

## 🙏 Acknowledgements

- [Ollama](https://ollama.com) — local LLM runtime
- [Tauri](https://tauri.app) — cross-platform desktop framework
- [Apache ECharts](https://echarts.apache.org) — graph visualization
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) — on-device speech recognition
