use crate::database::models::*;
use chrono::Utc;
use std::path::Path;
use super::persistence::Persistence;

pub struct Detection;

impl Detection {
    /// Detect save locations for a game
    pub fn detect_save_locations(
        tx: &rusqlite::Transaction<'_>,
        game_id: i64,
        request: &AddGameRequest,
        pcgw_locations: Option<Vec<SaveLocation>>,
    ) -> Result<Vec<SaveLocation>, String> {
        let mut locations = Vec::new();

        // Use PCGW locations if available
        if let Some(pcgw_locs) = pcgw_locations {
            for loc in pcgw_locs {
                let id = Persistence::insert_save_location(tx, game_id, &loc)?;
                let mut loc_with_id = loc;
                loc_with_id.id = id;
                locations.push(loc_with_id);
            }
        }
        
        // Try manifest-based detection first
        if let Some(manifest_locations) = Self::detect_from_manifest(request)? {
            for loc in manifest_locations {
                let id = Persistence::insert_save_location(tx, game_id, &loc)?;
                let mut loc_with_id = loc;
                loc_with_id.id = id;
                locations.push(loc_with_id);
            }
        }

        // Fallback to heuristic detection if no locations found yet
        if locations.is_empty() {
            let heuristic_locations = Self::detect_from_heuristics(request)?;
            for loc in heuristic_locations {
                let id = Persistence::insert_save_location(tx, game_id, &loc)?;
                let mut loc_with_id = loc;
                loc_with_id.id = id;
                locations.push(loc_with_id);
            }
        }

        Ok(locations)
    }

    /// Try to detect save locations from manifest data
    fn detect_from_manifest(_request: &AddGameRequest) -> Result<Option<Vec<SaveLocation>>, String> {
        // This would integrate with ManifestResolver to find game data
        // For now, return None to use heuristics
        Ok(None)
    }

    /// Detect save locations using common patterns
    fn detect_from_heuristics(request: &AddGameRequest) -> Result<Vec<SaveLocation>, String> {
        let mut locations = Vec::new();

        match request.platform.as_str() {
            "steam" => {
                // Common Steam save locations
                if let Some(app_id) = &request.platform_app_id {
                    locations.push(SaveLocation {
                        id: 0, // Will be set after insert
                        game_id: 0, // Will be set by caller
                        path_pattern: format!("%STEAMUSER%/userdata/*/{}", app_id),
                        path_type: "directory".to_string(),
                        platform: Some("windows".to_string()),
                        save_type: "auto".to_string(),
                        file_patterns: Some(r#"["*.sav", "*.save", "*.dat"]"#.to_string()),
                        exclude_patterns: None,
                        is_relative_to_user: true,
                        environment_variable: Some("%APPDATA%".to_string()),
                        priority: 10,
                        detection_method: Some("heuristic".to_string()),
                        community_confirmed: false,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    });
                }
            }
            _ => {
                // Generic fallback - use installation directory
                if let Some(install_path) = &request.installation_path {
                    locations.push(SaveLocation {
                        id: 0,
                        game_id: 0,
                        path_pattern: format!("{}/save", install_path),
                        path_type: "directory".to_string(),
                        platform: None,
                        save_type: "auto".to_string(),
                        file_patterns: Some(r#"["*.sav", "*.save", "*.dat", "*.json"]"#.to_string()),
                        exclude_patterns: None,
                        is_relative_to_user: false,
                        environment_variable: None,
                        priority: 5,
                        detection_method: Some("heuristic".to_string()),
                        community_confirmed: false,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    });
                }
            }
        }

        Ok(locations)
    }

    /// Scan for existing save files
    pub fn scan_existing_saves(
        tx: &rusqlite::Transaction,
        game_id: i64,
        save_locations: &[SaveLocation],
    ) -> Result<Vec<DetectedSave>, String> {
        let mut detected_saves = Vec::new();

        for location in save_locations {
            // Resolve the actual path (simplified - would need path resolution logic)
            let resolved_paths = Self::resolve_save_paths(location)?;

            for actual_path in resolved_paths {
                if Path::new(&actual_path).exists() {
                    let id = Persistence::insert_detected_save(tx, game_id, location.id, &actual_path)?;
                    detected_saves.push(DetectedSave {
                        id,
                        game_id,
                        save_location_id: location.id,
                        actual_path,
                        current_hash: None, // Would compute hash
                        file_size: None,    // Would get file size
                        last_modified: Some(Utc::now()),
                        first_detected: Utc::now(),
                        last_checked: Utc::now(),
                        is_active: true,
                        metadata_json: None,
                    });
                }
            }
        }

        Ok(detected_saves)
    }

    /// Resolve save paths from patterns (simplified)
    fn resolve_save_paths(location: &SaveLocation) -> Result<Vec<String>, String> {
        // This would implement path resolution logic
        // For now, return the pattern as-is if it's an absolute path
        if Path::new(&location.path_pattern).is_absolute() {
            Ok(vec![location.path_pattern.clone()])
        } else {
            Ok(vec![])
        }
    }
}
