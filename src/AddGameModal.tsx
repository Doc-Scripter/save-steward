import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { GameSearch } from "./components/GameSearch";

interface AddGameModalProps {
  isOpen: boolean;
  onClose: () => void;
  onGameAdded: () => void;
  editGame?: {
    id: number;
    name: string;
    platform: string;
    platform_app_id?: string;
    executable_path?: string;
    installation_path?: string;
  };
}

interface GameFormData {
  name: string;
  platform: string;
  platform_app_id: string;
  executable_path: string;
  installation_path: string;
}

const PLATFORMS = [
  { value: "steam", label: "Steam" },
  { value: "epic", label: "Epic Games" },
  { value: "gog", label: "GOG" },
  { value: "standalone", label: "Standalone" },
  { value: "origin", label: "Origin" },
  { value: "uplay", label: "Uplay" },
  { value: "other", label: "Other" },
];

function AddGameModal({ isOpen, onClose, onGameAdded, editGame }: AddGameModalProps) {
  const [mode, setMode] = useState<'search' | 'manual'>('search');
  const [step, setStep] = useState<number>(1); // 1: Search, 2: Select Folder, 3: Confirm
  
  const [formData, setFormData] = useState<GameFormData>({
    name: "",
    platform: "steam",
    platform_app_id: "",
    executable_path: "",
    installation_path: "",
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedGame, setSelectedGame] = useState<any>(null);

  // Update form when editGame changes
  useEffect(() => {
    if (editGame) {
      setMode('manual');
      setFormData({
        name: editGame.name,
        platform: editGame.platform,
        platform_app_id: editGame.platform_app_id || "",
        executable_path: editGame.executable_path || "",
        installation_path: editGame.installation_path || "",
      });
    } else {
      resetForm();
    }
  }, [editGame]);

  const resetForm = () => {
    setMode('search');
    setStep(1);
    setFormData({
      name: "",
      platform: "steam",
      platform_app_id: "",
      executable_path: "",
      installation_path: "",
    });
    setSelectedGame(null);
    setError(null);
  };

  const handleGameSelect = (game: any) => {
    console.log('[DEBUG] Game selected from PGWK:', game);
    console.log('[DEBUG] Steam ID:', game.steam_id);
    setSelectedGame(game);
    setFormData(prev => ({
      ...prev,
      name: game.name,
      platform_app_id: game.steam_id || "",
      platform: game.steam_id ? "steam" : "standalone"
    }));
    console.log('[DEBUG] Platform App ID set to:', game.steam_id || "(empty)");
    setStep(2);
  };

  const handleFolderSelect = async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: true,
      });

      if (selected && typeof selected === "string") {
        setFormData(prev => ({ ...prev, installation_path: selected }));
        
        // Auto-detect executable
        try {
          const exePath = await invoke<string>("detect_game_executable", {
            folderPath: selected,
            gameName: formData.name
          });
          setFormData(prev => ({ ...prev, executable_path: exePath }));
        } catch (e) {
          console.warn("Could not auto-detect executable:", e);
        }

        // Auto-detect save location (future)
        // const saveLocs = await invoke("get_pcgw_save_locations", { gameName: formData.name });
        
        setStep(3);
      }
    } catch (err) {
      console.error("Failed to select folder:", err);
    }
  };

  const handleInputChange = (field: keyof GameFormData, value: string) => {
    setFormData(prev => ({ ...prev, [field]: value }));
    if (error) setError(null);
  };

  const handleBrowseExecutable = async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
      });
      
      if (selected && typeof selected === "string") {
        handleInputChange("executable_path", selected);
      }
    } catch (err) {
      console.error("Failed to open file dialog:", err);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    setError(null);

    try {
      const requestData = {
        name: formData.name.trim(),
        platform: formData.platform,
        platform_app_id: formData.platform_app_id.trim() || null,
        executable_path: formData.executable_path.trim() || null,
        installation_path: formData.installation_path.trim() || null,
        icon_base64: null,
        icon_path: formData.executable_path.trim() || null,
      };

      if (editGame) {
        await invoke("update_game_sync", { gameId: editGame.id, request: requestData });
      } else {
        await invoke("add_manual_game_sync", { request: requestData });
      }

      onGameAdded();
      handleClose();
    } catch (err) {
      console.error("Failed to save game:", err);
      setError(err instanceof Error ? err.message : "Failed to save game");
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleClose = () => {
    resetForm();
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay">
      <div className="modal-content" style={{ maxWidth: '600px', width: '90%' }}>
        <div className="modal-header">
          <h2>{editGame ? "Edit Game" : "Add New Game"}</h2>
          <button onClick={handleClose} className="close-button">×</button>
        </div>

        {!editGame && (
          <div style={{ display: 'flex', gap: '10px', marginBottom: '20px', borderBottom: '1px solid #eee', paddingBottom: '10px' }}>
            <button 
              onClick={() => { setMode('search'); setStep(1); }}
              style={{ 
                background: mode === 'search' ? '#6366f1' : 'transparent',
                color: mode === 'search' ? 'white' : 'inherit',
                border: 'none', padding: '8px 16px', borderRadius: '4px', cursor: 'pointer'
              }}
            >
              Search & Auto-Detect
            </button>
            <button 
              onClick={() => setMode('manual')}
              style={{ 
                background: mode === 'manual' ? '#6366f1' : 'transparent',
                color: mode === 'manual' ? 'white' : 'inherit',
                border: 'none', padding: '8px 16px', borderRadius: '4px', cursor: 'pointer'
              }}
            >
              Manual Entry
            </button>
          </div>
        )}

        {mode === 'search' && step === 1 && (
          <div>
            <p style={{ marginBottom: '10px' }}>Search for a game to auto-fill details from PCGamingWiki.</p>
            <GameSearch onSelect={handleGameSelect} />
          </div>
        )}

        {mode === 'search' && step === 2 && (
          <div style={{ textAlign: 'center', padding: '20px' }}>
            <h3>Locate Installation</h3>
            <p>Please select the folder where <strong>{formData.name}</strong> is installed.</p>
            <button 
              onClick={handleFolderSelect}
              style={{
                marginTop: '10px',
                padding: '12px 24px',
                background: '#6366f1',
                color: 'white',
                border: 'none',
                borderRadius: '6px',
                fontSize: '1.1em',
                cursor: 'pointer'
              }}
            >
              Select Installation Folder
            </button>
            <div style={{ marginTop: '20px' }}>
              <button 
                onClick={() => setStep(1)}
                style={{ background: 'transparent', border: 'none', color: '#666', cursor: 'pointer', textDecoration: 'underline' }}
              >
                Back to Search
              </button>
            </div>
          </div>
        )}

        {(mode === 'manual' || step === 3) && (
          <form onSubmit={handleSubmit} className="add-game-form">
            {error && <div className="error-message">{error}</div>}

            <div className="form-group">
              <label htmlFor="name">Game Name *</label>
              <input
                id="name"
                type="text"
                value={formData.name}
                onChange={(e) => handleInputChange("name", e.target.value)}
                required
              />
            </div>

            <div className="form-row">
              <div className="form-group">
                <label htmlFor="platform">Platform *</label>
                <select
                  id="platform"
                  value={formData.platform}
                  onChange={(e) => handleInputChange("platform", e.target.value)}
                >
                  {PLATFORMS.map(platform => (
                    <option key={platform.value} value={platform.value}>
                      {platform.label}
                    </option>
                  ))}
                </select>
              </div>
              <div className="form-group">
                <label htmlFor="platform_app_id">
                  {formData.platform === "steam" ? "Steam App ID *" : "App ID"}
                </label>
                <input
                  id="platform_app_id"
                  type="text"
                  value={formData.platform_app_id}
                  onChange={(e) => handleInputChange("platform_app_id", e.target.value)}
                  placeholder={formData.platform === "steam" ? "123456" : "App ID"}
                  required={formData.platform === "steam"}
                />
              </div>
            </div>

            <div className="form-group">
              <label htmlFor="installation_path">Installation Path</label>
              <input
                id="installation_path"
                type="text"
                value={formData.installation_path}
                onChange={(e) => handleInputChange("installation_path", e.target.value)}
              />
            </div>

            <div className="form-group">
              <label htmlFor="executable_path">Executable Path *</label>
              <div style={{ display: "flex", gap: "8px" }}>
                <input
                  id="executable_path"
                  type="text"
                  value={formData.executable_path}
                  onChange={(e) => handleInputChange("executable_path", e.target.value)}
                  style={{ flex: 1 }}
                />
                <button
                  type="button"
                  onClick={handleBrowseExecutable}
                  className="browse-button"
                  style={{ padding: "8px 16px", background: "#6366f1", color: "white", border: "none", borderRadius: "6px", cursor: "pointer" }}
                >
                  Browse...
                </button>
              </div>
            </div>

            <div className="form-actions">
              {step === 3 && (
                <button type="button" onClick={() => setStep(2)} className="cancel-button" style={{ marginRight: 'auto' }}>
                  Back
                </button>
              )}
              <button type="button" onClick={handleClose} className="cancel-button">
                Cancel
              </button>
              <button type="submit" disabled={isSubmitting} className="submit-button">
                {isSubmitting ? "Saving..." : (editGame ? "Update Game" : "Add Game")}
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}

export default AddGameModal;
