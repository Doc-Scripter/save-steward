pub mod manager;
pub mod monitor;
pub mod retention;
pub mod integration;

pub use manager::AutoBackupManager;
pub use monitor::{SaveMonitor, SaveMonitorHandle};
pub use retention::RetentionPolicy;
pub use integration::BackupIntegrationLayer;

use crate::database::DatabaseConnection;
use crate::database::DatabaseResult;
use crate::manifest::ManifestResolver;

/// Events emitted by the auto-backup system
#[derive(Debug, Clone)]
pub enum BackupEvent {
    GameSessionStarted { game_id: i64, process_id: u32 },
    GameSessionEnded { game_id: i64, session_id: String },
    BackupTriggered { game_id: i64, backup_type: BackupType },
    BackupCompleted { game_id: i64, backup_id: String },
    BackupFailed { game_id: i64, error: String },
}

/// Types of backups that can be created
#[derive(Debug, Clone, Copy)]
pub enum BackupType {
    /// Created when game session starts (baseline)
    SessionStart,
    /// Created during gameplay when files change
    RealTime,
    /// Created when game session ends (final snapshot)
    SessionEnd,
    /// Manually triggered backup
    Manual,
}

/// Configuration for the auto-backup system
#[derive(Debug, Clone)]
pub struct AutoBackupConfig {
    /// Minimum confidence score required to trigger backups (default: 80.0)
    pub min_confidence_threshold: f32,
    /// Maximum number of backups per game profile (default: 3)
    pub max_backups_per_game: usize,
    /// Delay before creating real-time backups (debouncing) in seconds
    pub real_time_backup_delay: u64,
    /// Whether to enable real-time monitoring during gameplay
    pub enable_real_time_backup: bool,
    /// Whether to create session-start backups
    pub enable_session_backups: bool,
    /// Whether to create session-end backups
    pub enable_final_backups: bool,
}

impl Default for AutoBackupConfig {
    fn default() -> Self {
        Self {
            min_confidence_threshold: 80.0,
            max_backups_per_game: 3,
            real_time_backup_delay: 10, // 10 seconds debouncing
            enable_real_time_backup: true,
            enable_session_backups: true,
            enable_final_backups: true,
        }
    }
}

/// Represents an active game session being monitored
#[derive(Debug, Clone)]
pub struct GameSession {
    pub game_id: i64,
    pub session_id: String,
    pub process_id: u32,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub last_backup_time: Option<chrono::DateTime<chrono::Utc>>,
    pub backup_count: usize,
    pub monitored_paths: Vec<String>,
}

impl GameSession {
    pub fn new(game_id: i64, process_id: u32) -> Self {
        Self {
            game_id,
            session_id: uuid::Uuid::new_v4().to_string(),
            process_id,
            started_at: chrono::Utc::now(),
            last_backup_time: None,
            backup_count: 0,
            monitored_paths: Vec::new(),
        }
    }

    pub fn should_create_backup(&self, config: &AutoBackupConfig, backup_type: BackupType) -> bool {
        match backup_type {
            BackupType::RealTime => {
                if !config.enable_real_time_backup {
                    return false;
                }

                // Check debouncing delay
                if let Some(last_backup) = self.last_backup_time {
                    let seconds_since_last = (chrono::Utc::now() - last_backup).num_seconds();
                    if seconds_since_last < config.real_time_backup_delay as i64 {
                        return false;
                    }
                }
                true
            }
            BackupType::SessionStart => config.enable_session_backups,
            BackupType::SessionEnd => config.enable_final_backups,
            BackupType::Manual => true,
        }
    }

    pub fn record_backup(&mut self) {
        self.last_backup_time = Some(chrono::Utc::now());
        self.backup_count += 1;
    }

    pub fn duration(&self) -> chrono::Duration {
        chrono::Utc::now() - self.started_at
    }
}

/// Result type for backup operations
pub type BackupResult<T> = Result<T, BackupError>;

// Error types for backup operations
#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("File system error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Date/time parsing error: {0}")]
    ChronoParse(#[from] chrono::ParseError),

    #[error("Detection error: {0}")]
    Detection(#[from] crate::detection::DetectionError),

    #[error("Manifest resolution error: {0}")]
    Manifest(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Retention policy error: {0}")]
    Retention(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("File monitoring error: {0}")]
    Notify(#[from] notify::Error),

    #[error("Invalid configuration: {0}")]
    Configuration(String),
}

impl From<BackupError> for String {
    fn from(error: BackupError) -> String {
        error.to_string()
    }
}
