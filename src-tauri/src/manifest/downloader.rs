use crate::database::DatabaseResult;
use crate::database::connection::DatabasePaths;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Ludusavi manifest downloader and cache manager
pub struct ManifestDownloader {
    client: Client,
    cache_duration: Duration,
    cache_dir: PathBuf,
    manifest_url: String,
}

impl ManifestDownloader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            cache_duration: Duration::from_secs(24 * 3600), // 24 hours
            cache_dir: DatabasePaths::cache_directory(),
            manifest_url: "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml".to_string(),
        }
    }

    /// Get manifest content, from cache or download
    pub async fn get_manifest(&self) -> DatabaseResult<String> {
        if let Ok(cached) = self.get_cached_manifest() {
            if !self.is_cache_expired(&cached) {
                return Ok(cached.content);
            }
        }

        let content = self.download_manifest().await?;
        self.cache_manifest(&content)?;
        Ok(content)
    }

    /// Download fresh manifest
    async fn download_manifest(&self) -> DatabaseResult<String> {
        let response = self.client
            .get(&self.manifest_url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::database::DatabaseError::Io(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("HTTP {} error", response.status()),
                )
            ).into());
        }

        let content = response.text().await?;
        self.validate_content(&content)?;
        Ok(content)
    }

    /// Basic content validation
    fn validate_content(&self, content: &str) -> DatabaseResult<()> {
        if !content.contains("version:") || !content.contains("games:") {
            return Err(crate::database::DatabaseError::Io(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid manifest content",
                )
            ).into());
        }
        Ok(())
    }

    /// Cache manifest content
    fn cache_manifest(&self, content: &str) -> DatabaseResult<()> {
        fs::create_dir_all(&self.cache_dir)?;

        let cached = CachedManifest {
            content: content.to_string(),
            downloaded_at: SystemTime::now(),
            version: Self::extract_version(content),
        };

        let cache_file = self.cache_file_path();
        let json = serde_json::to_string(&cached)?;
        fs::write(cache_file, json)?;
        Ok(())
    }

    /// Get cached manifest
    fn get_cached_manifest(&self) -> DatabaseResult<CachedManifest> {
        let cache_file = self.cache_file_path();
        if !cache_file.exists() {
            return Err(crate::database::DatabaseError::Io(
                std::io::Error::new(std::io::ErrorKind::NotFound, "Cache not found")
            ).into());
        }

        let content = fs::read_to_string(cache_file)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Check if cache is expired
    fn is_cache_expired(&self, cached: &CachedManifest) -> bool {
        SystemTime::now()
            .duration_since(cached.downloaded_at)
            .map(|age| age > self.cache_duration)
            .unwrap_or(true)
    }

    /// Extract version from manifest content
    fn extract_version(content: &str) -> Option<String> {
        content.lines()
            .find(|line| line.starts_with("version:"))
            .and_then(|line| line.split(':').nth(1))
            .map(|v| v.trim_matches(|c| c == '\'' || c == '"').to_string())
    }

    /// Get cache status info
    pub fn get_cache_info(&self) -> DatabaseResult<CacheInfo> {
        let cache_file = self.cache_file_path();
        let exists = cache_file.exists();

        let info = if exists {
            let cached = self.get_cached_manifest()?;
            let metadata = fs::metadata(&cache_file)?;
            CacheInfo {
                exists: true,
                size_bytes: metadata.len(),
                last_downloaded: Some(cached.downloaded_at),
                expired: self.is_cache_expired(&cached),
                version: cached.version,
            }
        } else {
            CacheInfo {
                exists: false,
                size_bytes: 0,
                last_downloaded: None,
                expired: true,
                version: None,
            }
        };

        Ok(info)
    }

    fn cache_file_path(&self) -> PathBuf {
        self.cache_dir.join("ludusavi_manifest.json")
    }

    /// Force refresh cache
    pub async fn refresh_cache(&self) -> DatabaseResult<String> {
        self.clear_cache()?;
        self.get_manifest().await
    }

    /// Clear cache
    pub fn clear_cache(&self) -> DatabaseResult<()> {
        let cache_file = self.cache_file_path();
        if cache_file.exists() {
            fs::remove_file(cache_file)?;
        }
        Ok(())
    }

    /// Set cache duration (for testing)
    #[cfg(test)]
    pub fn set_cache_duration(&mut self, duration: Duration) {
        self.cache_duration = duration;
    }
}

/// Cached manifest data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedManifest {
    pub content: String,
    pub downloaded_at: SystemTime,
    pub version: Option<String>,
}

/// Cache information
#[derive(Debug)]
pub struct CacheInfo {
    pub exists: bool,
    pub size_bytes: u64,
    pub last_downloaded: Option<SystemTime>,
    pub expired: bool,
    pub version: Option<String>,
}

impl CacheInfo {
    pub fn cache_age(&self) -> Option<String> {
        self.last_downloaded.and_then(|time| {
            SystemTime::now().duration_since(time).ok().map(|d| {
                let hours = d.as_secs() / 3600;
                if hours > 24 {
                    format!("{} days ago", hours / 24)
                } else if hours > 0 {
                    format!("{} hours ago", hours)
                } else {
                    format!("{} minutes ago", d.as_secs() / 60)
                }
            })
        })
    }
}
