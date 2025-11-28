mod database;
mod manifest;
mod detection;
mod auto_backup;
mod game_manager;
mod launch_utils;

use crate::database::connection::{EncryptedDatabase, DatabasePaths};
use crate::database::models::AddGameRequest;
use crate::game_manager::GameManager;
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
            launch_game
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
