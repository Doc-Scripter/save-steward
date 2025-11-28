mod database;
mod manifest;
mod detection;
mod auto_backup;
mod game_manager;
mod launch_utils;
mod git_manager;
mod pcgaming_wiki;

use crate::pcgaming_wiki::PcgwClient;

use crate::database::connection::{EncryptedDatabase, DatabasePaths};
use crate::database::models::AddGameRequest;
use crate::game_manager::GameManager;
use crate::git_manager::{GitSaveManager, types::*};
use crate::launch_utils::launch_game_enhanced;
use std::sync::Arc;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn identify_game_by_pid(pid: u32) -> Result<serde_json::Value, String> {
    // Initialize database connection and manifest resolver
    // This would normally be injected as a dependency
    // For now, return a placeholder response
    Ok(serde_json::json!({
        "identified": false,
        "message": "Game identification engine initialized but not fully integrated with Tauri commands yet",
        "pid": pid
    }))
}

#[tauri::command]
async fn scan_running_games() -> Result<serde_json::Value, String> {
    // Placeholder implementation
    Ok(serde_json::json!({
        "running_games": [],
        "message": "Game scanning functionality not yet connected to frontend"
    }))
}

#[tauri::command]
async fn add_manual_game(request: AddGameRequest) -> Result<serde_json::Value, String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    // Ensure schema is initialized
    db.initialize_schema()
        .await
        .map_err(|e| format!("Schema initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Add the game
    let result = GameManager::add_manual_game(&db_conn, request).await?;

    Ok(serde_json::to_value(result).map_err(|e| format!("Serialization error: {}", e))?)
}

#[tauri::command]
async fn add_manual_game_sync(request: AddGameRequest) -> Result<serde_json::Value, String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    // Ensure schema is initialized
    db.initialize_schema()
        .await
        .map_err(|e| format!("Schema initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Add the game
    let result = GameManager::add_manual_game(&db_conn, request).await?;

    Ok(serde_json::to_value(result).map_err(|e| format!("Serialization error: {}", e))?)
}

#[tauri::command]
async fn get_all_games() -> Result<serde_json::Value, String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Get all games
    let games = GameManager::get_all_games(&db_conn).await?;

    Ok(serde_json::to_value(games).map_err(|e| format!("Serialization error: {}", e))?)
}

#[tauri::command]
async fn launch_game(executable_path: String, installation_path: Option<String>) -> Result<String, String> {
    // For Unity games and other complex launch scenarios, we need to use the installation directory
    // Try to get the installation directory from the parameter or parse from executable path
    
    let install_dir = if let Some(install_path) = installation_path {
        if !install_path.is_empty() {
            install_path
        } else {
            // Parse from executable path
            get_install_dir_from_executable(&executable_path)
        }
    } else {
        // Parse from executable path
        get_install_dir_from_executable(&executable_path)
    };

    // Use the enhanced game launcher
    match launch_game_enhanced(&install_dir, &executable_path).await {
        Ok(result) => Ok(result),
        Err(e) => {
            // Fallback to the original simple launcher if enhanced version fails
            println!("Enhanced launcher failed: {}, falling back to basic launcher", e);
            use std::process::Command;
            
            #[cfg(target_os = "windows")]
            {
                Command::new(&executable_path)
                    .spawn()
                    .map_err(|err| format!("Failed to launch game: {}", err))?;
            }
            
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("sh")
                    .arg("-c")
                    .arg(&executable_path)
                    .spawn()
                    .map_err(|err| format!("Failed to launch game: {}. Make sure the file has executable permissions (chmod +x)", err))?;
            }
            
            Ok(format!("Launched: {}", executable_path))
        }
    }
}

fn get_install_dir_from_executable(executable_path: &str) -> String {
    if let Some(parent) = std::path::Path::new(executable_path).parent() {
        if parent.components().count() > 1 {
            // If executable is in a subdirectory, use the parent directory as install dir
            parent.to_string_lossy().to_string()
        } else {
            // If executable is in root or no parent, use current directory as fallback
            ".".to_string()
        }
    } else {
        ".".to_string()
    }
}

#[tauri::command]
async fn update_game_sync(game_id: i64, request: AddGameRequest) -> Result<serde_json::Value, String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Update the game
    let result = GameManager::update_game(&db_conn, game_id, request).await?;

    Ok(serde_json::to_value(result).map_err(|e| format!("Serialization error: {}", e))?)
}

#[tauri::command]
async fn delete_game_sync(game_id: i64) -> Result<(), String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Delete the game
    GameManager::delete_game(&db_conn, game_id).await?;

    Ok(())
}

#[tauri::command]
async fn enable_git_for_game(game_id: i64) -> Result<String, String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Initialize Git repository
    let git_manager = GitSaveManager::new(db_conn.clone());
    git_manager.initialize_game_repo(game_id).await
        .map_err(|e| format!("Failed to initialize Git repository: {}", e))
}

#[tauri::command]
async fn create_save_checkpoint(game_id: i64, message: String) -> Result<String, String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Create save checkpoint
    let git_manager = GitSaveManager::new(db_conn.clone());
    git_manager.create_save_checkpoint(game_id, &message).await
        .map_err(|e| format!("Failed to create save checkpoint: {}", e))
}

#[tauri::command]
async fn create_save_branch(game_id: i64, branch_name: String, description: Option<String>) -> Result<(), String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Create save branch
    let git_manager = GitSaveManager::new(db_conn.clone());
    git_manager.create_save_branch(game_id, &branch_name, description.as_deref()).await
        .map_err(|e| format!("Failed to create save branch: {}", e))
}

#[tauri::command]
async fn switch_save_branch(game_id: i64, branch_name: String) -> Result<(), String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Switch save branch
    let git_manager = GitSaveManager::new(db_conn.clone());
    git_manager.switch_save_branch(game_id, &branch_name).await
        .map_err(|e| format!("Failed to switch save branch: {}", e))
}

#[tauri::command]
async fn restore_to_commit(game_id: i64, commit_hash: String) -> Result<(), String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Restore to commit
    let git_manager = GitSaveManager::new(db_conn.clone());
    git_manager.restore_to_commit(game_id, &commit_hash).await
        .map_err(|e| format!("Failed to restore to commit: {}", e))
}

#[tauri::command]
async fn restore_to_timestamp(game_id: i64, timestamp: String) -> Result<String, String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Parse timestamp
    let target_time = chrono::DateTime::parse_from_rfc3339(&timestamp)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| format!("Invalid timestamp format: {}", e))?;

    // Restore to timestamp
    let git_manager = GitSaveManager::new(db_conn.clone());
    git_manager.restore_to_timestamp(game_id, target_time).await
        .map_err(|e| format!("Failed to restore to timestamp: {}", e))
}

#[tauri::command]
async fn get_git_history(game_id: i64, branch: Option<String>) -> Result<serde_json::Value, String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Get save history
    let git_manager = GitSaveManager::new(db_conn.clone());
    let history = git_manager.get_save_history(game_id).await
        .map_err(|e| format!("Failed to get git history: {}", e))?;

    // Convert to JSON
    serde_json::to_value(history).map_err(|e| format!("Serialization error: {}", e))
}

#[tauri::command]
async fn sync_to_cloud(game_id: i64) -> Result<serde_json::Value, String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));

    // Sync to cloud
    let git_manager = GitSaveManager::new(db_conn.clone());
    let sync_result = git_manager.sync_to_cloud(game_id).await
        .map_err(|e| format!("Failed to sync to cloud: {}", e))?;

    // Convert to JSON
    serde_json::to_value(sync_result).map_err(|e| format!("Serialization error: {}", e))
}

#[tauri::command]
async fn search_pcgw_games(query: String) -> Result<serde_json::Value, String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));
    
    let client = PcgwClient::new();
    // We need to access the inner Arc<Mutex<Connection>> from EncryptedDatabase
    // But EncryptedDatabase doesn't expose it directly as Arc<Mutex<Connection>>.
    // It seems I made a mistake assuming I could easily get it.
    // EncryptedDatabase wraps Connection directly, not Arc<Mutex<Connection>>.
    // So I can't pass it to PcgwClient which expects Arc<Mutex<Connection>>.
    
    // WORKAROUND: For now, I will modify PcgwClient to accept Arc<Mutex<EncryptedDatabase>>? 
    // No, that creates circular dependency.
    
    // I must expose the connection from EncryptedDatabase or change PcgwClient to take something else.
    // Since I am in lib.rs, I have control.
    
    // Let's assume for a moment I can get a connection.
    // Actually, EncryptedDatabase is defined in crate::database::connection.
    // I should check if I can clone the connection or something.
    // But rusqlite Connection is not cloneable.
    
    // Okay, I will modify PcgwClient to take `&Connection` for the high-level methods too, 
    // and handle the locking in lib.rs.
    
    // Wait, I can't pass &Connection to async fn in lib.rs either!
    
    // This is the same problem as before.
    
    // The only way is to use the "manual caching" pattern in lib.rs too.
    
    let client = PcgwClient::new();
    let cache_key = format!("search:{}", query);
    
    // 1. Check cache
    {
        let conn_guard = db_conn.lock().await;
        let conn = conn_guard.get_connection();
        // PcgwCache::get is sync, so this is fine
        if let Ok(Some(cached_json)) = crate::pcgaming_wiki::cache::PcgwCache::get(conn, &cache_key) {
             let response: crate::pcgaming_wiki::models::CargoQueryResponse<crate::pcgaming_wiki::models::PcgwGameInfo> = serde_json::from_str(&cached_json).map_err(|e| e.to_string())?;
             // I need to access map_search_results but it's private or I need to make it public
             // or just duplicate logic here.
             // Let's make map_search_results public in client.rs? 
             // Or just use client.parse_search_results_json (I need to add this).
             
             // Let's assume I added parse_search_results_json to client.rs
             // return Ok(serde_json::to_value(client.parse_search_results_json(&cached_json).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?);
        }
    }
    
    // 2. Fetch from API (no lock)
    let response_text = client.fetch_search_results_raw(&query).await.map_err(|e| e.to_string())?;
    
    // 3. Cache response
    {
        let conn_guard = db_conn.lock().await;
        let conn = conn_guard.get_connection();
        let _ = crate::pcgaming_wiki::cache::PcgwCache::set(conn, &cache_key, &response_text, 1);
    }
    
    // 4. Parse and return
    // I need a helper to parse.
    // I'll add `parse_search_results_json` to PcgwClient.
    
    // For now, I'll just do it manually here to save time on another edit to client.rs
    let response: crate::pcgaming_wiki::models::CargoQueryResponse<crate::pcgaming_wiki::models::PcgwGameInfo> = serde_json::from_str(&response_text).map_err(|e| e.to_string())?;
    let results: Vec<crate::pcgaming_wiki::models::GameSearchResult> = response.cargoquery.into_iter().map(|item| {
            let info = item.title;
            crate::pcgaming_wiki::models::GameSearchResult {
                name: info.page_name,
                steam_id: info.steam_appid,
                developers: info.developers,
                publishers: info.publishers,
            }
    }).collect();
    
    Ok(serde_json::to_value(results).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn get_pcgw_save_locations(game_name: String) -> Result<serde_json::Value, String> {
    // Initialize database connection
    let db_path = DatabasePaths::database_file();
    let db = EncryptedDatabase::new(&db_path, "default_password")
        .await
        .map_err(|e| format!("Database initialization error: {}", e))?;

    let db_conn = Arc::new(tokio::sync::Mutex::new(db));
    
    let client = PcgwClient::new();
    let cache_key = format!("save_loc:{}", game_name);
    
    // 1. Check cache
    {
        let conn_guard = db_conn.lock().await;
        let conn = conn_guard.get_connection();
        if let Ok(Some(cached_json)) = crate::pcgaming_wiki::cache::PcgwCache::get(conn, &cache_key) {
             let result = client.parse_save_locations_json(&cached_json).map_err(|e| e.to_string())?;
             return Ok(serde_json::to_value(result).map_err(|e| e.to_string())?);
        }
    }
    
    // 2. Fetch from API
    let response_text = client.fetch_save_locations_raw(&game_name).await.map_err(|e| e.to_string())?;
    
    // 3. Cache response
    {
        let conn_guard = db_conn.lock().await;
        let conn = conn_guard.get_connection();
        let _ = crate::pcgaming_wiki::cache::PcgwCache::set(conn, &cache_key, &response_text, 7);
    }
    
    // 4. Parse and return
    let result = client.parse_save_locations_json(&response_text).map_err(|e| e.to_string())?;
    
    Ok(serde_json::to_value(result).map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn detect_game_executable(folder_path: String, game_name: String) -> Result<String, String> {
    let path = std::path::Path::new(&folder_path);
    if !path.exists() || !path.is_dir() {
        return Err("Invalid folder path".to_string());
    }

    // Simple heuristic: look for .exe files
    // Bonus: try to match game name
    let mut best_match: Option<String> = None;
    let mut any_exe: Option<String> = None;
    
    // Normalize game name for matching (remove special chars, lowercase)
    let normalized_name = game_name.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "");

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "exe" {
                    let file_name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let normalized_file = file_name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
                    
                    let full_path = path.to_string_lossy().to_string();
                    
                    // Exact match or contains
                    if normalized_file.contains(&normalized_name) || normalized_name.contains(&normalized_file) {
                        best_match = Some(full_path.clone());
                        break; // Found a good candidate
                    }
                    
                    if any_exe.is_none() {
                        any_exe = Some(full_path);
                    }
                }
            }
        }
    }

    match best_match.or(any_exe) {
        Some(p) => Ok(p),
        None => Err("No executable found in directory".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            identify_game_by_pid,
            scan_running_games,
            add_manual_game_sync,
            get_all_games,
            update_game_sync,
            delete_game_sync,
            delete_game_sync,
            launch_game,
            search_pcgw_games,
            get_pcgw_save_locations,
            detect_game_executable
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
