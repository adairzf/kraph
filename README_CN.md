# Kraph — 个人知识图谱

**中文** | [English](./README.md)

> 一款本地优先的桌面应用，由大语言模型驱动，将你的笔记转化为活跃的知识图谱。

Kraph 自动从你输入的文字中提取人物、地点、事件、时间等实体及其关系，构建成可视化的知识网络，支持自然语言探索与问答——所有数据运行在你的设备上，完全私密。

---

## ✨ 功能特性

- **自动实体提取** — 粘贴或输入任意文字，自动识别人物、地点、事件等实体，无需手动标注
- **知识融合推理** — 新增记忆时，自动与历史记录关联融合，"我二哥"和"李明"会被识别为同一人
- **交互式知识图谱** — 力导向图谱，节点可拖拽、可缩放，点击任意节点查看完整档案与关联记忆
- **实体档案** — 点击节点查看完整档案：属性、关系、所有关联记忆
- **自然语言问答** — 直接问"Alice 是谁？"，AI 从你的记忆中精准作答
- **Markdown 存储** — 每条记忆以 `.md` 文件保存，带 YAML frontmatter，永远人类可读
- **语音输入** — 集成本地 Whisper 语音识别，说话即可录入，无需任何云端服务（macOS）
- **中英文双语界面** — 支持简体中文与 English 实时切换，无需重启
- **多 AI 后端** — 支持本地 Ollama、DeepSeek API 或任意 OpenAI 兼容接口

---

## 🖥️ 技术栈

| 层级 | 技术 |
|------|------|
| 桌面运行时 | [Tauri 2](https://tauri.app)（Rust + WebView） |
| 前端 | [Vue 3](https://vuejs.org) + TypeScript + [Vite](https://vitejs.dev) |
| UI 组件库 | [Element Plus](https://element-plus.org) |
| 图表 | [Apache ECharts](https://echarts.apache.org) via vue-echarts |
| 状态管理 | [Pinia](https://pinia.vuejs.org) |
| 国际化 | [Vue I18n](https://vue-i18n.intlify.dev) |
| 数据库 | SQLite（via [rusqlite](https://github.com/rusqlite/rusqlite)） |
| AI（本地） | [Ollama](https://ollama.com) |
| AI（云端） | DeepSeek API / OpenAI API |
| 语音识别 | [whisper.cpp](https://github.com/ggerganov/whisper.cpp) |

---

## 🚀 快速开始

### 环境准备

- [Node.js](https://nodejs.org) ≥ 18 以及 [pnpm](https://pnpm.io)（或 npm）
- [Rust 工具链](https://rustup.rs)（stable）
- [Tauri CLI 依赖](https://tauri.app/start/prerequisites/)（根据你的操作系统）

AI 功能至少需要以下**一种**：
- 本地安装 [Ollama](https://ollama.com)（推荐，完全私密）
- [DeepSeek](https://platform.deepseek.com) 或 [OpenAI](https://platform.openai.com) API Key

### 安装步骤

```bash
# 1. 克隆仓库
git clone https://github.com/feng138168/kraph.git
cd kraph

# 2. 安装前端依赖
pnpm install

# 3. 开发模式运行
pnpm tauri dev

# 4. 构建发布包
pnpm tauri build
```

---

## ⚙️ AI 后端配置

首次启动后，点击 **设置（⚙️）** 标签页选择 AI 提供商：

### 方案 A — DeepSeek API（推荐）

性价比最高，无本地硬件要求，中文理解出色，API 价格极低。

1. 前往 [platform.deepseek.com](https://platform.deepseek.com) 注册并获取 API Key
2. 在设置中选择 **DeepSeek**，粘贴 Key 并保存
3. 推荐模型：`deepseek-chat`

### 方案 B — 本地 Ollama（完全私密）

> ⚠️ **硬件要求**：运行 qwen2.5:7b 至少需要 16GB 内存和较新的 GPU 或 Apple Silicon。低配设备上实体理解可能不完整。

1. [安装 Ollama](https://ollama.com/download) 或使用应用内一键安装按钮
2. 拉取推荐模型：
   ```bash
   ollama pull qwen2.5:7b
   ```
3. 在设置中选择 **本地 Ollama** 并保存

### 方案 C — OpenAI 兼容接口

1. 填写 API Key 和 Base URL（支持 Azure OpenAI、本地代理等）
2. 在设置中选择 **OpenAI**，填写信息并保存

---

## 📁 数据存储

所有数据存储在本地操作系统的应用数据目录：

| 平台 | 路径 |
|------|------|
| macOS | `~/Library/Application Support/me.kraph.app/` |
| Windows | `%APPDATA%\me.kraph.app\` |
| Linux | `~/.local/share/me.kraph.app/` |

目录内容：
- `database/kraph.db` — SQLite 数据库（实体、关系、记忆）
- `memories/YYYY/MM/` — 每条记忆对应一个 Markdown 文件（含 YAML frontmatter）
- `model_config.json` — 已保存的模型配置

Markdown 文件是纯文本格式，可在任意编辑器打开，也可以同步到你的笔记工具中。

---

## 🏗️ 项目结构

```
kraph/
├── src/                        # Vue 3 前端
│   ├── components/             # UI 组件
│   │   ├── InputPanel.vue      # 记忆输入 + 语音录制
│   │   ├── EditorPanel.vue     # 记忆编辑器（Markdown）
│   │   ├── GraphPanel.vue      # ECharts 知识图谱
│   │   ├── MemoryList.vue      # 侧边栏记忆列表
│   │   ├── SearchPanel.vue     # 实体搜索 + 问答
│   │   ├── CharacterCard.vue   # 实体档案视图
│   │   ├── ModelSettings.vue   # AI 提供商配置
│   │   └── OllamaSetupDialog.vue  # 一键安装 Ollama
│   ├── stores/                 # Pinia 状态管理
│   ├── types/                  # TypeScript 类型定义
│   ├── utils/tauriApi.ts       # Tauri 命令绑定
│   └── i18n/                   # 语言包（en-US, zh-CN）
└── src-tauri/                  # Rust 后端
    └── src/
        ├── lib.rs              # Tauri 命令处理器
        ├── database.rs         # SQLite 数据层
        ├── model_client.rs     # 通用 LLM 客户端（Ollama / OpenAI 兼容）
        ├── model_config.rs     # 配置持久化
        ├── ollama.rs           # Ollama 集成 + 提示词
        ├── ollama_installer.rs # 自动下载 Ollama 安装包
        ├── file_manager.rs     # Markdown 文件读写
        └── whisper.rs          # Whisper 语音识别
```

---

## 🤝 参与贡献

欢迎提交 Issue 和 Pull Request！重大改动请先开 Issue 讨论。

1. Fork 本仓库
2. 创建功能分支：`git checkout -b feature/my-feature`
3. 提交改动：`git commit -m 'feat: add my feature'`
4. 推送分支：`git push origin feature/my-feature`
5. 发起 Pull Request

---

## 📄 许可证

[MIT](LICENSE)

---

## 🙏 致谢

- [Ollama](https://ollama.com) — 本地 LLM 运行时
- [Tauri](https://tauri.app) — 跨平台桌面框架
- [Apache ECharts](https://echarts.apache.org) — 图谱可视化
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) — 设备端语音识别
