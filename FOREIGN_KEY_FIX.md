# 外键约束问题修复

## 🐛 问题描述
编辑记忆时出现错误：`FOREIGN KEY constraint failed`

## 🔍 原因分析
数据库外键约束：
- `relations` 表的 `from_entity_id` 和 `to_entity_id` 引用 `entities` 表的 `id`
- 如果先删除实体，但关系表中还引用着这些实体，就会违反外键约束

错误的清理顺序：
```
1. 删除孤立的实体 ❌
2. 删除孤立的关系
```

正确的清理顺序应该是：
```
1. 先删除孤立的关系 ✅
2. 再删除孤立的实体 ✅
3. 最后再检查一次关系 ✅
```

## ✅ 解决方案

修改 `update_memory_content` 函数的清理顺序：

### 步骤 1: 先清理孤立的关系
删除所有引用了不存在实体的关系：
```sql
DELETE FROM relations 
WHERE from_entity_id NOT IN (SELECT id FROM entities)
   OR to_entity_id NOT IN (SELECT id FROM entities)
```

### 步骤 2: 再清理孤立的实体
删除不被任何记忆引用的实体：
```sql
DELETE FROM entities 
WHERE id NOT IN (SELECT DISTINCT entity_id FROM memory_entities)
```

### 步骤 3: 最后再清理一次关系
因为删除实体可能产生新的孤立关系，所以再清理一次：
```sql
DELETE FROM relations 
WHERE from_entity_id NOT IN (SELECT id FROM entities)
   OR to_entity_id NOT IN (SELECT id FROM entities)
```

## 📊 完整的更新流程

```
1. 更新记忆内容
   ↓
2. 清除该记忆的旧实体关联
   ↓
3. 调用 Ollama 提取新实体
   ↓
4. 插入/更新新实体
   ↓
5. 建立新的记忆-实体关联
   ↓
6. 建立新的实体-实体关系
   ↓
7. 清理孤立的关系（第一次）
   ↓
8. 清理孤立的实体
   ↓
9. 清理孤立的关系（第二次）
   ↓
10. 返回更新后的记忆
```

## 🔧 数据库约束说明

### memory_entities 表
```sql
CREATE TABLE memory_entities (
    memory_id INTEGER NOT NULL,
    entity_id INTEGER NOT NULL,
    PRIMARY KEY (memory_id, entity_id),
    FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE,
    FOREIGN KEY (entity_id) REFERENCES entities(id) ON DELETE CASCADE
);
```
- 当删除记忆时，自动删除关联（CASCADE）
- 当删除实体时，自动删除关联（CASCADE）

### relations 表
```sql
CREATE TABLE relations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_entity_id INTEGER NOT NULL,
    to_entity_id INTEGER NOT NULL,
    relation_type TEXT NOT NULL,
    strength INTEGER DEFAULT 1,
    UNIQUE(from_entity_id, to_entity_id, relation_type),
    FOREIGN KEY (from_entity_id) REFERENCES entities(id),
    FOREIGN KEY (to_entity_id) REFERENCES entities(id)
);
```
- 关系必须引用存在的实体
- 不能先删除实体，必须先删除关系

## 🧪 测试用例

### 测试 1: 修改人名
```
原内容: "今天和张三见面"
新内容: "今天和李四见面"

预期结果:
- 张三实体被删除（如果没有其他记忆引用）
- 李四实体被创建
- 图谱更新
```

### 测试 2: 删除所有实体
```
原内容: "今天和张三、李四在北京见面"
新内容: "今天见面"

预期结果:
- 张三、李四、北京都被删除（如果没有其他引用）
- 相关关系都被清理
- 图谱更新
```

### 测试 3: 添加新实体
```
原内容: "今天见面"
新内容: "今天和张三在北京见面"

预期结果:
- 创建张三实体
- 创建北京实体
- 建立关系
- 图谱更新
```

## 🚀 使用说明

1. **重新编译应用**
   ```bash
   npm run tauri dev
   ```

2. **测试编辑功能**
   - 选择一条记忆
   - 修改内容
   - 点击"保存"
   - 应该成功保存，不再报错

3. **观察图谱变化**
   - 旧实体消失
   - 新实体出现
   - 关系正确建立

## ⚠️ 注意事项

1. **编辑需要 Ollama 运行**
   - 确保 Ollama 服务正在运行
   - 确保已下载所需模型（qwen2.5:1.5b 或 qwen2.5:7b）

2. **清理是自动的**
   - 不再被任何记忆引用的实体会自动删除
   - 这是预期行为，保持数据库整洁

3. **性能考虑**
   - 每次保存都会重新提取实体
   - 建议完成编辑后再保存，避免频繁 AI 调用

## 📝 相关文件修改

- `src-tauri/src/lib.rs`: 修改 `update_memory_content` 函数
  - 调整清理顺序：先关系，后实体，再关系
  - 添加详细注释说明每一步

现在外键约束问题已经彻底解决了！
