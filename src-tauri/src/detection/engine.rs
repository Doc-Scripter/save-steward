use crate::detection::{
    ProcessInfo, IdentificationEvidence, GameIdentification, GameCandidate,
    DetectionError, process_monitor::ProcessMonitor,
    executable_analysis::ExecutableAnalyzer, platform::PlatformApiClient,
    runtime_detection::RuntimeDetector, confidence_scorer::ConfidenceScorer
};
use crate::database::DatabaseConnection;
use crate::manifest::ManifestResolver;
use rusqlite::{params, Connection};
use std::collections::{HashMap, HashSet};
use chrono::Utc;
use tokio::sync::RwLock;

pub struct GameIdentificationEngine {
    db_conn: DatabaseConnection,
    process_monitor: ProcessMonitor,
    executable_analyzer: ExecutableAnalyzer,
    platform_client: PlatformApiClient,
    runtime_detector: RuntimeDetector,
    confidence_scorer: ConfidenceScorer,
    manifest_resolver: ManifestResolver,
    cache: RwLock<HashMap<String, GameIdentification>>,
    monitored_processes: RwLock<HashMap<u32, ProcessInfo>>,
}

impl GameIdentificationEngine {
    pub fn new(db_conn: DatabaseConnection, manifest_resolver: ManifestResolver) -> Self {
        Self {
            db_conn,
            process_monitor: ProcessMonitor::new(),
            executable_analyzer: ExecutableAnalyzer::new(),
            platform_client: PlatformApiClient::new(),
            runtime_detector: RuntimeDetector::new(),
            confidence_scorer: ConfidenceScorer::new(),
            manifest_resolver,
            cache: RwLock::new(HashMap::new()),
            monitored_processes: RwLock::new(HashMap::new()),
        }
    }

    pub async fn start_monitoring(&self) -> Result<(), DetectionError> {
        self.process_monitor.start_monitoring().await?;
        Ok(())
    }

    pub async fn identify_game_by_process(&self, pid: u32) -> Result<GameIdentification, DetectionError> {
        // Check cache first
        let process_key = format!("process_{}", pid);
        {
            let cache = self.cache.read().await;
            if let Some(result) = cache.get(&process_key) {
                return Ok(result.clone());
            }
        }

        // Get process information
        let process_info = self.process_monitor.get_process_info(pid).await?;
        let result = self.identify_game_from_process(&process_info).await?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(process_key, result.clone());
        }

        Ok(result)
    }

    pub async fn identify_game_from_path(&self, executable_path: &str) -> Result<GameIdentification, DetectionError> {
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(result) = cache.get(executable_path) {
                return Ok(result.clone());
            }
        }

        // Analyze executable
        let signature = self.executable_analyzer.analyze_executable(executable_path).await?;

        // Get platform information
        let platform_info = self.platform_client.get_platform_info(executable_path).await?;

        // Get runtime detection hints
        let runtime_signatures = self.runtime_detector.detect_runtime_signatures(executable_path).await?;

        // Build evidence
        let evidence = IdentificationEvidence {
            executable_hash: Some(signature.file_hash),
            window_title_patterns: runtime_signatures.window_title_patterns,
            process_name: std::path::Path::new(executable_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            platform_ids: platform_info.platform_ids,
            file_signature: Some(signature.product_name),
            installation_path: self.get_installation_path(executable_path),
            game_features: Vec::new(), // Will be populated from manifest
        };

        let result = self.identify_from_evidence(&evidence).await?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(executable_path.to_string(), result.clone());
        }

        Ok(result)
    }

    pub async fn scan_running_games(&self) -> Result<Vec<GameIdentification>, DetectionError> {
        let running_processes = self.process_monitor.get_running_processes().await?;
        let mut identifications = Vec::new();

        for process in running_processes {
            if self.is_game_process(&process).await {
                match self.identify_game_from_process(&process).await {
                    Ok(identification) => identifications.push(identification),
                    Err(e) => eprintln!("Failed to identify process {}: {}", process.pid, e),
                }
            }
        }

        Ok(identifications)
    }

    async fn identify_game_from_process(&self, process_info: &ProcessInfo) -> Result<GameIdentification, DetectionError> {
        // Analyze executable
        let signature = self.executable_analyzer.analyze_executable(&process_info.executable_path).await?;

        // Get platform information
        let platform_info = self.platform_client.get_platform_info(&process_info.executable_path).await?;

        // Get runtime detection
        let runtime_data = self.runtime_detector.detect_from_process(process_info).await?;

        // Build evidence
        let evidence = IdentificationEvidence {
            executable_hash: Some(signature.file_hash),
            window_title_patterns: vec![
                runtime_data.current_window_title.unwrap_or_default()
            ],
            process_name: process_info.name.clone(),
            platform_ids: platform_info.platform_ids,
            file_signature: Some(signature.product_name),
            installation_path: self.get_installation_path(&process_info.executable_path),
            game_features: Vec::new(),
        };

        self.identify_from_evidence(&evidence).await
    }

    async fn identify_from_evidence(&self, evidence: &IdentificationEvidence) -> Result<GameIdentification, DetectionError> {
        let conn = self.db_conn.lock().await;

        // Search database for matches
        let mut candidate_games = Vec::new();
        let mut identification_methods = Vec::new();

        // Search by executable hash
        if let Some(hash) = &evidence.executable_hash {
            if let Ok(game_ids) = self.find_games_by_hash(&conn, hash) {
                for game_id in game_ids {
                    candidate_games.push(self.build_candidate(&conn, game_id, hash.clone(), 95.0).await?);
                }
                identification_methods.push("executable_hash".to_string());
            }
        }

        // Search by platform IDs
        for platform_id in &evidence.platform_ids {
            if let Ok(game_ids) = self.find_games_by_platform_id(&conn, &platform_id.platform, &platform_id.app_id) {
                for game_id in game_ids {
                    if !candidate_games.iter().any(|c| c.game_id == game_id) {
                        candidate_games.push(self.build_candidate(&conn, game_id, format!("{}_{}", platform_id.platform, platform_id.app_id), platform_id.confidence_weight).await?);
                    }
                }
                identification_methods.push(platform_id.platform.clone());
            }
        }

        // Search by process name patterns
        if let Ok(game_ids) = self.find_games_by_process_name(&conn, &evidence.process_name) {
            for game_id in game_ids {
                if !candidate_games.iter().any(|c| c.game_id == game_id) {
                    candidate_games.push(self.build_candidate(&conn, game_id, format!("process_{}", evidence.process_name), 60.0).await?);
                }
            }
            identification_methods.push("process_name".to_string());
        }

        // Calculate overall confidence
        let (confidence_score, requires_confirmation, conflict_reason) =
            self.confidence_scorer.calculate_overall_confidence(&candidate_games);

        let selected_game = if candidate_games.len() == 1 {
            Some(candidate_games[0].game_id)
        } else if candidate_games.len() > 1 {
            // Use platform priority for conflicts
            self.resolve_conflict(&candidate_games)
        } else {
            None
        };

        Ok(GameIdentification {
            game_id: selected_game,
            candidate_games,
            confidence_score,
            confidence_level: confidence_score.into(),
            identification_methods,
            process_info: None, // Will be set by caller if available
            requires_manual_confirmation: requires_confirmation,
            identified_at: Utc::now(),
            conflict_reason,
        })
    }

    fn resolve_conflict(&self, candidates: &[GameCandidate]) -> Option<i64> {
        // Platform priority: Steam > Epic > GOG > Others
        let platform_priority = |platform: &Option<String>| -> i32 {
            match platform.as_deref() {
                Some("steam") => 4,
                Some("epic") => 3,
                Some("gog") => 2,
                Some(_) => 1,
                None => 0,
            }
        };

        candidates.iter()
            .max_by_key(|c| (
                c.confidence_score as i32,
                platform_priority(&c.platform)
            ))
            .map(|c| c.game_id)
    }

    async fn build_candidate(&self, conn: &Connection, game_id: i64, matched_identifier: String, confidence: f32) -> Result<GameCandidate, DetectionError> {
        let mut stmt = conn.prepare(
            "SELECT name, platform, platform_app_id FROM games WHERE id = ?"
        )?;

        let candidate = stmt.query_row(params![game_id], |row| {
            Ok(GameCandidate {
                game_id,
                name: row.get(0)?,
                confidence_score: confidence,
                matched_identifiers: vec![matched_identifier],
                platform: row.get(1)?,
                platform_app_id: row.get(2)?,
            })
        })?;

        Ok(candidate)
    }

    fn find_games_by_hash(&self, conn: &Connection, hash: &str) -> Result<Vec<i64>, DetectionError> {
        let mut stmt = conn.prepare(
            "SELECT game_id FROM game_identifiers WHERE identifier_type = 'executable_hash' AND identifier_value = ?"
        )?;

        let rows = stmt.query_map(params![hash], |row| row.get(0))?;
        let mut game_ids = Vec::new();

        for row in rows {
            game_ids.push(row?);
        }

        Ok(game_ids)
    }

    fn find_games_by_platform_id(&self, conn: &Connection, platform: &str, app_id: &str) -> Result<Vec<i64>, DetectionError> {
        let mut stmt = conn.prepare(
            "SELECT id FROM games WHERE platform = ? AND platform_app_id = ?"
        )?;

        let rows = stmt.query_map(params![platform, app_id], |row| row.get(0))?;
        let mut game_ids = Vec::new();

        for row in rows {
            game_ids.push(row?);
        }

        Ok(game_ids)
    }

    fn find_games_by_process_name(&self, conn: &Connection, process_name: &str) -> Result<Vec<i64>, DetectionError> {
        let mut stmt = conn.prepare(
            "SELECT game_id FROM game_identifiers WHERE identifier_type = 'process_name' AND identifier_value LIKE ?"
        )?;

        let pattern = format!("%{}%", process_name.to_lowercase());
        let rows = stmt.query_map(params![pattern], |row| row.get(0))?;
        let mut game_ids = Vec::new();

        for row in rows {
            game_ids.push(row?);
        }

        Ok(game_ids)
    }

    fn get_installation_path(&self, executable_path: &str) -> Option<String> {
        std::path::Path::new(executable_path)
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_string_lossy().to_string())
    }

    async fn is_game_process(&self, process_info: &ProcessInfo) -> bool {
        // Simple heuristic: check if process name doesn't contain system processes
        let system_processes = HashSet::from([
            "explorer.exe", "svchost.exe", "winlogon.exe", "csrss.exe", "smss.exe",
            "services.exe", "lsass.exe", "wininit.exe", "system", "init", "systemd",
            "launchd", "kernel_task", "WindowServer", "Dock"
        ]);

        // Check file size (games are typically larger than 10MB)
        if let Ok(metadata) = tokio::fs::metadata(&process_info.executable_path).await {
            if metadata.len() < 10_000_000 { // 10MB
                return false;
            }
        }

        !system_processes.contains(&process_info.name.to_lowercase().as_str())
    }

    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}
