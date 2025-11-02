use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::auto_backup::*;
use crate::database::DatabaseConnection;
use crate::manifest::ManifestResolver;
use crate::auto_backup::retention::RetentionConfig;

/// Main orchestrator for the auto-backup system
#[derive(Clone)]
pub struct AutoBackupManager {
    pub db_conn: DatabaseConnection,
    pub manifest_resolver: ManifestResolver,
    pub monitor: Arc<SaveMonitor>,
    pub retention_policy: RetentionPolicy,
    pub config: AutoBackupConfig,
    pub game_sessions: Arc<RwLock<HashMap<i64, GameSession>>>,
    pub event_sender: tokio::sync::broadcast::Sender<BackupEvent>,
}

impl AutoBackupManager {
    pub fn new(
        db_conn: DatabaseConnection,
        manifest_resolver: ManifestResolver,
        config: AutoBackupConfig,
    ) -> Self {
        let monitor = Arc::new(SaveMonitor::new());
        let retention_policy = RetentionPolicy::new(RetentionConfig::default())
            .with_database(db_conn.clone());

        let (tx, _) = tokio::sync::broadcast::channel(100);

        Self {
            db_conn,
            manifest_resolver,
            monitor,
            retention_policy,
            config,
            game_sessions: Arc::new(RwLock::new(HashMap::new())),
            event_sender: tx,
        }
    }

    /// Get event receiver for monitoring backup events
    pub fn get_event_receiver(&self) -> tokio::sync::broadcast::Receiver<BackupEvent> {
        self.event_sender.subscribe()
    }

    /// Handle a game identification event and start backup process
    pub async fn handle_game_identification(
        &self,
        game_id: i64,
        process_id: u32,
        confidence_score: f32
    ) -> BackupResult<()> {
        // Check if confidence meets threshold
        if confidence_score < self.config.min_confidence_threshold {
            return Ok(()); // Too low confidence, skip
        }

        // Check if we already have an active session for this game
        if self.get_active_session(game_id).await.is_some() {
            return Ok(()); // Already monitoring this game
        }

        // Create new game session
        let mut session = GameSession::new(game_id, process_id);

        // Resolve save locations using manifest data
        let save_paths = self.resolve_save_locations(game_id).await?;
        session.monitored_paths = save_paths.clone();

        // Start monitoring save directories
        self.monitor.start_monitoring_game(game_id, save_paths).await?;

        // Create initial backup (session start)
        if session.should_create_backup(&self.config, BackupType::SessionStart) {
            let backup_id = format!("session_start_{}", Uuid::new_v4().simple());
            self.create_backup(game_id, &backup_id, BackupType::SessionStart).await?;
            session.record_backup();
        }

        // Store session
        let mut sessions = self.game_sessions.write().await;
        sessions.insert(game_id, session);

        // Send event
        let _ = self.event_sender.send(BackupEvent::GameSessionStarted {
            game_id,
            process_id,
        });

        Ok(())
    }

    /// Handle game exit/stop monitoring
    pub async fn handle_game_exit(&self, game_id: i64) -> BackupResult<()> {
        // Get session
        let session_opt = {
            let mut sessions = self.game_sessions.write().await;
            sessions.remove(&game_id)
        };

        if let Some(mut session) = session_opt {
            // Stop monitoring
            self.monitor.stop_monitoring_game(game_id).await?;

            // Create final backup
            if session.should_create_backup(&self.config, BackupType::SessionEnd) {
                let backup_id = format!("session_end_{}", Uuid::new_v4().simple());
                self.create_backup(game_id, &backup_id, BackupType::SessionEnd).await?;
            }

            // Send event
            let _ = self.event_sender.send(BackupEvent::GameSessionEnded {
                game_id,
                session_id: session.session_id.clone(),
            });
        }

        Ok(())
    }

    /// Handle backup trigger event from file monitoring
    pub async fn handle_backup_trigger(&self, game_id: i64, backup_type: BackupType) -> BackupResult<()> {
        // Check if we have an active session
        if self.get_active_session(game_id).await.is_none() {
            return Ok(()); // No active session for this game
        }

        // Check with retention policy
        let (should_create, deleted_backup) = self.retention_policy.should_create_backup(game_id).await?;

        if let Some(deleted_id) = deleted_backup {
            eprintln!("Auto-deleted old backup: {}", deleted_id);
        }

        if !should_create {
            eprintln!("Skipping backup creation due to retention policy");
            return Ok(());
        }

        // Create the backup
        let backup_id = format!("{:?}_{}", backup_type, Uuid::new_v4().simple()).to_lowercase();
        self.create_backup(game_id, &backup_id, backup_type).await?;

        // Update session
        if let Some(session) = self.get_active_session(game_id).await.as_mut() {
            session.record_backup();
        }

        // Send event
        let _ = self.event_sender.send(BackupEvent::BackupCompleted {
            game_id,
            backup_id,
        });

        Ok(())
    }

    /// Manually create backup for a game
    pub async fn create_manual_backup(&self, game_id: i64) -> BackupResult<String> {
        let backup_id = format!("manual_{}", Uuid::new_v4().simple());

        // Check retention policy
        let (should_create, deleted_backup) = self.retention_policy.should_create_backup(game_id).await?;
        if let Some(deleted_id) = deleted_backup {
            eprintln!("Auto-deleted old backup: {}", deleted_id);
        }

        if !should_create {
            return Err(BackupError::Retention(
                "Cannot create backup - retention policy prevents it".to_string()
            ));
        }

        self.create_backup(game_id, &backup_id, BackupType::Manual).await?;
        Ok(backup_id)
    }

    /// Get backup statistics for a game
    pub async fn get_backup_stats(&self, game_id: i64) -> BackupResult<crate::auto_backup::retention::BackupStats> {
        self.retention_policy.get_backup_stats(game_id).await
    }

    /// Get active game session information
    pub async fn get_active_sessions(&self) -> Vec<GameSession> {
        let sessions = self.game_sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Clean up old data
    pub async fn cleanup(&self) -> BackupResult<()> {
        // Clean up old event debouncing data
        self.monitor.cleanup_old_events().await;

        // Clean up old sessions (stale sessions older than 24 hours)
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(24);

        let mut sessions = self.game_sessions.write().await;
        let stale_games: Vec<i64> = sessions.iter()
            .filter(|(_, session)| session.started_at < cutoff)
            .map(|(&game_id, _)| game_id)
            .collect();

        for game_id in stale_games {
            let session = sessions.remove(&game_id);
            if let Some(session) = session {
                // Stop monitoring if session was removed
                if let Err(e) = self.monitor.stop_monitoring_game(game_id).await {
                    eprintln!("Error stopping monitoring for stale session {}: {}", game_id, e);
                }

                let _ = self.event_sender.send(BackupEvent::GameSessionEnded {
                    game_id,
                    session_id: session.session_id,
                });
            }
        }

        Ok(())
    }

    async fn resolve_save_locations(&self, game_id: i64) -> BackupResult<Vec<String>> {
        // This would query the manifest resolver to get save paths for a game
        // For now, return a placeholder - this needs to integrate with the actual manifest system
        // TODO: Connect to ManifestResolver API

        // Placeholder implementation - in practice this would:
        // 1. Look up game in database to get manifest_id
        // 2. Use ManifestResolver to resolve actual save paths from manifest
        // 3. Return list of concrete paths to monitor

        eprintln!("TODO: Resolve actual save locations for game_id: {}", game_id);

        // For now, return empty list - will be replaced with proper manifest integration
        Ok(Vec::new())
    }

    async fn create_backup(&self, game_id: i64, backup_id: &str, backup_type: BackupType) -> BackupResult<()> {
        // This would create the actual backup file
        // For now, record the backup in the retention policy

        // TODO: Implement actual backup creation:
        // 1. Find save locations for the game
        // 2. Compress/backup the save files to a timestamped archive
        // 3. Store in backup directory
        // 4. Record in retention system

        let backup_path = format!("/tmp/backup_{}_{}.zip", game_id, backup_id); // Placeholder

        self.retention_policy.record_backup(
            game_id,
            backup_id.to_string(),
            retention::BackupType::from(backup_type),
            &backup_path,
            None, // Size not known yet
        ).await?;

        Ok(())
    }

    async fn get_active_session(&self, game_id: i64) -> Option<GameSession> {
        let sessions = self.game_sessions.read().await;
        sessions.get(&game_id).cloned()
    }
}

impl Drop for AutoBackupManager {
    fn drop(&mut self) {
        // Clean shutdown - stop all monitoring
        // Note: In async context, this would need careful handling
        eprintln!("AutoBackupManager shutting down - stopping all monitoring");
    }
}
