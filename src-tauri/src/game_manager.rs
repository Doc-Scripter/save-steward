pub mod persistence;
pub mod detection;
pub mod pcgw;
pub mod utils;

use crate::database::models::*;
use std::sync::Arc;
use crate::pcgaming_wiki::PcgwClient;

use self::persistence::Persistence;
use self::detection::Detection;
use self::pcgw::PcgwIntegration;
use self::utils::Utils;

pub struct GameManager;

impl GameManager {
    /// Add a game manually with automatic save location detection
    pub async fn add_manual_game(
        db: &Arc<tokio::sync::Mutex<crate::database::connection::EncryptedDatabase>>,
        request: AddGameRequest,
    ) -> Result<GameWithSaves, String> {
        // 1. Pre-fetch PCGamingWiki data (outside transaction)
        let mut pcgw_save_locations: Option<Vec<SaveLocation>> = None;
        let mut pcgw_response_text: Option<String> = None;
        let cache_key = format!("save_loc:{}", request.name);
        
        // Check cache first
        {
            let conn_guard = db.lock().await;
            let conn = conn_guard.get_connection().await;
            if let Ok(Some(cached_json)) = crate::pcgaming_wiki::cache::PcgwCache::get(&conn, &cache_key) {
                 let client = PcgwClient::new();
                 if let Ok(result) = client.parse_save_locations_json(&cached_json) {
                     // Convert result to SaveLocation objects
                     pcgw_save_locations = Some(PcgwIntegration::convert_pcgw_locations(&result));
                 }
            }
        }
        
        // If not in cache, fetch from API
        if pcgw_save_locations.is_none() {
            let client = PcgwClient::new();
            if let Ok(text) = client.fetch_save_locations_raw(&request.name).await {
                pcgw_response_text = Some(text.clone());
                if let Ok(result) = client.parse_save_locations_json(&text) {
                     pcgw_save_locations = Some(PcgwIntegration::convert_pcgw_locations(&result));
                }
            }
        }

        let conn_guard = db.lock().await;
        let mut conn = conn_guard.get_connection().await;

        // Start transaction
        let tx = conn.transaction().map_err(|e| format!("Transaction error: {}", e))?;

        // Update cache if we fetched new data
        if let Some(text) = pcgw_response_text {
            let _ = crate::pcgaming_wiki::cache::PcgwCache::set(&tx, &cache_key, &text, 7);
        }

        // Insert game
        let game_id = Persistence::insert_game(&tx, &request)?;

        // Update game with PCGW data if available (including executables)
        if pcgw_save_locations.is_some() {
            // Fetch PCGW executables and update game record
            if let Some(page_name) = PcgwIntegration::extract_pcgw_page_name(&request.name) {
                if let Some(executables_json) = PcgwIntegration::fetch_pcgw_executables(&page_name) {
                    Persistence::update_game_platform_executables(&tx, game_id, &executables_json)?;
                }
            }
        }

        // Detect and insert save locations (passing pre-fetched data)
        let save_locations = Detection::detect_save_locations(&tx, game_id, &request, pcgw_save_locations)?;

        // Scan for existing saves
        let detected_saves = Detection::scan_existing_saves(&tx, game_id, &save_locations)?;

        tx.commit().map_err(|e| format!("Commit error: {}", e))?;

        Ok(GameWithSaves {
            game: Persistence::get_game_by_id(&*conn, game_id)?,
            save_locations: save_locations,
            detected_saves,
            user_config: None,
        })
    }

    // Delegate methods to submodules

    pub fn insert_game(tx: &rusqlite::Transaction, request: &AddGameRequest) -> Result<i64, String> {
        Persistence::insert_game(tx, request)
    }

    pub fn detect_save_locations(
        tx: &rusqlite::Transaction<'_>,
        game_id: i64,
        request: &AddGameRequest,
        pcgw_locations: Option<Vec<SaveLocation>>,
    ) -> Result<Vec<SaveLocation>, String> {
        Detection::detect_save_locations(tx, game_id, request, pcgw_locations)
    }

    pub fn insert_save_location(
        tx: &rusqlite::Transaction,
        game_id: i64,
        location: &SaveLocation,
    ) -> Result<i64, String> {
        Persistence::insert_save_location(tx, game_id, location)
    }

    pub fn scan_existing_saves(
        tx: &rusqlite::Transaction,
        game_id: i64,
        save_locations: &[SaveLocation],
    ) -> Result<Vec<DetectedSave>, String> {
        Detection::scan_existing_saves(tx, game_id, save_locations)
    }

    pub fn insert_detected_save(
        tx: &rusqlite::Transaction,
        game_id: i64,
        save_location_id: i64,
        actual_path: &str,
    ) -> Result<i64, String> {
        Persistence::insert_detected_save(tx, game_id, save_location_id, actual_path)
    }

    pub fn get_game_by_id(conn: &rusqlite::Connection, game_id: i64) -> Result<Game, String> {
        Persistence::get_game_by_id(conn, game_id)
    }

    pub async fn get_all_games(
        db: &std::sync::Arc<tokio::sync::Mutex<crate::database::connection::EncryptedDatabase>>,
    ) -> Result<Vec<Game>, String> {
        Persistence::get_all_games(db).await
    }

    pub async fn update_game(
        db: &Arc<tokio::sync::Mutex<crate::database::connection::EncryptedDatabase>>,
        game_id: i64,
        request: AddGameRequest,
    ) -> Result<Game, String> {
        Persistence::update_game(db, game_id, request).await
    }

    pub async fn delete_game(
        db: &Arc<tokio::sync::Mutex<crate::database::connection::EncryptedDatabase>>,
        game_id: i64,
    ) -> Result<(), String> {
        Persistence::delete_game(db, game_id).await
    }

    pub fn extract_pcgw_page_name(game_name: &str) -> Option<String> {
        PcgwIntegration::extract_pcgw_page_name(game_name)
    }

    pub fn fetch_pcgw_executables(page_name: &str) -> Option<String> {
        PcgwIntegration::fetch_pcgw_executables(page_name)
    }

    pub fn update_game_platform_executables(tx: &rusqlite::Transaction, game_id: i64, executables_json: &str) -> Result<(), String> {
        Persistence::update_game_platform_executables(tx, game_id, executables_json)
    }

    pub fn get_current_platform() -> &'static str {
        Utils::get_current_platform()
    }

    pub fn get_platform_executable(game: &Game) -> Option<String> {
        Utils::get_platform_executable(game)
    }

    pub fn convert_pcgw_locations(result: &crate::pcgaming_wiki::models::SaveLocationResult) -> Vec<SaveLocation> {
        PcgwIntegration::convert_pcgw_locations(result)
    }
}
