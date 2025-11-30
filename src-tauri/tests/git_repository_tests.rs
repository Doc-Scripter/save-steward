// Integration tests for Git repository module

use save_steward_lib::database::connection::{Database, DatabasePaths};
use save_steward_lib::git_manager::repository::initialize_master_repo;
use std::sync::Arc;
use tokio::sync::Mutex;
use tempfile::TempDir;

/// Helper function to create a test database
async fn create_test_database() -> (Arc<Mutex<Database>>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    
    let db = Database::new_at_path(&db_path)
        .await
        .expect("Failed to create database");
    
    (Arc::new(Mutex::new(db)), temp_dir)
}

#[tokio::test]
async fn test_initialize_master_repo() {
    let (db, _temp_dir) = create_test_database().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    let result = initialize_master_repo(&db, repo_path).await;
    
    assert!(result.is_ok(), "Repository initialization should succeed");
    assert!(result.unwrap().contains(repo_path));
    
    // Verify .git directory exists
    let git_dir = temp_repo.path().join(".git");
    assert!(git_dir.exists(), ".git directory should exist");
}

#[tokio::test]
async fn test_gitignore_creation() {
    let (db, _temp_dir) = create_test_database().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Verify .gitignore exists and contains correct content
    let gitignore_path = temp_repo.path().join(".gitignore");
    assert!(gitignore_path.exists(), ".gitignore should exist");
    
    let content = std::fs::read_to_string(gitignore_path).expect("Failed to read .gitignore");
    assert!(content.contains("gamename-save-name"), ".gitignore should use correct branch naming");
    assert!(content.contains("*.sav"), ".gitignore should include save file extensions");
}

#[tokio::test]
async fn test_gitattributes_creation() {
    let (db, _temp_dir) = create_test_database().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Verify .gitattributes exists and contains LFS configuration
    let gitattributes_path = temp_repo.path().join(".gitattributes");
    assert!(gitattributes_path.exists(), ".gitattributes should exist");
    
    let content = std::fs::read_to_string(gitattributes_path).expect("Failed to read .gitattributes");
    assert!(content.contains("filter=lfs"), ".gitattributes should configure Git LFS");
    assert!(content.contains("*.sav"), ".gitattributes should track save files with LFS");
}

#[tokio::test]
async fn test_initial_commit_created() {
    let (db, _temp_dir) = create_test_database().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    initialize_master_repo(&db, repo_path).await.expect("Failed to initialize repo");
    
    // Verify initial commit exists
    let repo = git2::Repository::open(repo_path).expect("Failed to open repository");
    let head = repo.head().expect("Repository should have HEAD");
    assert!(head.is_branch(), "HEAD should point to a branch");
    
    // Verify commit exists
    let commit = head.peel_to_commit().expect("Should have a commit");
    assert!(commit.message().unwrap_or("").contains("Initial"), "Initial commit should exist");
}

#[tokio::test]
async fn test_repository_reinit_idempotent() {
    let (db, _temp_dir) = create_test_database().await;
    let temp_repo = TempDir::new().expect("Failed to create temp repo dir");
    let repo_path = temp_repo.path().to_str().unwrap();
    
    // Initialize once
    let result1 = initialize_master_repo(&db, repo_path).await;
    assert!(result1.is_ok());
    
    // Initialize again (should be idempotent)
    let result2 = initialize_master_repo(&db, repo_path).await;
    assert!(result2.is_ok());
}
