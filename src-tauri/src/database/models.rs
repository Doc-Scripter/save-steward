use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub platform: String, // steam, epic, gog, standalone, other
    pub platform_app_id: Option<String>,
    pub executable_path: Option<String>,
    pub installation_path: Option<String>,
    pub genre: Option<String>,
    pub release_date: Option<String>,
    pub cover_image_url: Option<String>,
    pub icon_base64: Option<String>, // Base64 encoded icon
    pub icon_path: Option<String>, // Original exe path for auto-update
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveLocation {
    pub id: i64,
    pub game_id: i64,
    pub path_pattern: String,
    pub path_type: String, // directory, file, registry
    pub platform: Option<String>, // windows, macos, linux
    pub save_type: String, // auto, manual, cloud
    pub file_patterns: Option<String>, // JSON array
    pub exclude_patterns: Option<String>, // JSON array
    pub is_relative_to_user: bool,
    pub environment_variable: Option<String>,
    pub priority: i32, // 1-10
    pub detection_method: Option<String>,
    pub community_confirmed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedSave {
    pub id: i64,
    pub game_id: i64,
    pub save_location_id: i64,
    pub actual_path: String,
    pub current_hash: Option<String>,
    pub file_size: Option<i64>,
    pub last_modified: Option<DateTime<Utc>>,
    pub first_detected: DateTime<Utc>,
    pub last_checked: DateTime<Utc>,
    pub is_active: bool,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveVersion {
    pub id: i64,
    pub detected_save_id: i64,
    pub version_number: i32,
    pub backup_path: String,
    pub compressed_size: Option<i64>,
    pub original_hash: String,
    pub compressed_hash: String,
    pub compression_method: String,
    pub created_at: DateTime<Utc>,
    pub backup_reason: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameIdentifier {
    pub id: i64,
    pub game_id: i64,
    pub identifier_type: String, // executable_hash, window_title, process_name
    pub identifier_value: String,
    pub confidence_score: f32, // 0.0 to 1.0
    pub detection_context: Option<String>, // runtime, installation, manual
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGame {
    pub id: i64,
    pub game_id: i64,
    pub custom_name: Option<String>,
    pub custom_install_path: Option<String>,
    pub custom_save_path: Option<String>,
    pub is_favorite: bool,
    pub backup_enabled: bool,
    pub auto_backup_interval: i32, // seconds
    pub max_versions: i32,
    pub compression_level: i32, // 1-22 for zstd
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Response types for frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct GameWithSaves {
    pub game: Game,
    pub save_locations: Vec<SaveLocation>,
    pub detected_saves: Vec<DetectedSave>,
    pub user_config: Option<UserGame>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveBackupInfo {
    pub detected_save: DetectedSave,
    pub versions: Vec<SaveVersion>,
    pub last_backup: Option<DateTime<Utc>>,
    pub total_size: u64,
}

// Request types for operations
#[derive(Debug, Deserialize)]
pub struct CreateGameRequest {
    pub name: String,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub platform: String,
    pub platform_app_id: Option<String>,
    pub executable_path: Option<String>,
    pub installation_path: Option<String>,
    pub genre: Option<String>,
    pub cover_image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddGameRequest {
    pub name: String,
    pub platform: String,
    pub platform_app_id: Option<String>,
    pub executable_path: Option<String>,
    pub installation_path: Option<String>,
    pub icon_base64: Option<String>, // Base64 encoded icon
    pub icon_path: Option<String>, // Original exe path for icon extraction
}

#[derive(Debug, Deserialize)]
pub struct UpdateSaveLocationRequest {
    pub path_pattern: String,
    pub path_type: String,
    pub platform: Option<String>,
    pub file_patterns: Option<String>,
    pub exclude_patterns: Option<String>,
    pub environment_variable: Option<String>,
    pub detection_method: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BackupRequest {
    pub detected_save_id: i64,
    pub reason: String,
    pub compression_level: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct BackupResult {
    pub version_id: i64,
    pub backup_path: String,
    pub compressed_size: u64,
    pub created_at: DateTime<Utc>,
}

// Platform enums
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GamePlatform {
    Steam,
    Epic,
    Gog,
    Standalone,
    Origin,
    Uplay,
    Other(String),
}

impl GamePlatform {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "steam" => Self::Steam,
            "epic" => Self::Epic,
            "gog" => Self::Gog,
            "standalone" => Self::Standalone,
            "origin" => Self::Origin,
            "uplay" => Self::Uplay,
            other => Self::Other(other.to_string()),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Steam => "steam".to_string(),
            Self::Epic => "epic".to_string(),
            Self::Gog => "gog".to_string(),
            Self::Standalone => "standalone".to_string(),
            Self::Origin => "origin".to_string(),
            Self::Uplay => "uplay".to_string(),
            Self::Other(s) => s.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SaveLocationType {
    Directory,
    File,
    Registry,
}

impl SaveLocationType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "directory" => Self::Directory,
            "file" => Self::File,
            "registry" => Self::Registry,
            _ => Self::Directory, // default
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Directory => "directory".to_string(),
            Self::File => "file".to_string(),
            Self::Registry => "registry".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupReason {
    Auto,
    Manual,
    PreRestore,
    OnDemand,
}

impl BackupReason {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "auto" => Self::Auto,
            "manual" => Self::Manual,
            "pre_restore" => Self::PreRestore,
            "on_demand" => Self::OnDemand,
            _ => Self::Manual,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Auto => "auto".to_string(),
            Self::Manual => "manual".to_string(),
            Self::PreRestore => "pre_restore".to_string(),
            Self::OnDemand => "on_demand".to_string(),
        }
    }
}
