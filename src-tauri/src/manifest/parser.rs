use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::database::DatabaseResult;

/// Main manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LudusaviManifest {
    pub version: String,
    pub games: HashMap<String, GameManifest>,
}

/// Individual game entry in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameManifest {
    #[serde(rename = "name")]
    pub display_name: Option<String>,
    #[serde(rename = "type")]
    pub game_type: Option<String>,
    pub steam: Option<SteamInfo>,
    pub files: Option<HashMap<String, FileConfig>>,
    pub registry: Option<HashMap<String, RegistryConfig>>,
    #[serde(rename = "installDir")]
    pub install_dir: Option<InstallDirConfig>,
    #[serde(rename = "wine")]
    pub wine_config: Option<WineConfig>,
}

/// Steam-specific information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamInfo {
    pub id: Option<u64>,
}

/// File/directory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    pub tags: Option<Vec<String>>,
    pub when: Option<HashMap<String, String>>,
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub tags: Option<Vec<String>>,
    pub when: Option<HashMap<String, String>>,
}

/// Install directory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallDirConfig {
    pub tags: Option<Vec<String>>,
    pub when: Option<HashMap<String, String>>,
}

/// Wine configuration for Linux/macOS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WineConfig {
    pub prefix: Option<String>,
    pub exe: Option<String>,
    pub dllOverrides: Option<HashMap<String, String>>,
}

/// Manifest parser
pub struct ManifestParser;

impl ManifestParser {
    /// Parse YAML content into manifest structure
    pub fn parse_yaml(yaml_content: &str) -> DatabaseResult<LudusaviManifest> {
        let manifest: LudusaviManifest = serde_yaml::from_str(yaml_content)?;
        Ok(manifest)
    }

    /// Extract platform-specific save information for a game
    pub fn extract_save_locations(
        _game_name: &str,
        game_manifest: &GameManifest,
        _platform: &Platform,
    ) -> Vec<ManifestSaveLocation> {
        let mut locations = Vec::new();

        // Extract file/directory locations
        if let Some(files) = &game_manifest.files {
            for (relative_path, config) in files {
                // Check if this config applies to current platform
                if Self::config_applies_to_platform(&config.when, _platform) {
                    let tags = config.tags.clone().unwrap_or_default();

                    locations.push(ManifestSaveLocation {
                        relative_path: relative_path.clone(),
                        location_type: SaveLocationType::File,
                        tags,
                        platform: Some(_platform.to_string()),
                        detection_confidence: Self::calculate_confidence_score(&config.when, _platform),
                    });
                }
            }
        }

        // Extract registry locations (Windows only)
        if _platform == &Platform::Windows {
            if let Some(registry) = &game_manifest.registry {
                for (key_path, config) in registry {
                    if Self::config_applies_to_platform(&config.when, _platform) {
                        let tags = config.tags.clone().unwrap_or_default();

                        locations.push(ManifestSaveLocation {
                            relative_path: key_path.clone(),
                            location_type: SaveLocationType::Registry,
                            tags,
                            platform: Some("windows".to_string()),
                            detection_confidence: Self::calculate_confidence_score(&config.when, _platform),
                        });
                    }
                }
            }
        }

        locations
    }

    /// Check if a configuration condition applies to the current platform
    fn config_applies_to_platform(
        when: &Option<HashMap<String, String>>,
        platform: &Platform,
    ) -> bool {
        if let Some(conditions) = when {
            if let Some(os_condition) = conditions.get("os") {
                return Self::platform_matches_condition(platform, os_condition);
            }
        }
        // No conditions means it applies to all platforms
        true
    }

    /// Check if platform matches a condition
    fn platform_matches_condition(_platform: &Platform, condition: &str) -> bool {
        match _platform {
            Platform::Windows => {
                condition.contains("windows") || condition.contains("win")
            }
            Platform::Macos => {
                condition.contains("macos") || condition.contains("osx") || condition.contains("darwin")
            }
            Platform::Linux => {
                condition.contains("linux")
            }
        }
    }

    /// Calculate confidence score for detection (0-100)
    fn calculate_confidence_score(
        when: &Option<HashMap<String, String>>,
        platform: &Platform,
    ) -> u8 {
        let mut score = 90; // Base high confidence for manifest data

        if let Some(conditions) = when {
            if conditions.contains_key("os") {
                score += 5; // Platform-specific condition
            }
        }

        score.min(100)
    }

    /// Get platform info from manifest
    pub fn get_platform_info(game_manifest: &GameManifest) -> Option<String> {
        if let Some(steam) = &game_manifest.steam {
            if steam.id.is_some() {
                return Some("steam".to_string());
            }
        }

        // Could extend for Epic, GOG, etc.
        None
    }

    /// Get platform app ID
    pub fn get_platform_app_id(game_manifest: &GameManifest) -> Option<String> {
        if let Some(steam) = &game_manifest.steam {
            if let Some(id) = steam.id {
                return Some(id.to_string());
            }
        }
        None
    }
}

/// Platform enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Windows,
    Macos,
    Linux,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::Macos
        } else {
            Platform::Linux
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Platform::Windows => "windows".to_string(),
            Platform::Macos => "macos".to_string(),
            Platform::Linux => "linux".to_string(),
        }
    }
}

/// Save location type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SaveLocationType {
    File,
    Registry,
}

impl SaveLocationType {
    pub fn to_string(&self) -> String {
        match self {
            SaveLocationType::File => "file".to_string(),
            SaveLocationType::Registry => "registry".to_string(),
        }
    }
}

/// Extracted save location from manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSaveLocation {
    pub relative_path: String,
    pub location_type: SaveLocationType,
    pub tags: Vec<String>,
    pub platform: Option<String>,
    pub detection_confidence: u8,
}

/// Game info extracted from manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestGameInfo {
    pub name: String,
    pub display_name: Option<String>,
    pub platform: Option<String>,
    pub platform_app_id: Option<String>,
    pub save_locations: Vec<ManifestSaveLocation>,
}
