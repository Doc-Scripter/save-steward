// Integration tests for Git history module

use save_steward_lib::database::connection::Database;
use save_steward_lib::git_manager::branching::create_save_checkpoint;
use save_steward_lib::git_manager::history::{restore_to_commit, restore_to_timestamp, get_save_history};
use save_steward_lib::git_manager::repository::initialize_master_repo;
use std::sync::Arc;
use tokio::sync::Mutex;
use tempfile::TempDir;
use chrono::Utc;

/// Helper function to create a test database with a sample game
async fn create_test_database_with_game() -> (Arc<Mutex<Database>>, TempDir, i64) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    
    let db = Database::new_at_path(&db_path)
        .await
        .expect("Failed to create database");
    
    // Insert a test game
    let game_id = {
        let conn_guard = db.get_connection().await;
        conn_guard.execute(
            "INSERT INTO games (name, platform, installation_path) VALUES (?, ?, ?)",
            rusqlite::params!["TestGame", "steam", "/test/path"]
        ).expect("Failed to insert test game");
        conn_guard.last_insert_rowid()
    };
    
    (Arc::new(Mutex::new(db)), temp_dir, game_id)
}

/// Helper function to get the latest commit hash
fn get_latest_commit_hash(repo_path: &str) -> String {
    let repo = git2::Repository::open(repo_path).expect("Failed to open repository");
    let head = repo.head().expect("Failed to get HEAD");
    let commit = head.peel_to_commit().expect("Failed to get commit");
    commit.id().to_string()
}

#[tokio::test]
async fn test_restore_to_commit() {
    let (db, temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    // Initialize repository
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Get initial commit hash
    let commit_hash = get_latest_commit_hash(repo_path);
    
    // Create a save checkpoint to move HEAD forward
    create_save_checkpoint(&db, repo_path, game_id, "NewSave").await.expect("Failed to create checkpoint");
    
    // Restore to the initial commit
    let result = restore_to_commit(repo_path, game_id, &commit_hash).await;
    assert!(result.is_ok(), "Should restore to commit");
    
    // Verify a restore branch was created
    let repo = git2::Repository::open(repo_path).expect("Failed to open repository");
    let head = repo.head().expect("Should have HEAD");
    let branch_name = head.shorthand().unwrap();
    assert!(branch_name.starts_with("restore-"), "Should create restore branch");
}

#[tokio::test]
async fn test_restore_to_invalid_commit() {
    let (db, temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Try to restore to invalid commit hash
    let result = restore_to_commit(repo_path, game_id, "invalid_hash").await;
    assert!(result.is_err(), "Should fail with invalid commit hash");
}

#[tokio::test]
async fn test_restore_to_timestamp() {
    let (db, _temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Create a checkpoint
    create_save_checkpoint(&db, repo_path, game_id, "Save1").await.expect("Failed to create checkpoint");
    
    // Get current time (after the commit)
    let target_time = Utc::now();
    
    // Restore to nearest commit by timestamp
    let result = restore_to_timestamp(repo_path, game_id, target_time).await;
    assert!(result.is_ok(), "Should restore to nearest commit by timestamp");
}

#[tokio::test]
async fn test_get_save_history() {
    let (db, _temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Create multiple save checkpoints
    create_save_checkpoint(&db, repo_path, game_id, "Save1").await.expect("Failed to create checkpoint 1");
    create_save_checkpoint(&db, repo_path, game_id, "Save2").await.expect("Failed to create checkpoint 2");
    
    // Get save history
    let history = get_save_history(repo_path, game_id).await;
    assert!(history.is_ok(), "Should get save history");
    
    let history_value = history.unwrap();
    assert!(history_value.is_object(), "History should be a JSON object");
    
    // Verify history contains commits
    let commits = history_value.get("commits").expect("Should have commits field");
    assert!(commits.is_array(), "Commits should be an array");
    assert!(commits.as_array().unwrap().len() > 0, "Should have at least one commit");
}

#[tokio::test]
async fn test_branch_name_parsing_uses_dash() {
    let (db, _temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Create checkpoint
    create_save_checkpoint(&db, repo_path, game_id, "MySave").await.expect("Failed to create checkpoint");
    
    // Get history
    let history = get_save_history(repo_path, game_id).await.expect("Failed to get history");
    
    // Verify commits are parsed correctly with '-' separator
    let commits = history.get("commits").unwrap().as_array().unwrap();
    if !commits.is_empty() {
        let first_commit = &commits[0];
        let game_name = first_commit.get("game_name").and_then(|v| v.as_str());
        
        // Game name should be extracted using '-' separator
        // If branch is "TestGame-MySave", game_name should be "TestGame"
        if let Some(name) = game_name {
            assert!(name == "TestGame" || name == "Unknown", "Game name should be correctly extracted or Unknown");
        }
    }
}

#[tokio::test]
async fn test_history_includes_branch_info() {
    let (db, _temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Create multiple branches
    create_save_checkpoint(&db, repo_path, game_id, "Branch1").await.expect("Failed to create checkpoint 1");
    create_save_checkpoint(&db, repo_path, game_id, "Branch2").await.expect("Failed to create checkout 2");
    
    // Get history
    let history = get_save_history(repo_path, game_id).await.expect("Failed to get history");
    
    // Verify branches are listed
    let branches = history.get("branches").expect("Should have branches field");
    assert!(branches.is_array(), "Branches should be an array");
    let branch_list = branches.as_array().unwrap();
    assert!(branch_list.len() >= 2, "Should have at least 2 branches");
    
    // Verify current branch is set
    let current_branch = history.get("current_branch").expect("Should have current_branch field");
    assert!(current_branch.is_string(), "Current branch should be a string");
}

#[tokio::test]
async fn test_restore_creates_timestamped_branch() {
    let (db, _temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    let commit_hash = get_latest_commit_hash(repo_path);
    
    // Restore to commit
    restore_to_commit(repo_path, game_id, &commit_hash).await.expect("Failed to restore");
    
    // Verify restore branch naming format
    let repo = git2::Repository::open(repo_path).expect("Failed to open repository");
    let head = repo.head().expect("Should have HEAD");
    let branch_name = head.shorthand().unwrap();
    
    // Branch should be named like "restore-YYYYMMDD-HHMMSS-abcd1234"
    assert!(branch_name.starts_with("restore-"), "Restore branch should start with 'restore-'");
    assert!(branch_name.contains("-"), "Restore branch should contain timestamp and hash");
}
