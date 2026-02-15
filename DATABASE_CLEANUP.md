# 数据库清理功能 - 修复脏数据问题

## 🐛 问题分析

### 错误信息
```
保存失败: FOREIGN KEY constraint failed
删除失败: FOREIGN KEY constraint failed
```

### 根本原因
数据库中存在**脏数据**：
1. `memory_entities` 表中引用了不存在的 `memory_id` 或 `entity_id`
2. `relations` 表中引用了不存在的 `entity_id`
3. 外键约束阻止了删除操作

### 为什么会产生脏数据？
- 之前的代码在删除时顺序错误（先删实体再删关系）
- 某些异常情况下的数据不一致
- 数据库迁移或手动修改导致的问题

## ✅ 解决方案

### 1. 修复删除顺序
修改了 `delete_memory` 和 `update_memory_content` 函数：
```
正确顺序：
1. 删除关系 → 2. 删除实体 → 3. 再检查关系
```

### 2. 添加数据库清理功能
新增 `cleanup_database` 函数，彻底清理所有脏数据：

#### 清理步骤
1. **清理 memory_entities 中引用不存在的 memory**
   ```sql
   DELETE FROM memory_entities 
   WHERE memory_id NOT IN (SELECT id FROM memories)
   ```

2. **清理 memory_entities 中引用不存在的 entity**
   ```sql
   DELETE FROM memory_entities 
   WHERE entity_id NOT IN (SELECT id FROM entities)
   ```

3. **清理 relations 中引用不存在的 entity**
   ```sql
   DELETE FROM relations 
   WHERE from_entity_id NOT IN (SELECT id FROM entities)
      OR to_entity_id NOT IN (SELECT id FROM entities)
   ```

4. **清理孤立的实体**
   ```sql
   DELETE FROM entities 
   WHERE id NOT IN (SELECT DISTINCT entity_id FROM memory_entities)
   ```

5. **再次清理 relations**
   ```sql
   DELETE FROM relations 
   WHERE from_entity_id NOT IN (SELECT id FROM entities)
      OR to_entity_id NOT IN (SELECT id FROM entities)
   ```

## 🚀 使用方法

### 方法 1: 使用界面按钮（推荐）
1. 重新启动应用
   ```bash
   npm run tauri dev
   ```

2. 在顶部标题栏找到 **🧹 清理数据库** 按钮

3. 点击按钮

4. 等待提示 "数据库清理完成"

5. 图谱会自动刷新

### 方法 2: 手动调用 API
```typescript
import { cleanupDatabase } from './utils/tauriApi'

try {
  const msg = await cleanupDatabase()
  console.log(msg) // "数据库清理完成"
} catch (e) {
  console.error('清理失败:', e)
}
```

## 📝 修改的文件

### 后端 (Rust)
1. **src-tauri/src/database.rs**
   - 添加 `cleanup_database()`: 全面清理脏数据
   - 修复 `delete_memory()`: 正确的删除顺序

2. **src-tauri/src/lib.rs**
   - 添加 `cleanup_db` 命令
   - 修复 `update_memory_content`: 正确的清理顺序
   - 注册新命令

### 前端 (TypeScript/Vue)
1. **src/utils/tauriApi.ts**
   - 添加 `cleanupDatabase()` API 函数

2. **src/App.vue**
   - 添加清理按钮
   - 添加 `onCleanupDatabase()` 方法
   - 添加按钮样式

## 🧪 测试步骤

### 1. 清理数据库
```bash
# 启动应用
npm run tauri dev

# 点击 "🧹 清理数据库" 按钮
# 应该显示 "数据库清理完成" ✅
```

### 2. 测试保存
```
选择一条记忆 → 修改内容 → 点击保存
应该显示 "保存成功" ✅
图谱自动更新 ✅
```

### 3. 测试删除
```
选择一条记忆 → 点击删除 → 确认
应该显示 "删除成功" ✅
图谱自动更新 ✅
```

## 📊 数据库结构说明

### 表关系图
```
memories (记忆)
    ↓ (1:N)
memory_entities (关联表)
    ↓ (N:1)
entities (实体)
    ↓ (1:N)
relations (关系)
    ↓ (N:1)
entities (实体)
```

### 外键约束
```sql
-- memory_entities 表
FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE
FOREIGN KEY (entity_id) REFERENCES entities(id) ON DELETE CASCADE

-- relations 表
FOREIGN KEY (from_entity_id) REFERENCES entities(id)
FOREIGN KEY (to_entity_id) REFERENCES entities(id)
```

## ⚠️ 注意事项

### 1. 清理是安全的
- 只删除脏数据和孤立记录
- 不会删除有效的记忆内容
- 可以放心使用

### 2. 什么时候需要清理？
- 出现外键约束错误时
- 发现图谱中有异常节点时
- 数据库迁移或升级后
- 定期维护（可选）

### 3. 清理后的影响
- 孤立的实体会被删除
- 断裂的关系会被清除
- 图谱会变得更整洁
- 不影响记忆内容本身

## 🔍 故障排查

### 如果清理后仍然出错

1. **检查 Ollama 状态**
   ```
   确保 Ollama 服务正在运行
   确保已下载 qwen2.5:1.5b 或 qwen2.5:7b 模型
   ```

2. **查看浏览器控制台**
   ```
   按 F12 打开开发者工具
   查看 Console 中的错误信息
   ```

3. **重置数据库（最后手段）**
   ```bash
   # 备份数据
   # 然后删除数据库文件重新开始
   rm ~/Library/Application\ Support/memoryai/database/memoryai.db
   ```

## 💡 最佳实践

1. **定期清理**
   - 建议每隔一段时间点击清理按钮
   - 保持数据库整洁

2. **编辑前清理**
   - 如果遇到保存错误，先清理再重试
   
3. **导入数据后清理**
   - 如果从其他来源导入数据，建议清理一次

## 📈 预期效果

清理前：
```
❌ 保存失败: FOREIGN KEY constraint failed
❌ 删除失败: FOREIGN KEY constraint failed
```

清理后：
```
✅ 数据库清理完成
✅ 保存成功
✅ 删除成功
✅ 图谱正常更新
```

现在数据库问题应该彻底解决了！
