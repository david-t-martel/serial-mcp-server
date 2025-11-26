use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id: String,                // UUID string
    pub device_id: String,         // user provided logical device id
    pub port_name: Option<String>, // current physical port (if bound)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed: i32, // 0 = open, 1 = closed (use integer for sqlite boolean compatibility)
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: i64,
    pub session_id: String,
    pub role: String,              // logical semantic role
    pub direction: Option<String>, // optional: sent|received for device traffic
    pub content: String,
    pub features: Option<String>, // comma separated feature tags (e.g. "error,warning,command")
    pub latency_ms: Option<i64>,  // optional measured round-trip or processing latency
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct SessionStore {
    pool: SqlitePool,
}

impl SessionStore {
    pub async fn new(database_url: &str) -> sqlx::Result<Self> {
        // If this is a file path (sqlite://path/to/file.db) ensure directory exists
        if let Some(rest) = database_url.strip_prefix("sqlite://") {
            if !rest.starts_with(':') {
                // avoid in-memory or special forms
                if let Some(parent) = Path::new(rest).parent() {
                    if !parent.as_os_str().is_empty() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                }
            }
        }
        let pool = SqlitePool::connect(database_url).await?;
        Self::run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    async fn run_migrations(pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            device_id TEXT NOT NULL,
            port_name TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            closed INTEGER NOT NULL DEFAULT 0
        )"#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            direction TEXT,
            content TEXT NOT NULL,
            features TEXT,
            latency_ms INTEGER,
            created_at TEXT NOT NULL,
            FOREIGN KEY(session_id) REFERENCES sessions(id)
        )"#,
        )
        .execute(pool)
        .await?;
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id)"#)
            .execute(pool)
            .await?;
        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_messages_session_role ON messages(session_id, role)"#,
        )
        .execute(pool)
        .await?;
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_messages_features ON messages(features)"#)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Explicit helper to idempotently create / migrate the database without keeping a pool instance.
    pub async fn ensure_database(database_url: &str) -> sqlx::Result<()> {
        let store = Self::new(database_url).await?;
        let _ = store.pool.acquire().await?;
        Ok(())
    }

    pub async fn create_session(
        &self,
        device_id: &str,
        port_name: Option<&str>,
    ) -> sqlx::Result<Session> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO sessions (id, device_id, port_name, created_at, updated_at, closed) VALUES (?1, ?2, ?3, ?4, ?5, 0)")
            .bind(&id).bind(device_id).bind(port_name).bind(now).bind(now)
            .execute(&self.pool).await?;
        Ok(Session {
            id,
            device_id: device_id.to_string(),
            port_name: port_name.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
            closed: 0,
        })
    }

    pub async fn get_session(&self, id: &str) -> sqlx::Result<Option<Session>> {
        sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = ?1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn append_message(
        &self,
        session_id: &str,
        role: &str,
        direction: Option<&str>,
        content: &str,
        features: Option<&str>,
        latency_ms: Option<i64>,
    ) -> sqlx::Result<(i64, DateTime<Utc>)> {
        let now = Utc::now();
        // Use a single connection so last_insert_rowid() is correct for the just-executed INSERT
        let mut conn = self.pool.acquire().await?;
        sqlx::query("INSERT INTO messages (session_id, role, direction, content, features, latency_ms, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .bind(session_id).bind(role).bind(direction).bind(content).bind(features).bind(latency_ms).bind(now)
            .execute(&mut *conn).await?;
        let last_id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
            .fetch_one(&mut *conn)
            .await?;
        sqlx::query("UPDATE sessions SET updated_at = ?1 WHERE id = ?2")
            .bind(now)
            .bind(session_id)
            .execute(&mut *conn)
            .await?;
        Ok((last_id, now))
    }

    pub async fn list_messages(&self, session_id: &str, limit: i64) -> sqlx::Result<Vec<Message>> {
        sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE session_id = ?1 ORDER BY id ASC LIMIT ?2",
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn filter_messages(
        &self,
        session_id: &str,
        role: Option<&str>,
        feature_substring: Option<&str>,
        direction: Option<&str>,
        limit: i64,
    ) -> sqlx::Result<Vec<Message>> {
        // Build dynamic SQL using anonymous positional parameters so we don't have to number them conditionally.
        // This avoids mismatches when some optional filters are omitted.
        let mut sql = String::from("SELECT * FROM messages WHERE session_id = ?");
        if role.is_some() {
            sql.push_str(" AND role = ?");
        }
        if direction.is_some() {
            sql.push_str(" AND direction = ?");
        }
        if feature_substring.is_some() {
            sql.push_str(" AND features LIKE ?");
        }
        sql.push_str(" ORDER BY id ASC LIMIT ?");

        let mut query = sqlx::query_as::<_, Message>(&sql).bind(session_id);
        if let Some(r) = role {
            query = query.bind(r);
        }
        if let Some(d) = direction {
            query = query.bind(d);
        }
        if let Some(f) = feature_substring {
            query = query.bind(format!("%{}%", f));
        }
        query = query.bind(limit);
        query.fetch_all(&self.pool).await
    }

    pub async fn export_features_index(&self, session_id: &str) -> sqlx::Result<serde_json::Value> {
        // Aggregate features into counts
        let rows = sqlx::query(
            "SELECT features FROM messages WHERE session_id = ?1 AND features IS NOT NULL",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        use std::collections::HashMap;
        let mut counts: HashMap<String, u64> = HashMap::new();
        for row in rows {
            let fval: Option<String> = row.try_get("features").ok();
            if let Some(fstr) = fval {
                for tag in fstr.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    *counts.entry(tag.to_string()).or_insert(0) += 1;
                }
            }
        }
        Ok(serde_json::json!({"session_id": session_id, "feature_counts": counts}))
    }

    pub async fn export_messages_with_features(
        &self,
        session_id: &str,
        feature_filter: Option<&str>,
    ) -> sqlx::Result<serde_json::Value> {
        let mut q = "SELECT * FROM messages WHERE session_id = ?1".to_string();
        if feature_filter.is_some() {
            q.push_str(" AND features LIKE ?2");
        }
        q.push_str(" ORDER BY id ASC");
        let mut query = sqlx::query_as::<_, Message>(&q).bind(session_id);
        if let Some(f) = feature_filter {
            query = query.bind(format!("%{}%", f));
        }
        let msgs = query.fetch_all(&self.pool).await?;
        Ok(serde_json::json!({"session_id": session_id, "messages": msgs}))
    }

    pub async fn close_session(&self, session_id: &str) -> sqlx::Result<()> {
        sqlx::query("UPDATE sessions SET closed = 1, updated_at = ?1 WHERE id = ?2")
            .bind(Utc::now())
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn export_session_json(&self, session_id: &str) -> sqlx::Result<serde_json::Value> {
        if let Some(sess) = self.get_session(session_id).await? {
            let msgs = self.list_messages(session_id, i64::MAX).await?;
            let v = serde_json::json!({
                "session": sess,
                "messages": msgs
            });
            Ok(v)
        } else {
            Ok(serde_json::json!({"error": "not found"}))
        }
    }

    /// Lightweight stats for a session without pulling all messages.
    pub async fn session_stats(&self, session_id: &str) -> sqlx::Result<Option<serde_json::Value>> {
        // Use a single connection/transaction-like sequence
        let mut conn = self.pool.acquire().await?;
        // Count + last id + first timestamp + last timestamp
        let row = sqlx::query("SELECT COUNT(*) as cnt, MAX(id) as last_id, MIN(created_at) as first_ts, MAX(created_at) as last_ts FROM messages WHERE session_id = ?1")
            .bind(session_id).fetch_one(&mut *conn).await?;
        let count: i64 = row.try_get("cnt").unwrap_or(0);
        if count == 0 {
            return Ok(None);
        }
        let last_id: Option<i64> = row.try_get("last_id").ok();
        let first_ts: Option<String> = row.try_get("first_ts").ok();
        let last_ts: Option<String> = row.try_get("last_ts").ok();
        let rate_per_min = if let (Some(first), Some(last)) = (first_ts.as_ref(), last_ts.as_ref())
        {
            if let (Ok(ft), Ok(lt)) = (ft_parse(first), ft_parse(last)) {
                let secs = (lt - ft).num_seconds().max(1);
                (count as f64) / (secs as f64 / 60.0)
            } else {
                0.0
            }
        } else {
            0.0
        };
        Ok(Some(serde_json::json!({
            "session_id": session_id,
            "message_count": count,
            "last_message_id": last_id,
            "messages_per_min": rate_per_min
        })))
    }
}

fn ft_parse(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    s.parse::<DateTime<Utc>>()
}

// MCP integration will wrap this store (future tools: create_session, resume_session, append_message, export_session)

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to construct an in-memory shared SQLite URL so that the pool shares the same DB.
    fn memory_db() -> &'static str {
        "sqlite::memory:?cache=shared"
    }

    #[tokio::test]
    async fn create_and_get_session() {
        let store = SessionStore::new(memory_db()).await.expect("init store");
        let s = store
            .create_session("device-1", Some("COM1"))
            .await
            .expect("create");
        let fetched = store
            .get_session(&s.id)
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(fetched.device_id, "device-1");
        assert_eq!(fetched.port_name.as_deref(), Some("COM1"));
        assert_eq!(fetched.closed, 0);
    }

    #[tokio::test]
    async fn append_list_filter_and_feature_index() {
        let store = SessionStore::new(memory_db()).await.expect("init store");
        let s = store.create_session("devA", None).await.expect("create");

        let (id1, ts1) = store
            .append_message(&s.id, "system", None, "init", None, None)
            .await
            .expect("append1");
        let (id2, ts2) = store
            .append_message(
                &s.id,
                "user",
                Some("sent"),
                "command RUN",
                Some("command"),
                Some(12),
            )
            .await
            .expect("append2");
        let (id3, ts3) = store
            .append_message(
                &s.id,
                "device",
                Some("received"),
                "ERR timeout",
                Some("error,warning"),
                Some(34),
            )
            .await
            .expect("append3");
        assert!(id1 < id2 && id2 < id3, "message ids should be ascending");
        assert!(
            ts1 <= ts2 && ts2 <= ts3,
            "timestamps should be non-decreasing"
        );

        let all = store.list_messages(&s.id, 100).await.expect("list");
        assert_eq!(all.len(), 3);

        // Filter by role
        let user_msgs = store
            .filter_messages(&s.id, Some("user"), None, None, 50)
            .await
            .expect("filter role");
        assert_eq!(user_msgs.len(), 1);
        assert_eq!(user_msgs[0].role, "user");

        // Filter by direction
        let received = store
            .filter_messages(&s.id, None, None, Some("received"), 50)
            .await
            .expect("filter direction");
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].direction.as_deref(), Some("received"));

        // Filter by feature substring (should match message with error,warning)
        let error_like = store
            .filter_messages(&s.id, None, Some("error"), None, 50)
            .await
            .expect("filter feature");
        assert_eq!(error_like.len(), 1);
        assert!(error_like[0].features.as_deref().unwrap().contains("error"));

        // Feature index
        let idx = store
            .export_features_index(&s.id)
            .await
            .expect("feature index");
        let counts = idx
            .get("feature_counts")
            .and_then(|v| v.as_object())
            .expect("counts object");
        assert_eq!(
            counts.get("command").and_then(|v| v.as_u64()).unwrap_or(0),
            1
        );
        assert_eq!(counts.get("error").and_then(|v| v.as_u64()).unwrap_or(0), 1);
        assert_eq!(
            counts.get("warning").and_then(|v| v.as_u64()).unwrap_or(0),
            1
        );
    }
}
