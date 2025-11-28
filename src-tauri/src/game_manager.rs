use crate::database::{models::*};
use std::path::Path;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use crate::pcgaming_wiki::PcgwClient;

pub struct GameManager;

impl GameManager {
    /// Add a game manually with automatic save location detection
    pub async fn add_manual_game(
        db: &Arc<tokio::sync::Mutex<crate::database::connection::EncryptedDatabase>>,
        request: AddGameRequest,
    ) -> Result<GameWithSaves, String> {
        // 1. Pre-fetch PCGamingWiki data (outside transaction)
        let mut pcgw_save_locations: Option<Vec<SaveLocation>> = None;
        let mut pcgw_response_text: Option<String> = None;
        let cache_key = format!("save_loc:{}", request.name);
        
        // Check cache first
        {
            let conn_guard = db.lock().await;
            let conn = conn_guard.get_connection().await;
            if let Ok(Some(cached_json)) = crate::pcgaming_wiki::cache::PcgwCache::get(&conn, &cache_key) {
                 let client = PcgwClient::new();
                 if let Ok(result) = client.parse_save_locations_json(&cached_json) {
                     // Convert result to SaveLocation objects
                     pcgw_save_locations = Some(Self::convert_pcgw_locations(&result));
                 }
            }
        }
        
        // If not in cache, fetch from API
        if pcgw_save_locations.is_none() {
            let client = PcgwClient::new();
            if let Ok(text) = client.fetch_save_locations_raw(&request.name).await {
                pcgw_response_text = Some(text.clone());
                if let Ok(result) = client.parse_save_locations_json(&text) {
                     pcgw_save_locations = Some(Self::convert_pcgw_locations(&result));
                }
            }
        }

        let conn_guard = db.lock().await;
        let mut conn = conn_guard.get_connection().await;

        // Start transaction
        let tx = conn.transaction().map_err(|e| format!("Transaction error: {}", e))?;

        // Update cache if we fetched new data
        if let Some(text) = pcgw_response_text {
            let _ = crate::pcgaming_wiki::cache::PcgwCache::set(&tx, &cache_key, &text, 7);
        }

        // Insert game
        let game_id = Self::insert_game(&tx, &request)?;

        // Update game with PCGW data if available (including executables)
        if pcgw_save_locations.is_some() {
            // Fetch PCGW executables and update game record
            if let Some(page_name) = Self::extract_pcgw_page_name(&request.name) {
                if let Some(executables_json) = Self::fetch_pcgw_executables(&page_name) {
                    Self::update_game_platform_executables(&tx, game_id, &executables_json)?;
                }
            }
        }

        // Detect and insert save locations (passing pre-fetched data)
        let save_locations = Self::detect_save_locations(&tx, game_id, &request, pcgw_save_locations)?;

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
            "INSERT INTO games (name, platform, platform_app_id,
                              executable_path, installation_path, platform_executables,
                              icon_base64, icon_path, created_at, updated_at, is_active)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
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
        ).map_err(|e| format!("Insert game error: {}", e))?;

        Ok(tx.last_insert_rowid())
    }

    /// Detect save locations for a game
    fn detect_save_locations(
        tx: &rusqlite::Transaction<'_>,
        game_id: i64,
        request: &AddGameRequest,
        pcgw_locations: Option<Vec<SaveLocation>>,
    ) -> Result<Vec<SaveLocation>, String> {
        let mut locations = Vec::new();
        // but our db is Arc<Mutex<EncryptedDatabase>>.
        // We need to refactor PcgwClient or just use a fresh connection for it if possible, 
        // or skip caching for this specific call if it's too complex.
        
        // Actually, PcgwClient::new takes Arc<Mutex<Connection>>. 
        // Our EncryptedDatabase wraps Connection.
        // For now, let's skip the API call inside this transaction to avoid deadlock/complexity 
        // and rely on a separate update step or just use heuristics for now.
        
        // WAIT: We can't easily use PcgwClient here because of the transaction lock.
        // The `add_manual_game` holds a lock on `db`.
        // If `PcgwClient` tries to lock `db` again, it might deadlock if not careful (though reentrant mutex might work).
        // But `EncryptedDatabase` is inside `tokio::sync::Mutex`.
        
        // Alternative: Fetch PCGW data BEFORE starting the transaction in `add_manual_game`.
        
        // Let's revert this change signature and do it in `add_manual_game`.
        
        // ... (keeping original implementation for now, will modify add_manual_game instead)
        
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

    /// Parse timestamp string from database to DateTime
    fn parse_timestamp(timestamp_str: &str) -> Result<DateTime<Utc>, String> {
        DateTime::parse_from_rfc3339(timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| format!("Failed to parse timestamp '{}': {}", timestamp_str, e))
    }

    /// Get game by ID
    fn get_game_by_id(conn: &rusqlite::Connection, game_id: i64) -> Result<Game, String> {
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
        db: &std::sync::Arc<tokio::sync::Mutex<crate::database::connection::EncryptedDatabase>>,
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
        db: &Arc<tokio::sync::Mutex<crate::database::connection::EncryptedDatabase>>,
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
            rusqlite::params![
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

    /// Delete a game and all associated data
    pub async fn delete_game(
        db: &Arc<tokio::sync::Mutex<crate::database::connection::EncryptedDatabase>>,
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

    /// Extract PCGW page name from game name (simplified - could be enhanced)
    fn extract_pcgw_page_name(game_name: &str) -> Option<String> {
        // Convert to PCGW page name format (spaces to underscores, clean special chars)
        let clean_name = game_name.replace(|c: char| !c.is_alphanumeric() && c != ' ', "_");
        Some(clean_name)
    }

    /// Fetch executables from PCGW wikitext
    fn fetch_pcgw_executables(page_name: &str) -> Option<String> {
        use std::process::Command;
        use regex::Regex;

        // Fetch wikitext using curl
        let output = Command::new("curl")
            .arg("-s")
            .arg(format!("https://www.pcgamingwiki.com/w/api.php?action=parse&page={}&prop=wikitext&format=json", page_name))
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
        let wikitext = json["parse"]["wikitext"]["*"].as_str()?;

        // Parse executables from wikitext
        let mut executables = std::collections::HashMap::new();

        // Extract {{file|...}} templates
        let file_regex = Regex::new(r"\{\{file\|([^}]+)\}\}").ok()?;
        let mut file_matches: Vec<String> = file_regex.captures_iter(wikitext)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_lowercase()))
            .collect();

        // Filter for likely executable files
        file_matches.retain(|file| {
            file.ends_with(".exe") ||
            file.ends_with(".sh") ||
            file.ends_with(".bin") ||
            file.ends_with(".run") ||
            file.ends_with(".x86_64") ||
            file.ends_with(".app") ||
            !file.contains(".") // Files without extension (common on Linux)
        });

        // Check platform indicators in wikitext
        let has_linux = wikitext.contains("Linux") || wikitext.contains("linux");
        let has_windows = wikitext.contains("Windows") || wikitext.contains("windows");
        let has_macos = wikitext.contains("OS X") || wikitext.contains("macOS") || wikitext.contains("Mac");

        // Assign executables to platforms (simplified logic)
        let mut unassigned_files = Vec::new();
        for file in &file_matches {
            if file.ends_with(".exe") {
                executables.entry("windows".to_string()).or_insert_with(Vec::new).push(file.clone());
            } else if file.ends_with(".app") {
                executables.entry("macos".to_string()).or_insert_with(Vec::new).push(file.clone());
            } else if file.ends_with(".sh") || file.contains("run") || has_linux {
                executables.entry("linux".to_string()).or_insert_with(Vec::new).push(file.clone());
            } else {
                unassigned_files.push(file.clone());
            }
        }

        // Fallback: if we found likely executables but couldn't assign platforms,
        // assign to current platform
        if executables.is_empty() && !file_matches.is_empty() {
            #[cfg(target_os = "linux")]
            executables.insert("linux".to_string(), file_matches);
            #[cfg(target_os = "windows")]
            executables.insert("windows".to_string(), file_matches);
            #[cfg(target_os = "macos")]
            executables.insert("macos".to_string(), file_matches);
        } else if !unassigned_files.is_empty() {
            // Assign unassigned generic executables to current platform
            #[cfg(target_os = "linux")]
            executables.entry("linux".to_string()).or_insert_with(Vec::new).extend(unassigned_files);
            #[cfg(target_os = "windows")]
            executables.entry("windows".to_string()).or_insert_with(Vec::new).extend(unassigned_files);
            #[cfg(target_os = "macos")]
            executables.entry("macos".to_string()).or_insert_with(Vec::new).extend(unassigned_files);
        }

        // Convert to JSON string
        serde_json::to_string(&executables).ok()
    }

    /// Update game with platform executables
    fn update_game_platform_executables(tx: &rusqlite::Transaction, game_id: i64, executables_json: &str) -> Result<(), String> {
        tx.execute(
            "UPDATE games SET platform_executables = ?, updated_at = ? WHERE id = ?",
            rusqlite::params![
                executables_json,
                Utc::now().to_rfc3339(),
                game_id,
            ],
        ).map_err(|e| format!("Update game executables error: {}", e))?;

        Ok(())
    }

    /// Get current system platform string
    pub fn get_current_platform() -> &'static str {
        #[cfg(target_os = "linux")]
        { "linux" }
        #[cfg(target_os = "windows")]
        { "windows" }
        #[cfg(target_os = "macos")]
        { "macos" }
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        { "unknown" }
    }

    /// Get executable path for current platform from stored data
    pub fn get_platform_executable(game: &Game) -> Option<String> {
        let platform = Self::get_current_platform();

        if let Some(executables_json) = &game.platform_executables {
            if let Ok(executables) = serde_json::from_str::<std::collections::HashMap<String, Vec<String>>>(executables_json) {
                if let Some(platform_files) = executables.get(platform) {
                    // Return first executable for this platform
                    platform_files.first().cloned()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn convert_pcgw_locations(result: &crate::pcgaming_wiki::models::SaveLocationResult) -> Vec<SaveLocation> {
        let mut locations = Vec::new();

        // Windows paths
        for path in &result.windows {
            locations.push(SaveLocation {
                id: 0,
                game_id: 0,
                path_pattern: path.clone(),
                path_type: "directory".to_string(),
                platform: Some("windows".to_string()),
                save_type: "auto".to_string(),
                file_patterns: Some(r#"["*"]"#.to_string()), // Default to all files
                exclude_patterns: None,
                is_relative_to_user: false, // Paths are already resolved
                environment_variable: None,
                priority: 8,
                detection_method: Some("pcgamingwiki".to_string()),
                community_confirmed: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
        }

        // Linux paths
        for path in &result.linux {
            locations.push(SaveLocation {
                id: 0,
                game_id: 0,
                path_pattern: path.clone(),
                path_type: "directory".to_string(),
                platform: Some("linux".to_string()),
                save_type: "auto".to_string(),
                file_patterns: Some(r#"["*"]"#.to_string()),
                exclude_patterns: None,
                is_relative_to_user: false,
                environment_variable: None,
                priority: 8,
                detection_method: Some("pcgamingwiki".to_string()),
                community_confirmed: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
        }

        locations
    }
}
