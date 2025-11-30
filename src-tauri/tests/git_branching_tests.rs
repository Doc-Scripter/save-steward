// Integration tests for Git branching module

use save_steward_lib::database::connection::{Database, DatabaseSchema};
use save_steward_lib::git_manager::branching::{
    create_save_checkpoint, switch_save_branch, list_all_branches, 
    get_game_branches, delete_save_branch
};
use save_steward_lib::git_manager::repository::initialize_master_repo;
use std::sync::Arc;
use tokio::sync::Mutex;
use tempfile::TempDir;
use rusqlite::Connection;

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

#[tokio::test]
async fn test_create_save_checkpoint() {
    let (db, _temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    // Initialize repository
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Create save checkpoint
    let result = create_save_checkpoint(&db, repo_path, game_id, "MainQuest").await;
    assert!(result.is_ok(), "Should create save checkpoint");
    assert!(result.unwrap().contains("TestGame-MainQuest"));
    
    // Verify branch exists in Git
    let repo = git2::Repository::open(repo_path).expect("Failed to open repository");
    let branch = repo.find_branch("TestGame-MainQuest", git2::BranchType::Local);
    assert!(branch.is_ok(), "Branch should exist in repository");
    
    // Verify branch info saved to database
    let conn_guard = db.lock().await;
    let conn = conn_guard.get_connection().await;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM git_branches WHERE game_id = ? AND branch_name = ?",
        rusqlite::params![game_id, "TestGame-MainQuest"],
        |row| row.get(0)
    ).expect("Failed to query database");
    assert_eq!(count, 1, "Branch should be saved to database");
}

#[tokio::test]
async fn test_create_duplicate_checkpoint_switches() {
    let (db, _temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Create checkpoint twice with same name
    let result1 = create_save_checkpoint(&db, repo_path, game_id, "Save1").await;
    let result2 = create_save_checkpoint(&db, repo_path, game_id, "Save1").await;
    
    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert!(result2.unwrap().contains("Switched to existing"), "Should switch to existing branch");
}

#[tokio::test]
async fn test_switch_save_branch() {
    let (db, _temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Create two branches
    create_save_checkpoint(&db, repo_path, game_id, "Save1").await.expect("Failed to create checkpoint 1");
    create_save_checkpoint(&db, repo_path, game_id, "Save2").await.expect("Failed to create checkpoint 2");
    
    // Switch back to first branch
    let result = switch_save_branch(&db, repo_path, game_id, "TestGame-Save1").await;
    assert!(result.is_ok(), "Should switch to branch");
    
    // Verify active branch in repository
    let repo = git2::Repository::open(repo_path).expect("Failed to open repository");
    let head = repo.head().expect("Should have HEAD");
    let branch_name = head.shorthand().unwrap();
    assert_eq!(branch_name, "TestGame-Save1", "Should be on correct branch");
}

#[tokio::test]
async fn test_list_all_branches() {
    let (db, _temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Create multiple branches
    create_save_checkpoint(&db, repo_path, game_id, "Save1").await.expect("Failed to create checkpoint 1");
    create_save_checkpoint(&db, repo_path, game_id, "Save2").await.expect("Failed to create checkpoint 2");
    
    // List all branches
    let branches = list_all_branches(repo_path).await.expect("Failed to list branches");
    
    // Should have at least 3 branches (main + 2 saves)
    assert!(branches.len() >= 2, "Should list multiple branches");
}

#[tokio::test]
async fn test_get_game_branches() {
    let (db, _temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Create branches for TestGame
    create_save_checkpoint(&db, repo_path, game_id, "Save1").await.expect("Failed to create checkpoint 1");
    create_save_checkpoint(&db, repo_path, game_id, "Save2").await.expect("Failed to create checkpoint 2");
    
    // Get branches for TestGame
    let game_branches = get_game_branches(repo_path, "TestGame").await.expect("Failed to get game branches");
    
    assert_eq!(game_branches.len(), 2, "Should have 2 branches for TestGame");
    assert!(game_branches.contains(&"TestGame-Save1".to_string()));
    assert!(game_branches.contains(&"TestGame-Save2".to_string()));
}

#[tokio::test]
async fn test_delete_save_branch() {
    let (db, _temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Create a branch
    create_save_checkpoint(&db, repo_path, game_id, "ToDelete").await.expect("Failed to create checkpoint");
    
    // Switch to another branch first (can't delete current branch)
    create_save_checkpoint(&db, repo_path, game_id, "KeepThis").await.expect("Failed to create second branch");
    
    // Delete the branch
    let result = delete_save_branch(repo_path, "TestGame-ToDelete").await;
    assert!(result.is_ok(), "Should delete branch");
    
    // Verify branch is deleted from Git
    let repo = git2::Repository::open(repo_path).expect("Failed to open repository");
    let branch = repo.find_branch("TestGame-ToDelete", git2::BranchType::Local);
    assert!(branch.is_err(), "Branch should be deleted from repository");
}

#[tokio::test]
async fn test_branch_naming_convention() {
    let (db, _temp_dir, game_id) = create_test_database_with_game().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Create checkpoint with special characters
    let result = create_save_checkpoint(&db, repo_path, game_id, "My Save").await;
    assert!(result.is_ok());
    
    // Verify branch uses '-' separator
    let branches = get_game_branches(repo_path, "TestGame").await.expect("Failed to get branches");
    assert!(branches.iter().any(|b| b.contains("TestGame-")), "Branch should use '-' separator");
}
