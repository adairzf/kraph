#!/bin/bash

# MemoryAI 数据库查看脚本
# 使用方法: ./view_database.sh

DB_PATH="$HOME/Library/Application Support/com.zhoufengdai.memoryai/database/memoryai.db"

if [ ! -f "$DB_PATH" ]; then
    echo "❌ 数据库文件不存在: $DB_PATH"
    exit 1
fi

echo "📊 MemoryAI 数据库概览"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 统计信息
echo "📈 数据统计:"
sqlite3 "$DB_PATH" << 'EOF'
.mode column
.headers on
SELECT 
    (SELECT COUNT(*) FROM memories) as 记忆数量,
    (SELECT COUNT(*) FROM entities) as 实体数量,
    (SELECT COUNT(*) FROM relations) as 关系数量,
    (SELECT COUNT(*) FROM entity_aliases) as 别名数量;
EOF

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 实体列表
echo "👥 实体列表:"
sqlite3 "$DB_PATH" << 'EOF'
.mode column
.headers on
SELECT 
    id,
    type as 类型,
    name as 名称,
    substr(created_at, 1, 19) as 创建时间
FROM entities
ORDER BY id;
EOF

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 别名关系
echo "🏷️  实体别名:"
sqlite3 "$DB_PATH" << 'EOF'
.mode column
.headers on
SELECT 
    e.name as 主名称,
    ea.alias as 别名
FROM entity_aliases ea
JOIN entities e ON ea.entity_id = e.id;
EOF

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 关系列表
echo "🔗 实体关系:"
sqlite3 "$DB_PATH" << 'EOF'
.mode column
.headers on
SELECT 
    e1.name as 从实体,
    r.relation_type as 关系,
    e2.name as 到实体,
    r.strength as 强度
FROM relations r
JOIN entities e1 ON r.from_entity_id = e1.id
JOIN entities e2 ON r.to_entity_id = e2.id
ORDER BY r.strength DESC;
EOF

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 最近记忆
echo "📝 最近记忆 (最多显示5条):"
sqlite3 "$DB_PATH" << 'EOF'
.mode column
.headers on
SELECT 
    id,
    substr(content, 1, 60) as 内容预览,
    substr(created_at, 1, 19) as 创建时间
FROM memories
ORDER BY created_at DESC
LIMIT 5;
EOF

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 记忆实体关联
echo "🔍 记忆与实体关联:"
sqlite3 "$DB_PATH" << 'EOF'
.mode column
.headers on
SELECT 
    m.id as 记忆ID,
    substr(m.content, 1, 40) as 内容,
    GROUP_CONCAT(e.name, ', ') as 关联实体
FROM memories m
JOIN memory_entities me ON m.id = me.memory_id
JOIN entities e ON me.entity_id = e.id
GROUP BY m.id
ORDER BY m.id DESC;
EOF

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "✅ 查看完成！"
echo ""
echo "💡 提示:"
echo "  - 使用 DB Browser: brew install --cask db-browser-for-sqlite"
echo "  - 使用命令行: sqlite3 \"$DB_PATH\""
echo "  - 表结构: sqlite3 \"$DB_PATH\" \".schema\""
echo ""
