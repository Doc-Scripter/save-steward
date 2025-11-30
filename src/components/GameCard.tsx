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
  const [showMenu, setShowMenu] = useState(false);
  const [showGitManager, setShowGitManager] = useState(false);
  const [showCheckpointDialog, setShowCheckpointDialog] = useState(false);
  const [checkpointName, setCheckpointName] = useState('');
  const [checkpointDescription, setCheckpointDescription] = useState('');
  const [isCreatingCheckpoint, setIsCreatingCheckpoint] = useState(false);
  const [checkpointError, setCheckpointError] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // Close menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setShowMenu(false);
      }
    };

    if (showMenu) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [showMenu]);

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
          <div style={{ position: 'relative' }} ref={menuRef}>
            <button 
              className="icon-btn" 
              onClick={() => {
                console.log('3 dots button clicked!', 'Current showMenu:', showMenu);
                const newShowMenu = !showMenu;
                console.log('Setting showMenu to:', newShowMenu);
                setShowMenu(newShowMenu);
                console.log('showMenu state updated');
              }}
            >
              <MoreHorizontal size={16} />
            </button>
            {showMenu && (
              <div className="dropdown-menu">
                <button className="dropdown-item" onClick={() => { 
                  console.log('Create Checkpoint clicked');
                  setShowCheckpointDialog(true);
                  setShowMenu(false); 
                }}>
                  <Save size={14} />
                  <span>Create Checkpoint</span>
                </button>
                <button className="dropdown-item" onClick={() => { 
                  console.log('Edit Game clicked');
                  onEdit?.(); 
                  setShowMenu(false); 
                }}>
                  <Edit size={14} />
                  <span>Edit Game</span>
                </button>
                <button className="dropdown-item danger" onClick={() => { 
                  console.log('Delete clicked');
                  onDelete?.(); 
                  setShowMenu(false); 
                }}>
                  <Trash2 size={14} />
                  <span>Delete</span>
                </button>
              </div>
            )}
          </div>
        </div>

        {/* Checkpoint Creation Dialog */}
        {showCheckpointDialog && (
          <div className="modal-overlay" onClick={() => setShowCheckpointDialog(false)}>
            <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{
              maxWidth: '500px',
              padding: '1.5rem'
            }}>
              <h3 style={{ marginTop: 0, marginBottom: '1rem' }}>Create Save Checkpoint</h3>
              {checkpointError && (
                <div style={{
                  padding: '0.75rem',
                  marginBottom: '1rem',
                  backgroundColor: 'rgba(239, 68, 68, 0.1)',
                  border: '1px solid rgba(239, 68, 68, 0.3)',
                  borderRadius: '6px',
                  color: '#ef4444'
                }}>
                  {checkpointError}
                </div>
              )}
              <div style={{ marginBottom: '1rem' }}>
                <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: 500 }}>
                  Save Name <span style={{ color: '#ef4444' }}>*</span>
                </label>
                <input
                  type="text"
                  value={checkpointName}
                  onChange={(e) => setCheckpointName(e.target.value)}
                  placeholder="e.g., Before Boss Fight"
                  className="form-input"
                  style={{ width: '100%' }}
                  autoFocus
                />
              </div>
              <div style={{ marginBottom: '1.5rem' }}>
                <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: 500 }}>
                  Description (optional)
                </label>
                <textarea
                  value={checkpointDescription}
                  onChange={(e) => setCheckpointDescription(e.target.value)}
                  placeholder="Add any notes about this save point..."
                  className="form-input"
                  style={{ width: '100%', minHeight: '80px', resize: 'vertical' }}
                />
              </div>
              <div style={{ display: 'flex', gap: '0.75rem', justifyContent: 'flex-end' }}>
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
