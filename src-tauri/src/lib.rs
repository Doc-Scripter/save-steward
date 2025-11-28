mod database;
mod manifest;
mod detection;
mod auto_backup;
mod game_manager;
mod launch_utils;
mod git_manager;

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
