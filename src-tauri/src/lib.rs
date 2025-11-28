mod database;
mod manifest;
mod detection;
mod auto_backup;
mod game_manager;

use crate::database::connection::{EncryptedDatabase, DatabasePaths};
use crate::database::models::AddGameRequest;
use crate::game_manager::GameManager;
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
async fn launch_game(executable_path: String) -> Result<String, String> {
    use std::process::Command;
    
    // Determine the platform and launch accordingly
    #[cfg(target_os = "windows")]
    {
        // Windows: Direct execution
        Command::new(&executable_path)
            .spawn()
            .map_err(|e| format!("Failed to launch game: {}", e))?;
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // Linux/Unix: Use sh -c to handle binaries properly
        // This ensures the executable bit is respected and environment is set up
        Command::new("sh")
            .arg("-c")
            .arg(&executable_path)
            .spawn()
            .map_err(|e| format!("Failed to launch game: {}. Make sure the file has executable permissions (chmod +x)", e))?;
    }
    
    Ok(format!("Launched: {}", executable_path))
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
            launch_game
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
