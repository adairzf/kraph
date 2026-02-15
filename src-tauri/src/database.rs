//! SQLite 数据库模块：实体、关系、记忆及关联表

use chrono::Utc;
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

/// 全局数据库连接（通过 State 管理）
pub struct DbState(pub Mutex<Option<Connection>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: i64,
    #[serde(rename = "type")]
    pub entity_type: String, // Person / Location / Time / Event
    pub name: String,
    pub attributes: Option<String>, // JSON string
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: i64,
    pub from_entity_id: i64,
    pub to_entity_id: i64,
    pub relation_type: String,
    pub strength: i32,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: i64,
    pub content: String,
    pub md_file_path: Option<String>,
    pub created_at: String,
    pub tags: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntity {
    pub memory_id: i64,
    pub entity_id: i64,
}

/// 初始化数据库：创建或打开 DB 并执行建表
pub fn init_db(db_path: &Path) -> SqliteResult<Connection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;
    }
    let conn = Connection::open(db_path)?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            name TEXT NOT NULL,
            attributes TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now')),
            UNIQUE(type, name)
        );

        CREATE TABLE IF NOT EXISTS entity_aliases (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entity_id INTEGER NOT NULL,
            alias TEXT NOT NULL UNIQUE,
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY (entity_id) REFERENCES entities(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS relations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            from_entity_id INTEGER NOT NULL,
            to_entity_id INTEGER NOT NULL,
            relation_type TEXT NOT NULL,
            strength INTEGER DEFAULT 1,
            created_at TEXT DEFAULT (datetime('now')),
            UNIQUE(from_entity_id, to_entity_id, relation_type),
            FOREIGN KEY (from_entity_id) REFERENCES entities(id),
            FOREIGN KEY (to_entity_id) REFERENCES entities(id)
        );

        CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            md_file_path TEXT UNIQUE,
            created_at TEXT DEFAULT (datetime('now')),
            tags TEXT
        );

        CREATE TABLE IF NOT EXISTS memory_entities (
            memory_id INTEGER NOT NULL,
            entity_id INTEGER NOT NULL,
            PRIMARY KEY (memory_id, entity_id),
            FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE,
            FOREIGN KEY (entity_id) REFERENCES entities(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_entities_type_name ON entities(type, name);
        CREATE INDEX IF NOT EXISTS idx_entity_aliases_entity ON entity_aliases(entity_id);
        CREATE INDEX IF NOT EXISTS idx_entity_aliases_alias ON entity_aliases(alias);
        CREATE INDEX IF NOT EXISTS idx_relations_from ON relations(from_entity_id);
        CREATE INDEX IF NOT EXISTS idx_relations_to ON relations(to_entity_id);
        CREATE INDEX IF NOT EXISTS idx_memory_entities_memory ON memory_entities(memory_id);
        CREATE INDEX IF NOT EXISTS idx_memory_entities_entity ON memory_entities(entity_id);
        "#,
    )?;
    Ok(())
}

/// 插入或获取实体（按 type+name 唯一）
pub fn upsert_entity(
    conn: &Connection,
    entity_type: &str,
    name: &str,
    attributes: Option<&str>,
) -> SqliteResult<i64> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        r#"
        INSERT INTO entities (type, name, attributes, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?4)
        ON CONFLICT(type, name) DO UPDATE SET
            attributes = COALESCE(excluded.attributes, attributes),
            updated_at = excluded.updated_at
        "#,
        params![entity_type, name, attributes, now],
    )?;
    let id: i64 = conn.query_row(
        "SELECT id FROM entities WHERE type = ?1 AND name = ?2",
        params![entity_type, name],
        |row| row.get(0),
    )?;
    Ok(id)
}

pub fn get_entity_by_id(conn: &Connection, id: i64) -> SqliteResult<Entity> {
    conn.query_row(
        "SELECT id, type, name, attributes, created_at, updated_at FROM entities WHERE id = ?1",
        params![id],
        |row| {
            Ok(Entity {
                id: row.get(0)?,
                entity_type: row.get(1)?,
                name: row.get(2)?,
                attributes: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
}

pub fn get_entity_by_name(conn: &Connection, name: &str) -> SqliteResult<Option<Entity>> {
    // 先尝试直接匹配实体名称
    let mut stmt = conn.prepare(
        "SELECT id, type, name, attributes, created_at, updated_at FROM entities WHERE name LIKE ?1 LIMIT 1",
    )?;
    let mut rows = stmt.query(params![format!("%{}%", name)])?;
    if let Some(row) = rows.next()? {
        return Ok(Some(Entity {
            id: row.get(0)?,
            entity_type: row.get(1)?,
            name: row.get(2)?,
            attributes: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        }));
    }
    
    // 如果没找到，尝试通过别名查找
    let entity_id_opt: Option<i64> = conn
        .query_row(
            "SELECT entity_id FROM entity_aliases WHERE alias LIKE ?1 LIMIT 1",
            params![format!("%{}%", name)],
            |row| row.get(0),
        )
        .ok();
    
    if let Some(entity_id) = entity_id_opt {
        return get_entity_by_id(conn, entity_id).map(Some);
    }
    
    Ok(None)
}

/// 为实体添加别名
pub fn add_entity_alias(conn: &Connection, entity_id: i64, alias: &str) -> SqliteResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO entity_aliases (entity_id, alias) VALUES (?1, ?2)",
        params![entity_id, alias],
    )?;
    Ok(())
}

/// 获取实体的所有别名
pub fn get_entity_aliases(conn: &Connection, entity_id: i64) -> SqliteResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT alias FROM entity_aliases WHERE entity_id = ?1")?;
    let rows = stmt.query_map(params![entity_id], |row| row.get(0))?;
    rows.collect()
}

/// 通过名称或别名查找实体ID
pub fn find_entity_id_by_name_or_alias(conn: &Connection, name: &str) -> SqliteResult<Option<i64>> {
    // 先尝试精确匹配实体名称
    let entity_id_opt: Option<i64> = conn
        .query_row(
            "SELECT id FROM entities WHERE name = ?1 LIMIT 1",
            params![name],
            |row| row.get(0),
        )
        .ok();
    
    if entity_id_opt.is_some() {
        return Ok(entity_id_opt);
    }
    
    // 尝试通过别名查找
    let alias_id_opt: Option<i64> = conn
        .query_row(
            "SELECT entity_id FROM entity_aliases WHERE alias = ?1 LIMIT 1",
            params![name],
            |row| row.get(0),
        )
        .ok();
    
    Ok(alias_id_opt)
}

/// 合并两个实体（将source的所有关系和别名转移到target）
pub fn merge_entities(conn: &Connection, source_id: i64, target_id: i64) -> SqliteResult<()> {
    // 1. 将source的别名转移到target
    conn.execute(
        "UPDATE OR IGNORE entity_aliases SET entity_id = ?1 WHERE entity_id = ?2",
        params![target_id, source_id],
    )?;
    
    // 2. 将source的记忆关联转移到target
    conn.execute(
        "UPDATE OR IGNORE memory_entities SET entity_id = ?1 WHERE entity_id = ?2",
        params![target_id, source_id],
    )?;
    
    // 3. 将source作为from的关系转移到target
    conn.execute(
        "UPDATE OR IGNORE relations SET from_entity_id = ?1 WHERE from_entity_id = ?2",
        params![target_id, source_id],
    )?;
    
    // 4. 将source作为to的关系转移到target
    conn.execute(
        "UPDATE OR IGNORE relations SET to_entity_id = ?1 WHERE to_entity_id = ?2",
        params![target_id, source_id],
    )?;
    
    // 5. 将source的名称作为target的别名
    let source_entity = get_entity_by_id(conn, source_id)?;
    add_entity_alias(conn, target_id, &source_entity.name)?;
    
    // 6. 删除source实体（级联删除会清理剩余引用）
    conn.execute("DELETE FROM entities WHERE id = ?1", params![source_id])?;
    
    Ok(())
}

pub fn list_entities(conn: &Connection) -> SqliteResult<Vec<Entity>> {
    let mut stmt = conn.prepare(
        "SELECT id, type, name, attributes, created_at, updated_at FROM entities ORDER BY type, name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Entity {
            id: row.get(0)?,
            entity_type: row.get(1)?,
            name: row.get(2)?,
            attributes: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

/// 插入关系（若已存在则增加 strength）
pub fn upsert_relation(
    conn: &Connection,
    from_entity_id: i64,
    to_entity_id: i64,
    relation_type: &str,
) -> SqliteResult<()> {
    conn.execute(
        r#"
        INSERT INTO relations (from_entity_id, to_entity_id, relation_type, strength)
        VALUES (?1, ?2, ?3, 1)
        ON CONFLICT(from_entity_id, to_entity_id, relation_type) DO UPDATE SET strength = strength + 1
        "#,
        params![from_entity_id, to_entity_id, relation_type],
    )?;
    Ok(())
}

pub fn list_relations(conn: &Connection) -> SqliteResult<Vec<Relation>> {
    let mut stmt = conn.prepare(
        "SELECT id, from_entity_id, to_entity_id, relation_type, strength, created_at FROM relations",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Relation {
            id: row.get(0)?,
            from_entity_id: row.get(1)?,
            to_entity_id: row.get(2)?,
            relation_type: row.get(3)?,
            strength: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn insert_memory(
    conn: &Connection,
    content: &str,
    md_file_path: Option<&str>,
    tags: Option<&str>,
) -> SqliteResult<i64> {
    conn.execute(
        "INSERT INTO memories (content, md_file_path, tags) VALUES (?1, ?2, ?3)",
        params![content, md_file_path, tags],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn link_memory_entity(conn: &Connection, memory_id: i64, entity_id: i64) -> SqliteResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO memory_entities (memory_id, entity_id) VALUES (?1, ?2)",
        params![memory_id, entity_id],
    )?;
    Ok(())
}

pub fn get_memory_by_id(conn: &Connection, id: i64) -> SqliteResult<Memory> {
    conn.query_row(
        "SELECT id, content, md_file_path, created_at, tags FROM memories WHERE id = ?1",
        params![id],
        |row| {
            Ok(Memory {
                id: row.get(0)?,
                content: row.get(1)?,
                md_file_path: row.get(2)?,
                created_at: row.get(3)?,
                tags: row.get(4)?,
            })
        },
    )
}

pub fn list_memories(conn: &Connection) -> SqliteResult<Vec<Memory>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, md_file_path, created_at, tags FROM memories ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Memory {
            id: row.get(0)?,
            content: row.get(1)?,
            md_file_path: row.get(2)?,
            created_at: row.get(3)?,
            tags: row.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn update_memory(
    conn: &Connection,
    id: i64,
    content: &str,
    tags: Option<&str>,
) -> SqliteResult<()> {
    conn.execute(
        "UPDATE memories SET content = ?1, tags = ?2 WHERE id = ?3",
        params![content, tags, id],
    )?;
    Ok(())
}

pub fn delete_memory(conn: &Connection, id: i64) -> SqliteResult<()> {
    // 删除记忆（memory_entities 关联会因为外键 CASCADE 自动删除）
    conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
    
    // 先清理孤立的关系（引用不存在的实体）
    conn.execute(
        r#"DELETE FROM relations
           WHERE from_entity_id NOT IN (SELECT id FROM entities)
              OR to_entity_id NOT IN (SELECT id FROM entities)"#,
        [],
    )?;
    
    // 再清理孤立的实体（不被任何记忆引用）
    conn.execute(
        "DELETE FROM entities WHERE id NOT IN (SELECT DISTINCT entity_id FROM memory_entities)",
        [],
    )?;
    
    // 最后再清理一次关系（因为删除实体可能产生新的孤立关系）
    conn.execute(
        r#"DELETE FROM relations
           WHERE from_entity_id NOT IN (SELECT id FROM entities)
              OR to_entity_id NOT IN (SELECT id FROM entities)"#,
        [],
    )?;
    
    Ok(())
}

pub fn clear_memory_entities(conn: &Connection, memory_id: i64) -> SqliteResult<()> {
    conn.execute(
        "DELETE FROM memory_entities WHERE memory_id = ?1",
        params![memory_id],
    )?;
    Ok(())
}

/// 清理数据库中的脏数据和孤立记录
pub fn cleanup_database(conn: &Connection) -> SqliteResult<()> {
    // 临时禁用外键约束，以便清理脏数据
    conn.execute("PRAGMA foreign_keys = OFF", [])?;
    
    // 1. 清理 memory_entities 中引用不存在的 memory 的记录
    conn.execute(
        "DELETE FROM memory_entities WHERE memory_id NOT IN (SELECT id FROM memories)",
        [],
    )?;
    
    // 2. 清理 memory_entities 中引用不存在的 entity 的记录
    conn.execute(
        "DELETE FROM memory_entities WHERE entity_id NOT IN (SELECT id FROM entities)",
        [],
    )?;
    
    // 3. 清理 relations 中引用不存在的 entity 的记录
    conn.execute(
        r#"DELETE FROM relations 
           WHERE from_entity_id NOT IN (SELECT id FROM entities)
              OR to_entity_id NOT IN (SELECT id FROM entities)"#,
        [],
    )?;
    
    // 4. 清理孤立的实体（不被任何记忆引用）
    conn.execute(
        "DELETE FROM entities WHERE id NOT IN (SELECT DISTINCT entity_id FROM memory_entities)",
        [],
    )?;
    
    // 5. 再次清理 relations（因为删除实体可能产生新的孤立关系）
    conn.execute(
        r#"DELETE FROM relations 
           WHERE from_entity_id NOT IN (SELECT id FROM entities)
              OR to_entity_id NOT IN (SELECT id FROM entities)"#,
        [],
    )?;
    
    // 重新启用外键约束
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    
    Ok(())
}

/// 清空所有数据（保留表结构）
pub fn clear_all_data(conn: &Connection) -> SqliteResult<()> {
    conn.execute("PRAGMA foreign_keys = OFF", [])?;
    
    // 按照依赖顺序删除数据
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

pub fn get_entity_ids_for_memory(conn: &Connection, memory_id: i64) -> SqliteResult<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT entity_id FROM memory_entities WHERE memory_id = ?1")?;
    let rows = stmt.query_map(params![memory_id], |row| row.get(0))?;
    rows.collect()
}

pub fn get_memories_for_entity(conn: &Connection, entity_id: i64) -> SqliteResult<Vec<Memory>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT m.id, m.content, m.md_file_path, m.created_at, m.tags
        FROM memories m
        INNER JOIN memory_entities me ON m.id = me.memory_id
        WHERE me.entity_id = ?1
        ORDER BY m.created_at DESC
        "#,
    )?;
    let rows = stmt.query_map(params![entity_id], |row| {
        Ok(Memory {
            id: row.get(0)?,
            content: row.get(1)?,
            md_file_path: row.get(2)?,
            created_at: row.get(3)?,
            tags: row.get(4)?,
        })
    })?;
    rows.collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub attributes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub strength: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
}

pub fn get_graph_data(conn: &Connection) -> SqliteResult<GraphData> {
    let entities = list_entities(conn)?;
    let relations = list_relations(conn)?;
    let id_to_name: std::collections::HashMap<i64, String> =
        entities.iter().map(|e| (e.id, e.name.clone())).collect();
    let nodes: Vec<GraphNode> = entities
        .into_iter()
        .map(|e| GraphNode {
            id: e.id.to_string(),
            name: e.name,
            node_type: e.entity_type,
            attributes: e.attributes,
        })
        .collect();
    let links: Vec<GraphLink> = relations
        .into_iter()
        .filter(|r| id_to_name.contains_key(&r.from_entity_id) && id_to_name.contains_key(&r.to_entity_id))
        .map(|r| GraphLink {
            source: r.from_entity_id.to_string(),
            target: r.to_entity_id.to_string(),
            relation: r.relation_type,
            strength: r.strength,
        })
        .collect();
    Ok(GraphData { nodes, links })
}
