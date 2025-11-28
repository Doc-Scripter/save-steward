import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Search } from "lucide-react";
import "./App.css";
import Sidebar from "./components/Sidebar";
import QuickActions from "./components/QuickActions";
import GameCard, { GameData } from "./components/GameCard";
import AddGameModal from "./AddGameModal";

function App() {
  const [activeView, setActiveView] = useState('games');
  const [isAddGameModalOpen, setIsAddGameModalOpen] = useState(false);
  const [games, setGames] = useState<GameData[]>([]);
  const [loading, setLoading] = useState(true);

  // Fetch games from backend
  const fetchGames = async () => {
    try {
      setLoading(true);
      const result = await invoke<any[]>("get_all_games");
      
      // Transform backend Game model to frontend GameData
      const transformedGames: GameData[] = result.map((game: any) => ({
        id: game.id,
        name: game.name,
        version: game.platform_app_id || "v1.0",
        lastSave: "Not tracked",
        versionCount: 0,
        branchCount: 1,
        status: "Active" as const,
        bannerColor: "#4a148c", // Default color
        icon: game.icon_base64 || undefined,
        executablePath: game.executable_path || undefined,
      }));
      
      setGames(transformedGames);
    } catch (error) {
      console.error("Failed to fetch games:", error);
    } finally {
      setLoading(false);
    }
  };

  // Load games on mount
  useEffect(() => {
    fetchGames();
  }, []);

  // Handle game launch
  const handleLaunchGame = async (game: GameData) => {
    if (!game.executablePath) {
      console.error("No executable path for game:", game.name);
      return;
    }

    try {
      await invoke("launch_game", { executablePath: game.executablePath });
      console.log("Launched game:", game.name);
    } catch (error) {
      console.error("Failed to launch game:", error);
    }
  };

  // Handle game added
  const handleGameAdded = () => {
    fetchGames(); // Refresh the games list
  };

  return (
    <div className="app-container">
      <Sidebar activeView={activeView} setActiveView={setActiveView} />
      
      <main className="main-content">
        <header className="top-bar">
          <div className="breadcrumb">
            <h2>Game Library</h2>
            <span className="badge">{games.length} games tracked</span>
          </div>
          
          <div className="top-actions">
            <div className="search-bar">
              <span className="search-icon"><Search size={18} /></span>
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
            {loading ? (
              <p>Loading games...</p>
            ) : games.length === 0 ? (
              <p>No games added yet. Click "+ Add Game" to get started!</p>
            ) : (
              games.map(game => (
                <GameCard key={game.id} game={game} onLaunch={() => handleLaunchGame(game)} />
              ))
            )}
          </div>
        </div>
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

