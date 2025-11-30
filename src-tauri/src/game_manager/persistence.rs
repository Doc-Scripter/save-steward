use crate::database::models::*;
use chrono::{DateTime, Utc};
use rusqlite::params;
use std::sync::Arc;

pub struct Persistence;

impl Persistence {
    /// Insert game into database
    pub fn insert_game(tx: &rusqlite::Transaction, request: &AddGameRequest) -> Result<i64, String> {
        match tx.execute(
            "INSERT INTO games (name, platform, platform_app_id,
                              executable_path, installation_path, platform_executables,
                              icon_base64, icon_path, created_at, updated_at, is_active)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                request.name,
                request.platform,
                request.platform_app_id,
                request.executable_path,
                request.installation_path,
                request.platform_executables,
                request.icon_base64,
                request.icon_path,
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
                true,
            ],
        ) {
            Ok(_) => {
                let rowid = tx.last_insert_rowid();
                crate::logger::info("DATABASE", &format!("Successfully inserted game '{}' into database", request.name), None);
                Ok(rowid)
            }
            Err(e) => {
                crate::logger::error("DATABASE", &format!("Failed to insert game '{}': {}", request.name, e), None);
                Err(format!("Insert game error: {}", e))
            }
        }
    }

    /// Insert save location into database
    pub fn insert_save_location(
        tx: &rusqlite::Transaction,
        game_id: i64,
        location: &SaveLocation,
    ) -> Result<i64, String> {
        tx.execute(
            "INSERT INTO save_locations (game_id, path_pattern, path_type, platform, save_type,
                                       file_patterns, exclude_patterns, is_relative_to_user,
                                       environment_variable, priority, detection_method,
                                       community_confirmed, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                game_id,
                location.path_pattern,
                location.path_type,
                location.platform,
                location.save_type,
                location.file_patterns,
                location.exclude_patterns,
                location.is_relative_to_user,
                location.environment_variable,
                location.priority,
                location.detection_method,
                location.community_confirmed,
                location.created_at.to_rfc3339(),
                location.updated_at.to_rfc3339(),
            ],
        ).map_err(|e| format!("Insert save location error: {}", e))?;

        Ok(tx.last_insert_rowid())
    }

    /// Insert detected save into database
    pub fn insert_detected_save(
        tx: &rusqlite::Transaction,
        game_id: i64,
        save_location_id: i64,
        actual_path: &str,
    ) -> Result<i64, String> {
        tx.execute(
            "INSERT INTO detected_saves (game_id, save_location_id, actual_path,
                                        first_detected, last_checked, is_active)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                game_id,
                save_location_id,
                actual_path,
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
                true,
            ],
        ).map_err(|e| format!("Insert detected save error: {}", e))?;

        Ok(tx.last_insert_rowid())
    }

    /// Parse timestamp string from database to DateTime
    pub fn parse_timestamp(timestamp_str: &str) -> Result<DateTime<Utc>, String> {
        DateTime::parse_from_rfc3339(timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| format!("Failed to parse timestamp '{}': {}", timestamp_str, e))
    }

    /// Get game by ID
    pub fn get_game_by_id(conn: &rusqlite::Connection, game_id: i64) -> Result<Game, String> {
        let mut stmt = conn.prepare(
            "SELECT id, name, developer, publisher, platform, platform_app_id,
                    executable_path, installation_path, platform_executables,
                    genre, release_date, cover_image_url, icon_base64, icon_path,
                    created_at, updated_at, is_active
             FROM games WHERE id = ?"
        ).map_err(|e| format!("Prepare statement error: {}", e))?;

        let game = stmt.query_row([game_id], |row| {
            let created_at_str: String = row.get(14)?;
            let updated_at_str: String = row.get(15)?;

            let created_at = Self::parse_timestamp(&created_at_str)
                .unwrap_or_else(|_| Utc::now());
            let updated_at = Self::parse_timestamp(&updated_at_str)
                .unwrap_or_else(|_| Utc::now());

            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                developer: row.get(2)?,
                publisher: row.get(3)?,
                platform: row.get(4)?,
                platform_app_id: row.get(5)?,
                executable_path: row.get(6)?,
                installation_path: row.get(7)?,
                platform_executables: row.get(8)?,
                genre: row.get(9)?,
                release_date: row.get(10)?,
                cover_image_url: row.get(11)?,
                icon_base64: row.get(12)?,
                icon_path: row.get(13)?,
                created_at,
                updated_at,
                is_active: row.get(16)?,
            })
        }).map_err(|e| format!("Query game error: {}", e))?;

        Ok(game)
    }

    /// Get all active games
    pub async fn get_all_games(
        db: &std::sync::Arc<tokio::sync::Mutex<crate::database::connection::Database>>,
    ) -> Result<Vec<Game>, String> {
        let conn_guard = db.lock().await;
        let conn = conn_guard.get_connection().await;

        let mut stmt = conn.prepare(
            "SELECT id, name, developer, publisher, platform, platform_app_id,
                    executable_path, installation_path, platform_executables,
                    genre, release_date, cover_image_url, icon_base64, icon_path,
                    created_at, updated_at, is_active
             FROM games WHERE is_active = TRUE ORDER BY name ASC"
        ).map_err(|e| format!("Prepare statement error: {}", e))?;

        let games = stmt.query_map([], |row| {
            let created_at_str: String = row.get(14)?;
            let updated_at_str: String = row.get(15)?;

            let created_at = Self::parse_timestamp(&created_at_str)
                .unwrap_or_else(|_| Utc::now());
            let updated_at = Self::parse_timestamp(&updated_at_str)
                .unwrap_or_else(|_| Utc::now());

            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                developer: row.get(2)?,
                publisher: row.get(3)?,
                platform: row.get(4)?,
                platform_app_id: row.get(5)?,
                executable_path: row.get(6)?,
                installation_path: row.get(7)?,
                platform_executables: row.get(8)?,
                genre: row.get(9)?,
                release_date: row.get(10)?,
                cover_image_url: row.get(11)?,
                icon_base64: row.get(12)?,
                icon_path: row.get(13)?,
                created_at,
                updated_at,
                is_active: row.get(16)?,
            })
        })
        .map_err(|e| format!("Query games error: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect games error: {}", e))?;

        Ok(games)
    }

    /// Update an existing game
    pub async fn update_game(
        db: &Arc<tokio::sync::Mutex<crate::database::connection::Database>>,
        game_id: i64,
        request: AddGameRequest,
    ) -> Result<Game, String> {
        let conn_guard = db.lock().await;
        let mut conn = conn_guard.get_connection().await;

        // Update the game
        conn.execute(
            "UPDATE games SET name = ?, platform = ?, platform_app_id = ?,
                            executable_path = ?, installation_path = ?, 
                            icon_base64 = ?, icon_path = ?, updated_at = ?
             WHERE id = ?",
            params![
                request.name,
                request.platform,
                request.platform_app_id,
                request.executable_path,
                request.installation_path,
                request.icon_base64,
                request.icon_path,
                Utc::now().to_rfc3339(),
                game_id,
            ],
        ).map_err(|e| format!("Update game error: {}", e))?;

        // Return the updated game
        Self::get_game_by_id(&conn, game_id)
    }

    /// Update game with platform executables
    pub fn update_game_platform_executables(tx: &rusqlite::Transaction, game_id: i64, executables_json: &str) -> Result<(), String> {
        tx.execute(
            "UPDATE games SET platform_executables = ?, updated_at = ? WHERE id = ?",
            params![
                executables_json,
                Utc::now().to_rfc3339(),
                game_id,
            ],
        ).map_err(|e| format!("Update game executables error: {}", e))?;

        Ok(())
    }

    /// Delete a game and all associated data
    pub async fn delete_game(
        db: &Arc<tokio::sync::Mutex<crate::database::connection::Database>>,
        game_id: i64,
    ) -> Result<(), String> {
        let conn_guard = db.lock().await;
        let mut conn = conn_guard.get_connection().await;

        // Start transaction
        let tx = conn.transaction().map_err(|e| format!("Transaction error: {}", e))?;

        // Helper function to safely execute delete with better error handling
        let safe_delete = |tx: &rusqlite::Transaction, table: &str, game_id: i64| -> Result<(), String> {
            match tx.execute(&format!("DELETE FROM {} WHERE game_id = ?", table), [game_id]) {
                Ok(_) => Ok(()),
                Err(rusqlite::Error::SqliteFailure(_, _)) => {
                    // Table doesn't exist or other SQLite error - log and continue
                    eprintln!("Warning: Table '{}' does not exist or other error, skipping deletion", table);
                    Ok(())
                }
                Err(e) => Err(format!("Failed to delete from {}: {}", table, e)),
            }
        };

        // Delete in reverse dependency order to avoid foreign key constraint issues
        safe_delete(&tx, "git_save_snapshots", game_id)?;
        safe_delete(&tx, "cloud_sync_log", game_id)?;
        safe_delete(&tx, "git_save_commits", game_id)?;
        safe_delete(&tx, "git_branches", game_id)?;
        safe_delete(&tx, "git_repositories", game_id)?;
        
        // Handle save_versions (which references detected_saves)
        match tx.execute(
            "DELETE FROM save_versions WHERE detected_save_id IN (SELECT id FROM detected_saves WHERE game_id = ?)", 
            [game_id]
        ) {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(_, _)) => {
                eprintln!("Warning: save_versions table does not exist or other error, skipping");
                Ok(())
            }
            Err(e) => Err(format!("Failed to delete save_versions: {}", e)),
        }?;
        
        safe_delete(&tx, "detected_saves", game_id)?;
        safe_delete(&tx, "save_locations", game_id)?;
        safe_delete(&tx, "user_games", game_id)?;
        safe_delete(&tx, "game_identifiers", game_id)?;
        
        // Finally delete the game itself
        let rows_affected = tx.execute("DELETE FROM games WHERE id = ?", [game_id])
            .map_err(|e| format!("Failed to delete game: {}", e))?;
            
        if rows_affected == 0 {
            return Err(format!("Game with id {} not found", game_id));
        }

        // Commit transaction
        tx.commit().map_err(|e| format!("Commit error: {}", e))?;

        Ok(())
    }
}
