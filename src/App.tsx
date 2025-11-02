import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";
import AddGameModal from "./AddGameModal";

// Types for our application
interface Game {
  id: number;
  name: string;
  platform: string;
  platform_app_id?: string;
  last_detection_time?: string;
  backup_count: number;
  is_monitoring: boolean;
  confidence_score: number;
}

interface Notification {
  id: string;
  type: 'info' | 'success' | 'warning' | 'error';
  message: string;
  timestamp: Date;
}

interface AppSettings {
  confidence_threshold: number;
  max_backups_per_game: number;
  enable_auto_backup: boolean;
  enable_real_time_backup: boolean;
}

function App() {
  const [games, setGames] = useState<Game[]>([]);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [settings, setSettings] = useState<AppSettings>({
    confidence_threshold: 80.0,
    max_backups_per_game: 3,
    enable_auto_backup: true,
    enable_real_time_backup: true,
  });
  const [activeView, setActiveView] = useState<'dashboard' | 'games' | 'settings'>('dashboard');
  const [isScanning, setIsScanning] = useState(false);
  const [isAddGameModalOpen, setIsAddGameModalOpen] = useState(false);
  const [systemStatus, setSystemStatus] = useState({
    active_monitoring: 0,
    games_protected: 0,
    total_backups: 0,
  });

  // Initialize application and set up event listeners
  useEffect(() => {
    initializeApp();
    setupEventListeners();

    return () => {
      // Cleanup event listeners
    };
  }, []);

  const initializeApp = async () => {
    try {
      // Load initial data from backend
      await loadGameLibrary();
      await loadSystemStatus();
    } catch (error) {
      console.error('Failed to initialize app:', error);
      addNotification('error', 'Failed to initialize the application');
    }
  };

  const setupEventListeners = async () => {
    try {
      // Listen for backup events from the backend
      await listen('backup-events', (event: any) => {
        const payload = event.payload;
        handleBackupEvent(payload);
      });
    } catch (error) {
      console.error('Failed to setup event listeners:', error);
    }
  };

  const loadGameLibrary = async () => {
    try {
      // This would call a Tauri command to get all monitored games
      // For now, show placeholder logic
      const gameList = [
        {
          id: 1,
          name: "Sample Game 1",
          platform: "steam",
          platform_app_id: "123456",
          backup_count: 2,
          is_monitoring: true,
          confidence_score: 95.0,
        },
        {
          id: 2,
          name: "Sample Game 2",
          platform: "epic",
          backup_count: 1,
          is_monitoring: false,
          confidence_score: 88.0,
        },
      ];
      setGames(gameList);
    } catch (error) {
      console.error('Failed to load game library:', error);
    }
  };

  const loadSystemStatus = async () => {
    try {
      setSystemStatus({
        active_monitoring: games.filter(g => g.is_monitoring).length,
        games_protected: games.length,
        total_backups: games.reduce((sum, g) => sum + g.backup_count, 0),
      });
    } catch (error) {
      console.error('Failed to load system status:', error);
    }
  };

  const handleBackupEvent = (event: any) => {
    const eventType = event.type;
    const message = `Game ${event.game_id}: ${event.action}`;

    switch (eventType) {
      case 'game_session_started':
        addNotification('info', `Started monitoring game ${event.game_id}`);
        break;
      case 'backup_created':
        addNotification('success', `Backup created for game ${event.game_id}`);
        break;
      case 'game_session_ended':
        addNotification('info', `Stopped monitoring game ${event.game_id}`);
        break;
      case 'backup_failed':
        addNotification('error', `Backup failed for game ${event.game_id}: ${event.error}`);
        break;
      default:
        addNotification('info', message);
    }

    // Refresh data
    loadGameLibrary();
    loadSystemStatus();
  };

  const addNotification = (type: Notification['type'], message: string) => {
    const notification: Notification = {
      id: Date.now().toString(),
      type,
      message,
      timestamp: new Date(),
    };

    setNotifications(prev => [notification, ...prev.slice(0, 9)]); // Keep last 10
  };

  const scanRunningGames = async () => {
    setIsScanning(true);
    try {
      const result = await invoke("scan_running_games") as any;
      addNotification('info', `Scan completed. Found ${result.games?.length || 0} games`);
    } catch (error) {
      console.error('Failed to scan games:', error);
      addNotification('error', 'Failed to scan running games');
    } finally {
      setIsScanning(false);
    }
  };

  const createManualBackup = async (gameId: number) => {
    try {
      const backupId = await invoke("create_manual_backup", { gameId }) as string;
      addNotification('success', `Manual backup created: ${backupId}`);
      loadGameLibrary(); // Refresh game data
    } catch (error) {
      console.error('Failed to create manual backup:', error);
      addNotification('error', `Failed to create backup for game ${gameId}`);
    }
  };

  const toggleGameMonitoring = async (game: Game) => {
    // This would toggle monitoring on/off for a specific game
    addNotification('info', `${game.is_monitoring ? 'Stopped' : 'Started'} monitoring ${game.name}`);
    // In real implementation, this would call a Tauri command
  };

  const clearNotifications = () => {
    setNotifications([]);
  };

  const handleGameAdded = () => {
    addNotification('success', 'Game added successfully!');
    loadGameLibrary(); // Refresh the game list
    loadSystemStatus(); // Update system status
  };

  return (
    <div className="app">
      {/* Header */}
      <header className="app-header">
        <div className="header-content">
          <div className="logo-section">
            <h1>🎮 Save Steward</h1>
            <p>Automated Game Save Protection</p>
          </div>
          <nav className="nav-tabs">
            <button
              className={activeView === 'dashboard' ? 'active' : ''}
              onClick={() => setActiveView('dashboard')}
            >
              Dashboard
            </button>
            <button
              className={activeView === 'games' ? 'active' : ''}
              onClick={() => setActiveView('games')}
            >
              Game Library ({games.length})
            </button>
            <button
              className={activeView === 'settings' ? 'active' : ''}
              onClick={() => setActiveView('settings')}
            >
              Settings
            </button>
          </nav>
          <div className="header-actions">
            <button
              onClick={scanRunningGames}
              disabled={isScanning}
              className="scan-button"
            >
              {isScanning ? '🔍 Scanning...' : '🔍 Scan Games'}
            </button>
          </div>
        </div>
      </header>

      {/* System Status Bar */}
      <div className="status-bar">
        <div className="status-item">
          <span className="status-icon">📊</span>
          <span>Games Protected: {systemStatus.games_protected}</span>
        </div>
        <div className="status-item">
          <span className="status-icon">👁️</span>
          <span>Active Monitoring: {systemStatus.active_monitoring}</span>
        </div>
        <div className="status-item">
          <span className="status-icon">💾</span>
          <span>Total Backups: {systemStatus.total_backups}</span>
        </div>
      </div>

      {/* Main Content */}
      <main className="main-content">
        {activeView === 'dashboard' && (
          <div className="dashboard">
            {/* Recent Activity */}
            <section className="recent-activity">
              <h2>Recent Activity</h2>
              <div className="activity-list">
                {notifications.slice(0, 5).map(notification => (
                  <div key={notification.id} className={`activity-item ${notification.type}`}>
                    <span className={`activity-icon ${notification.type}`}>
                      {notification.type === 'success' ? '✅' :
                       notification.type === 'error' ? '❌' :
                       notification.type === 'warning' ? '⚠️' : 'ℹ️'}
                    </span>
                    <span className="activity-message">{notification.message}</span>
                    <span className="activity-time">
                      {notification.timestamp.toLocaleTimeString()}
                    </span>
                  </div>
                ))}
                {notifications.length === 0 && (
                  <div className="empty-activity">No recent activity</div>
                )}
              </div>
              {notifications.length > 0 && (
                <button
                  onClick={clearNotifications}
                  className="clear-notifications"
                >
                  Clear History
                </button>
              )}
            </section>

            {/* Active Monitoring */}
            <section className="active-monitoring">
              <h2>Active Sessions</h2>
              <div className="game-grid">
                {games.filter(g => g.is_monitoring).map(game => (
                  <div key={game.id} className="game-card active-session">
                    <div className="game-header">
                      <h3>{game.name}</h3>
                      <span className="platform-badge">{game.platform}</span>
                    </div>
                    <div className="monitoring-indicator">
                      <span className="pulse-dot"></span>
                      Currently Monitoring
                    </div>
                    <div className="game-stats">
                      <div className="stat">
                        <span className="stat-label">Backups</span>
                        <span className="stat-value">{game.backup_count}/3</span>
                      </div>
                      <div className="stat">
                        <span className="stat-label">Confidence</span>
                        <span className="stat-value">{game.confidence_score}%</span>
                      </div>
                    </div>
                  </div>
                ))}
                {games.filter(g => g.is_monitoring).length === 0 && (
                  <div className="no-active-sessions">
                    No games currently being monitored
                  </div>
                )}
              </div>
            </section>

            {/* Quick Actions */}
            <section className="quick-actions">
              <h2>Quick Actions</h2>
              <div className="action-buttons">
                <button onClick={scanRunningGames} disabled={isScanning}>
                  🔍 Scan for Running Games
                </button>
                <button onClick={() => setActiveView('settings')}>
                  ⚙️ Configure Settings
                </button>
              </div>
            </section>
          </div>
        )}

        {activeView === 'games' && (
          <div className="games-view">
            <div className="games-header">
              <h2>Game Library</h2>
              <button
                onClick={() => setIsAddGameModalOpen(true)}
                className="add-game-button"
              >
                ➕ Add Game
              </button>
            </div>
            <div className="game-grid">
              {games.map(game => (
                <div key={game.id} className="game-card">
                  <div className="game-header">
                    <h3>{game.name}</h3>
                    <span className={`platform-badge ${game.platform}`}>
                      {game.platform.toUpperCase()}
                    </span>
                  </div>

                  <div className="game-status">
                    <div className={`status-indicator ${game.is_monitoring ? 'active' : 'inactive'}`}>
                      <span className="status-dot"></span>
                      {game.is_monitoring ? 'Monitoring Active' : 'Not Monitoring'}
                    </div>
                  </div>

                  <div className="game-stats">
                    <div className="stat">
                      <span className="stat-label">Backups</span>
                      <span className="stat-value">{game.backup_count}/3</span>
                    </div>
                    <div className="stat">
                      <span className="stat-label">Confidence</span>
                      <span className="stat-value">{game.confidence_score}%</span>
                    </div>
                  </div>

                  <div className="game-actions">
                    <button
                      onClick={() => createManualBackup(game.id)}
                      className="backup-button"
                    >
                      💾 Manual Backup
                    </button>
                    <button
                      onClick={() => toggleGameMonitoring(game)}
                      className={game.is_monitoring ? 'stop-button' : 'start-button'}
                    >
                      {game.is_monitoring ? '⏹️ Stop' : '▶️ Monitor'}
                    </button>
                  </div>
                </div>
              ))}

              {games.length === 0 && (
                <div className="empty-library">
                  <h3>No Games Detected Yet</h3>
                  <p>Run some games and they'll appear here automatically!</p>
                  <button onClick={scanRunningGames} disabled={isScanning}>
                    {isScanning ? 'Scanning...' : 'Scan Now'}
                  </button>
                </div>
              )}
            </div>
          </div>
        )}

        {activeView === 'settings' && (
          <div className="settings-view">
            <h2>Settings</h2>

            <section className="settings-section">
              <h3>Detection Settings</h3>
              <div className="setting-item">
                <label htmlFor="confidence-threshold">
                  Minimum Confidence Threshold: {settings.confidence_threshold}%
                </label>
                <input
                  id="confidence-threshold"
                  type="range"
                  min="50"
                  max="100"
                  value={settings.confidence_threshold}
                  onChange={(e) => setSettings(prev => ({
                    ...prev,
                    confidence_threshold: parseInt(e.target.value)
                  }))}
                />
                <div className="slider-labels">
                  <span>50%</span>
                  <span>100%</span>
                </div>
              </div>
            </section>

            <section className="settings-section">
              <h3>Backup Policy</h3>
              <div className="setting-item">
                <label htmlFor="max-backups">
                  Maximum Backups per Game: {settings.max_backups_per_game}
                </label>
                <select
                  id="max-backups"
                  value={settings.max_backups_per_game}
                  onChange={(e) => setSettings(prev => ({
                    ...prev,
                    max_backups_per_game: parseInt(e.target.value)
                  }))}
                >
                  <option value="1">1 Backup</option>
                  <option value="2">2 Backups</option>
                  <option value="3">3 Backups (Recommended)</option>
                  <option value="5">5 Backups</option>
                  <option value="10">10 Backups</option>
                </select>
              </div>
            </section>

            <section className="settings-section">
              <h3>Automation</h3>
              <div className="setting-item">
                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={settings.enable_auto_backup}
                    onChange={(e) => setSettings(prev => ({
                      ...prev,
                      enable_auto_backup: e.target.checked
                    }))}
                  />
                  Enable Automatic Backup Creation
                </label>
              </div>
              <div className="setting-item">
                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={settings.enable_real_time_backup}
                    onChange={(e) => setSettings(prev => ({
                      ...prev,
                      enable_real_time_backup: e.target.checked
                    }))}
                  />
                  Enable Real-time Save Monitoring
                </label>
              </div>
            </section>

            <section className="settings-section">
              <button className="save-settings-button">
                💾 Save Settings
              </button>
              <p className="settings-note">
                Settings are automatically applied to new game sessions.
                Existing monitoring sessions will continue with old settings until restarted.
              </p>
            </section>
          </div>
        )}
      </main>

      <AddGameModal
        isOpen={isAddGameModalOpen}
        onClose={() => setIsAddGameModalOpen(false)}
        onGameAdded={handleGameAdded}
      />
    </div>
  );
}

export default App;
