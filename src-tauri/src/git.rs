//! Core Git repository management for Save Steward
//!
//! This module provides the fundamental Git operations needed for save version control,
//! including repository initialization, commit management, and branch operations.

use git2::{Repository, Commit, Branch, Tree, Index, Signature};
use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use crate::database::connection::Database;
use crate::git_manager::types::*;

/// Manages Git repositories for game saves
pub struct GitRepositoryManager {
    /// Database connection for metadata
    db: Arc<Mutex<Database>>,
    /// Cache of open repositories
    repo_cache: std::collections::HashMap<i64, Repository>,
    /// Base directory for save repositories
    base_repo_path: PathBuf,
}

impl GitRepositoryManager {
    /// Create new repository manager
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        let base_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("save-steward")
            .join("git-repos");
        
        // Ensure base directory exists
        std::fs::create_dir_all(&base_path).unwrap_or(());
        
        Self {
            db,
            repo_cache: std::collections::HashMap::new(),
            base_repo_path: base_path,
        }
    }

    /// Initialize a new Git repository for a game
    pub async fn initialize_repository(&self, game_id: i64) -> Result<String> {
        let repo_path = self.get_repo_path(game_id);
        
        // Create repository directory
        std::fs::create_dir_all(&repo_path)?;
        
        // Initialize Git repository
        let repo = Repository::init(&repo_path)
            .context(format!("Failed to initialize Git repository for game {}", game_id))?;
        
        // Configure repository for binary files
        self.configure_gitattributes(&repo_path)?;
        
        // Create initial commit
        self.create_initial_commit(&repo).await?;
        
        // Cache the repository
        self.repo_cache.insert(game_id, repo);
        
        // Save repository configuration to database
        self.save_repo_config(game_id, &repo_path).await?;
        
        Ok(repo_path.to_string_lossy().to_string())
    }

    /// Get or load repository for a game
    pub async fn get_repository(&mut self, game_id: i64) -> Result<&Repository> {
        if !self.repo_cache.contains_key(&game_id) {
            let repo_path = self.get_repo_path(game_id);
            
            if !repo_path.exists() {
                return Err(anyhow::anyhow!("Repository not found for game {}", game_id));
            }
            
            let repo = Repository::open(&repo_path)
                .context(format!("Failed to open repository for game {}", game_id))?;
            
            self.repo_cache.insert(game_id, repo);
        }
        
        self.repo_cache.get(&game_id)
            .ok_or_else(|| anyhow::anyhow!("Repository not found for game {}", game_id))
    }

    /// Commit a save file to the repository
    pub async fn commit_save_file(
        &mut self, 
        game_id: i64, 
        filename: &str, 
        data: &[u8], 
        message: &str
    ) -> Result<String> {
        let start_time = std::time::Instant::now();
        
        // Get repository
        let repo = self.get_repository(game_id).await?;
        
        // Write file to repository
        let file_path = PathBuf::from("saves").join(filename);
        let full_path = repo.workdir().unwrap().join(&file_path);
        
        // Ensure saves directory exists
        std::fs::create_dir_all(full_path.parent().unwrap())?;
        
        // Write file
        std::fs::write(&full_path, data)
            .context("Failed to write save file to repository")?;
        
        // Stage file in Git index
        let mut index = repo.index()
            .context("Failed to get repository index")?;
        
        index.add_path(&file_path)
            .context("Failed to add file to Git index")?;
        
        index.write()
            .context("Failed to write Git index")?;
        
        // Get current signature
        let signature = self.get_signature(repo)?;
        
        // Create commit
        let tree_id = index.write_tree()
            .context("Failed to write tree")?;
        let tree = repo.find_tree(tree_id)
            .context("Failed to find tree")?;
        
        let head_commit = if repo.head_detached().is_ok() {
            None
        } else {
            Some(repo.head()?.peel_to_commit()?)
        };
        
        let commit_id = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            head_commit.as_ref()
        ).context("Failed to create commit")?;
        
        let commit_hash = commit_id.to_string();
        
        // Update HEAD to point to new commit
        if let Ok(mut head) = repo.head() {
            let branch = head.as_branch().unwrap();
            let _ = branch.set_target(commit_id);
        }
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        tracing::info!(
            "Committed save file for game {} in {}ms", 
            game_id, 
            execution_time
        );
        
        Ok(commit_hash)
    }

    /// Create a new branch
    pub async fn create_branch(&mut self, game_id: i64, branch_name: &str, description: Option<&str>) -> Result<()> {
        // Validate branch name
        if !self.is_valid_branch_name(branch_name) {
            return Err(anyhow::anyhow!("Invalid branch name: {}", branch_name));
        }
        
        let repo = self.get_repository(game_id).await?;
        
        // Check if branch already exists
        if repo.branch(branch_name).is_ok() {
            return Err(anyhow::anyhow!("Branch '{}' already exists", branch_name));
        }
        
        // Get current HEAD
        let head_commit = repo.head()?.peel_to_commit()?;
        
        // Create new branch
        let branch = repo.branch(branch_name, &head_commit, true)
            .context(format!("Failed to create branch '{}'", branch_name))?;
        
        // Switch to new branch
        repo.set_head(&format!("refs/heads/{}", branch_name))
            .context("Failed to switch to new branch")?;
        
        // Save branch info to database
        self.save_branch_info(game_id, branch_name, description).await?;
        
        tracing::info!("Created branch '{}' for game {}", branch_name, game_id);
        
        Ok(())
    }

    /// Switch to a different branch
    pub async fn switch_branch(&mut self, game_id: i64, branch_name: &str) -> Result<()> {
        let repo = self.get_repository(game_id).await?;
        
        // Check if branch exists
        if repo.branch(branch_name).is_err() {
            return Err(anyhow::anyhow!("Branch '{}' does not exist", branch_name));
        }
        
        // Switch to branch
        repo.set_head(&format!("refs/heads/{}", branch_name))
            .context(format!("Failed to switch to branch '{}'", branch_name))?;
        
        // Checkout working directory
        let mut checkout_builder = git2::build::CheckoutBuilder::new();
        checkout_builder.force();
        repo.checkout_head(Some(&mut checkout_builder))
            .context("Failed to checkout HEAD")?;
        
        // Update branch info in database
        self.update_active_branch(game_id, branch_name).await?;
        
        tracing::info!("Switched to branch '{}' for game {}", branch_name, game_id);
        
        Ok(())
    }

    /// Get commit history for a game
    pub async fn get_commit_history(&mut self, game_id: i64) -> Result<Vec<GitCommitInfo>> {
        let repo = self.get_repository(game_id).await?;
        
        // Get current branch
        let current_branch = self.get_current_branch(repo)?;
        let branch_name = current_branch.name()?.unwrap_or("main");
        
        // Get all commits on current branch
        let mut commits = Vec::new();
        let mut revwalk = repo.revwalk()
            .context("Failed to create revision walk")?;
        
        revwalk.push_head()
            .context("Failed to push HEAD to revision walk")?;
        
        for oid in revwalk {
            let oid = oid?;
            let commit = repo.find_commit(oid)
                .context("Failed to find commit")?;
            
            let commit_info = self.commit_to_info(&commit, branch_name)?;
            commits.push(commit_info);
        }
        
        Ok(commits)
    }

    /// Get commit data for restoration
    pub async fn get_commit_data(&mut self, game_id: i64, commit_hash: &str) -> Result<Vec<u8>> {
        let repo = self.get_repository(game_id).await?;
        
        // Find commit
        let oid = git2::Oid::from_str(commit_hash)
            .context("Invalid commit hash")?;
        let commit = repo.find_commit(oid)
            .context("Commit not found")?;
        
        // Get tree
        let tree = commit.tree()
            .context("Failed to get commit tree")?;
        
        // Find save file in tree
        let save_file = tree.get_name("saves")
            .and_then(|entry| repo.find_tree(entry.id()).ok())
            .and_then(|saves_tree| {
                saves_tree.iter().find_map(|entry| {
                    if entry.kind() == Some(git2::ObjectType::Blob) {
                        Some(entry)
                    } else {
                        None
                    }
                })
            })
            .context("Save file not found in commit")?;
        
        // Get file content
        let blob = repo.find_blob(save_file.id())
            .context("Failed to find blob")?;
        
        Ok(blob.content().to_vec())
    }

    /// Get commits before a specific timestamp
    pub async fn get_commits_before_timestamp(
        &mut self, 
        game_id: i64, 
        timestamp: DateTime<Utc>
    ) -> Result<Vec<GitCommitInfo>> {
        let mut commits = self.get_commit_history(game_id).await?;
        
        // Filter commits before timestamp
        commits.retain(|commit| commit.timestamp <= timestamp);
        
        // Sort by timestamp (newest first)
        commits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        Ok(commits)
    }

    /// Helper methods
    fn get_repo_path(&self, game_id: i64) -> PathBuf {
        self.base_repo_path.join(format!("game_{}", game_id))
    }

    fn configure_gitattributes(&self, repo_path: &Path) -> Result<()> {
        let gitattributes_path = repo_path.join(".gitattributes");
        
        let attributes = "# Git attributes for Save Steward
*.sav filter=lfs diff=lfs merge=lfs -text
*.save filter=lfs diff=lfs merge=lfs -text
*.zst filter=lfs diff=lfs merge=lfs -text
";
        
        std::fs::write(&gitattributes_path, attributes)
            .context("Failed to write .gitattributes file")?;
        
        Ok(())
    }

    async fn create_initial_commit(&self, repo: &Repository) -> Result<()> {
        let signature = self.get_signature(repo)?;
        
        // Create empty tree
        let tree_id = {
            let mut index = repo.index()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;
        
        // Create initial commit
        repo.commit(
            Some("refs/heads/main"),
            &signature,
            &signature,
            "Initial commit - Save Steward repository",
            &tree,
            None
        )?;
        
        Ok(())
    }

    fn get_signature(&self, repo: &Repository) -> Result<Signature> {
        let config = repo.config()?;
        let name = config.get_string("user.name").unwrap_or_else(|_| "Save Steward".to_string());
        let email = config.get_string("user.email").unwrap_or_else(|_| "save-steward@local".to_string());
        
        Signature::now(&name, &email)
            .context("Failed to create signature")
    }

    fn get_current_branch(&self, repo: &Repository) -> Result<Branch> {
        repo.head()
            .and_then(|head| head.as_branch().cloned())
            .context("Failed to get current branch")
    }

    fn is_valid_branch_name(&self, name: &str) -> bool {
        // Basic validation for branch names
        !name.is_empty() && 
        name.len() <= 255 &&
        !name.contains(' ') &&
        !name.starts_with('.') &&
        !name.ends_with('.') &&
        !name.contains("..") &&
        !name.contains("//")
    }

    fn commit_to_info(&self, commit: &Commit, branch_name: &str) -> Result<GitCommitInfo> {
        let timestamp = DateTime::from_timestamp(commit.time().seconds(), 0)
            .unwrap_or_else(|| Utc::now());
        
        Ok(GitCommitInfo {
            hash: commit.id().to_string(),
            message: commit.message().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("").to_string(),
            email: commit.author().email().unwrap_or("").to_string(),
            timestamp,
            branch: branch_name.to_string(),
            file_count: 1, // We typically store one file per commit
            total_size: commit.size() as usize,
            cloud_synced: false, // TODO: Check against database
            cloud_sync_url: None,
        })
    }

    async fn save_repo_config(&self, game_id: i64, repo_path: &Path) -> Result<()> {
        let db = self.db.lock().await;
        let conn = db.get_connection().await?;
        
        conn.execute(
            "INSERT OR REPLACE INTO git_repositories (game_id, local_path, created_at, last_sync_at)
             VALUES (?, ?, ?, ?)",
            rusqlite::params![
                game_id,
                repo_path.to_string_lossy(),
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339()
            ]
        )?;
        
        Ok(())
    }

    async fn save_branch_info(&self, game_id: i64, branch_name: &str, description: Option<&str>) -> Result<()> {
        let db = self.db.lock().await;
        let conn = db.get_connection().await?;
        
        conn.execute(
            "INSERT INTO git_branches (game_id, branch_name, description, created_at)
             VALUES (?, ?, ?, ?)",
            rusqlite::params![
                game_id,
                branch_name,
                description.unwrap_or(""),
                Utc::now().to_rfc3339()
            ]
        )?;
        
        Ok(())
    }

    async fn update_active_branch(&self, game_id: i64, branch_name: &str) -> Result<()> {
        let db = self.db.lock().await;
        let conn = db.get_connection().await?;
        
        // Reset all branches for this game
        conn.execute(
            "UPDATE git_branches SET is_active = 0 WHERE game_id = ?",
            [game_id]
        )?;
        
        // Set current branch as active
        conn.execute(
            "UPDATE git_branches SET is_active = 1 WHERE game_id = ? AND branch_name = ?",
            [game_id, branch_name]
        )?;
        
        Ok(())
    }
}
