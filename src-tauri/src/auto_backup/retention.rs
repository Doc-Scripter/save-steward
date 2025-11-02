use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use chrono::{Utc, DateTime};
use tokio::sync::RwLock;
use rusqlite::params;

use crate::database::{DatabaseConnection, DatabaseResult};
use crate::auto_backup::{BackupError, BackupResult, BackupType as GlobalBackupType};

/// Manages retention policies for game backups
#[derive(Clone)]
pub struct RetentionPolicy {
    db_conn: Option<DatabaseConnection>,
    config: RetentionConfig,
    cache: Arc<RwLock<HashMap<i64, Vec<GameBackup>>>>, // game_id -> backups
}

#[derive(Debug, Clone)]
pub struct RetentionConfig {
    /// Maximum number of backups per game (default: 3)
    pub max_backups_per_game: usize,
    /// Whether to compress older backups more aggressively
    pub aggressive_compression: bool,
    /// Minimum age before a backup can be deleted (in hours)
    pub minimum_backup_age_hours: u64,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            max_backups_per_game: 3,
            aggressive_compression: true,
            minimum_backup_age_hours: 1, // 1 hour minimum
        }
    }
}

/// Information about a game backup
#[derive(Debug, Clone)]
pub struct GameBackup {
    pub backup_id: String,
    pub game_id: i64,
    pub backup_type: BackupType,
    pub created_at: DateTime<Utc>,
    pub size_bytes: Option<u64>,
    pub compression_level: CompressionLevel,
    pub file_path: String,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum BackupType {
    SessionStart,
    RealTime,
    SessionEnd,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CompressionLevel {
    /// No compression (fastest, largest)
    None,
    /// Light compression
    Fast,
    /// Standard compression (default)
    Balanced,
    /// High compression (slowest, smallest)
    Maximum,
}

impl From<GlobalBackupType> for BackupType {
    fn from(global: GlobalBackupType) -> Self {
        match global {
            GlobalBackupType::SessionStart => BackupType::SessionStart,
            GlobalBackupType::RealTime => BackupType::RealTime,
            GlobalBackupType::SessionEnd => BackupType::SessionEnd,
            GlobalBackupType::Manual => BackupType::Manual,
        }
    }
}

impl RetentionPolicy {
    pub fn new(config: RetentionConfig) -> Self {
        Self {
            db_conn: None,
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_database(mut self, db_conn: DatabaseConnection) -> Self {
        self.db_conn = Some(db_conn);
        self
    }

    /// Check if a new backup should be created and handle retention
    pub async fn should_create_backup(&self, game_id: i64) -> BackupResult<(bool, Option<String>)> {
        let current_backups = self.get_game_backups(game_id).await?;
        let backup_count = current_backups.len();

        if backup_count < self.config.max_backups_per_game {
            // Room for another backup
            Ok((true, None))
        } else {
            // At limit - need to delete oldest
            if let Some(oldest_backup) = current_backups.first() {
                let age_hours = (Utc::now() - oldest_backup.created_at).num_hours() as u64;

                if age_hours >= self.config.minimum_backup_age_hours {
                    // Old enough to delete
                    self.delete_backup(&oldest_backup.backup_id).await?;
                    self.clear_cache_for_game(game_id).await;
                    Ok((true, Some(oldest_backup.backup_id.clone())))
                } else {
                    // Too new to delete - wait
                    Ok((false, None))
                }
            } else {
                // No backups found - allow creation
                Ok((true, None))
            }
        }
    }

    /// Record a new backup and apply retention policies
    pub async fn record_backup(&self, game_id: i64, backup_id: String, backup_type: BackupType, file_path: &str, size_bytes: Option<u64>) -> BackupResult<()> {
        // Determine compression level based on backup type and age
        let compression_level = self.get_compression_level(backup_type, 0);
        let backup = GameBackup {
            backup_id: backup_id.clone(),
            game_id,
            backup_type,
            created_at: Utc::now(),
            size_bytes,
            compression_level,
            file_path: file_path.to_string(),
        };

        // Store in database if available
        if let Some(conn) = &self.db_conn {
            self.insert_backup_record(conn, &backup).await?;
        }

        // Apply retention cleanup
        let deleted_backup = self.should_create_backup(game_id).await?;
        if let (false, _) = deleted_backup {
            // Retention prevented creation - should not happen here
        }

        // Update cache
        self.update_cache(game_id, backup).await;

        Ok(())
    }

    /// Get all backups for a game, sorted by creation time (oldest first)
    pub async fn get_game_backups(&self, game_id: i64) -> BackupResult<Vec<GameBackup>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(backups) = cache.get(&game_id) {
                return Ok(backups.clone());
            }
        }

        // Load from database if available
        if let Some(conn) = &self.db_conn {
            let backups = self.load_game_backups_from_db(conn, game_id).await?;
            // Cache the results
            let mut cache = self.cache.write().await;
            cache.insert(game_id, backups.clone());
            Ok(backups)
        } else {
            Ok(Vec::new())
        }
    }

    /// Delete a specific backup
    pub async fn delete_backup(&self, backup_id: &str) -> BackupResult<()> {
        // Remove from filesystem
        if let Some(conn) = &self.db_conn {
            let backup_info = self.get_backup_by_id(conn, backup_id).await?;
            if std::fs::remove_file(&backup_info.file_path).is_ok() {
                eprintln!("Deleted backup file: {}", backup_info.file_path);
            }

            // Remove from database
            conn.lock().await.execute(
                "DELETE FROM backups WHERE backup_id = ?",
                params![backup_id],
            )?;

            // Remove from cache
            self.clear_cache_for_game(backup_info.game_id).await;
        }

        Ok(())
    }

    /// Clean up old backups beyond retention policy
    pub async fn cleanup_old_backups(&self, game_id: i64) -> BackupResult<Vec<String>> {
        let current_backups = self.get_game_backups(game_id).await?;
        let mut deleted_backups = Vec::new();

        if current_backups.len() > self.config.max_backups_per_game {
            let to_delete = current_backups.len() - self.config.max_backups_per_game;

            for backup in current_backups.iter().take(to_delete) {
                self.delete_backup(&backup.backup_id).await?;
                deleted_backups.push(backup.backup_id.clone());
            }

            // Clear cache to refresh
            self.clear_cache_for_game(game_id).await;
        }

        Ok(deleted_backups)
    }

    /// Get backup statistics for a game
    pub async fn get_backup_stats(&self, game_id: i64) -> BackupResult<BackupStats> {
        let backups = self.get_game_backups(game_id).await?;

        let total_size = backups.iter()
            .filter_map(|b| b.size_bytes)
            .sum();

        let oldest_backup = backups.first().map(|b| b.created_at);
        let newest_backup = backups.last().map(|b| b.created_at);

        Ok(BackupStats {
            game_id,
            total_backups: backups.len(),
            total_size_bytes: total_size,
            oldest_backup,
            newest_backup,
            max_backups_allowed: self.config.max_backups_per_game,
        })
    }

    /// Update compression levels based on retention policy
    pub async fn optimize_compression(&self, game_id: i64) -> BackupResult<()> {
        if !self.config.aggressive_compression {
            return Ok(());
        }

        let backups = self.get_game_backups(game_id).await?;

        for (index, backup) in backups.iter().enumerate() {
            let level = self.get_compression_level(backup.backup_type, index);

            if level != backup.compression_level {
                // Would need to recompress the backup
                // This is a placeholder - actual recompression would be complex
                eprintln!("Would recompress {} from {:?} to {:?}", backup.backup_id, backup.compression_level, level);
            }
        }

        Ok(())
    }

    fn get_compression_level(&self, backup_type: BackupType, position_from_oldest: usize) -> CompressionLevel {
        match backup_type {
            BackupType::SessionStart | BackupType::SessionEnd => CompressionLevel::Balanced,
            BackupType::Manual => CompressionLevel::Balanced,
            BackupType::RealTime => {
                if self.config.aggressive_compression && position_from_oldest > 0 {
                    CompressionLevel::Maximum // Older real-time backups get maximum compression
                } else {
                    CompressionLevel::Balanced
                }
            }
        }
    }

    async fn insert_backup_record(&self, conn: &DatabaseConnection, backup: &GameBackup) -> BackupResult<()> {
        let conn_guard = conn.lock().await;
        conn_guard.execute(
            r#"
            INSERT INTO backups (backup_id, game_id, backup_type, created_at, file_path, compression_level)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            params![
                &backup.backup_id,
                &backup.game_id,
                serde_json::to_string(&backup.backup_type)?,
                &backup.created_at.to_rfc3339(),
                &backup.file_path,
                serde_json::to_string(&backup.compression_level)?
            ],
        )?;

        Ok(())
    }

    async fn load_game_backups_from_db(&self, conn: &DatabaseConnection, game_id: i64) -> BackupResult<Vec<GameBackup>> {
        let conn_guard = conn.lock().await;
        let mut stmt = conn_guard.prepare(
            "SELECT backup_id, backup_type, created_at, file_path, compression_level FROM backups WHERE game_id = ? ORDER BY created_at ASC"
        )?;

        let mut backups = Vec::new();
        let rows = stmt.query_map(params![game_id], |row| {
            Ok((
                row.get::<_, String>(0)?,  // backup_id
                row.get::<_, String>(1)?,  // backup_type
                row.get::<_, String>(2)?,  // created_at
                row.get::<_, String>(3)?,  // file_path
                row.get::<_, String>(4)?,  // compression_level
            ))
        })?;

        for row in rows {
            let (backup_id, type_str, created_at_str, file_path, level_str) = row?;

            let backup_type: BackupType = match serde_json::from_str(&type_str) {
                Ok(bt) => bt,
                Err(e) => return Err(rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)).into()),
            };
            let created_at: DateTime<Utc> = match DateTime::parse_from_rfc3339(&created_at_str) {
                Ok(dt) => dt.into(),
                Err(e) => return Err(rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)).into()),
            };
            let compression_level: CompressionLevel = serde_json::from_str(&level_str)?;

            let backup = GameBackup {
                backup_id,
                game_id,
                backup_type,
                created_at,
                size_bytes: std::fs::metadata(&file_path).ok().map(|m| m.len()),
                compression_level,
                file_path,
            };

            backups.push(backup);
        }

        Ok(backups)
    }

    async fn get_backup_by_id(&self, conn: &DatabaseConnection, backup_id: &str) -> BackupResult<GameBackup> {
        let conn_guard = conn.lock().await;
        let mut stmt = conn_guard.prepare(
            "SELECT game_id, backup_type, created_at, file_path FROM backups WHERE backup_id = ?"
        )?;

        let backup = stmt.query_row(params![backup_id], |row| {
            let game_id: i64 = row.get(0)?;
            let type_str: String = row.get(1)?;
            let created_at_str: String = row.get(2)?;
            let file_path: String = row.get(3)?;

            let backup_type: BackupType = match serde_json::from_str(&type_str) {
                Ok(bt) => bt,
                Err(e) => return Err(rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)).into()),
            };
            let created_at: DateTime<Utc> = match DateTime::parse_from_rfc3339(&created_at_str) {
                Ok(dt) => dt.into(),
                Err(e) => return Err(rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)).into()),
            };

            Ok(GameBackup {
                backup_id: backup_id.to_string(),
                game_id,
                backup_type,
                created_at,
                size_bytes: std::fs::metadata(&file_path).ok().map(|m| m.len()),
                compression_level: CompressionLevel::Balanced, // Default if not stored
                file_path,
            })
        })?;

        Ok(backup)
    }

    async fn update_cache(&self, game_id: i64, backup: GameBackup) {
        let mut cache = self.cache.write().await;
        cache.entry(game_id)
            .or_insert_with(Vec::new)
            .push(backup);

        // Keep cache sorted by creation time
        if let Some(backups) = cache.get_mut(&game_id) {
            backups.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        }
    }

    async fn clear_cache_for_game(&self, game_id: i64) {
        let mut cache = self.cache.write().await;
        cache.remove(&game_id);
    }
}

/// Statistics about backups for a game
#[derive(Debug)]
pub struct BackupStats {
    pub game_id: i64,
    pub total_backups: usize,
    pub total_size_bytes: u64,
    pub oldest_backup: Option<DateTime<Utc>>,
    pub newest_backup: Option<DateTime<Utc>>,
    pub max_backups_allowed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retention_policy_basic() {
        let policy = RetentionPolicy::new(RetentionConfig::default());

        // Should allow first backup
        let (should_create, deleted) = policy.should_create_backup(123).await.unwrap();
        assert!(should_create);
        assert!(deleted.is_none());
    }

    #[test]
    fn test_compression_levels() {
        let policy = RetentionPolicy::new(RetentionConfig {
            aggressive_compression: true,
            ..Default::default()
        });

        // New backups should be balanced
        assert_eq!(policy.get_compression_level(BackupType::RealTime, 0), CompressionLevel::Balanced);
        // Older backups should be compressed more aggressively
        assert_eq!(policy.get_compression_level(BackupType::RealTime, 1), CompressionLevel::Maximum);
    }
}
