//! Types and data structures for Git integration

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Information about a Git commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommitInfo {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub timestamp: DateTime<Utc>,
    pub branch: String,
    pub file_count: usize,
    pub total_size: usize,
    pub cloud_synced: bool,
    pub cloud_sync_url: Option<String>,
}

/// Information about a Git branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBranchInfo {
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub last_commit: Option<String>,
    pub commit_count: usize,
    pub created_at: DateTime<Utc>,
    pub protected: bool,
}

/// Result of cloud synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncResult {
    pub success: bool,
    pub provider: CloudProvider,
    pub repository_url: Option<String>,
    pub sync_url: Option<String>,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Cloud storage providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider {
    GitHub,
    GitLab,
    Gitea,
    SelfHosted,
}

/// Cloud synchronization status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncStatus {
    pub game_id: i64,
    pub provider: CloudProvider,
    pub last_sync: Option<DateTime<Utc>>,
    pub sync_status: SyncStatus,
    pub remote_url: Option<String>,
    pub error_message: Option<String>,
}

/// Synchronization status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncStatus {
    Pending,
    Syncing,
    Success,
    Failed,
    NotConfigured,
}

/// Metadata for Git-saved files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSaveMetadata {
    pub id: Uuid,
    pub game_id: i64,
    pub commit_hash: String,
    pub branch_name: String,
    pub file_path: String,
    pub original_filename: String,
    pub file_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f32,
    pub checksum_sha256: String,
    pub save_type: SaveType,
    pub custom_name: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub is_current: bool,
}

/// Type of save file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SaveType {
    Auto,
    Manual,
    Checkpoint,
    ManualBackup,
}

/// Git repository configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepositoryConfig {
    pub id: Uuid,
    pub game_id: i64,
    pub local_path: String,
    pub remote_url: Option<String>,
    pub provider: Option<CloudProvider>,
    pub default_branch: String,
    pub auto_commit: bool,
    pub auto_branch: bool,
    pub git_lfs_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_sync: Option<DateTime<Utc>>,
}

/// Git operation errors
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),
    
    #[error("Git operation failed: {0}")]
    GitOperationFailed(String),
    
    #[error("Invalid branch name: {0}")]
    InvalidBranchName(String),
    
    #[error("Commit not found: {0}")]
    CommitNotFound(String),
    
    #[error("Cloud sync failed: {0}")]
    CloudSyncFailed(String),
    
    #[error("File operation failed: {0}")]
    FileOperationFailed(String),
    
    #[error("Compression failed: {0}")]
    CompressionFailed(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Git-specific configuration for games
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameGitConfig {
    pub game_id: i64,
    pub git_enabled: bool,
    pub auto_save: bool,
    pub save_interval_minutes: u32,
    pub branch_strategy: BranchStrategy,
    pub cloud_sync_enabled: bool,
    pub cloud_provider: Option<CloudProvider>,
    pub max_versions: u32,
    pub compression_level: i32,
    pub git_ignore_large_files: bool,
    pub max_file_size_mb: u32,
}

/// Branch naming strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BranchStrategy {
    SingleMain,
    DateBased,
    GameState,
    ManualNaming,
}

/// Result of a Git operation
#[derive(Debug, Clone)]
pub struct GitOperationResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

impl<T> GitOperationResult<T> {
    pub fn new(success: bool, data: Option<T>, error: Option<String>, execution_time_ms: u64) -> Self {
        Self {
            success,
            data,
            error,
            execution_time_ms,
        }
    }
}

/// Utility type alias for Git operation results
pub type GitResult<T> = Result<GitOperationResult<T>, GitError>;

/// Git commit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommitStats {
    pub total_commits: usize,
    pub branches: usize,
    pub contributors: usize,
    pub first_commit: Option<DateTime<Utc>>,
    pub last_commit: Option<DateTime<Utc>>,
    pub avg_commits_per_day: f64,
    pub most_active_day: Option<String>,
    pub file_types: std::collections::HashMap<String, usize>,
}

/// Configuration for Git LFS (Large File Storage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLfsConfig {
    pub enabled: bool,
    pub track_patterns: Vec<String>,
    pub max_file_size: usize,
    pub compression_level: i32,
}

/// Branch merge information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchMergeInfo {
    pub source_branch: String,
    pub target_branch: String,
    pub merge_commit: Option<String>,
    pub conflicts: Vec<String>,
    pub merge_successful: bool,
    pub merged_at: Option<DateTime<Utc>>,
}

/// Save comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveComparison {
    pub old_commit: String,
    pub new_commit: String,
    pub file_diffs: Vec<FileDiff>,
    pub size_change: isize,
    pub timestamp_diff: chrono::Duration,
}

/// File difference information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub filename: String,
    pub change_type: ChangeType,
    pub size_change: Option<isize>,
    pub additions: usize,
    pub deletions: usize,
}

/// Type of file change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}
