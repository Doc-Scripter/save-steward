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
  const [editingGame, setEditingGame] = useState<GameData | null>(null);
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
        platform: game.platform || "steam",
        installation_path: game.installation_path || undefined,
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

  // Handle game launch with enhanced support for Unity games
  const handleLaunchGame = async (game: GameData) => {
    if (!game.executablePath) {
      console.error("No executable path for game:", game.name);
      return;
    }

    try {
      console.log(`Launching game: ${game.name} (${game.executablePath})`);
      
      // The backend now handles finding the best launcher and setting up the environment
      const result = await invoke("launch_game", { 
        executablePath: game.executablePath,
        installationPath: game.installation_path || ""
      });
      
      console.log("Launch result:", result);
      console.log("Launched game:", game.name);
    } catch (error) {
      console.error("Failed to launch game:", game.name, error);
      
      // Show user-friendly error message
      const errorMessage = error instanceof Error ? error.message : String(error);
      if (errorMessage.includes("UnityPlayer.so")) {
        alert(`Failed to launch ${game.name}. Unity games often require specific launcher scripts. Make sure the game is properly installed and has executable permissions.`);
      } else {
        alert(`Failed to launch ${game.name}: ${errorMessage}`);
      }
    }
  };

  // Handle game added/updated
  const handleGameAdded = () => {
    fetchGames(); // Refresh the games list
    setEditingGame(null); // Clear editing state
  };

  // Handle edit game
  const handleEditGame = (game: GameData) => {
    setEditingGame(game);
    setIsAddGameModalOpen(true);
  };

  // Handle delete game
  const handleDeleteGame = async (game: GameData) => {
    if (confirm(`Are you sure you want to delete "${game.name}"?`)) {
      try {
        await invoke("delete_game_sync", { gameId: game.id });
        console.log("Deleted game:", game.name);
        fetchGames(); // Refresh the list
      } catch (error) {
        console.error("Failed to delete game:", error);
        alert(`Failed to delete game: ${error}`);
      }
    }
  };

  // Handle modal close
  const handleCloseModal = () => {
    setIsAddGameModalOpen(false);
    setEditingGame(null);
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
                <GameCard 
                  key={game.id} 
                  game={game} 
                  onLaunch={() => handleLaunchGame(game)}
                  onEdit={() => handleEditGame(game)}
                  onDelete={() => handleDeleteGame(game)}
                />
              ))
            )}
          </div>
        </div>
      </main>

      <AddGameModal
        isOpen={isAddGameModalOpen}
        onClose={handleCloseModal}
        onGameAdded={handleGameAdded}
        editGame={editingGame ? {
          id: editingGame.id,
          name: editingGame.name,
          platform: editingGame.platform || "steam",
          platform_app_id: editingGame.version,
          executable_path: editingGame.executablePath,
          installation_path: editingGame.installation_path,
        } : undefined}
      />
    </div>
  );
}

export default App;
