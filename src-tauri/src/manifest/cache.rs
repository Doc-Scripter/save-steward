use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::database::DatabaseResult;
use crate::manifest::parser::LudusaviManifest;
use crate::database::connection::DatabasePaths;

/// Manifest cache for parsed game data
pub struct ManifestCache {
    cache_dir: PathBuf,
    cache: HashMap<String, CachedGameInfo>,
}

impl ManifestCache {
    pub fn new() -> Self {
        Self {
            cache_dir: DatabasePaths::cache_directory().join("games"),
            cache: HashMap::new(),
        }
    }

    /// Load manifest and build game cache
    pub async fn load_manifest(&mut self, manifest_content: &str) -> DatabaseResult<()> {
        let manifest = crate::manifest::parser::ManifestParser::parse_yaml(manifest_content)?;
        self.build_game_cache(&manifest)?;
        self.persist_cache()?;
        Ok(())
    }

    /// Find games by name (fuzzy search)
    pub fn find_games(&self, search_term: &str, limit: usize) -> Vec<&CachedGameInfo> {
        let search_lower = search_term.to_lowercase();
        self.cache.values()
            .filter(|game| {
                game.name.to_lowercase().contains(&search_lower) ||
                game.display_name.as_ref()
                    .map(|n| n.to_lowercase().contains(&search_lower))
                    .unwrap_or(false)
            })
            .take(limit)
            .collect()
    }

    /// Get game by platform ID
    pub fn get_game_by_platform_id(&self, platform: &str, app_id: &str) -> Option<&CachedGameInfo> {
        self.cache.values()
            .find(|game| {
                game.platform == Some(platform.to_string()) &&
                game.platform_app_id == Some(app_id.to_string())
            })
    }

    /// Get total cached games
    pub fn game_count(&self) -> usize {
        self.cache.len()
    }

    /// Build cache from parsed manifest
    fn build_game_cache(&mut self, manifest: &LudusaviManifest) -> DatabaseResult<()> {
        let platform = crate::manifest::parser::Platform::current();
        let mut new_cache = HashMap::new();

        for (game_name, game_manifest) in &manifest.games {
            let save_locations = crate::manifest::parser::ManifestParser::extract_save_locations(
                game_name, game_manifest, &platform
            );

            let game_info = CachedGameInfo {
                name: game_name.clone(),
                display_name: game_manifest.display_name.clone(),
                platform: crate::manifest::parser::ManifestParser::get_platform_info(game_manifest),
                platform_app_id: crate::manifest::parser::ManifestParser::get_platform_app_id(game_manifest),
                save_location_count: save_locations.len(),
                save_locations,
            };

            new_cache.insert(game_name.clone(), game_info);
        }

        self.cache = new_cache;
        Ok(())
    }

    /// Load cached data from disk
    pub fn load_persisted_cache(&mut self) -> DatabaseResult<()> {
        let cache_file = self.cache_dir.join("manifest_games.json");

        if !cache_file.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&cache_file)?;
        self.cache = serde_json::from_str(&content)?;
        Ok(())
    }

    /// Persist cache to disk
    fn persist_cache(&self) -> DatabaseResult<()> {
        fs::create_dir_all(&self.cache_dir)?;
        let cache_file = self.cache_dir.join("manifest_games.json");

        let content = serde_json::to_string(&self.cache)?;
        fs::write(&cache_file, content)?;
        Ok(())
    }

    /// Clear persisted cache
    pub fn clear_persisted_cache(&self) -> DatabaseResult<()> {
        let cache_file = self.cache_dir.join("manifest_games.json");
        if cache_file.exists() {
            fs::remove_file(cache_file)?;
        }
        Ok(())
    }
}

/// Cached game information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedGameInfo {
    pub name: String,
    pub display_name: Option<String>,
    pub platform: Option<String>,
    pub platform_app_id: Option<String>,
    pub save_location_count: usize,
    pub save_locations: Vec<crate::manifest::parser::ManifestSaveLocation>,
}

impl CachedGameInfo {
    pub fn display_name(&self) -> &str {
        self.display_name.as_ref()
            .unwrap_or(&self.name)
    }

    pub fn has_save_locations(&self) -> bool {
        !self.save_locations.is_empty()
    }

    pub fn primary_platform_app_id(&self) -> Option<&str> {
        self.platform_app_id.as_deref()
    }
}

/// Cache statistics
pub struct CacheStats {
    pub total_games: usize,
    pub steam_games: usize,
    pub games_with_save_locations: usize,
}

impl ManifestCache {
    pub fn get_stats(&self) -> CacheStats {
        let total_games = self.cache.len();
        let steam_games = self.cache.values()
            .filter(|g| g.platform == Some("steam".to_string()))
            .count();
        let games_with_save_locations = self.cache.values()
            .filter(|g| g.has_save_locations())
            .count();

        CacheStats {
            total_games,
            steam_games,
            games_with_save_locations,
        }
    }
}
