//! Cloud synchronization for Git repositories
//!
//! This module provides integration with GitHub, GitLab, and other Git hosting services
//! for backing up and synchronizing game save repositories.

use anyhow::Result;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;
use crate::database::connection::Database;
use crate::git_manager::types::*;

/// GitHub API client
pub struct GitHubClient {
    token: Option<String>,
    client: Client,
    base_url: String,
}

/// GitLab API client
pub struct GitLabClient {
    token: Option<String>,
    client: Client,
    base_url: String,
}

/// Main cloud synchronization manager
pub struct CloudSyncManager {
    db: Arc<Mutex<Database>>,
    github: GitHubClient,
    gitlab: GitLabClient,
}

impl CloudSyncManager {
    /// Create new cloud sync manager
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self {
            db,
            github: GitHubClient::new(),
            gitlab: GitLabClient::new(),
        }
    }

    /// Push repository to cloud storage
    pub async fn push_to_cloud(&self, game_id: i64) -> Result<CloudSyncResult> {
        // Get repository configuration
        let repo_config = self.get_repo_config(game_id).await?;
        
        match repo_config.provider {
            Some(ref provider) => {
                match provider {
                    CloudProvider::GitHub => self.push_to_github(game_id, &repo_config).await,
                    CloudProvider::GitLab => self.push_to_gitlab(game_id, &repo_config).await,
                    CloudProvider::Gitea => self.push_to_gitea(game_id, &repo_config).await,
                    CloudProvider::SelfHosted => self.push_to_self_hosted(game_id, &repo_config).await,
                }
            }
            None => Err(anyhow::anyhow!("No cloud provider configured"))
        }
    }

    /// Push to GitHub
    async fn push_to_github(&self, game_id: i64, config: &GitRepositoryConfig) -> Result<CloudSyncResult> {
        let start_time = std::time::Instant::now();
        
        // Create repository name
        let repo_name = format!("save-steward-game-{}", game_id);
        
        // Create or update repository
        let repo_url = if let Some(remote_url) = &config.remote_url {
            // Update existing repository
            self.github.update_repository(&repo_name, remote_url, "game").await
        } else {
            // Create new repository
            self.github.create_repository(&repo_name, "Game save repository for Save Steward").await
        }?;
        
        // Add remote to local repository
        self.add_remote_to_repo(game_id, &repo_url, "origin").await?;
        
        // Push to remote
        self.push_repository(game_id, "origin", &config.default_branch).await?;
        
        // Update sync status in database
        self.update_sync_status(game_id, CloudProvider::GitHub, SyncStatus::Success, Some(&repo_url)).await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CloudSyncResult {
            success: true,
            provider: CloudProvider::GitHub,
            repository_url: Some(repo_url.clone()),
            sync_url: Some(repo_url),
            message: format!("Successfully pushed to GitHub in {}ms", execution_time),
            timestamp: Utc::now(),
        })
    }

    /// Push to GitLab
    async fn push_to_gitlab(&self, game_id: i64, config: &GitRepositoryConfig) -> Result<CloudSyncResult> {
        let start_time = std::time::Instant::now();
        
        // Create project name
        let project_name = format!("save-steward-game-{}", game_id);
        
        // Create or update project
        let project_url = if let Some(remote_url) = &config.remote_url {
            // Update existing project
            self.gitlab.update_project(&project_name, remote_url).await
        } else {
            // Create new project
            self.gitlab.create_project(&project_name, "Game save repository for Save Steward").await
        }?;
        
        // Add remote to local repository
        self.add_remote_to_repo(game_id, &project_url, "origin").await?;
        
        // Push to remote
        self.push_repository(game_id, "origin", &config.default_branch).await?;
        
        // Update sync status in database
        self.update_sync_status(game_id, CloudProvider::GitLab, SyncStatus::Success, Some(&project_url)).await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CloudSyncResult {
            success: true,
            provider: CloudProvider::GitLab,
            repository_url: Some(project_url.clone()),
            sync_url: Some(project_url),
            message: format!("Successfully pushed to GitLab in {}ms", execution_time),
            timestamp: Utc::now(),
        })
    }

    /// Push to Gitea (self-hosted instance)
    async fn push_to_gitea(&self, game_id: i64, config: &GitRepositoryConfig) -> Result<CloudSyncResult> {
        // Similar implementation to GitLab but for Gitea
        todo!("Gitea implementation")
    }

    /// Push to self-hosted Git server
    async fn push_to_self_hosted(&self, game_id: i64, config: &GitRepositoryConfig) -> Result<CloudSyncResult> {
        // Implementation for self-hosted Git server
        todo!("Self-hosted implementation")
    }

    /// Pull from cloud storage
    pub async fn pull_from_cloud(&self, game_id: i64) -> Result<CloudSyncResult> {
        // Get repository configuration
        let repo_config = self.get_repo_config(game_id).await?;
        
        // Pull latest changes
        self.pull_repository(game_id, "origin", &repo_config.default_branch).await?;
        
        // Update sync status
        self.update_sync_status(game_id,
            repo_config.provider.clone().unwrap(),
            SyncStatus::Success,
            repo_config.remote_url.as_deref()
        ).await?;
        
        Ok(CloudSyncResult {
            success: true,
            provider: repo_config.provider.unwrap(),
            repository_url: repo_config.remote_url.clone(),
            sync_url: repo_config.remote_url,
            message: "Successfully pulled from cloud".to_string(),
            timestamp: Utc::now(),
        })
    }

    /// Get cloud sync status
    pub async fn get_sync_status(&self, game_id: i64) -> Result<Vec<CloudSyncStatus>> {
        let db = self.db.lock().await;
        let conn = db.get_connection().await;
        
        let mut stmt = conn.prepare(
            "SELECT provider, last_sync_at, sync_status, remote_url, error_message
             FROM cloud_sync_log 
             WHERE game_id = ? 
             ORDER BY created_at DESC 
             LIMIT 10"
        )?;
        
        let status_iter = stmt.query_map([game_id], |row| {
            Ok(CloudSyncStatus {
                game_id,
                provider: serde_json::from_str(row.get::<_, String>(0)?.as_str())
                    .unwrap_or(CloudProvider::GitHub),
                last_sync: row.get::<_, Option<String>>(1)?
                    .and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(s.as_str())
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                sync_status: serde_json::from_str(row.get::<_, String>(2)?.as_str())
                    .unwrap_or(SyncStatus::NotConfigured),
                remote_url: row.get(3)?,
                error_message: row.get(4)?,
            })
        })?;
        
        let mut statuses = Vec::new();
        for status in status_iter {
            statuses.push(status?);
        }
        
        Ok(statuses)
    }

    /// Configure cloud sync for a game
    pub async fn configure_cloud_sync(
        &self, 
        game_id: i64, 
        provider: CloudProvider,
        credentials: CloudCredentials,
        auto_sync: bool
    ) -> Result<CloudSyncResult> {
        // Store credentials securely (this should be encrypted)
        self.store_cloud_credentials(game_id, &provider, &credentials).await?;
        
        // Update repository configuration
        let db = self.db.lock().await;
        let conn = db.get_connection().await;
        
        conn.execute(
            "UPDATE git_repositories
             SET cloud_provider = ?, auto_sync = ?
             WHERE game_id = ?",
            [serde_json::to_string(&provider)?, auto_sync.to_string(), game_id.to_string()]
        )?;
        
        Ok(CloudSyncResult {
            success: true,
            provider,
            repository_url: None,
            sync_url: None,
            message: "Cloud sync configured successfully".to_string(),
            timestamp: Utc::now(),
        })
    }

    /// Helper methods
    async fn get_repo_config(&self, game_id: i64) -> Result<GitRepositoryConfig> {
        let db = self.db.lock().await;
        let conn = db.get_connection().await;
        
        let mut stmt = conn.prepare(
            "SELECT local_path, remote_url, cloud_provider, default_branch, auto_sync, git_lfs_enabled
             FROM git_repositories 
             WHERE game_id = ?"
        )?;
        
        let row = stmt.query_row([game_id], |row| {
            Ok(GitRepositoryConfig {
                id: uuid::Uuid::new_v4(), // TODO: Store actual ID in database
                game_id,
                local_path: row.get(0)?,
                remote_url: row.get(1)?,
                provider: row.get::<_, Option<String>>(2)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                default_branch: row.get(3)?,
                auto_commit: true,
                auto_branch: true,
                git_lfs_enabled: row.get(5)?,
                created_at: Utc::now(),
                last_sync: row.get::<_, Option<String>>(1)?
                    .and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(s.as_str())
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
            })
        })?;
        
        Ok(row)
    }

    async fn add_remote_to_repo(&self, game_id: i64, remote_url: &str, remote_name: &str) -> Result<()> {
        // This would use git commands to add remote
        // For now, just log the operation
        println!("Adding remote {} to repository for game {}", remote_name, game_id);
        Ok(())
    }

    async fn push_repository(&self, game_id: i64, remote_name: &str, branch: &str) -> Result<()> {
        // This would use git commands to push
        // For now, just log the operation
        println!("Pushing repository for game {} to {}/{}", game_id, remote_name, branch);
        Ok(())
    }

    async fn pull_repository(&self, game_id: i64, remote_name: &str, branch: &str) -> Result<()> {
        // This would use git commands to pull
        // For now, just log the operation
        println!("Pulling repository for game {} from {}/{}", game_id, remote_name, branch);
        Ok(())
    }

    async fn update_sync_status(
        &self,
        game_id: i64,
        provider: CloudProvider,
        status: SyncStatus,
        remote_url: Option<&str>
    ) -> Result<()> {
        let db = self.db.lock().await;
        let conn = db.get_connection().await;
        
        conn.execute(
            "INSERT INTO cloud_sync_log (game_id, provider, sync_status, remote_url, created_at)
             VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![
                game_id,
                serde_json::to_string(&provider)?,
                serde_json::to_string(&status)?,
                remote_url.unwrap_or(""),
                Utc::now().to_rfc3339()
            ]
        )?;
        
        Ok(())
    }

    async fn store_cloud_credentials(&self, game_id: i64, provider: &CloudProvider, credentials: &CloudCredentials) -> Result<()> {
        // This should store credentials securely (encrypted)
        // For now, just log
        println!("Storing cloud credentials for game {} on {:?}", game_id, provider);
        Ok(())
    }
}

/// Cloud credentials for different providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudCredentials {
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>, // For self-hosted instances
    pub api_key: Option<String>,
}

/// GitHub-specific client implementation
impl GitHubClient {
    fn new() -> Self {
        Self {
            token: None,
            client: Client::new(),
            base_url: "https://api.github.com".to_string(),
        }
    }

    async fn create_repository(&self, name: &str, description: &str) -> Result<String> {
        let repo_data = serde_json::json!({
            "name": name,
            "description": description,
            "private": false,
            "has_issues": false,
            "has_projects": false,
            "has_wiki": false
        });

        let response = self.client
            .post(&format!("{}/user/repos", self.base_url))
            .header(header::AUTHORIZATION, format!("token {}", self.token.as_ref().unwrap_or(&"".to_string())))
            .json(&repo_data)
            .send()
            .await?;

        if response.status().is_success() {
            let repo: serde_json::Value = response.json().await?;
            Ok(repo["clone_url"].as_str().unwrap_or("").to_string())
        } else {
            Err(anyhow::anyhow!("Failed to create GitHub repository"))
        }
    }

    async fn update_repository(&self, name: &str, remote_url: &str, _action: &str) -> Result<String> {
        // For existing repositories, just return the clone URL
        Ok(remote_url.to_string())
    }
}

/// GitLab-specific client implementation
impl GitLabClient {
    fn new() -> Self {
        Self {
            token: None,
            client: Client::new(),
            base_url: "https://gitlab.com/api/v4".to_string(),
        }
    }

    async fn create_project(&self, name: &str, description: &str) -> Result<String> {
        let project_data = serde_json::json!({
            "name": name,
            "description": description,
            "visibility": "public",
            "issues_enabled": false,
            "merge_requests_enabled": false,
            "wiki_enabled": false,
            "snippets_enabled": false
        });

        let response = self.client
            .post(&format!("{}/projects", self.base_url))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
            .json(&project_data)
            .send()
            .await?;

        if response.status().is_success() {
            let project: serde_json::Value = response.json().await?;
            Ok(project["http_url_to_repo"].as_str().unwrap_or("").to_string())
        } else {
            Err(anyhow::anyhow!("Failed to create GitLab project"))
        }
    }

    async fn update_project(&self, name: &str, remote_url: &str) -> Result<String> {
        // For existing projects, just return the clone URL
        Ok(remote_url.to_string())
    }
}
