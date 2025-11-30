use crate::git_manager::GitSaveManager;

#[tauri::command]
pub async fn enable_git_for_game(_game_id: i64) -> Result<String, String> {
    // Ensure database is ready using flag file approach
    let db_conn = crate::database::connection::ensure_database_ready().await?;

    // Initialize Git repository
    let git_manager = GitSaveManager::new(db_conn.clone());
    git_manager.initialize_master_repo().await
        .map_err(|e| format!("Failed to initialize Git repository: {}", e))
}

#[tauri::command]
pub async fn create_save_checkpoint(game_id: i64, message: String) -> Result<String, String> {
    // Ensure database is ready using flag file approach
    let db_conn = crate::database::connection::ensure_database_ready().await?;

    // Create save checkpoint
    let git_manager = GitSaveManager::new(db_conn.clone());
    git_manager.create_save_checkpoint(game_id, &message).await
        .map_err(|e| format!("Failed to create save checkpoint: {}", e))
}

#[tauri::command]
pub async fn create_save_branch(game_id: i64, branch_name: String, description: Option<String>) -> Result<(), String> {
    // Ensure database is ready using flag file approach
    let db_conn = crate::database::connection::ensure_database_ready().await?;

    // Create save branch
    let git_manager = GitSaveManager::new(db_conn.clone());
    git_manager.create_save_branch(game_id, &branch_name, description.as_deref()).await
        .map_err(|e| format!("Failed to create save branch: {}", e))
}

#[tauri::command]
pub async fn switch_save_branch(game_id: i64, branch_name: String) -> Result<(), String> {
    // Ensure database is ready using flag file approach
    let db_conn = crate::database::connection::ensure_database_ready().await?;

    // Switch save branch
    let git_manager = GitSaveManager::new(db_conn.clone());
    git_manager.switch_save_branch(game_id, &branch_name).await
        .map_err(|e| format!("Failed to switch save branch: {}", e))
}

#[tauri::command]
pub async fn restore_to_commit(game_id: i64, commit_hash: String) -> Result<(), String> {
    // Ensure database is ready using flag file approach
    let db_conn = crate::database::connection::ensure_database_ready().await?;

    // Restore to commit
    let git_manager = GitSaveManager::new(db_conn.clone());
    git_manager.restore_to_commit(game_id, &commit_hash).await
        .map_err(|e| format!("Failed to restore to commit: {}", e))
}

#[tauri::command]
pub async fn restore_to_timestamp(game_id: i64, timestamp: String) -> Result<String, String> {
    // Ensure database is ready using flag file approach
    let db_conn = crate::database::connection::ensure_database_ready().await?;

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
pub async fn get_git_history(game_id: i64, _branch: Option<String>) -> Result<serde_json::Value, String> {
    // Ensure database is ready using flag file approach
    let db_conn = crate::database::connection::ensure_database_ready().await?;

    // Get save history
    let git_manager = GitSaveManager::new(db_conn.clone());
    let history = git_manager.get_save_history(game_id).await
        .map_err(|e| format!("Failed to get git history: {}", e))?;

    // Convert to JSON
    serde_json::to_value(history).map_err(|e| format!("Serialization error: {}", e))
}

#[tauri::command]
pub async fn sync_to_cloud(game_id: i64) -> Result<serde_json::Value, String> {
    // Ensure database is ready using flag file approach
    let db_conn = crate::database::connection::ensure_database_ready().await?;

    // Sync to cloud
    let git_manager = GitSaveManager::new(db_conn.clone());
    let sync_result = git_manager.sync_to_cloud(game_id).await
        .map_err(|e| format!("Failed to sync to cloud: {}", e))?;

    // Convert to JSON
    serde_json::to_value(sync_result).map_err(|e| format!("Serialization error: {}", e))
}
