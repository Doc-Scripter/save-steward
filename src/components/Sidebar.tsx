import React from 'react';
import { 
  Gamepad2, 
  GitBranch, 
  History, 
  Cloud, 
  Rocket, 
  Leaf, 
  Skull, 
  HardDrive,
  HelpCircle
} from 'lucide-react';

interface SidebarProps {
  activeView: string;
  setActiveView: (view: string) => void;
}

const Sidebar: React.FC<SidebarProps> = ({ activeView, setActiveView }) => {
  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <div className="logo-icon">🛡️</div>
        <div className="logo-text">
          <h1>Save Steward</h1>
          <span className="version">v2.1.4</span>
        </div>
      </div>

      <nav className="sidebar-nav">
        <button 
          className={`nav-item ${activeView === 'games' ? 'active' : ''}`}
          onClick={() => setActiveView('games')}
        >
          <span className="icon"><Gamepad2 size={20} /></span>
          Games
        </button>
        <button 
          className={`nav-item ${activeView === 'branches' ? 'active' : ''}`}
          onClick={() => setActiveView('branches')}
        >
          <span className="icon"><GitBranch size={20} /></span>
          Branches
        </button>
        <button 
          className={`nav-item ${activeView === 'history' ? 'active' : ''}`}
          onClick={() => setActiveView('history')}
        >
          <span className="icon"><History size={20} /></span>
          History
        </button>
        <button 
          className={`nav-item ${activeView === 'sync' ? 'active' : ''}`}
          onClick={() => setActiveView('sync')}
        >
          <span className="icon"><Cloud size={20} /></span>
          Sync Status
          <span className="status-dot online"></span>
        </button>
      </nav>

      <div className="recent-games-section">
        <h3>RECENT GAMES</h3>
        <div className="recent-game-item">
          <span className="game-icon purple"><Rocket size={14} /></span>
          <span>Cyberpunk 2077</span>
        </div>
        <div className="recent-game-item">
          <span className="game-icon green"><Leaf size={14} /></span>
          <span>Stardew Valley</span>
        </div>
        <div className="recent-game-item">
          <span className="game-icon red"><Skull size={14} /></span>
          <span>DOOM Eternal</span>
        </div>
      </div>

      <div className="sidebar-footer">
        <div className="storage-info">
          <div className="storage-text">
            <span><HardDrive size={12} style={{marginRight: 4, verticalAlign: 'middle'}}/> Storage Used</span>
            <span>2.3 GB / 50 GB</span>
          </div>
          <div className="storage-bar">
            <div className="storage-progress" style={{ width: '4.6%' }}></div>
          </div>
        </div>
        <button className="help-button"><HelpCircle size={14} /></button>
      </div>
    </aside>
  );
};

export default Sidebar;
