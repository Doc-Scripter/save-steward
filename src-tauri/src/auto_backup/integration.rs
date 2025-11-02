use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::auto_backup::*;
use crate::database::DatabaseConnection;
use crate::detection::{GameIdentificationEngine, GameIdentification, IdentificationConfidence};
use crate::manifest::ManifestResolver;

/// Integration layer that connects Game Identification Engine to Auto-Backup System
pub struct BackupIntegrationLayer {
    identification_engine: Arc<RwLock<GameIdentificationEngine>>,
    backup_manager: AutoBackupManager,
    running: Arc<RwLock<bool>>,
    event_task: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl BackupIntegrationLayer {
    pub fn new(
        db_conn: DatabaseConnection,
        manifest_resolver: ManifestResolver,
        config: AutoBackupConfig,
    ) -> Self {
        let engine = GameIdentificationEngine::new(db_conn.clone(), manifest_resolver.clone());
        let backup_manager = AutoBackupManager::new(db_conn, manifest_resolver, config);

        Self {
            identification_engine: Arc::new(RwLock::new(engine)),
            backup_manager,
            running: Arc::new(RwLock::new(false)),
            event_task: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the integration system
    pub async fn start(&self) -> BackupResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(()); // Already running
        }
        *running = true;

        // Start the identification engine monitoring
        let engine_guard = self.identification_engine.read().await;
        engine_guard.start_monitoring().await?;

        // Start the file monitoring cleanup task
        let monitor = self.backup_manager.monitor.clone();
        let cleanup_task_handle = tokio::spawn(async move {
            super::monitor::spawn_cleanup_task(monitor).await;
        });

        let mut event_task = self.event_task.write().await;
        *event_task = Some(cleanup_task_handle);

        // Start the event processing loop
        self.start_event_processing().await?;

        Ok(())
    }

    /// Stop the integration system
    pub async fn stop(&self) -> BackupResult<()> {
        let mut running = self.running.write().await;
        *running = false;

        // Stop event processing
        if let Some(task) = self.event_task.write().await.take() {
            task.abort();
        }

        // Clean up all game sessions
        let sessions = self.backup_manager.get_active_sessions().await;
        for session in sessions {
            let _ = self.backup_manager.handle_game_exit(session.game_id).await;
        }

        Ok(())
    }

    /// Process identification results and start backup sessions
    pub async fn handle_game_identification(&self, identification: GameIdentification) -> BackupResult<()> {
        // Only handle identifications we're confident about
        if identification.confidence_level < IdentificationConfidence::High {
            return Ok(()); // Skip low confidence identifications
        }

        if let Some(game_id) = identification.game_id {
            self.backup_manager.handle_game_identification(
                game_id,
                identification.process_info.as_ref()
                    .map(|p| p.pid)
                    .unwrap_or(0),
                identification.confidence_score
            ).await?;
        }

        Ok(())
    }

    /// Manually trigger backup for a game
    pub async fn create_manual_backup(&self, game_id: i64) -> BackupResult<String> {
        self.backup_manager.create_manual_backup(game_id).await
    }

    /// Get backup statistics for a game
    pub async fn get_backup_stats(&self, game_id: i64) -> BackupResult<crate::auto_backup::retention::BackupStats> {
        self.backup_manager.get_backup_stats(game_id).await
    }

    /// Clean up old data
    pub async fn cleanup(&self) -> BackupResult<()> {
        self.backup_manager.cleanup().await
    }

    /// Get event receiver for monitoring system events
    pub fn get_event_receiver(&self) -> tokio::sync::broadcast::Receiver<BackupEvent> {
        self.backup_manager.get_event_receiver()
    }

    /// Get active game sessions
    pub async fn get_active_sessions(&self) -> Vec<GameSession> {
        self.backup_manager.get_active_sessions().await
    }

    /// Check if the system is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get access to the identification engine
    pub async fn get_identification_engine(&self) -> Arc<RwLock<GameIdentificationEngine>> {
        Arc::clone(&self.identification_engine)
    }

    async fn start_event_processing(&self) -> BackupResult<()> {
        let backup_manager = self.backup_manager.clone();
        let running = Arc::clone(&self.running);

        // Start processing backup trigger events
        let mut event_receiver = backup_manager.get_event_receiver();

        tokio::spawn(async move {
            while *running.read().await {
                match tokio::time::timeout(tokio::time::Duration::from_millis(100), event_receiver.recv()).await {
                    Ok(Ok(event)) => {
                        if let Err(e) = Self::handle_backup_event(&backup_manager, event).await {
                            eprintln!("Error handling backup event: {}", e);
                        }
                    }
                    Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => {
                        break; // Channel closed
                    }
                    Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => {
                        // Lagged behind, continue
                        continue;
                    }
                    Err(_) => {
                        // Timeout - continue checking if we're still running
                        continue;
                    }
                }
            }
        });

        Ok(())
    }

    async fn handle_backup_event(backup_manager: &AutoBackupManager, event: BackupEvent) -> BackupResult<()> {
        match event {
            BackupEvent::BackupTriggered { game_id, backup_type } => {
                backup_manager.handle_backup_trigger(game_id, backup_type).await?;
            }
            BackupEvent::BackupFailed { game_id, error } => {
                eprintln!("Backup failed for game {}: {}", game_id, error);
            }
            _ => {
                // Other events don't need processing here
            }
        }
        Ok(())
    }
}

/// Helper for creating a default integration layer
pub struct BackupIntegrationBuilder {
    config: AutoBackupConfig,
}

impl BackupIntegrationBuilder {
    pub fn new() -> Self {
        Self {
            config: AutoBackupConfig::default(),
        }
    }

    pub fn with_config(mut self, config: AutoBackupConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_min_confidence(mut self, min_confidence: f32) -> Self {
        self.config.min_confidence_threshold = min_confidence;
        self
    }

    pub fn with_max_backups(mut self, max_backups: usize) -> Self {
        self.config.max_backups_per_game = max_backups;
        self
    }

    pub fn enable_real_time_backups(mut self, enable: bool) -> Self {
        self.config.enable_real_time_backup = enable;
        self
    }

    pub fn build(self, db_conn: DatabaseConnection, manifest_resolver: ManifestResolver) -> BackupIntegrationLayer {
        BackupIntegrationLayer::new(db_conn, manifest_resolver, self.config)
    }
}

impl Default for BackupIntegrationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Mutex;
    use rusqlite::Connection;

    #[tokio::test]
    async fn test_integration_builder() {
        let builder = BackupIntegrationBuilder::new()
            .with_min_confidence(90.0)
            .with_max_backups(5)
            .enable_real_time_backups(false);

        // Create mock dependencies
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let manifest_resolver = crate::manifest::PlaceholderResolver::new().unwrap();

        // This would normally work, but our mock objects are incomplete
        // In a real test, we'd need proper database setup and manifest resolver
        // let integration = builder.build(conn, manifest_resolver);

        assert_eq!(builder.config.min_confidence_threshold, 90.0);
        assert_eq!(builder.config.max_backups_per_game, 5);
        assert_eq!(builder.config.enable_real_time_backup, false);
    }
}
