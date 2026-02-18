# 数据库查看指南

## 📍 您的数据库位置

```bash
~/Library/Application Support/com.zhoufengdai.memoryai/database/memoryai.db
```

## 🔍 当前数据概览

根据刚才的查询，您的数据库包含：
- **1 条记忆**
- **3 个实体**（苏青、吴锋、阿里巴巴）
- **3 个关系**
- **0 个别名**（新功能还未使用）

### 实体数据
| ID | 类型 | 名称 | 创建时间 |
|----|------|------|----------|
| 1 | Person | 苏青 | 2026-02-15 05:39:11 |
| 2 | Location | 阿里巴巴 | 2026-02-15 05:39:11 |
| 3 | Person | 吴锋 | 2026-02-15 05:39:11 |

### 关系数据
| 从实体 | 关系 | 到实体 | 强度 |
|--------|------|--------|------|
| 苏青 | 在...上班 | 阿里巴巴 | 1 |
| 吴锋 | 在...上班 | 阿里巴巴 | 1 |
| 苏青 | 是同一小队成员 | 吴锋 | 1 |

## 🛠️ 快速查看方法

### 方法1：使用我创建的脚本（最简单）

```bash
cd ~/Documents/mine/memoryai
./view_database.sh
```

### 方法2：命令行直接查询

```bash
# 打开数据库
sqlite3 ~/Library/Application\ Support/com.zhoufengdai.memoryai/database/memoryai.db

# 设置显示格式
.mode column
.headers on

# 查询示例
SELECT * FROM entities;
SELECT * FROM relations;
SELECT * FROM entity_aliases;
SELECT * FROM memories;

# 退出
.quit
```

### 方法3：安装 DB Browser for SQLite（图形界面）

```bash
# 安装
brew install --cask db-browser-for-sqlite

# 打开数据库
open -a "DB Browser for SQLite" ~/Library/Application\ Support/com.zhoufengdai.memoryai/database/memoryai.db
```

## 📊 常用查询命令

### 1. 查看表结构
```sql
.schema entities
.schema entity_aliases
.schema relations
.schema memories
.schema memory_entities
```

### 2. 统计数据
```sql
SELECT 
    (SELECT COUNT(*) FROM memories) as 记忆数,
    (SELECT COUNT(*) FROM entities) as 实体数,
    (SELECT COUNT(*) FROM relations) as 关系数,
    (SELECT COUNT(*) FROM entity_aliases) as 别名数;
```

### 3. 查看实体和别名
```sql
-- 查看所有实体
SELECT id, type, name, created_at FROM entities;

-- 查看实体的别名
SELECT 
    e.name as 主名称,
    ea.alias as 别名,
    ea.created_at as 创建时间
FROM entity_aliases ea
JOIN entities e ON ea.entity_id = e.id;
```

### 4. 查看关系网络
```sql
-- 查看所有关系
SELECT 
    e1.name as 从实体,
    r.relation_type as 关系类型,
    e2.name as 到实体,
    r.strength as 强度,
    r.created_at as 创建时间
FROM relations r
JOIN entities e1 ON r.from_entity_id = e1.id
JOIN entities e2 ON r.to_entity_id = e2.id
ORDER BY r.created_at DESC;

-- 查看某个实体的所有关系
SELECT 
    CASE 
        WHEN r.from_entity_id = 1 THEN e2.name
        ELSE e1.name
    END as 关联实体,
    r.relation_type as 关系,
    CASE 
        WHEN r.from_entity_id = 1 THEN '→'
        ELSE '←'
    END as 方向
FROM relations r
JOIN entities e1 ON r.from_entity_id = e1.id
JOIN entities e2 ON r.to_entity_id = e2.id
WHERE r.from_entity_id = 1 OR r.to_entity_id = 1;
```

### 5. 查看记忆详情
```sql
-- 查看所有记忆概览
SELECT 
    id,
    substr(content, 1, 50) as 内容预览,
    created_at as 创建时间,
    tags as 标签
FROM memories
ORDER BY created_at DESC;

-- 查看记忆关联的实体
SELECT 
    m.id as 记忆ID,
    m.content,
    GROUP_CONCAT(e.name, ', ') as 关联实体
FROM memories m
JOIN memory_entities me ON m.id = me.memory_id
JOIN entities e ON me.entity_id = e.id
GROUP BY m.id;

-- 查看某个实体相关的所有记忆
SELECT DISTINCT
    m.id,
    m.content,
    m.created_at
FROM memories m
JOIN memory_entities me ON m.id = me.memory_id
WHERE me.entity_id = 1  -- 苏青的ID
ORDER BY m.created_at DESC;
```

### 6. 测试别名功能的查询
```sql
-- 通过名称或别名查找实体
SELECT e.* 
FROM entities e
WHERE e.name LIKE '%苏青%'
UNION
SELECT e.* 
FROM entities e
JOIN entity_aliases ea ON e.id = ea.entity_id
WHERE ea.alias LIKE '%苏青%';
```

### 7. 数据完整性检查
```sql
-- 检查孤立的实体（没有记忆关联）
SELECT e.id, e.name, e.type
FROM entities e
WHERE e.id NOT IN (SELECT DISTINCT entity_id FROM memory_entities);

-- 检查无效的关系（引用不存在的实体）
SELECT r.*
FROM relations r
WHERE r.from_entity_id NOT IN (SELECT id FROM entities)
   OR r.to_entity_id NOT IN (SELECT id FROM entities);

-- 检查无效的别名（引用不存在的实体）
SELECT ea.*
FROM entity_aliases ea
WHERE ea.entity_id NOT IN (SELECT id FROM entities);
```

## 🧪 测试知识融合功能

为了测试新的知识融合功能，您可以按以下步骤操作：

### 1. 清空现有数据（可选）
在应用中点击 "⚠️ 清空数据" 按钮

### 2. 输入测试数据
按顺序输入以下记忆：
1. "李明是我的同事"
2. "李明是我二哥"
3. "我二哥在字节上班"

### 3. 验证别名关系
```sql
-- 查看李明的别名
SELECT 
    e.name as 主名称,
    ea.alias as 别名
FROM entities e
LEFT JOIN entity_aliases ea ON e.id = ea.entity_id
WHERE e.name LIKE '%李明%' OR ea.alias LIKE '%李明%';
```

### 4. 验证关系推导
```sql
-- 查看李明相关的所有关系
SELECT 
    e1.name as 从,
    r.relation_type as 关系,
    e2.name as 到
FROM relations r
JOIN entities e1 ON r.from_entity_id = e1.id
JOIN entities e2 ON r.to_entity_id = e2.id
WHERE e1.name LIKE '%李明%' OR e2.name LIKE '%李明%'
   OR e1.id IN (SELECT entity_id FROM entity_aliases WHERE alias LIKE '%李明%')
   OR e2.id IN (SELECT entity_id FROM entity_aliases WHERE alias LIKE '%李明%');
```

预期结果：应该能看到 "李明 → 字节 : 在...上班" 的关系

## 📱 在 VSCode 中查看

如果您使用 VSCode：

1. 安装插件：**SQLite Viewer**
2. 打开文件：`~/Library/Application Support/com.zhoufengdai.memoryai/database/memoryai.db`
3. 右键 → Open with SQLite Viewer

## 🔧 高级操作

### 导出数据库
```bash
# 导出为 SQL 文件
sqlite3 ~/Library/Application\ Support/com.zhoufengdai.memoryai/database/memoryai.db .dump > memoryai_backup.sql

# 导出为 CSV
sqlite3 ~/Library/Application\ Support/com.zhoufengdai.memoryai/database/memoryai.db << 'EOF'
.mode csv
.headers on
.output entities.csv
SELECT * FROM entities;
.output relations.csv
SELECT * FROM relations;
.output stdout
EOF
```

### 备份数据库
```bash
cp ~/Library/Application\ Support/com.zhoufengdai.memoryai/database/memoryai.db \
   ~/memoryai_backup_$(date +%Y%m%d_%H%M%S).db
```

## 💡 提示

- 别名表 `entity_aliases` 是新功能，当前为空是正常的
- 只有使用知识融合功能保存记忆时，才会自动填充别名数据
- 可以使用 `.mode` 改变显示格式：`column`、`line`、`csv`、`json` 等
- 使用 `.help` 查看所有 sqlite3 命令

---

创建时间：2026-02-15
数据库版本：支持知识融合 v2.0
