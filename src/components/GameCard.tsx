import React, { useState, useRef, useEffect } from 'react';
import { Play, History, MoreHorizontal, CheckCircle, AlertCircle, WifiOff, RefreshCw, Cloud, Edit, Trash2 } from 'lucide-react';

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
          <button className="icon-btn"><History size={16} /></button>
          <div style={{ position: 'relative' }} ref={menuRef}>
            <button className="icon-btn" onClick={() => {
              console.log('3 dots button clicked!', 'Current showMenu:', showMenu);
              const newShowMenu = !showMenu;
              console.log('Setting showMenu to:', newShowMenu);
              setShowMenu(newShowMenu);
              console.log('showMenu state updated');
            }}>
              <MoreHorizontal size={16} />
            </button>
            {showMenu && (
              <div className="dropdown-menu">
                <div style={{ fontSize: '10px', color: 'yellow', padding: '2px', border: '1px solid yellow' }}>
                  DEBUG: Dropdown menu rendered! showMenu = {String(showMenu)}
                </div>
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
            {!showMenu && (
              <div style={{ fontSize: '10px', color: 'red', padding: '2px', position: 'absolute', top: '100%', right: 0, zIndex: 9999 }}>
                DEBUG: Dropdown NOT rendered. showMenu = {String(showMenu)}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default GameCard;
