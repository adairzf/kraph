# 清空数据指南

本文档说明如何清空 MemoryAI 应用中的所有数据。

## 方法一：使用应用内按钮（推荐）

### 1. 清理数据库（轻量级）
点击应用顶部的 **🧹 清理数据库** 按钮，会：
- 清理孤立的实体（不被任何记忆引用）
- 清理无效的关系（引用不存在的实体）
- 清理损坏的关联记录
- **不会删除有效的记忆和实体**

适用场景：
- 删除记忆后清理残留数据
- 修复数据不一致问题
- 定期维护数据库

### 2. 清空所有数据（危险操作）⚠️
点击应用顶部的 **⚠️ 清空数据** 按钮，会：
- **删除所有记忆**
- **删除所有实体和关系**
- **删除所有别名**
- **删除所有记忆文件**
- **重置自增ID**

⚠️ **警告**：此操作不可恢复！执行前会弹出二次确认对话框。

适用场景：
- 测试新功能前清空测试数据
- 重新开始使用应用
- 数据严重损坏需要重建

## 方法二：手动删除数据库文件

如果应用无法启动或出现严重问题，可以手动删除数据库文件：

### macOS
```bash
rm -rf ~/Library/Application\ Support/com.memoryai.app/database/memoryai.db
rm -rf ~/Library/Application\ Support/com.memoryai.app/memories/
```

### Windows
```powershell
Remove-Item -Path "$env:APPDATA\com.memoryai.app\database\memoryai.db" -Force
Remove-Item -Path "$env:APPDATA\com.memoryai.app\memories\" -Recurse -Force
```

### Linux
```bash
rm -rf ~/.config/com.memoryai.app/database/memoryai.db
rm -rf ~/.config/com.memoryai.app/memories/
```

重启应用后，会自动创建新的空数据库。

## 方法三：使用命令行工具

如果您有开发环境，可以使用以下命令：

```bash
# 进入项目目录
cd /path/to/memoryai

# 清空数据（需要应用正在运行）
# 通过 Tauri API 调用
npm run tauri dev
# 然后在开发者控制台执行：
# await window.__TAURI__.invoke('clear_all_data_cmd')
```

## 数据库结构说明

清空操作会删除以下表的所有数据：

1. **memories** - 记忆内容
2. **entities** - 实体（人物、地点、时间、事件）
3. **entity_aliases** - 实体别名
4. **relations** - 实体之间的关系
5. **memory_entities** - 记忆与实体的关联

## 清空后的状态

执行清空操作后：
- ✅ 数据库表结构保持完整
- ✅ 应用可以正常使用
- ✅ 自增ID会重置为1
- ✅ 记忆文件夹会被清空

首次添加新记忆时：
- 记忆ID从1开始
- 实体ID从1开始
- 所有功能正常工作

## 备份建议

在清空数据前，建议先备份：

### 1. 备份数据库
```bash
# macOS
cp ~/Library/Application\ Support/com.memoryai.app/database/memoryai.db ~/memoryai-backup-$(date +%Y%m%d).db

# Windows (PowerShell)
Copy-Item "$env:APPDATA\com.memoryai.app\database\memoryai.db" "$env:USERPROFILE\Desktop\memoryai-backup-$(Get-Date -Format 'yyyyMMdd').db"

# Linux
cp ~/.config/com.memoryai.app/database/memoryai.db ~/memoryai-backup-$(date +%Y%m%d).db
```

### 2. 备份记忆文件
```bash
# macOS
cp -r ~/Library/Application\ Support/com.memoryai.app/memories ~/memoryai-memories-backup-$(date +%Y%m%d)

# Windows (PowerShell)
Copy-Item -Recurse "$env:APPDATA\com.memoryai.app\memories" "$env:USERPROFILE\Desktop\memoryai-memories-backup-$(Get-Date -Format 'yyyyMMdd')"

# Linux
cp -r ~/.config/com.memoryai.app/memories ~/memoryai-memories-backup-$(date +%Y%m%d)
```

## 恢复备份

如果需要恢复备份的数据：

1. 关闭应用
2. 将备份的数据库文件复制回原位置
3. 将备份的 memories 文件夹复制回原位置
4. 重启应用

```bash
# macOS 恢复示例
cp ~/memoryai-backup-20260215.db ~/Library/Application\ Support/com.memoryai.app/database/memoryai.db
cp -r ~/memoryai-memories-backup-20260215 ~/Library/Application\ Support/com.memoryai.app/memories
```

## 常见问题

### Q: 清空数据后，是否需要重新安装 Ollama 模型？
A: 不需要。Ollama 模型存储在独立位置，清空应用数据不影响模型。

### Q: 清空数据是否会影响应用设置？
A: 不会。应用配置存储在独立位置，只有记忆数据会被清空。

### Q: 误操作清空了数据，能恢复吗？
A: 如果没有提前备份，数据无法恢复。建议在清空前务必确认。

### Q: 清理数据库和清空数据有什么区别？
A: 
- **清理数据库**：只删除孤立和无效数据，保留有效记忆
- **清空数据**：删除所有数据，相当于重新开始

### Q: 清空数据后知识图谱为空，是正常的吗？
A: 是的，这是正常现象。添加新记忆后图谱会重新构建。

## 技术实现

### 后端函数（Rust）

```rust
// 清空所有数据
pub fn clear_all_data(conn: &Connection) -> SqliteResult<()> {
    conn.execute("PRAGMA foreign_keys = OFF", [])?;
    
    // 删除所有表的数据
    conn.execute("DELETE FROM memory_entities", [])?;
    conn.execute("DELETE FROM relations", [])?;
    conn.execute("DELETE FROM entity_aliases", [])?;
    conn.execute("DELETE FROM memories", [])?;
    conn.execute("DELETE FROM entities", [])?;
    
    // 重置自增ID
    conn.execute("DELETE FROM sqlite_sequence", [])?;
    
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    
    Ok(())
}
```

### 前端调用（TypeScript）

```typescript
export async function clearAllData(): Promise<string> {
  return invoke('clear_all_data_cmd')
}
```

---

创建时间：2026-02-15
更新时间：2026-02-15
