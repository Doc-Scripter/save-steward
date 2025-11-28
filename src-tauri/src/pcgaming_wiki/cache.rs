use rusqlite::{Connection, OptionalExtension, Result};
use chrono::{DateTime, Utc, Duration};

pub struct PcgwCache;

impl PcgwCache {
    pub fn get(conn: &Connection, key: &str) -> Result<Option<String>> {
        let now = Utc::now().to_rfc3339();

        let mut stmt = conn.prepare(
            "SELECT response_json FROM pcgw_cache WHERE query_key = ? AND expires_at > ?"
        )?;

        let result: Option<String> = stmt.query_row([key, &now], |row| {
            row.get(0)
        }).optional()?;

        Ok(result)
    }

    pub fn set(conn: &Connection, key: &str, value: &str, ttl_days: i64) -> Result<()> {
        let now = Utc::now();
        let expires_at = now + Duration::days(ttl_days);

        conn.execute(
            "INSERT OR REPLACE INTO pcgw_cache (query_key, response_json, fetched_at, expires_at) VALUES (?, ?, ?, ?)",
            (
                key,
                value,
                now.to_rfc3339(),
                expires_at.to_rfc3339(),
            ),
        )?;

        Ok(())
    }
}
