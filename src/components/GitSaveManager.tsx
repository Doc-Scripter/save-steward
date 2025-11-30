import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Game {
  id: number;
  name: string;
  platform: string;
  executable_path: string;
  installation_path: string;
}

interface GitCommit {
  hash: string;
  message: string;
  timestamp: string;
  branch: string;
}

interface GitBranch {
  name: string;
  description: string;
  is_active: boolean;
  last_commit_hash: string;
}

interface GitHistoryItem {
  commits: GitCommit[];
  branches: GitBranch[];
  current_branch: string;
}

export const GitSaveManager: React.FC<{ game: Game }> = ({ game }) => {
  const [gitEnabled, setGitEnabled] = useState(false);
  const [gitHistory, setGitHistory] = useState<GitHistoryItem | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadGitHistory();
  }, [game.id]);

  const loadGitHistory = async () => {
    try {
      setIsLoading(true);
      setError(null);
      const history = await invoke('get_git_history', { gameId: game.id });
      setGitHistory(history as GitHistoryItem);
      setGitEnabled(true);
    } catch (err) {
      console.error('Failed to load Git history:', err);
      setGitEnabled(false);
      setError('Git is not enabled for this game');
    } finally {
      setIsLoading(false);
    }
  };

  const enableGit = async () => {
    try {
      setIsLoading(true);
      setError(null);
      await invoke('enable_git_for_game', { gameId: game.id });
      setGitEnabled(true);
      await loadGitHistory();
    } catch (err) {
      setError(`Failed to enable Git: ${err}`);
    } finally {
      setIsLoading(false);
    }
  };

  const switchBranch = async (branchName: string) => {
    try {
      setIsLoading(true);
      setError(null);
      await invoke('switch_save_branch', {
        gameId: game.id,
        branchName
      });
      await loadGitHistory();
    } catch (err) {
      setError(`Failed to switch branch: ${err}`);
    } finally {
      setIsLoading(false);
    }
  };

  const restoreToCommit = async (commitHash: string) => {
    if (!confirm('Are you sure you want to restore to this commit? This will overwrite your current save.')) {
      return;
    }
    
    try {
      setIsLoading(true);
      setError(null);
      await invoke('restore_to_commit', {
        gameId: game.id,
        commitHash
      });
      alert('Save restored successfully!');
    } catch (err) {
      setError(`Failed to restore to commit: ${err}`);
    } finally {
      setIsLoading(false);
    }
  };

  const syncToCloud = async () => {
    try {
      setIsLoading(true);
      setError(null);
      const result = await invoke('sync_to_cloud', { gameId: game.id });
      alert(`Cloud sync completed: ${JSON.stringify(result)}`);
    } catch (err) {
      setError(`Failed to sync to cloud: ${err}`);
    } finally {
      setIsLoading(false);
    }
  };

  if (isLoading && !gitHistory) {
    return (
      <div className="git-save-manager">
        <div className="loading">Loading Git history...</div>
      </div>
    );
  }

  if (!gitEnabled) {
    return (
      <div className="git-save-manager">
        <div className="git-not-enabled">
          <h3>Git Save Versioning Not Enabled</h3>
          <p>Enable Git version control for this game to track save changes and create checkpoints.</p>
          <button 
            className="btn btn-primary"
            onClick={enableGit}
            disabled={isLoading}
          >
            {isLoading ? 'Enabling...' : 'Enable Git Versioning'}
          </button>
          {error && <div className="error">{error}</div>}
        </div>
      </div>
    );
  }

  return (
    <div className="git-save-manager">
      <h3>Save History</h3>
      {error && <div className="error">{error}</div>}
      
      {/* Current Status */}
      <div className="git-status">
        <div className="status-item">
          <strong>Current Branch:</strong> {gitHistory?.current_branch || 'main'}
        </div>
        <div className="status-item">
          <strong>Commits:</strong> {gitHistory?.commits?.length || 0}
        </div>
        <div className="status-item">
          <strong>Branches:</strong> {gitHistory?.branches?.length || 0}
        </div>
      </div>

      {/* Actions */}
      <div className="git-actions">
        <div className="action-section">
          <button
            className="btn btn-info"
            onClick={syncToCloud}
            disabled={isLoading}
          >
            Sync to Cloud
          </button>
        </div>
      </div>

      {/* Branches */}
      {gitHistory?.branches && gitHistory.branches.length > 0 && (
        <div className="branches-section">
          <h4>Branches</h4>
          <div className="branches-list">
            {gitHistory.branches.map((branch) => (
              <div key={branch.name} className={`branch-item ${branch.is_active ? 'active' : ''}`}>
                <div className="branch-info">
                  <strong>{branch.name}</strong>
                  {branch.description && <span className="branch-description">{branch.description}</span>}
                  {branch.is_active && <span className="active-badge">Active</span>}
                </div>
                <div className="branch-actions">
                  {!branch.is_active && (
                    <button
                      className="btn btn-sm"
                      onClick={() => switchBranch(branch.name)}
                      disabled={isLoading}
                    >
                      Switch
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Commit History */}
      {gitHistory?.commits && gitHistory.commits.length > 0 && (
        <div className="commits-section">
          <h4>Commit History</h4>
          <div className="commits-list">
            {gitHistory.commits.map((commit) => (
              <div key={commit.hash} className="commit-item">
                <div className="commit-info">
                  <div className="commit-hash">{commit.hash.substring(0, 8)}</div>
                  <div className="commit-message">{commit.message}</div>
                  <div className="commit-meta">
                    <span className="commit-branch">{commit.branch}</span>
                    <span className="commit-timestamp">{new Date(commit.timestamp).toLocaleString()}</span>
                  </div>
                </div>
                <div className="commit-actions">
                  <button
                    className="btn btn-sm btn-danger"
                    onClick={() => restoreToCommit(commit.hash)}
                    disabled={isLoading}
                  >
                    Restore
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {gitHistory && gitHistory.commits?.length === 0 && (
        <div className="no-commits">
          <p>No commits yet. Create your first checkpoint to get started!</p>
        </div>
      )}
    </div>
  );
};
