import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface AddGameModalProps {
  isOpen: boolean;
  onClose: () => void;
  onGameAdded: () => void;
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

function AddGameModal({ isOpen, onClose, onGameAdded }: AddGameModalProps) {
  const [formData, setFormData] = useState<GameFormData>({
    name: "",
    platform: "steam",
    platform_app_id: "",
    executable_path: "",
    installation_path: "",
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleInputChange = (field: keyof GameFormData, value: string) => {
    setFormData(prev => ({ ...prev, [field]: value }));
    if (error) setError(null);
  };

  const handleBrowseExecutable = async () => {
    console.log("Browse button clicked!");
    try {
      console.log("Opening file dialog...");
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{
          name: "Executable",
          extensions: ["exe"]
        }]
      });
      
      console.log("Dialog result:", selected);
      
      if (selected && typeof selected === "string") {
        console.log("Selected file:", selected);
        handleInputChange("executable_path", selected);
        // Auto-fill installation path if empty
        if (!formData.installation_path) {
          const installPath = selected.substring(0, selected.lastIndexOf("\\"));
          handleInputChange("installation_path", installPath);
        }
      } else {
        console.log("No file selected or dialog cancelled");
      }
    } catch (err) {
      console.error("Failed to open file dialog:", err);
      alert("Error opening file dialog: " + err);
    }
  };

  const validateForm = (): string | null => {
    if (!formData.name.trim()) return "Game name is required";
    if (!formData.platform) return "Platform is required";
    if (formData.platform === "steam" && !formData.platform_app_id.trim()) {
      return "Steam App ID is required for Steam games";
    }
    return null;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    const validationError = validateForm();
    if (validationError) {
      setError(validationError);
      return;
    }

    setIsSubmitting(true);
    setError(null);

    try {
      const requestData = {
        name: formData.name.trim(),
        platform: formData.platform,
        platform_app_id: formData.platform_app_id.trim() || null,
        executable_path: formData.executable_path.trim() || null,
        installation_path: formData.installation_path.trim() || null,
        icon_base64: null, // Will handle icon extraction later
        icon_path: formData.executable_path.trim() || null,
      };

      await invoke("add_manual_game_sync", { request: requestData });

      onGameAdded();
      handleClose();
    } catch (err) {
      console.error("Failed to add game:", err);
      setError(err instanceof Error ? err.message : "Failed to add game");
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleClose = () => {
    setFormData({
      name: "",
      platform: "steam",
      platform_app_id: "",
      executable_path: "",
      installation_path: "",
    });
    setError(null);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay">
      <div className="modal-content">
        <div className="modal-header">
          <h2>Add New Game</h2>
          <button onClick={handleClose} className="close-button">×</button>
        </div>

        <form onSubmit={handleSubmit} className="add-game-form">
          {error && <div className="error-message">{error}</div>}

          <div className="form-group">
            <label htmlFor="name">Game Name *</label>
            <input
              id="name"
              type="text"
              value={formData.name}
              onChange={(e) => handleInputChange("name", e.target.value)}
              placeholder="Enter game name"
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
              placeholder="C:\Program Files\GameFolder"
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
                placeholder="C:\Program Files\GameFolder\game.exe"
                style={{ flex: 1 }}
              />
              <button
                type="button"
                onClick={handleBrowseExecutable}
                className="browse-button"
                style={{
                  padding: "8px 16px",
                  background: "#6366f1",
                  color: "white",
                  border: "none",
                  borderRadius: "6px",
                  cursor: "pointer",
                  whiteSpace: "nowrap"
                }}
              >
                Browse...
              </button>
            </div>
          </div>

          <div className="form-actions">
            <button type="button" onClick={handleClose} className="cancel-button">
              Cancel
            </button>
            <button type="submit" disabled={isSubmitting} className="submit-button">
              {isSubmitting ? "Adding Game..." : "Add Game"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

export default AddGameModal;
