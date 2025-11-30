use crate::database::models::AddGameRequest;
use crate::game_manager::GameManager;
use crate::pcgaming_wiki::PcgwClient;
use std::sync::Arc;

#[tauri::command]
pub async fn add_manual_game(request: AddGameRequest) -> Result<serde_json::Value, String> {
    // Ensure database is ready using flag file approach
    let db_conn = crate::database::connection::ensure_database_ready().await?;

    // Add the game
    let result = GameManager::add_manual_game(&db_conn, request).await?;

    Ok(serde_json::to_value(result).map_err(|e| format!("Serialization error: {}", e))?)
}

#[tauri::command]
pub async fn add_manual_game_sync(request: AddGameRequest) -> Result<serde_json::Value, String> {
    // Ensure database is ready using flag file approach
    let db_conn = match crate::database::connection::ensure_database_ready().await {
        Ok(conn) => conn,
        Err(e) => {
            crate::logger::error("GAME_COMMAND", "Failed to ensure database ready", Some(&e));
            return Err(format!("Database unavailable: {}", e));
        }
    };

    // Add the game
    let result = match GameManager::add_manual_game(&db_conn, request).await {
        Ok(r) => r,
        Err(e) => {
            crate::logger::error("GAME_COMMAND", "Failed to add manual game", Some(&e));
            return Err(e);
        }
    };

    match serde_json::to_value(result) {
        Ok(v) => Ok(v),
        Err(e) => {
            crate::logger::error("GAME_COMMAND", "Failed to serialize game result", Some(&e.to_string()));
            Err(format!("Serialization error: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_all_games() -> Result<serde_json::Value, String> {
    // Ensure database is ready using flag file approach
    let db_conn = match crate::database::connection::ensure_database_ready().await {
        Ok(conn) => conn,
        Err(e) => {
            crate::logger::error("GAME_COMMAND", "Failed to ensure database ready for get_all_games", Some(&e));
            return Err(format!("Database unavailable: {}", e));
        }
    };

    // Get all games
    let games = match GameManager::get_all_games(&db_conn).await {
        Ok(g) => g,
        Err(e) => {
            crate::logger::error("GAME_COMMAND", "Failed to get all games", Some(&e));
            return Err(e);
        }
    };

    match serde_json::to_value(games) {
        Ok(v) => Ok(v),
        Err(e) => {
            crate::logger::error("GAME_COMMAND", "Failed to serialize games list", Some(&e.to_string()));
            Err(format!("Serialization error: {}", e))
        }
    }
}

#[tauri::command]
pub async fn update_game_sync(game_id: i64, request: AddGameRequest) -> Result<serde_json::Value, String> {
    // Ensure database is ready using flag file approach
    let db_conn = match crate::database::connection::ensure_database_ready().await {
        Ok(conn) => conn,
        Err(e) => {
            crate::logger::error("GAME_COMMAND", "Failed to ensure database ready for update_game", Some(&e));
            return Err(format!("Database unavailable: {}", e));
        }
    };

    // Update the game
    let result = match GameManager::update_game(&db_conn, game_id, request).await {
        Ok(r) => r,
        Err(e) => {
            crate::logger::error("GAME_COMMAND", &format!("Failed to update game {}", game_id), Some(&e));
            return Err(e);
        }
    };

    match serde_json::to_value(result) {
        Ok(v) => Ok(v),
        Err(e) => {
            crate::logger::error("GAME_COMMAND", "Failed to serialize updated game", Some(&e.to_string()));
            Err(format!("Serialization error: {}", e))
        }
    }
}

#[tauri::command]
pub async fn delete_game_sync(game_id: i64) -> Result<(), String> {
    // Ensure database is ready using flag file approach
    let db_conn = match crate::database::connection::ensure_database_ready().await {
        Ok(conn) => conn,
        Err(e) => {
            crate::logger::error("GAME_COMMAND", "Failed to ensure database ready for delete_game", Some(&e));
            return Err(format!("Database unavailable: {}", e));
        }
    };

    // Delete the game
    match GameManager::delete_game(&db_conn, game_id).await {
        Ok(()) => Ok(()),
        Err(e) => {
            crate::logger::error("GAME_COMMAND", &format!("Failed to delete game {}", game_id), Some(&e));
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn search_pcgw_games(query: String) -> Result<serde_json::Value, String> {
    // Ensure database is ready using flag file approach
    let db_conn = crate::database::connection::ensure_database_ready().await?;
    
    let client = PcgwClient::new();
    let cache_key = format!("search:{}", query);
    
    // 1. Check cache
    {
        let conn_guard = db_conn.lock().await;
        let conn = conn_guard.get_connection().await;
        // PcgwCache::get is sync, so this is fine
        if let Ok(Some(cached_json)) = crate::pcgaming_wiki::cache::PcgwCache::get(&conn, &cache_key) {
            let _response: crate::pcgaming_wiki::models::CargoQueryResponse<crate::pcgaming_wiki::models::PcgwGameInfo> = serde_json::from_str(&cached_json).map_err(|e| e.to_string())?;
            // Logic duplicated from lib.rs for now as map_search_results is not easily accessible without more refactoring
            // Ideally this logic should be in PcgwClient
        }
    }
    
    // 2. Fetch from API (no lock)
    let response_text = client.fetch_search_results_raw(&query).await.map_err(|e| e.to_string())?;
    
    // 3. Cache response
    {
        let conn_guard = db_conn.lock().await;
        let conn = conn_guard.get_connection().await;
        let _ = crate::pcgaming_wiki::cache::PcgwCache::set(&conn, &cache_key, &response_text, 1);
    }
    
    // 4. Parse and return
    let response: crate::pcgaming_wiki::models::CargoQueryResponse<crate::pcgaming_wiki::models::PcgwGameInfo> = serde_json::from_str(&response_text).map_err(|e| e.to_string())?;
    let results: Vec<crate::pcgaming_wiki::models::GameSearchResult> = response.cargoquery.into_iter().map(|item| {
            let info = item.title;

            // Parse Steam AppID - take the first one (main game, not DLC)
            let steam_id = if let Some(appids_str) = &info.steam_appid {
                let main_appid = appids_str.split(',').next().unwrap_or("").trim();
                if main_appid.is_empty() {
                    None
                } else {
                    Some(main_appid.to_string())
                }
            } else {
                None
            };

            crate::pcgaming_wiki::models::GameSearchResult {
                // Use search query as name (could be improved with page name lookup)
                name: query.clone(),
                steam_id,
                publishers: info.publishers,
                cover_image_url: None, // Will be populated separately via wikitext
            }
    }).collect();
    
    println!("[DEBUG] PGWK Search Results: {:?}", results);
    
    Ok(serde_json::to_value(results).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn get_pcgw_save_locations(game_name: String) -> Result<serde_json::Value, String> {
    // Ensure database is ready using flag file approach
    let db_conn = crate::database::connection::ensure_database_ready().await?;
    
    let client = PcgwClient::new();
    let cache_key = format!("save_loc:{}", game_name);
    
    // 1. Check cache
    {
        let conn_guard = db_conn.lock().await;
        let conn = conn_guard.get_connection().await;
        if let Ok(Some(cached_json)) = crate::pcgaming_wiki::cache::PcgwCache::get(&conn, &cache_key) {
             let result = client.parse_save_locations_json(&cached_json).map_err(|e| e.to_string())?;
             return Ok(serde_json::to_value(result).map_err(|e| e.to_string())?);
        }
    }
    
    // 2. Fetch from API
    let response_text = client.fetch_save_locations_raw(&game_name).await.map_err(|e| e.to_string())?;
    
    // 3. Cache response
    {
        let conn_guard = db_conn.lock().await;
        let conn = conn_guard.get_connection().await;
        let _ = crate::pcgaming_wiki::cache::PcgwCache::set(&conn, &cache_key, &response_text, 7);
    }
    
    // 4. Parse and return
    let result = client.parse_save_locations_json(&response_text).map_err(|e| e.to_string())?;
    
    Ok(serde_json::to_value(result).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn detect_game_executable(folder_path: String, game_name: String) -> Result<String, String> {
    // First try to find stored executable data
    let db_path = crate::database::connection::DatabasePaths::database_file();
    if let Ok(db) = crate::database::connection::Database::new(&db_path).await {
        let db_conn = std::sync::Arc::new(tokio::sync::Mutex::new(db));

        // Get all games and find one that matches
        if let Ok(games) = crate::game_manager::GameManager::get_all_games(&db_conn).await {
            // Try exact name match first, then partial match
            let matching_game = games.iter()
                .find(|g| g.name.to_lowercase() == game_name.to_lowercase())
                .or_else(|| {
                    let normalized_search = game_name.to_lowercase()
                        .replace(|c: char| !c.is_alphanumeric(), "");
                    games.iter().find(|g| {
                        let normalized_game = g.name.to_lowercase()
                            .replace(|c: char| !c.is_alphanumeric(), "");
                        normalized_game.contains(&normalized_search) || normalized_search.contains(&normalized_game)
                    })
                });

            if let Some(game) = matching_game {
                // Try to get platform-specific executable first
                if let Some(platform_exe) = crate::game_manager::GameManager::get_platform_executable(game) {
                    // Combine with installation path if available
                    let base_path = if let Some(install_path) = &game.installation_path {
                        std::path::Path::new(install_path)
                    } else {
                        std::path::Path::new(&folder_path)
                    };

                    let exe_path = base_path.join(platform_exe);
                    if exe_path.exists() {
                        return Ok(exe_path.to_string_lossy().to_string());
                    }
                }

                // Fallback to legacy executable_path
                if let Some(legacy_exe) = &game.executable_path {
                    if !legacy_exe.is_empty() {
                        let exe_path = std::path::Path::new(legacy_exe);
                        if exe_path.exists() {
                            return Ok(legacy_exe.clone());
                        } else {
                            // Try relative to folder_path
                            let relative_path = std::path::Path::new(&folder_path).join(legacy_exe);
                            if relative_path.exists() {
                                return Ok(relative_path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: directory scanning with OS-aware logic
    detect_executable_in_directory(folder_path, game_name) // Pass arguments directly
}

// Helper function, not a command itself, but used by detect_game_executable
// We can make it public if needed, or just keep it private here.
// Wait, lib.rs had it as a separate function but not a command? No, it wasn't marked #[tauri::command].
// But detect_game_executable calls it.
fn detect_executable_in_directory(folder_path: String, game_name: String) -> Result<String, String> {
    use std::os::unix::fs::PermissionsExt;
    
    let path = std::path::Path::new(&folder_path);
    if !path.exists() || !path.is_dir() {
        return Err("Invalid folder path".to_string());
    }

    let mut best_match: Option<String> = None;
    let mut any_executable: Option<String> = None;

    // Normalize game name for matching
    let normalized_name = game_name.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "");

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let file_name = entry_path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let is_executable = {
                #[cfg(target_os = "windows")]
                {
                    // On Windows, check for .exe extension
                    entry_path.extension()
                        .map(|ext| ext.to_string_lossy().to_lowercase() == "exe")
                        .unwrap_or(false)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    // On Unix-like systems, check if it's a file first, then check permissions
                    // This prevents directories with execute bit from being detected
                    if entry_path.is_file() {
                        if let Ok(metadata) = entry_path.metadata() {
                            let permissions = metadata.permissions();
                            permissions.mode() & 0o111 != 0 // Check if any execute bit is set
                        } else {
                            false
                        }
                    } else {
                        false  // Skip directories entirely
                    }
                }
            };

            if is_executable {
                let full_path = entry_path.to_string_lossy().to_string();
                let normalized_file = file_name.to_lowercase()
                    .replace(|c: char| !c.is_alphanumeric(), "");

                // Skip shared libraries and other non-game files
                let is_library = file_name.to_lowercase().ends_with(".so") ||
                                file_name.to_lowercase().contains(".so.") ||
                                file_name.to_lowercase().ends_with(".dll") ||
                                file_name.to_lowercase().ends_with(".dylib");
                
                if is_library {
                    continue; // Skip libraries
                }

                // Check for game name match (higher priority)
                if normalized_file.contains(&normalized_name) || normalized_name.contains(&normalized_file) {
                    best_match = Some(full_path.clone());
                    break; // Found a good match
                }

                // Also check common executable patterns
                let is_common_exe = {
                    #[cfg(target_os = "windows")]
                    {
                        file_name.to_lowercase().ends_with(".exe") ||
                        file_name.to_lowercase().ends_with(".bat") ||
                        file_name.to_lowercase().ends_with(".cmd")
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        // Prioritize specific executable extensions
                        file_name.to_lowercase().ends_with(".x86_64") ||
                        file_name.to_lowercase().ends_with(".x86") ||
                        file_name.to_lowercase().ends_with(".sh") ||
                        file_name.to_lowercase().ends_with(".bin") ||
                        file_name.to_lowercase().ends_with(".run") ||
                        file_name == "run" ||
                        file_name.starts_with("start") ||
                        file_name.starts_with("launch") ||
                        (!file_name.contains(".") && entry_path.is_file()) // Files without extension (but not directories)
                    }
                };

                if is_common_exe && any_executable.is_none() {
                    any_executable = Some(full_path);
                }
            }
        }
    }

    match best_match.or(any_executable) {
        Some(p) => Ok(p),
        None => Err("No executable found in directory".to_string())
    }
}
