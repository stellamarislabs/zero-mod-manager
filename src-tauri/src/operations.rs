use crate::{error::Result, models::OperationRecord};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde_json::Value;
use uuid::Uuid;

pub fn record(
    conn: &Connection,
    kind: &str,
    status: &str,
    summary: &str,
    detail: Value,
) -> Result<OperationRecord> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let finished = matches!(status, "completed" | "failed" | "rolled-back").then(|| now.clone());
    conn.execute(
        "INSERT INTO operations(id,kind,status,summary,detail_json,started_at,finished_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![id, kind, status, summary, detail.to_string(), now, finished],
    )?;
    Ok(OperationRecord {
        id,
        kind: kind.into(),
        status: status.into(),
        summary: summary.into(),
        detail,
        started_at: now,
        finished_at: finished,
    })
}

pub fn list(conn: &Connection, limit: usize) -> Result<Vec<OperationRecord>> {
    let mut statement = conn.prepare(
        "SELECT id,kind,status,summary,detail_json,started_at,finished_at
         FROM operations ORDER BY started_at DESC LIMIT ?1",
    )?;
    let rows = statement
        .query_map([limit.min(250) as i64], |row| {
            let detail: String = row.get(4)?;
            Ok(OperationRecord {
                id: row.get(0)?,
                kind: row.get(1)?,
                status: row.get(2)?,
                summary: row.get(3)?,
                detail: serde_json::from_str(&detail).unwrap_or(Value::Null),
                started_at: row.get(5)?,
                finished_at: row.get(6)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}
