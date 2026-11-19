use crate::models::{AppFilterItem, ClipItem, ContentType};
use rusqlite::{params, Connection, Result};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Database {
    conn: std::sync::Mutex<Connection>,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
        init_tables(&conn)?;

        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }

    pub fn insert_or_update(
        &self,
        content: &str,
        content_hash: &str,
        content_type: &ContentType,
        source_app: &str,
        window_title: &str,
    ) -> Result<ClipItem> {
        let conn = self.conn.lock().unwrap();
        let now = current_timestamp();

        // Check if hash exists anywhere in clips
        let mut check_stmt = conn.prepare(
            "SELECT id, is_pinned, created_at FROM clips WHERE content_hash = ?1 ORDER BY last_used_at DESC LIMIT 1",
        )?;
        let existing = check_stmt
            .query_row(params![content_hash], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)? != 0,
                    row.get::<_, i64>(2)?,
                ))
            })
            .ok();

        if let Some((id, is_pinned, created_at)) = existing {
            conn.execute(
                "UPDATE clips SET last_used_at = ?1, source_app = ?2, window_title = ?3 WHERE id = ?4",
                params![now, source_app, window_title, id],
            )?;

            Ok(ClipItem {
                id,
                content: content.to_string(),
                content_hash: content_hash.to_string(),
                content_type: content_type.clone(),
                source_app: source_app.to_string(),
                window_title: window_title.to_string(),
                is_pinned,
                created_at,
                last_used_at: now,
            })
        } else {
            // Atomic UPSERT: if another event squeezed through, update instead of failing or creating duplicates
            conn.execute(
                "INSERT INTO clips (content, content_hash, content_type, source_app, window_title, is_pinned, created_at, last_used_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6)
                 ON CONFLICT(content_hash) DO UPDATE SET
                     last_used_at = excluded.last_used_at,
                     source_app = excluded.source_app,
                     window_title = excluded.window_title",
                params![content, content_hash, content_type.as_str(), source_app, window_title, now],
            )?;

            let mut fetch_stmt = conn.prepare(
                "SELECT id, content, content_hash, content_type, source_app, window_title, is_pinned, created_at, last_used_at
                 FROM clips WHERE content_hash = ?1",
            )?;
            let clip = fetch_stmt.query_row(params![content_hash], row_to_clip)?;
            Ok(clip)
        }
    }

    pub fn get_clips(
        &self,
        query: Option<&str>,
        filter_app: Option<&str>,
        filter_type: Option<&str>,
        pinned_only: Option<bool>,
        limit: Option<u32>,
    ) -> Result<Vec<ClipItem>> {
        let conn = self.conn.lock().unwrap();
        let limit = limit.unwrap_or(100);

        let trimmed_query = query.unwrap_or("").trim();
        let use_search = !trimmed_query.is_empty();

        let mut conditions = Vec::new();
        let mut sql_params: Vec<String> = Vec::new();

        if pinned_only == Some(true) {
            conditions.push("c.is_pinned = 1".to_string());
        }

        if let Some(app) = filter_app {
            if !app.is_empty() && app != "All" {
                conditions.push("c.source_app = ?".to_string());
                sql_params.push(app.to_string());
            }
        }

        if let Some(ct) = filter_type {
            if !ct.is_empty() && ct != "all" {
                conditions.push("c.content_type = ?".to_string());
                sql_params.push(ct.to_string());
            }
        }

        if use_search {
            // First try FTS5 query, if syntax fails fall back to LIKE
            let fts_query = format!("*{}*", sanitize_fts_query(trimmed_query));
            let mut fts_conditions = conditions.clone();
            fts_conditions.push("c.id IN (SELECT rowid FROM clips_fts WHERE clips_fts MATCH ?)".to_string());

            let mut fts_params = sql_params.clone();
            fts_params.push(fts_query);

            let where_clause = if fts_conditions.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", fts_conditions.join(" AND "))
            };

            let fts_sql = format!(
                "SELECT c.id, c.content, c.content_hash, c.content_type, c.source_app, c.window_title, c.is_pinned, c.created_at, c.last_used_at
                 FROM clips c
                 {}
                 ORDER BY c.is_pinned DESC, c.last_used_at DESC
                 LIMIT {}",
                where_clause, limit
            );

            let param_refs: Vec<&dyn rusqlite::ToSql> = fts_params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
            if let Ok(mut stmt) = conn.prepare(&fts_sql) {
                if let Ok(rows) = stmt.query_map(&param_refs[..], row_to_clip) {
                    let items: Result<Vec<ClipItem>> = rows.collect();
                    if let Ok(list) = items {
                        return Ok(list);
                    }
                }
            }

            // Fallback to LIKE
            let like_pattern = format!("%{}%", trimmed_query);
            conditions.push("(c.content LIKE ? OR c.window_title LIKE ? OR c.source_app LIKE ?)".to_string());
            sql_params.push(like_pattern.clone());
            sql_params.push(like_pattern.clone());
            sql_params.push(like_pattern);
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT c.id, c.content, c.content_hash, c.content_type, c.source_app, c.window_title, c.is_pinned, c.created_at, c.last_used_at
             FROM clips c
             {}
             ORDER BY c.is_pinned DESC, c.last_used_at DESC
             LIMIT {}",
            where_clause, limit
        );

        let param_refs: Vec<&dyn rusqlite::ToSql> = sql_params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(&param_refs[..], row_to_clip)?;
        rows.collect()
    }

    pub fn get_clip_by_id(&self, id: i64) -> Result<Option<ClipItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content, content_hash, content_type, source_app, window_title, is_pinned, created_at, last_used_at
             FROM clips WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], row_to_clip)?;
        match rows.next() {
            Some(res) => res.map(Some),
            None => Ok(None),
        }
    }

    pub fn touch_last_used(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = current_timestamp();
        conn.execute("UPDATE clips SET last_used_at = ?1 WHERE id = ?2", params![now, id])?;
        Ok(())
    }

    pub fn toggle_pin(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clips SET is_pinned = CASE WHEN is_pinned = 1 THEN 0 ELSE 1 END WHERE id = ?1",
            params![id],
        )?;

        let mut stmt = conn.prepare("SELECT is_pinned FROM clips WHERE id = ?1")?;
        let is_pinned: i64 = stmt.query_row(params![id], |row| row.get(0))?;
        Ok(is_pinned != 0)
    }

    pub fn delete_clip(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clips WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_unpinned(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute("DELETE FROM clips WHERE is_pinned = 0", [])?;
        Ok(deleted)
    }

    pub fn get_apps_summary(&self) -> Result<Vec<AppFilterItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT source_app, COUNT(*) as count
             FROM clips
             WHERE source_app IS NOT NULL AND source_app != ''
             GROUP BY source_app
             ORDER BY count DESC
             LIMIT 15",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(AppFilterItem {
                source_app: row.get(0)?,
                count: row.get(1)?,
            })
        })?;

        rows.collect()
    }
}

fn init_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS clips (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            content_hash TEXT NOT NULL UNIQUE,
            content_type TEXT NOT NULL,
            source_app TEXT NOT NULL,
            window_title TEXT NOT NULL,
            is_pinned INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            last_used_at INTEGER NOT NULL
        );

        -- Purge any duplicate rows from prior runs, keeping the latest entry for each content_hash
        DELETE FROM clips WHERE id NOT IN (
            SELECT MAX(id) FROM clips GROUP BY content_hash
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_clips_unique_hash ON clips(content_hash);
        CREATE INDEX IF NOT EXISTS idx_clips_last_used ON clips(last_used_at DESC);
        CREATE INDEX IF NOT EXISTS idx_clips_pinned ON clips(is_pinned, last_used_at DESC);
        CREATE INDEX IF NOT EXISTS idx_clips_source_app ON clips(source_app);

        CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
            content,
            window_title,
            source_app,
            content='clips',
            content_rowid='id'
        );

        -- FTS triggers for data synchronization
        CREATE TRIGGER IF NOT EXISTS clips_ai AFTER INSERT ON clips BEGIN
            INSERT INTO clips_fts(rowid, content, window_title, source_app)
            VALUES (new.id, new.content, new.window_title, new.source_app);
        END;

        CREATE TRIGGER IF NOT EXISTS clips_ad AFTER DELETE ON clips BEGIN
            INSERT INTO clips_fts(clips_fts, rowid, content, window_title, source_app)
            VALUES ('delete', old.id, old.content, old.window_title, old.source_app);
        END;

        CREATE TRIGGER IF NOT EXISTS clips_au AFTER UPDATE ON clips BEGIN
            INSERT INTO clips_fts(clips_fts, rowid, content, window_title, source_app)
            VALUES ('delete', old.id, old.content, old.window_title, old.source_app);
            INSERT INTO clips_fts(rowid, content, window_title, source_app)
            VALUES (new.id, new.content, new.window_title, new.source_app);
        END;
        ",
    )?;

    Ok(())
}

fn row_to_clip(row: &rusqlite::Row) -> Result<ClipItem> {
    let content_type_str: String = row.get(3)?;
    Ok(ClipItem {
        id: row.get(0)?,
        content: row.get(1)?,
        content_hash: row.get(2)?,
        content_type: ContentType::from_str(&content_type_str),
        source_app: row.get(4)?,
        window_title: row.get(5)?,
        is_pinned: row.get::<_, i64>(6)? != 0,
        created_at: row.get(7)?,
        last_used_at: row.get(8)?,
    })
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn sanitize_fts_query(q: &str) -> String {
    // Remove characters that can break FTS5 token syntax
    q.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == ' ')
        .collect::<String>()
        .trim()
        .to_string()
}
