import React from 'react';
import { RefreshCw, GitFork, Clock, Shield } from 'lucide-react';

const QuickActions: React.FC = () => {
  return (
    <div className="quick-actions-grid">
      <div className="action-card">
        <div className="action-icon green-bg"><RefreshCw size={20} /></div>
        <div className="action-details">
          <h3>Sync All</h3>
          <span>2 pending</span>
        </div>
      </div>
      
      <div className="action-card">
        <div className="action-icon blue-bg"><GitFork size={20} /></div>
        <div className="action-details">
          <h3>Create Branch</h3>
          <span>New timeline</span>
        </div>
      </div>

      <div className="action-card">
        <div className="action-icon purple-bg"><Clock size={20} /></div>
        <div className="action-details">
          <h3>Recent Saves</h3>
          <span>View history</span>
        </div>
      </div>

      <div className="action-card">
        <div className="action-icon orange-bg"><Shield size={20} /></div>
        <div className="action-details">
          <h3>Backup</h3>
          <span>Create backup</span>
        </div>
      </div>
    </div>
  );
};

export default QuickActions;
