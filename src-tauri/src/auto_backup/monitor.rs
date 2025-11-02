use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{interval, timeout};
use notify::RecommendedWatcher;
use notify::RecursiveMode;
use notify::Watcher;

use crate::auto_backup::{BackupEvent, BackupType, BackupError, BackupResult};

/// Handles real-time monitoring of save file directories
pub struct SaveMonitor {
    monitors: Arc<RwLock<HashMap<String, MonitoredPath>>>,
    event_sender: tokio::sync::broadcast::Sender<BackupEvent>,
    debounced_events: Arc<RwLock<HashMap<String, tokio::time::Instant>>>,
}

impl SaveMonitor {
    pub fn new() -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(100);
        Self {
            monitors: Arc::new(RwLock::new(HashMap::new())),
            event_sender: tx,
            debounced_events: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_event_receiver(&self) -> tokio::sync::broadcast::Receiver<BackupEvent> {
        self.event_sender.subscribe()
    }

    /// Start monitoring a game session's save paths
    pub async fn start_monitoring_game(&self, game_id: i64, save_paths: Vec<String>) -> BackupResult<()> {
        let mut monitors = self.monitors.write().await;

        for path_str in save_paths {
            let path = PathBuf::from(&path_str);
            if path.exists() && path.is_dir() {
                self.start_monitoring_path(game_id, path, &mut monitors).await?;
            }
        }

        Ok(())
    }

    /// Stop monitoring a game's save paths
    pub async fn stop_monitoring_game(&self, game_id: i64) -> BackupResult<()> {
        let mut monitors = self.monitors.write().await;

        let keys_to_remove: Vec<String> = monitors.keys()
            .filter(|key| key.starts_with(&format!("game_{}_", game_id)))
            .cloned()
            .collect();

        for key in keys_to_remove {
            if let Some(mut monitor) = monitors.remove(&key) {
                monitor.watcher.configure(notify::Config::default())?;
                // The watcher will be dropped and stopped
            }
        }

        Ok(())
    }

    /// Get currently monitored paths for a game
    pub async fn get_monitored_paths(&self, game_id: i64) -> Vec<String> {
        let monitors = self.monitors.read().await;
        monitors.keys()
            .filter(|key| key.starts_with(&format!("game_{}_", game_id)))
            .map(|key| path_from_key(key))
            .collect()
    }

    /// Check if a path should trigger a backup (debounced)
    pub async fn should_trigger_backup(&self, game_id: i64, path: &str, debounce_ms: u64) -> bool {
        let key = format!("game_{}_{}", game_id, path);
        let now = tokio::time::Instant::now();

        let mut debounced = self.debounced_events.write().await;

        if let Some(last_time) = debounced.get(&key) {
            if now.duration_since(*last_time) < Duration::from_millis(debounce_ms) {
                return false;
            }
        }

        debounced.insert(key, now);
        true
    }

    async fn start_monitoring_path(
        &self,
        game_id: i64,
        path: PathBuf,
        monitors: &mut HashMap<String, MonitoredPath>,
    ) -> BackupResult<()> {
        let path_key = format!("game_{}_{}", game_id, path.display());
        let game_id_clone = game_id;
        let event_sender = self.event_sender.clone();

        // Create file watcher
        let closure_event_sender = event_sender.clone();
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<notify::Event, notify::Error>| {
                match result {
                    Ok(event) => {
                        // Convert to our event system
                        if should_handle_file_event(&event.kind) {
                            let event = BackupEvent::BackupTriggered {
                                game_id: game_id_clone,
                                backup_type: BackupType::RealTime,
                            };
                            let _ = closure_event_sender.send(event);
                        }
                    }
                    Err(e) => {
                        eprintln!("Watch error: {}", e);
                    }
                }
            },
            notify::Config::default(),
        )?;

        // Start watching the path
        watcher.watch(&path, RecursiveMode::Recursive)?;

        let monitored_path = MonitoredPath {
            path: path.clone(),
            game_id,
            watcher,
            last_change: None,
        };

        monitors.insert(path_key.clone(), monitored_path);

        // Send session start event
        let _ = event_sender.send(BackupEvent::GameSessionStarted {
            game_id,
            process_id: 0, // Will be filled by caller
        });

        Ok(())
    }

    /// Clean up old debounced events periodically
    pub async fn cleanup_old_events(&self) {
        let mut debounced = self.debounced_events.write().await;
        let now = tokio::time::Instant::now();

        debounced.retain(|_, time| {
            now.duration_since(*time) < Duration::from_secs(300) // 5 minutes
        });
    }
}

/// Handler for controlling the monitor from other threads
pub struct SaveMonitorHandle {
    monitor: Arc<SaveMonitor>,
}

impl SaveMonitorHandle {
    pub fn new(monitor: Arc<SaveMonitor>) -> Self {
        Self { monitor }
    }

    pub async fn start_monitoring_game(&self, game_id: i64, save_paths: Vec<String>) -> BackupResult<()> {
        self.monitor.start_monitoring_game(game_id, save_paths).await
    }

    pub async fn stop_monitoring_game(&self, game_id: i64) -> BackupResult<()> {
        self.monitor.stop_monitoring_game(game_id).await
    }

    pub async fn get_monitored_paths(&self, game_id: i64) -> Vec<String> {
        self.monitor.get_monitored_paths(game_id).await
    }
}

/// Represents a monitored save directory
#[derive(Debug)]
struct MonitoredPath {
    path: PathBuf,
    game_id: i64,
    watcher: RecommendedWatcher,
    last_change: Option<tokio::time::Instant>,
}

/// Extract path from monitor key
fn path_from_key(key: &str) -> String {
    if let Some(path_part) = key.split('_').nth(2) {
        path_part.to_string()
    } else {
        key.to_string()
    }
}

/// Determine if a file event should trigger a backup
fn should_handle_file_event(event_kind: &notify::EventKind) -> bool {
    match event_kind {
        notify::EventKind::Create(_) |
        notify::EventKind::Modify(_) |
        notify::EventKind::Remove(_) => true,
        _ => false,
    }
}

/// Background task that periodically cleans up debounced events
pub async fn spawn_cleanup_task(monitor: Arc<SaveMonitor>) {
    let mut interval = interval(Duration::from_secs(60)); // Clean every minute

    loop {
        interval.tick().await;
        monitor.cleanup_old_events().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_monitor_creation() {
        let monitor = SaveMonitor::new();
        let paths = monitor.get_monitored_paths(123).await;
        assert_eq!(paths.len(), 0);
    }

    #[tokio::test]
    async fn test_debounce_logic() {
        let monitor = SaveMonitor::new();

        // First call should return true
        let should1 = monitor.should_trigger_backup(123, "/test/path", 1000).await;
        assert!(should1);

        // Immediate second call should return false (debounced)
        let should2 = monitor.should_trigger_backup(123, "/test/path", 1000).await;
        assert!(!should2);

        // Wait longer than debounce period
        tokio::time::sleep(Duration::from_millis(50)).await;
        let should3 = monitor.should_trigger_backup(123, "/test/path", 10).await;
        assert!(should3);
    }
}
