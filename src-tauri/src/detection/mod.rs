pub mod engine;
pub mod platform;
pub mod process_monitor;
pub mod executable_analysis;
pub mod runtime_detection;
pub mod confidence_scorer;

pub use engine::GameIdentificationEngine;
pub use platform::{PlatformApiClient, PlatformGameInfo};
pub use process_monitor::ProcessMonitor;
pub use executable_analysis::{ExecutableAnalyzer, ExecutableSignature};
pub use runtime_detection::{RuntimeDetector, RuntimeSignature};
pub use confidence_scorer::{ConfidenceScorer, ConfidenceScore};

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum IdentificationConfidence {
    Definitive = 95,
    High = 80,
    Medium = 60,
    Low = 30,
    Uncertain = 10,
}

impl From<f32> for IdentificationConfidence {
    fn from(score: f32) -> Self {
        match score {
            s if s >= 95.0 => Self::Definitive,
            s if s >= 80.0 => Self::High,
            s if s >= 60.0 => Self::Medium,
            s if s >= 30.0 => Self::Low,
            _ => Self::Uncertain,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameIdentification {
    pub game_id: Option<i64>,
    pub candidate_games: Vec<GameCandidate>,
    pub confidence_score: f32,
    pub confidence_level: IdentificationConfidence,
    pub identification_methods: Vec<String>,
    pub process_info: Option<ProcessInfo>,
    pub requires_manual_confirmation: bool,
    pub identified_at: DateTime<Utc>,
    pub conflict_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameCandidate {
    pub game_id: i64,
    pub name: String,
    pub confidence_score: f32,
    pub matched_identifiers: Vec<String>,
    pub platform: Option<String>,
    pub platform_app_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub executable_path: String,
    pub window_title: Option<String>,
    pub parent_pid: Option<u32>,
    pub creation_time: Option<DateTime<Utc>>,
    pub memory_usage: u64,
    pub cpu_usage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentificationEvidence {
    pub executable_hash: Option<String>,
    pub window_title_patterns: Vec<String>,
    pub process_name: String,
    pub platform_ids: Vec<PlatformIdentifier>,
    pub file_signature: Option<String>,
    pub installation_path: Option<String>,
    pub game_features: Vec<GameFeature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformIdentifier {
    pub platform: String,
    pub app_id: String,
    pub confidence_weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameFeature {
    SteamAppId(String),
    EpicId(String),
    GogId(String),
    ExecutableHash(String),
    WindowTitlePattern(String),
    ProcessName(String),
    InstallationPath(String),
    SaveLocationPattern(String),
}

#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("Process monitoring failed: {0}")]
    ProcessMonitoringError(String),

    #[error("Executable analysis failed: {0}")]
    ExecutableAnalysisError(String),

    #[error("Platform API error: {0}")]
    PlatformApiError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Identification conflict: {0}")]
    ConflictError(String),
}
