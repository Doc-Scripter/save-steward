import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AddGameModalProps {
  isOpen: boolean;
  onClose: () => void;
  onGameAdded: () => void;
}

interface GameFormData {
  name: string;
  developer: string;
  publisher: string;
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
];

function AddGameModal({ isOpen, onClose, onGameAdded }: AddGameModalProps) {
  const [formData, setFormData] = useState<GameFormData>({
    name: "",
    developer: "",
    publisher: "",
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
        developer: formData.developer.trim() || null,
        publisher: formData.publisher.trim() || null,
        platform: formData.platform,
        platform_app_id: formData.platform_app_id.trim() || null,
        executable_path: formData.executable_path.trim() || null,
        installation_path: formData.installation_path.trim() || null,
      };

      await invoke("add_manual_game", { request: requestData });

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
      developer: "",
      publisher: "",
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
              <label htmlFor="developer">Developer</label>
              <input
                id="developer"
                type="text"
                value={formData.developer}
                onChange={(e) => handleInputChange("developer", e.target.value)}
                placeholder="Game developer"
              />
            </div>
            <div className="form-group">
              <label htmlFor="publisher">Publisher</label>
              <input
                id="publisher"
                type="text"
                value={formData.publisher}
                onChange={(e) => handleInputChange("publisher", e.target.value)}
                placeholder="Game publisher"
              />
            </div>
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
            <label htmlFor="executable_path">Executable Path</label>
            <input
              id="executable_path"
              type="text"
              value={formData.executable_path}
              onChange={(e) => handleInputChange("executable_path", e.target.value)}
              placeholder="C:\Program Files\GameFolder\game.exe"
            />
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
