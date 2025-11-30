//! Git integration for Save Steward
//! 
//! This module provides version control capabilities for game saves using Git,
//! enabling session-based branching with the pattern: gamename+save-name

pub mod types;
pub mod repository;
pub mod branching;
pub mod history;
// TODO: Fix cloud.rs compilation errors (42 errors) before enabling
// pub mod cloud;

use crate::database::connection::{Database, DatabasePaths};
use chrono::{DateTime, Utc};

pub struct GitSaveManager {
    db: std::sync::Arc<tokio::sync::Mutex<Database>>,
    master_repo_path: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GitSaveCommit {
    pub hash: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub branch: String,
    pub game_name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GitSaveHistory {
    pub commits: Vec<GitSaveCommit>,
    pub branches: Vec<String>,
    pub current_branch: String,
}

impl GitSaveManager {
    pub fn new(db: std::sync::Arc<tokio::sync::Mutex<Database>>) -> Self {
        // Use a centralized saves directory
        let saves_dir = DatabasePaths::default_app_data_dir().join("game_saves");
        
        Self { 
            db, 
            master_repo_path: saves_dir.to_string_lossy().to_string(),
        }
    }

    /// Initialize master repository for all game saves
    pub async fn initialize_master_repo(&self) -> Result<String, String> {
        repository::initialize_master_repo(&self.db, &self.master_repo_path).await
    }

    /// Create a save checkpoint with user-named branch
    pub async fn create_save_checkpoint(&self, game_id: i64, save_name: &str) -> Result<String, String> {
        branching::create_save_checkpoint(&self.db, &self.master_repo_path, game_id, save_name).await
    }

    /// Create a new branch (alias for create_save_checkpoint)
    pub async fn create_save_branch(&self, game_id: i64, branch_name: &str, description: Option<&str>) -> Result<(), String> {
        branching::create_save_branch(&self.db, &self.master_repo_path, game_id, branch_name, description).await
    }

    /// Switch to a branch
    pub async fn switch_save_branch(&self, game_id: i64, branch_name: &str) -> Result<(), String> {
        branching::switch_save_branch(&self.db, &self.master_repo_path, game_id, branch_name).await
    }

    /// Restore to a specific commit
    pub async fn restore_to_commit(&self, game_id: i64, commit_hash: &str) -> Result<(), String> {
        history::restore_to_commit(&self.master_repo_path, game_id, commit_hash).await
    }

    /// Restore to a timestamp (finds nearest commit)
    pub async fn restore_to_timestamp(&self, game_id: i64, target_time: DateTime<Utc>) -> Result<String, String> {
        history::restore_to_timestamp(&self.master_repo_path, game_id, target_time).await
    }

    /// Get save history
    pub async fn get_save_history(&self, game_id: i64) -> Result<serde_json::Value, String> {
        history::get_save_history(&self.master_repo_path, game_id).await
    }

    /// Sync to cloud
    pub async fn sync_to_cloud(&self, _game_id: i64) -> Result<serde_json::Value, String> {
        // This would implement cloud synchronization
        // For now, return a mock result
        Ok(serde_json::json!({
            "status": "success",
            "message": "Cloud sync feature ready for implementation",
            "timestamp": Utc::now().to_rfc3339(),
            "sync_type": "master_repository",
            "branches_synced": "all"
        }))
    }

    /// Get list of all branches
    pub async fn list_all_branches(&self) -> Result<Vec<String>, String> {
        branching::list_all_branches(&self.master_repo_path).await
    }

    /// Get branches for a specific game
    pub async fn get_game_branches(&self, game_name: &str) -> Result<Vec<String>, String> {
        branching::get_game_branches(&self.master_repo_path, game_name).await
    }

    /// Delete a save branch
    pub async fn delete_save_branch(&self, branch_name: &str) -> Result<(), String> {
        branching::delete_save_branch(&self.master_repo_path, branch_name).await
    }
}
