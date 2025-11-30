import React, { useState, useRef, useEffect } from 'react';
import { Play, History, MoreHorizontal, CheckCircle, AlertCircle, WifiOff, RefreshCw, Cloud, Edit, Trash2, Save } from 'lucide-react';
import { GitSaveManager } from './GitSaveManager';
import { invoke } from '@tauri-apps/api/core';

export interface GameData {
  id: number;
  name: string;
  version: string;
  lastSave: string;
  versionCount: number;
  branchCount: number;
  status: 'Active' | 'Syncing' | 'Offline' | 'Error' | 'Synced';
  bannerColor: string; // Placeholder for actual image
  icon?: string; // Base64 encoded icon
  executablePath?: string; // Path to exe for launching
  platform?: string; // Game platform (steam, epic, etc.)
  installation_path?: string; // Installation directory
}

interface GameCardProps {
  game: GameData;
  onLaunch?: () => void;
  onEdit?: () => void;
  onDelete?: () => void;
}

const GameCard: React.FC<GameCardProps> = ({ game, onLaunch, onEdit, onDelete }) => {
  const [showActionsModal, setShowActionsModal] = useState(false);
  const [showGitManager, setShowGitManager] = useState(false);
  const [showCheckpointDialog, setShowCheckpointDialog] = useState(false);
  const [checkpointName, setCheckpointName] = useState('');
  const [checkpointDescription, setCheckpointDescription] = useState('');
  const [isCreatingCheckpoint, setIsCreatingCheckpoint] = useState(false);
  const [checkpointError, setCheckpointError] = useState<string | null>(null);


  const getStatusColor = (status: string) => {
    switch (status) {
      case 'Active': return 'green';
      case 'Syncing': return 'yellow';
      case 'Offline': return 'grey';
      case 'Error': return 'red';
      case 'Synced': return 'blue';
      default: return 'grey';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Active': return <CheckCircle size={12} />;
      case 'Syncing': return <RefreshCw size={12} />;
      case 'Offline': return <WifiOff size={12} />;
      case 'Error': return <AlertCircle size={12} />;
      case 'Synced': return <Cloud size={12} />;
      default: return <CheckCircle size={12} />;
    }
  };

  return (
    <div className="game-card-new">
      <div className="game-banner" style={{ 
        backgroundColor: game.icon ? 'transparent' : game.bannerColor,
        backgroundImage: game.icon ? `url(data:image/png;base64,${game.icon})` : 'none',
        backgroundSize: 'cover',
        backgroundPosition: 'center',
      }}>
        {/* In a real app, this would be an <img> tag */}
        <div className={`status-badge ${getStatusColor(game.status)}`}>
          {getStatusIcon(game.status)}
          {game.status}
        </div>
      </div>
      
      <div className="game-info">
        <div className="game-header-row">
          <h3>{game.name}</h3>
          <span className="version-tag">{game.version}</span>
        </div>

        <div className="game-stats-grid">
          <div className="stat-row">
            <span className="label">Last Save:</span>
            <span className="value">{game.lastSave}</span>
          </div>
          <div className="stat-row">
            <span className="label">Versions:</span>
            <span className="value">{game.versionCount} commits</span>
          </div>
          <div className="stat-row">
            <span className="label">Branches:</span>
            <span className="value">{game.branchCount} active</span>
          </div>
        </div>

        <div className="game-actions-row">
          <button className="launch-btn" onClick={onLaunch}><Play size={14} style={{marginRight: 6, verticalAlign: 'middle'}}/> Launch</button>
          <button 
            className={`icon-btn ${showGitManager ? 'active' : ''}`}
            onClick={() => setShowGitManager(!showGitManager)}
            title="Save History & Version Control"
          >
            <History size={16} />
          </button>
          <button 
            className="icon-btn" 
            onClick={() => setShowActionsModal(true)}
            title="More options"
          >
            <MoreHorizontal size={16} />
          </button>
        </div>

        {/* Game Actions Modal */}
        {showActionsModal && (
          <div className="modal-overlay" onClick={() => setShowActionsModal(false)}>
            <div className="modal-content modal-small" onClick={(e) => e.stopPropagation()}>
              <div className="modal-header">
                <h3>Game Actions</h3>
                <button className="modal-close-btn" onClick={() => setShowActionsModal(false)}
                  title="Close">
                  <span className="close-icon">×</span>
                </button>
              </div>
              <div className="modal-actions-list">
                <button className="modal-action-item" onClick={() => { 
                  setShowCheckpointDialog(true);
                  setShowActionsModal(false); 
                }}>
                  <Save size={18} />
                  <span>Create Checkpoint</span>
                </button>
                <button className="modal-action-item" onClick={() => { 
                  onEdit?.(); 
                  setShowActionsModal(false); 
                }}>
                  <Edit size={18} />
                  <span>Edit Game</span>
                </button>
                <button className="modal-action-item danger" onClick={() => { 
                  onDelete?.(); 
                  setShowActionsModal(false); 
                }}>
                  <Trash2 size={18} />
                  <span>Delete Game</span>
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Checkpoint Creation Dialog */}
        {showCheckpointDialog && (
          <div className="modal-overlay" onClick={() => setShowCheckpointDialog(false)}>
            <div className="modal-content modal-medium" onClick={(e) => e.stopPropagation()}>
              <div className="modal-header">
                <h3>Create Save Checkpoint</h3>
                <button className="modal-close-btn" onClick={() => setShowCheckpointDialog(false)}
                  title="Close">
                  <span className="close-icon">×</span>
                </button>
              </div>
              <div className="modal-body">
                {checkpointError && (
                  <div className="error-box">
                    {checkpointError}
                  </div>
                )}
                <div className="form-group">
                  <label>
                    Save Name <span className="required">*</span>
                  </label>
                  <input
                    type="text"
                    value={checkpointName}
                    onChange={(e) => setCheckpointName(e.target.value)}
                    placeholder="e.g., Before Boss Fight"
                    className="form-input"
                    autoFocus
                  />
                </div>
                <div className="form-group">
                  <label>Description (optional)</label>
                  <textarea
                    value={checkpointDescription}
                    onChange={(e) => setCheckpointDescription(e.target.value)}
                    placeholder="Add any notes about this save point..."
                    className="form-input"
                    rows={3}
                  />
                </div>
              </div>
              <div className="modal-footer">
                <button
                  className="btn"
                  onClick={() => {
                    setShowCheckpointDialog(false);
                    setCheckpointName('');
                    setCheckpointDescription('');
                    setCheckpointError(null);
                  }}
                  disabled={isCreatingCheckpoint}
                >
                  Cancel
                </button>
                <button
                  className="btn btn-primary"
                  onClick={async () => {
                    if (!checkpointName.trim()) {
                      setCheckpointError('Please enter a save name');
                      return;
                    }
                    try {
                      setIsCreatingCheckpoint(true);
                      setCheckpointError(null);
                      const message = checkpointDescription.trim() 
                        ? `${checkpointName}\n\n${checkpointDescription}`
                        : checkpointName;
                      await invoke('create_save_checkpoint', {
                        gameId: game.id,
                        message
                      });
                      setShowCheckpointDialog(false);
                      setCheckpointName('');
                      setCheckpointDescription('');
                      // Refresh git history if it's visible
                      if (showGitManager) {
                        window.location.reload(); // Simple refresh for now
                      }
                    } catch (err) {
                      setCheckpointError(`Failed to create checkpoint: ${err}`);
                    } finally {
                      setIsCreatingCheckpoint(false);
                    }
                  }}
                  disabled={!checkpointName.trim() || isCreatingCheckpoint}
                >
                  {isCreatingCheckpoint ? 'Creating...' : 'Create Checkpoint'}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Git Save Manager - Expandable */}
        {showGitManager && (
          <div className="git-manager-container" style={{ 
            marginTop: '1rem',
            padding: '1rem',
            borderTop: '1px solid var(--border-color)',
            backgroundColor: 'var(--surface-color)',
            borderRadius: '8px'
          }}>
            <GitSaveManager game={{
              id: game.id,
              name: game.name,
              platform: game.platform || '',
              executable_path: game.executablePath || '',
              installation_path: game.installation_path || ''
            }} />
          </div>
        )}
      </div>
    </div>
  );
};

export default GameCard;
