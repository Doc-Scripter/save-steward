import React from 'react';
import { Play, History, MoreHorizontal, CheckCircle, AlertCircle, WifiOff, RefreshCw, Cloud } from 'lucide-react';

export interface GameData {
  id: number;
  name: string;
  version: string;
  lastSave: string;
  versionCount: number;
  branchCount: number;
  status: 'Active' | 'Syncing' | 'Offline' | 'Error' | 'Synced';
  bannerColor: string; // Placeholder for actual image
  icon?: string;
}

interface GameCardProps {
  game: GameData;
}

const GameCard: React.FC<GameCardProps> = ({ game }) => {
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
      <div className="game-banner" style={{ backgroundColor: game.bannerColor }}>
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
          <button className="launch-btn"><Play size={14} style={{marginRight: 6, verticalAlign: 'middle'}}/> Launch</button>
          <button className="icon-btn"><History size={16} /></button>
          <button className="icon-btn"><MoreHorizontal size={16} /></button>
        </div>
      </div>
    </div>
  );
};

export default GameCard;
