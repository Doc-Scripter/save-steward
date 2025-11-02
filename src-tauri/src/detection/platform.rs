use crate::detection::{ProcessInfo, DetectionError, PlatformIdentifier};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::time::{timeout, Duration};
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformGameInfo {
    pub platform_ids: Vec<PlatformIdentifier>,
    pub game_name: Option<String>,
    pub publisher: Option<String>,
    pub launcher_path: Option<String>,
    pub install_location: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlatformApiClient {
    http_client: Client,
    cache: std::sync::Arc<tokio::sync::RwLock<HashMap<String, PlatformGameInfo>>>,
}

impl PlatformApiClient {
    pub fn new() -> Self {
        Self {
            http_client: Client::builder()
                .user_agent("SaveSteward/1.0")
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
            cache: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_platform_info(&self, executable_path: &str) -> Result<PlatformGameInfo, DetectionError> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(info) = cache.get(executable_path) {
                return Ok(info.clone());
            }
        }

        let info = self.detect_platform_from_path(executable_path).await?;
        let processed_info = self.process_platform_info(info).await?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(executable_path.to_string(), processed_info.clone());
        }

        Ok(processed_info)
    }

    async fn detect_platform_from_path(&self, executable_path: &str) -> Result<PlatformGameInfo, DetectionError> {
        let path = Path::new(executable_path);
        let mut platform_ids = Vec::new();

        // Steam detection
        if let Some(steam_id) = self.detect_steam_app_id(path).await? {
            platform_ids.push(PlatformIdentifier {
                platform: "steam".to_string(),
                app_id: steam_id,
                confidence_weight: 95.0, // Very high confidence for Steam app IDs
            });
        }

        // Epic Games detection
        if let Some(epic_id) = self.detect_epic_game_id(path).await? {
            platform_ids.push(PlatformIdentifier {
                platform: "epic".to_string(),
                app_id: epic_id,
                confidence_weight: 90.0, // High confidence for Epic manifests
            });
        }

        // GOG detection
        if let Some(gog_id) = self.detect_gog_game_id(path).await? {
            platform_ids.push(PlatformIdentifier {
                platform: "gog".to_string(),
                app_id: gog_id,
                confidence_weight: 90.0, // High confidence for GOG manifests
            });
        }

        // EA Origin detection (Windows registry or path-based)
        if let Some(origin_id) = self.detect_origin_game_id(path).await? {
            platform_ids.push(PlatformIdentifier {
                platform: "origin".to_string(),
                app_id: origin_id,
                confidence_weight: 85.0,
            });
        }

        // Ubisoft Uplay detection
        if let Some(uplay_id) = self.detect_uplay_game_id(path).await? {
            platform_ids.push(PlatformIdentifier {
                platform: "uplay".to_string(),
                app_id: uplay_id,
                confidence_weight: 85.0,
            });
        }

        let game_name = self.extract_game_name_from_path(path);
        let install_location = path.parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_string_lossy().to_string());

        Ok(PlatformGameInfo {
            platform_ids,
            game_name,
            publisher: None, // Would need additional API calls
            launcher_path: None,
            install_location,
        })
    }

    async fn detect_steam_app_id(&self, path: &Path) -> Result<Option<String>, DetectionError> {
        // Look for Steam app ID in various ways:

        // 1. Check parent directories for appmanifest files
        if let Some(steamapps_dir) = self.find_steamapps_directory(path)? {
            if let Some(app_id) = self.scan_for_appmanifests(path, &steamapps_dir).await? {
                return Ok(Some(app_id));
            }
        }

        // 2. Check for steam_appid.txt files (common workaround)
        if let Some(app_id) = self.find_steam_appid_file(path).await? {
            return Ok(Some(app_id));
        }

        // 3. Check process environment or registry (would need additional platform-specific code)
        // For now, return None - this could be enhanced significantly

        Ok(None)
    }

    fn find_steamapps_directory(&self, path: &Path) -> Result<Option<std::path::PathBuf>, DetectionError> {
        let mut current = path.parent();

        while let Some(dir) = current {
            if dir.file_name().and_then(|n| n.to_str()) == Some("steamapps") {
                return Ok(Some(dir.to_path_buf()));
            }
            current = dir.parent();
        }

        Ok(None)
    }

    async fn scan_for_appmanifests(&self, _game_path: &Path, _steamapps_dir: &std::path::Path) -> Result<Option<String>, DetectionError> {
        // This would scan for appmanifest_*.acf files and parse them
        // For now, simplified implementation
        Ok(None)
    }

    async fn find_steam_appid_file(&self, path: &Path) -> Result<Option<String>, DetectionError> {
        let appid_path = path.parent().unwrap_or(path).join("steam_appid.txt");

        if appid_path.exists() {
            match tokio::fs::read_to_string(&appid_path).await {
                Ok(content) => {
                    let app_id = content.trim().to_string();
                    if !app_id.is_empty() {
                        return Ok(Some(app_id));
                    }
                }
                Err(_) => {} // File read error, continue
            }
        }

        Ok(None)
    }

    async fn detect_epic_game_id(&self, path: &Path) -> Result<Option<String>, DetectionError> {
        // Epic Games store detection
        // Look for Epic-specific paths or manifests

        // 1. Check for .egstore or similar Epic files
        if let Some(epic_manifest) = self.find_epic_manifest(path).await? {
            return Ok(Some(epic_manifest));
        }

        // 2. Check path patterns
        if let Some(epic_id) = self.extract_epic_id_from_path(path) {
            return Ok(Some(epic_id));
        }

        Ok(None)
    }

    async fn find_epic_manifest(&self, _path: &Path) -> Result<Option<String>, DetectionError> {
        // Epic Games manifest files would be in a different location
        // This requires knowing Epic installation directories
        Ok(None)
    }

    fn extract_epic_id_from_path(&self, path: &Path) -> Option<String> {
        // Look for Epic-specific path patterns
        // Epic often uses the catalog item ID in paths
        let path_str = path.to_string_lossy();

        // Pattern matching for Epic paths - this is speculative
        if path_str.contains("Epic Games") {
            // Could extract from path components, but this is complex
            // without actual Epic installation knowledge
        }

        None
    }

    async fn detect_gog_game_id(&self, path: &Path) -> Result<Option<String>, DetectionError> {
        // GOG detection
        // GOG Galaxy typically uses numeric IDs

        // Look for GOG-specific patterns
        if let Some(gog_id) = self.extract_gog_id_from_path(path) {
            return Ok(Some(gog_id));
        }

        // Could look for goggame-*.info files in GOG installations
        Ok(None)
    }

    fn extract_gog_id_from_path(&self, path: &Path) -> Option<String> {
        // GOG IDs are usually numeric
        let path_str = path.to_string_lossy();

        // Simple pattern match for GOG Galaxy directories
        let gog_re = Regex::new(r"goggame-(\d+)").ok()?;
        if let Some(caps) = gog_re.captures(&path_str) {
            if let Some(id_match) = caps.get(1) {
                return Some(id_match.as_str().to_string());
            }
        }

        None
    }

    async fn detect_origin_game_id(&self, _path: &Path) -> Result<Option<String>, DetectionError> {
        // EA Origin detection - typically Windows-only
        #[cfg(target_os = "windows")]
        {
            // Would need to check Windows registry for Origin game installations
            // This is complex and requires admin privileges in some cases
        }

        Ok(None)
    }

    async fn detect_uplay_game_id(&self, _path: &Path) -> Result<Option<String>, DetectionError> {
        // Ubisoft Uplay detection
        // Uplay typically stores game info in registry or specific directories
        Ok(None)
    }

    fn extract_game_name_from_path(&self, path: &Path) -> Option<String> {
        // Try to extract game name from the executable path
        // Usually the parent directory name is a good indicator

        path.parent()?
            .file_name()?
            .to_str()
            .map(|s| {
                // Clean up the name (remove version numbers, etc.)
                s.replace('_', " ")
                    .replace('-', " ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
    }

    async fn process_platform_info(&self, mut info: PlatformGameInfo) -> Result<PlatformGameInfo, DetectionError> {
        // Enhance the platform info with additional data if available

        // For Steam, we could fetch game details from local storage
        for platform_id in &info.platform_ids {
            match platform_id.platform.as_str() {
                "steam" => {
                    // In a full implementation, this could query Steam's local storage
                    // or API to get the actual game name
                }
                _ => {} // Other platforms could be supported here
            }
        }

        Ok(info)
    }

    pub async fn fetch_steam_game_info(&self, app_id: &str) -> Result<serde_json::Value, DetectionError> {
        // Fetch game info from Steam API (if needed)
        // This would be useful for getting game names and metadata
        // but requires Steam API key and is rate-limited

        let url = format!("https://store.steampowered.com/api/appdetails?appids={}&cc=us&l=english", app_id);

        let response = timeout(
            Duration::from_secs(5),
            self.http_client.get(&url).send()
        ).await.map_err(|_| DetectionError::PlatformApiError("Steam API timeout".to_string()))?
            .map_err(|e| DetectionError::PlatformApiError(format!("Steam API error: {}", e)))?;

        if !response.status().is_success() {
            return Err(DetectionError::PlatformApiError(
                format!("Steam API returned status: {}", response.status())
            ));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| DetectionError::PlatformApiError(format!("JSON parse error: {}", e)))?;

        Ok(json)
    }

    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn get_cached_info(&self, executable_path: &str) -> Option<PlatformGameInfo> {
        let cache = self.cache.read().await;
        cache.get(executable_path).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_platform_client_creation() {
        let client = PlatformApiClient::new();
        assert!(client.cache.read().await.is_empty());
    }

    #[test]
    fn test_game_name_extraction() {
        let client = PlatformApiClient::new();

        let test_cases = vec![
            ("/home/user/.steam/steam/steamapps/common/GameName/game.exe", "GameName"),
            ("/Program Files/Epic Games/Game/game.exe", "Game"),
            ("C:\\Games\\Some Game\\bin\\game.exe", "Some Game"),
        ];

        for (path_str, expected) in test_cases {
            let path = Path::new(path_str);
            let result = client.extract_game_name_from_path(path);
            assert_eq!(result.unwrap(), expected);
        }
    }
}
