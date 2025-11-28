use reqwest::Client;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use thiserror::Error;

use crate::pcgaming_wiki::{
    cache::PcgwCache,
    models::{CargoQueryResponse, GameSearchResult, PcgwGameInfo, PcgwSaveGameData, SaveLocationResult},
    query_builder::QueryBuilder,
    save_location_parser::SaveLocationParser,
};

#[derive(Error, Debug)]
pub enum PcgwError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct PcgwClient {
    http_client: Client,
}

impl PcgwClient {
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }

    // High-level methods with caching (requires Arc<Mutex<Connection>>)
    pub async fn search_games(&self, db: Arc<Mutex<Connection>>, name: &str) -> Result<Vec<GameSearchResult>, PcgwError> {
        let cache_key = format!("search:{}", name);

        // Check cache (sync)
        {
            let conn = db.lock().map_err(|_| PcgwError::Api("Database lock failed".to_string()))?;
            if let Ok(Some(cached_json)) = PcgwCache::get(&conn, &cache_key) {
                let response: CargoQueryResponse<PcgwGameInfo> = serde_json::from_str(&cached_json)?;
                return Ok(self.map_search_results(response));
            }
        }

        // Fetch from API
        let response_text = self.fetch_search_results_raw(name).await?;
        
        // Cache response (sync)
        {
            let conn = db.lock().map_err(|_| PcgwError::Api("Database lock failed".to_string()))?;
            let _ = PcgwCache::set(&conn, &cache_key, &response_text, 1);
        }

        let response: CargoQueryResponse<PcgwGameInfo> = serde_json::from_str(&response_text)?;
        Ok(self.map_search_results(response))
    }

    pub async fn get_save_locations(&self, db: Arc<Mutex<Connection>>, game_name: &str) -> Result<SaveLocationResult, PcgwError> {
        let cache_key = format!("save_loc:{}", game_name);

        // Check cache
        {
            let conn = db.lock().map_err(|_| PcgwError::Api("Database lock failed".to_string()))?;
            if let Ok(Some(cached_json)) = PcgwCache::get(&conn, &cache_key) {
                let response: CargoQueryResponse<PcgwSaveGameData> = serde_json::from_str(&cached_json)?;
                return Ok(self.parse_save_locations(response));
            }
        }

        // Fetch from API
        let response_text = self.fetch_save_locations_raw(game_name).await?;

        // Cache response
        {
            let conn = db.lock().map_err(|_| PcgwError::Api("Database lock failed".to_string()))?;
            let _ = PcgwCache::set(&conn, &cache_key, &response_text, 7);
        }

        let response: CargoQueryResponse<PcgwSaveGameData> = serde_json::from_str(&response_text)?;
        Ok(self.parse_save_locations(response))
    }

    // Low-level API methods (no caching)
    pub async fn fetch_search_results_raw(&self, name: &str) -> Result<String, PcgwError> {
        let url = QueryBuilder::build_search_query(name, 10)?;
        Ok(self.http_client.get(&url).send().await?.text().await?)
    }

    pub async fn fetch_save_locations_raw(&self, game_name: &str) -> Result<String, PcgwError> {
        let url = QueryBuilder::build_save_location_query(game_name)?;
        Ok(self.http_client.get(&url).send().await?.text().await?)
    }

    // Helper to parse raw JSON into SaveLocationResult (public for manual usage)
    pub fn parse_save_locations_json(&self, json: &str) -> Result<SaveLocationResult, PcgwError> {
        let response: CargoQueryResponse<PcgwSaveGameData> = serde_json::from_str(json)?;
        Ok(self.parse_save_locations(response))
    }

    fn map_search_results(&self, response: CargoQueryResponse<PcgwGameInfo>) -> Vec<GameSearchResult> {
        response.cargoquery.into_iter().map(|item| {
            let info = item.title;
            GameSearchResult {
                name: info.page_name,
                steam_id: info.steam_appid,
                developers: info.developers,
                publishers: info.publishers,
            }
        }).collect()
    }

    fn parse_save_locations(&self, response: CargoQueryResponse<PcgwSaveGameData>) -> SaveLocationResult {
        let mut result = SaveLocationResult {
            windows: Vec::new(),
            linux: Vec::new(),
            macos: Vec::new(),
            steam_play: Vec::new(),
        };

        for item in response.cargoquery {
            let data = item.title;
            
            if let Some(win) = data.windows {
                if let Some(path) = SaveLocationParser::resolve(&win) {
                    result.windows.push(path.to_string_lossy().to_string());
                }
            }
            if let Some(linux) = data.linux {
                if let Some(path) = SaveLocationParser::resolve(&linux) {
                    result.linux.push(path.to_string_lossy().to_string());
                }
            }
            // ... handle other platforms
        }
        
        result
    }
}
