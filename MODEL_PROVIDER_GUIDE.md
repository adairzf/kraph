# MemoryAI 模型配置更新

## 🎉 新功能

现在支持三种模型提供商：
1. **本地 Ollama**（完全免费，数据本地化）
2. **DeepSeek API**（国内可访问，价格便宜）
3. **OpenAI API**（效果最好，支持 GPT-4 等）

## 📦 新增文件

### 后端
- `src-tauri/src/model_config.rs` - 模型配置结构体
- `src-tauri/src/model_client.rs` - 统一的模型调用接口

### 前端
- `src/types/model-config.ts` - TypeScript 类型定义
- `src/components/ModelSettings.vue` - 设置界面组件

## 🔧 配置说明

### 1. 本地 Ollama（推荐新手）

**优点**：
- ✅ 完全免费
- ✅ 数据本地化，隐私安全
- ✅ 无需网络即可使用

**缺点**：
- ⚠️ 需要一定的显存（7b约8GB，14b约16GB）
- ⚠️ 首次需要下载模型

**配置示例**：
```json
{
  "provider": {
    "type": "ollama",
    "base_url": "http://localhost:11434",
    "model_name": "qwen2.5:7b",
    "extract_model_name": "qwen2.5:7b"
  },
  "temperature": 0.2,
  "max_tokens": 4096
}
```

**快速开始**：
```bash
# 1. 安装 Ollama
# Mac: brew install ollama
# Windows/Linux: 访问 https://ollama.com/download

# 2. 启动 Ollama 服务
ollama serve

# 3. 下载推荐模型
ollama pull qwen2.5:7b

# 4. （可选）下载更大的模型以获得更好的效果
ollama pull qwen2.5:14b
```

### 2. DeepSeek API（推荐日常使用）

**优点**：
- ✅ 国内可直接访问，无需科学上网
- ✅ 价格超便宜（1M tokens 约 ¥1）
- ✅ 速度快，效果好
- ✅ 无需本地显存

**缺点**：
- ⚠️ 需要付费（但很便宜）
- ⚠️ 数据会上传到云端

**配置示例**：
```json
{
  "provider": {
    "type": "deepseek",
    "api_key": "sk-xxxxxxxxxxxxxxxx",
    "base_url": "https://api.deepseek.com/v1",
    "model_name": "deepseek-chat"
  },
  "temperature": 0.2,
  "max_tokens": 4096
}
```

**快速开始**：
```bash
# 1. 注册 DeepSeek 账号
访问：https://platform.deepseek.com

# 2. 充值（最低¥10起）
进入控制台 -> 余额充值

# 3. 创建 API Key
进入控制台 -> API Keys -> 创建新的 Key

# 4. 在应用中配置
复制 API Key 到设置界面
```

**推荐模型**：
- `deepseek-chat`：通用对话模型，速度快
- `deepseek-reasoner`：推理模型，逻辑能力强，适合复杂任务

### 3. OpenAI API（追求极致）

**优点**：
- ✅ 效果最好（GPT-4）
- ✅ 生态成熟，稳定可靠
- ✅ 支持多种模型选择

**缺点**：
- ⚠️ 需要科学上网
- ⚠️ 价格较贵（gpt-4 约 $30/1M tokens）
- ⚠️ 国内访问不稳定

**配置示例**：
```json
{
  "provider": {
    "type": "openai",
    "api_key": "sk-xxxxxxxxxxxxxxxx",
    "base_url": "https://api.openai.com/v1",
    "model_name": "gpt-4"
  },
  "temperature": 0.2,
  "max_tokens": 4096
}
```

**兼容性说明**：
此配置也支持兼容 OpenAI 格式的其他 API，例如：
- Azure OpenAI
- 各种国内中转 API
- 本地部署的兼容接口

## 🎯 推荐配置方案

### 方案 A：完全免费（新手推荐）
```
提供商：本地 Ollama
模型：qwen2.5:7b
显存需求：8GB
速度：较快
准确度：★★★★☆
成本：免费
```

### 方案 B：性价比最高（日常推荐）
```
提供商：DeepSeek API
模型：deepseek-chat
显存需求：无
速度：很快
准确度：★★★★★
成本：约 ¥0.001/次提取
```

### 方案 C：极致效果（专业用户）
```
提供商：本地 Ollama
模型：qwen2.5:14b 或 qwen2.5:32b
显存需求：16GB/32GB
速度：较慢
准确度：★★★★★
成本：免费
```

### 方案 D：混合方案（最佳实践）
```
日常提取：DeepSeek API（快速便宜）
重要记忆：本地 Ollama 14b（准确本地）
简单问答：DeepSeek chat
复杂推理：DeepSeek reasoner
```

## 💡 使用建议

### 1. Temperature 参数设置
- `0.0-0.3`：确定性高，适合实体提取（推荐 0.2）
- `0.4-0.7`：平衡创造性和准确性，适合对话
- `0.8-1.0`：创造性高，不推荐用于知识提取

### 2. Max Tokens 设置
- **短文本**（<500字）：2048 tokens
- **中等文本**（500-2000字）：4096 tokens（推荐）
- **长文本**（>2000字）：6144-8192 tokens

### 3. 模型选择建议

**实体提取任务**（需要准确性）：
- 最低：qwen2.5:7b 或 deepseek-chat
- 推荐：qwen2.5:14b 或 deepseek-reasoner
- 最佳：qwen2.5:32b 或 gpt-4

**问答任务**（需要流畅性）：
- 最低：qwen2.5:7b
- 推荐：deepseek-chat
- 最佳：gpt-4

## 🔄 迁移指南

### 从旧版本升级

如果你之前使用的是硬编码的 Ollama 配置，升级后：

1. **自动迁移**：首次启动会自动创建默认配置（Ollama + qwen2.5:7b）
2. **配置文件位置**：`<app_data_dir>/model_config.json`
3. **无需手动修改代码**：所有配置通过UI界面管理

### 切换到 DeepSeek

1. 打开设置界面
2. 选择"DeepSeek API"
3. 填写 API Key
4. 点击"测试连接"验证
5. 点击"保存配置"

## 🐛 常见问题

### Q: DeepSeek API 测试失败？
A: 检查：
1. API Key 是否正确
2. 账户余额是否充足
3. 网络连接是否正常

### Q: Ollama 连接失败？
A: 检查：
1. Ollama 服务是否启动（`ollama serve`）
2. 端口是否被占用
3. 模型是否已下载（`ollama list`）

### Q: 实体提取效果不好？
A: 尝试：
1. 升级到更大的模型（7b → 14b → 32b）
2. 降低 temperature（建议 0.2）
3. 切换到 DeepSeek reasoner 模型

### Q: 速度太慢？
A: 优化：
1. 如果用 Ollama，确保使用GPU（需CUDA或Metal）
2. 切换到 DeepSeek API（云端推理更快）
3. 降低 max_tokens 设置

### Q: 如何查看配置文件？
A: 配置文件位于：
- **Mac**: `~/Library/Application Support/com.memoryai.app/model_config.json`
- **Windows**: `%APPDATA%\com.memoryai.app\model_config.json`
- **Linux**: `~/.local/share/com.memoryai.app/model_config.json`

## 📊 成本估算

### DeepSeek 价格（2024年价格）
- **deepseek-chat**: ¥1/1M tokens
- **deepseek-reasoner**: ¥2/1M tokens

**实际使用成本**：
- 每次实体提取：约 500-2000 tokens（¥0.0005-0.002）
- 每次问答：约 300-1000 tokens（¥0.0003-0.001）
- 每月使用 100 次：约 ¥0.05-0.2

### Ollama 成本
- **硬件成本**：需要8-32GB显存的GPU（一次性投入）
- **电费**：约 0.5-1元/小时（使用时）
- **软件成本**：免费

### OpenAI 价格（2024年价格）
- **GPT-4 Turbo**: $10/1M input tokens, $30/1M output tokens
- **GPT-3.5 Turbo**: $0.5/1M input tokens, $1.5/1M output tokens

## 🔐 安全性说明

### 数据隐私
- **Ollama**：所有数据本地处理，不上传云端
- **DeepSeek/OpenAI**：数据会上传到云端进行处理

### API Key 安全
- 配置文件存储在本地，不会上传
- 建议定期轮换 API Key
- 不要将配置文件分享给他人

### 敏感信息处理
如果处理敏感信息，推荐：
1. 使用本地 Ollama
2. 或者在上传前脱敏
3. 定期清理云端对话记录

## 📞 技术支持

如有问题，请：
1. 查看本文档的"常见问题"部分
2. 查看 `MODEL_CONFIG.md` 了解详细的模型对比
3. 提交 GitHub Issue

---

**更新日期**: 2026-02-15
**版本**: v2.0.0
