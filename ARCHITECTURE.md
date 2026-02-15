# 知识融合技术架构

## 系统架构图

```
┌─────────────────────────────────────────────────────────────┐
│                         用户输入                              │
│                  "我二哥在字节上班"                           │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                    快速实体提取                               │
│              (qwen2.5:1.5b - 轻量级)                         │
│  输出: [Person: 我二哥], [Organization: 字节]                │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                  查询历史记忆                                 │
│   根据提取的实体名，查找相关的历史记忆                         │
│   - 查询"我二哥"相关记忆 → 找到"李明是我二哥"                 │
│   - 查询"李明"相关记忆 → 找到"李明是我的同事"                 │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   知识融合推理                                │
│              (qwen2.5:7b - 强推理能力)                        │
│                                                              │
│  输入:                                                        │
│    历史: ["李明是我的同事", "李明是我二哥"]                   │
│    新增: "我二哥在字节上班"                                   │
│                                                              │
│  LLM分析:                                                    │
│    1. 实体识别: 李明 = 我二哥 (同一人)                        │
│    2. 关系推导: 我二哥→字节 ⟹ 李明→字节                      │
│                                                              │
│  输出 JSON:                                                  │
│    entities: [李明, 字节]                                    │
│    aliases: [{primary: "李明", alias: "我二哥"}]             │
│    relations: [{from: "李明", to: "字节", ...}]              │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   数据库更新                                  │
│                                                              │
│  1. 实体处理                                                  │
│     - 查找"李明" → 已存在(id=1)                               │
│     - 查找"我二哥" → 通过别名找到李明(id=1)                   │
│     - 查找"字节" → 新建(id=2)                                 │
│                                                              │
│  2. 别名管理                                                  │
│     INSERT INTO entity_aliases                               │
│       (entity_id=1, alias="我二哥")                          │
│                                                              │
│  3. 关系建立                                                  │
│     INSERT INTO relations                                    │
│       (from=1, to=2, type="在...上班")                       │
│                                                              │
│  4. 记忆关联                                                  │
│     INSERT INTO memory_entities                              │
│       (memory_id=3, entity_id=1)                            │
│       (memory_id=3, entity_id=2)                            │
└─────────────────────────────────────────────────────────────┘
```

## 数据库关系图

```
┌──────────────┐         ┌──────────────────┐
│   entities   │◄────────│ entity_aliases   │
│              │         │                  │
│ id (PK)      │         │ entity_id (FK)   │
│ type         │         │ alias (UNIQUE)   │
│ name         │         └──────────────────┘
│ attributes   │
└──────┬───────┘
       │                  ┌──────────────────┐
       │                  │    relations     │
       │◄─────────────────│                  │
       │                  │ from_entity_id   │
       │◄─────────────────│ to_entity_id     │
       │                  │ relation_type    │
       │                  │ strength         │
       │                  └──────────────────┘
       │
       │                  ┌──────────────────┐
       │                  │ memory_entities  │
       └──────────────────┤                  │
                          │ memory_id (FK)   │
                          │ entity_id (FK)   │
                          └────────┬─────────┘
                                   │
                          ┌────────▼─────────┐
                          │    memories      │
                          │                  │
                          │ id (PK)          │
                          │ content          │
                          │ md_file_path     │
                          │ created_at       │
                          │ tags             │
                          └──────────────────┘
```

## 关键数据流

### 1. 实体查找流程
```
输入: "我二哥"
  │
  ├─► 直接查询 entities.name → 未找到
  │
  └─► 查询 entity_aliases.alias → 找到 entity_id=1
      │
      └─► 返回 entities[id=1] = "李明"
```

### 2. 别名合并流程
```
场景: primary="李明"(id=1), alias="我二哥"(id=5)

1. 检测到需要合并
   │
2. 转移别名
   └─► UPDATE entity_aliases SET entity_id=1 WHERE entity_id=5
   
3. 转移记忆关联
   └─► UPDATE memory_entities SET entity_id=1 WHERE entity_id=5
   
4. 转移关系(from)
   └─► UPDATE relations SET from_entity_id=1 WHERE from_entity_id=5
   
5. 转移关系(to)
   └─► UPDATE relations SET to_entity_id=1 WHERE to_entity_id=5
   
6. 添加别名
   └─► INSERT INTO entity_aliases (entity_id=1, alias="我二哥")
   
7. 删除旧实体
   └─► DELETE FROM entities WHERE id=5
```

### 3. 知识融合决策树

```
新记忆输入
  │
  ├─ 是否有相关历史? 
  │   ├─ 否 → 简单提取 (qwen2.5:1.5b)
  │   └─ 是 ↓
  │
  ├─ 调用知识融合 (qwen2.5:7b)
  │   │
  │   ├─ 成功? 
  │   │   ├─ 是 → 使用融合结果
  │   │   └─ 否 → 回退到简单提取
  │   │
  │   └─ 输出: {entities, aliases, relations}
  │
  └─ 数据库更新
      ├─ 创建/查找实体
      ├─ 处理别名合并
      └─ 建立关系
```

## 性能优化策略

1. **两阶段提取**
   - 第一阶段：轻量模型快速提取（1.5b）
   - 第二阶段：强模型深度推理（7b）
   - 避免所有请求都用大模型

2. **历史记忆限制**
   - 每个实体最多取5条历史记忆
   - 避免prompt过长影响性能

3. **缓存与复用**
   - 实体ID映射缓存在内存
   - 减少重复数据库查询

4. **索引优化**
   ```sql
   CREATE INDEX idx_entity_aliases_alias ON entity_aliases(alias);
   CREATE INDEX idx_entity_aliases_entity ON entity_aliases(entity_id);
   ```

## 容错机制

```
知识融合调用
  │
  ├─ Ollama未运行? → 尝试自动启动
  │
  ├─ 模型未下载? → 自动pull模型
  │
  ├─ 知识融合失败? → 回退到简单提取
  │
  ├─ 简单提取失败? → 仅保存文本，不提取实体
  │
  └─ 数据库操作失败? → 返回错误，不影响历史数据
```

## 扩展点

### 1. 置信度评分
```rust
struct InferredRelation {
    from: String,
    to: String,
    relation: String,
    confidence: f32,  // 0.0 - 1.0
    source: RelationSource,  // Direct | Inferred
}
```

### 2. 冲突检测
```rust
fn detect_conflicts(conn: &Connection) -> Vec<Conflict> {
    // 检测矛盾关系
    // 例: A→B: 父亲 vs A→B: 儿子
}
```

### 3. 图遍历查询
```sql
-- 查找2-3跳关系
WITH RECURSIVE path AS (...)
SELECT * FROM path WHERE depth <= 3;
```

---

创建时间：2026-02-15
