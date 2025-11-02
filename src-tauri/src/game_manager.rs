use crate::database::{DatabaseConnection, models::*};
use std::path::Path;
use std::sync::Arc;
use chrono::Utc;

pub struct GameManager;

impl GameManager {
    /// Add a game manually with automatic save location detection
    pub async fn add_manual_game(
        db: &Arc<tokio::sync::Mutex<crate::database::connection::EncryptedDatabase>>,
        request: AddGameRequest,
    ) -> Result<GameWithSaves, String> {
        let conn_guard = db.lock().await;
        let mut conn = conn_guard.get_connection().await;

        // Start transaction
        let tx = conn.transaction().map_err(|e| format!("Transaction error: {}", e))?;

        // Insert game
        let game_id = Self::insert_game(&tx, &request)?;

        // Detect and insert save locations
        let save_locations = Self::detect_save_locations(&tx, game_id, &request)?;

        // Scan for existing saves
        let detected_saves = Self::scan_existing_saves(&tx, game_id, &save_locations)?;

        tx.commit().map_err(|e| format!("Commit error: {}", e))?;

        Ok(GameWithSaves {
            game: Self::get_game_by_id(&*conn, game_id)?,
            save_locations: save_locations,
            detected_saves,
            user_config: None,
        })
    }

    /// Insert game into database
    fn insert_game(tx: &rusqlite::Transaction, request: &AddGameRequest) -> Result<i64, String> {
        tx.execute(
            "INSERT INTO games (name, developer, publisher, platform, platform_app_id,
                              executable_path, installation_path, created_at, updated_at, is_active)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                request.name,
                request.developer,
                request.publisher,
                request.platform,
                request.platform_app_id,
                request.executable_path,
                request.installation_path,
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
                true,
            ],
        ).map_err(|e| format!("Insert game error: {}", e))?;

        Ok(tx.last_insert_rowid())
    }

    /// Detect save locations for a game
    fn detect_save_locations(
        tx: &rusqlite::Transaction,
        game_id: i64,
        request: &AddGameRequest,
    ) -> Result<Vec<SaveLocation>, String> {
        let mut locations = Vec::new();

        // Try manifest-based detection first
        if let Some(manifest_locations) = Self::detect_from_manifest(request)? {
            for loc in manifest_locations {
                let id = Self::insert_save_location(tx, game_id, &loc)?;
                let mut loc_with_id = loc;
                loc_with_id.id = id;
                locations.push(loc_with_id);
            }
        }

        // Fallback to heuristic detection if no manifest data
        if locations.is_empty() {
            let heuristic_locations = Self::detect_from_heuristics(request)?;
            for loc in heuristic_locations {
                let id = Self::insert_save_location(tx, game_id, &loc)?;
                let mut loc_with_id = loc;
                loc_with_id.id = id;
                locations.push(loc_with_id);
            }
        }

        Ok(locations)
    }

    /// Try to detect save locations from manifest data
    fn detect_from_manifest(_request: &AddGameRequest) -> Result<Option<Vec<SaveLocation>>, String> {
        // This would integrate with ManifestResolver to find game data
        // For now, return None to use heuristics
        Ok(None)
    }

    /// Detect save locations using common patterns
    fn detect_from_heuristics(request: &AddGameRequest) -> Result<Vec<SaveLocation>, String> {
        let mut locations = Vec::new();

        match request.platform.as_str() {
            "steam" => {
                // Common Steam save locations
                if let Some(app_id) = &request.platform_app_id {
                    locations.push(SaveLocation {
                        id: 0, // Will be set after insert
                        game_id: 0, // Will be set by caller
                        path_pattern: format!("%STEAMUSER%/userdata/*/{}", app_id),
                        path_type: "directory".to_string(),
                        platform: Some("windows".to_string()),
                        save_type: "auto".to_string(),
                        file_patterns: Some(r#"["*.sav", "*.save", "*.dat"]"#.to_string()),
                        exclude_patterns: None,
                        is_relative_to_user: true,
                        environment_variable: Some("%APPDATA%".to_string()),
                        priority: 10,
                        detection_method: Some("heuristic".to_string()),
                        community_confirmed: false,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    });
                }
            }
            _ => {
                // Generic fallback - use installation directory
                if let Some(install_path) = &request.installation_path {
                    locations.push(SaveLocation {
                        id: 0,
                        game_id: 0,
                        path_pattern: format!("{}/save", install_path),
                        path_type: "directory".to_string(),
                        platform: None,
                        save_type: "auto".to_string(),
                        file_patterns: Some(r#"["*.sav", "*.save", "*.dat", "*.json"]"#.to_string()),
                        exclude_patterns: None,
                        is_relative_to_user: false,
                        environment_variable: None,
                        priority: 5,
                        detection_method: Some("heuristic".to_string()),
                        community_confirmed: false,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    });
                }
            }
        }

        Ok(locations)
    }

    /// Insert save location into database
    fn insert_save_location(
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
            rusqlite::params![
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

    /// Scan for existing save files
    fn scan_existing_saves(
        tx: &rusqlite::Transaction,
        game_id: i64,
        save_locations: &[SaveLocation],
    ) -> Result<Vec<DetectedSave>, String> {
        let mut detected_saves = Vec::new();

        for location in save_locations {
            // Resolve the actual path (simplified - would need path resolution logic)
            let resolved_paths = Self::resolve_save_paths(location)?;

            for actual_path in resolved_paths {
                if Path::new(&actual_path).exists() {
                    let id = Self::insert_detected_save(tx, game_id, location.id, &actual_path)?;
                    detected_saves.push(DetectedSave {
                        id,
                        game_id,
                        save_location_id: location.id,
                        actual_path,
                        current_hash: None, // Would compute hash
                        file_size: None,    // Would get file size
                        last_modified: Some(Utc::now()),
                        first_detected: Utc::now(),
                        last_checked: Utc::now(),
                        is_active: true,
                        metadata_json: None,
                    });
                }
            }
        }

        Ok(detected_saves)
    }

    /// Resolve save paths from patterns (simplified)
    fn resolve_save_paths(location: &SaveLocation) -> Result<Vec<String>, String> {
        // This would implement path resolution logic
        // For now, return the pattern as-is if it's an absolute path
        if Path::new(&location.path_pattern).is_absolute() {
            Ok(vec![location.path_pattern.clone()])
        } else {
            Ok(vec![])
        }
    }

    /// Insert detected save into database
    fn insert_detected_save(
        tx: &rusqlite::Transaction,
        game_id: i64,
        save_location_id: i64,
        actual_path: &str,
    ) -> Result<i64, String> {
        tx.execute(
            "INSERT INTO detected_saves (game_id, save_location_id, actual_path,
                                        first_detected, last_checked, is_active)
             VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![
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

    /// Get game by ID
    fn get_game_by_id(conn: &rusqlite::Connection, game_id: i64) -> Result<Game, String> {
        let mut stmt = conn.prepare(
            "SELECT id, name, developer, publisher, platform, platform_app_id,
                    executable_path, installation_path, genre, release_date,
                    cover_image_url, created_at, updated_at, is_active
             FROM games WHERE id = ?"
        ).map_err(|e| format!("Prepare statement error: {}", e))?;

        let game = stmt.query_row([game_id], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                developer: row.get(2)?,
                publisher: row.get(3)?,
                platform: row.get(4)?,
                platform_app_id: row.get(5)?,
                executable_path: row.get(6)?,
                installation_path: row.get(7)?,
                genre: row.get(8)?,
                release_date: row.get(9)?,
                cover_image_url: row.get(10)?,
                created_at: Utc::now(), // TODO: Fix DateTime parsing
                updated_at: Utc::now(), // TODO: Fix DateTime parsing
                is_active: row.get(13)?,
            })
        }).map_err(|e| format!("Query game error: {}", e))?;

        Ok(game)
    }
}
