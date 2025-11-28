import { useState } from "react";
import "./App.css";
import Sidebar from "./components/Sidebar";
import QuickActions from "./components/QuickActions";
import GameCard, { GameData } from "./components/GameCard";
import AddGameModal from "./AddGameModal";

function App() {
  const [activeView, setActiveView] = useState('games');
  const [isAddGameModalOpen, setIsAddGameModalOpen] = useState(false);

  // Mock Data matching the screenshot
  const games: GameData[] = [
    {
      id: 1,
      name: "Cyberpunk 2077",
      version: "v1.63",
      lastSave: "2 hours ago",
      versionCount: 47,
      branchCount: 3,
      status: "Active",
      bannerColor: "#4a148c" // Purple placeholder
    },
    {
      id: 2,
      name: "Stardew Valley",
      version: "v1.5.6",
      lastSave: "5 minutes ago",
      versionCount: 156,
      branchCount: 1,
      status: "Syncing",
      bannerColor: "#2e7d32" // Green placeholder
    },
    {
      id: 3,
      name: "DOOM Eternal",
      version: "v6.66",
      lastSave: "1 day ago",
      versionCount: 23,
      branchCount: 2,
      status: "Offline",
      bannerColor: "#b71c1c" // Red placeholder
    },
    {
      id: 4,
      name: "The Witcher 3",
      version: "v4.04",
      lastSave: "3 days ago",
      versionCount: 89,
      branchCount: 4,
      status: "Active",
      bannerColor: "#1a237e" // Blue placeholder
    },
    {
      id: 5,
      name: "Subnautica",
      version: "v2.0",
      lastSave: "Corrupted",
      versionCount: 34,
      branchCount: 1,
      status: "Error",
      bannerColor: "#006064" // Cyan placeholder
    },
    {
      id: 6,
      name: "Factorio",
      version: "v1.1.87",
      lastSave: "1 week ago",
      versionCount: 201,
      branchCount: 6,
      status: "Synced",
      bannerColor: "#e65100" // Orange placeholder
    }
  ];

  return (
    <div className="app-container">
      <Sidebar activeView={activeView} setActiveView={setActiveView} />
      
      <main className="main-content">
        <header className="top-bar">
          <div className="breadcrumb">
            <h2>Game Library</h2>
            <span className="badge">12 games tracked</span>
          </div>
          
          <div className="top-actions">
            <div className="search-bar">
              <span className="search-icon">🔍</span>
              <input type="text" placeholder="Search games..." />
            </div>
            <button className="add-game-btn" onClick={() => setIsAddGameModalOpen(true)}>
              + Add Game
            </button>
          </div>
        </header>

        <div className="content-scroll">
          <QuickActions />
          
          <div className="games-grid">
            {games.map(game => (
              <GameCard key={game.id} game={game} />
            ))}
          </div>
        </div>
      </main>

      <AddGameModal
        isOpen={isAddGameModalOpen}
        onClose={() => setIsAddGameModalOpen(false)}
        onGameAdded={() => {}}
      />
    </div>
  );
}

export default App;

