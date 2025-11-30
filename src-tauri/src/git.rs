//! Legacy Git repository management for Save Steward
//! 
//! DEPRECATED: This module has been replaced by git_manager.
//! The git_manager module provides the modern master repository approach.
//! 
//! This file is kept for reference but will be removed in a future version.
//! All new development should use git_manager instead.

use std::collections::HashMap;

/// Legacy GitRepositoryManager - replaced by git_manager::GitSaveManager
/// 
/// This struct managed individual repositories per game.
/// 
/// **Migration Notice:**
/// - Use git_manager::GitSaveManager for new development
/// - Master repository approach is more efficient
/// - Branch naming changed from "gamename+save-name" to "gamename-save-name"
/// - Single .git folder for all games instead of multiple repositories
/// 
/// @deprecated
pub struct GitRepositoryManager {
    db: std::sync::Arc<tokio::sync::Mutex<super::database::connection::Database>>,
    repo_cache: HashMap<i64, git2::Repository>,
    base_repo_path: std::path::PathBuf,
}

/// @deprecated - Use git_manager::GitSaveManager::initialize_master_repo instead
impl GitRepositoryManager {
    pub fn new(_db: std::sync::Arc<tokio::sync::Mutex<super::database::connection::Database>>) -> Self {
        panic!("GitRepositoryManager is deprecated. Use git_manager::GitSaveManager instead.");
    }
    
    pub async fn initialize_repository(&self, _game_id: i64) -> Result<String, anyhow::Error> {
        panic!("GitRepositoryManager::initialize_repository is deprecated. Use git_manager::GitSaveManager::initialize_master_repo instead.");
    }
    
    pub async fn get_repository(&mut self, _game_id: i64) -> Result<&git2::Repository> {
        panic!("GitRepositoryManager::get_repository is deprecated. Use git_manager::GitSaveManager instead.");
    }
    
    pub async fn commit_save_file(
        &mut self, 
        _game_id: i64, 
        _filename: &str, 
        _data: &[u8], 
        _message: &str
    ) -> Result<String, anyhow::Error> {
        panic!("GitRepositoryManager::commit_save_file is deprecated. Use git_manager::GitSaveManager instead.");
    }
    
    pub async fn create_branch(&mut self, _game_id: i64, _branch_name: &str, _description: Option<&str>) -> Result<(), anyhow::Error> {
        panic!("GitRepositoryManager::create_branch is deprecated. Use git_manager::GitSaveManager::create_save_branch instead.");
    }
    
    pub async fn switch_branch(&mut self, _game_id: i64, _branch_name: &str) -> Result<(), anyhow::Error> {
        panic!("GitRepositoryManager::switch_branch is deprecated. Use git_manager::GitSaveManager::switch_save_branch instead.");
    }
    
    pub async fn get_commit_history(&mut self, _game_id: i64) -> Result<Vec<super::git_manager::types::GitCommitInfo>, anyhow::Error> {
        panic!("GitRepositoryManager::get_commit_history is deprecated. Use git_manager::GitSaveManager::get_save_history instead.");
    }
    
    pub async fn get_commit_data(&mut self, _game_id: i64, _commit_hash: &str) -> Result<Vec<u8>, anyhow::Error> {
        panic!("GitRepositoryManager::get_commit_data is deprecated. Use git_manager::GitSaveManager::restore_to_commit instead.");
    }
    
    pub async fn get_commits_before_timestamp(
        &mut self, 
        _game_id: i64, 
        _timestamp: chrono::DateTime<chrono::Utc>
    ) -> Result<Vec<super::git_manager::types::GitCommitInfo>, anyhow::Error> {
        panic!("GitRepositoryManager::get_commits_before_timestamp is deprecated. Use git_manager::GitSaveManager::restore_to_timestamp instead.");
    }
}
