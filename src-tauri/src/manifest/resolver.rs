use std::collections::HashMap;
use std::path::PathBuf;
use regex::Regex;
use crate::database::DatabaseResult;

/// Main placeholder resolver
#[derive(Clone)]
pub struct PlaceholderResolver {
    placeholders: HashMap<String, String>,
    regex: Regex,
}

impl PlaceholderResolver {
    pub fn new() -> DatabaseResult<Self> {
        let mut resolver = Self {
            placeholders: HashMap::new(),
            regex: Regex::new(r"\{\{\s*([^}]+)\s*\}\}")?,
        };

        resolver.init_placeholders()?;
        Ok(resolver)
    }

    /// Initialize platform-specific placeholders
    fn init_placeholders(&mut self) -> DatabaseResult<()> {
        // Steam placeholders
        self.add_steam_placeholders()?;
        // Standard Windows placeholders
        self.add_windows_placeholders()?;
        // Standard Unix placeholders
        self.add_unix_placeholders()?;
        // Cross-platform placeholders
        self.add_common_placeholders()?;

        Ok(())
    }

    /// Resolve all placeholders in a path template
    pub fn resolve_path(&self, template: &str) -> DatabaseResult<String> {
        let mut result = template.to_string();

        // Find all placeholder matches
        let captures: Vec<_> = self.regex.captures_iter(template).collect();

        for capture in captures {
            let placeholder = &capture[1]; // Extract the content inside {{ }}
            if let Some(resolved) = self.placeholders.get(placeholder.trim()) {
                let placeholder_pattern = format!("{{{{{}}}}}", placeholder);
                result = result.replace(&placeholder_pattern, resolved);
            }
        }

        Ok(Self::normalize_path_separators(&result))
    }

    /// Check if a template contains unresolved placeholders
    pub fn has_unresolved_placeholders(&self, template: &str) -> bool {
        self.regex.is_match(template) && !self.can_resolve_fully(template)
    }

    /// Check if all placeholders in a template can be resolved
    pub fn can_resolve_fully(&self, template: &str) -> bool {
        let captures: Vec<_> = self.regex.captures_iter(template).collect();

        for capture in captures {
            let placeholder = capture[1].trim();
            if !self.placeholders.contains_key(placeholder) {
                return false;
            }
        }

        true
    }

    /// Get list of unresolved placeholders in a template
    pub fn find_unresolved_placeholders(&self, template: &str) -> Vec<String> {
        let mut unresolved = Vec::new();
        let captures: Vec<_> = self.regex.captures_iter(template).collect();

        for capture in captures {
            let placeholder = capture[1].trim();
            if !self.placeholders.contains_key(placeholder) {
                unresolved.push(placeholder.to_string());
            }
        }

        unresolved
    }

    /// Normalize path separators based on current platform
    fn normalize_path_separators(path: &str) -> String {
        if cfg!(windows) {
            path.replace('/', "\\")
        } else {
            path.replace('\\', "/")
        }
    }

    /// Add Steam-related placeholders
    fn add_steam_placeholders(&mut self) -> DatabaseResult<()> {
        // Try to find Steam installation directory
        if let Ok(steam_path) = Self::find_steam_path() {
            // Steam root directory
            self.placeholders.insert("steam".to_string(), steam_path.to_string_lossy().to_string());

            // Common Steam subdirectories
            let steamapps = steam_path.join("steamapps");
            self.placeholders.insert("steamapps".to_string(), steamapps.to_string_lossy().to_string());

            let common = steam_path.join("steamapps").join("common");
            self.placeholders.insert("steam-common".to_string(), common.to_string_lossy().to_string());

            let userdata = steam_path.join("userdata");
            self.placeholders.insert("steam-userdata".to_string(), userdata.to_string_lossy().to_string());
        }

        Ok(())
    }

    /// Add Windows-specific placeholders
    fn add_windows_placeholders(&mut self) -> DatabaseResult<()> {
        if cfg!(target_os = "windows") {
            // Windows environment variables
            self.add_env_placeholder("appdata", "APPDATA")?;
            self.add_env_placeholder("localappdata", "LOCALAPPDATA")?;
            self.add_env_placeholder("userprofile", "USERPROFILE")?;
            self.add_env_placeholder("programdata", "PROGRAMDATA")?;
            self.add_env_placeholder("public", "PUBLIC")?;
            self.add_env_placeholder("programfiles", "ProgramFiles")?;
            self.add_env_placeholder("programfilesx86", "ProgramFiles(x86)")?;
            self.add_env_placeholder("windir", "windir")?;
            self.add_env_placeholder("systemroot", "SystemRoot")?;

            // Windows special folders
            self.add_special_folder("documents", "Personal")?;
            self.add_special_folder("desktop", "Desktop")?;
            self.add_special_folder("music", "My Music")?;
            self.add_special_folder("pictures", "My Pictures")?;
            self.add_special_folder("videos", "My Video")?;
            self.add_special_folder("savedgames", "Saved Games")?;
        }

        Ok(())
    }

    /// Add Unix-specific placeholders
    fn add_unix_placeholders(&mut self) -> DatabaseResult<()> {
        if !cfg!(target_os = "windows") {
            // Unix home directory
            if let Ok(home) = std::env::var("HOME") {
                self.placeholders.insert("home".to_string(), home.clone());

                // Standard XDG directories
                let config_home = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));
                self.placeholders.insert("xdg-config".to_string(), config_home);

                let data_home = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{}/.local/share", home));
                self.placeholders.insert("xdg-data".to_string(), data_home);

                let cache_home = std::env::var("XDG_CACHE_HOME").unwrap_or_else(|_| format!("{}/.cache", home));
                self.placeholders.insert("xdg-cache".to_string(), cache_home);

                // Common Unix directories
                self.placeholders.insert("documents".to_string(), format!("{}/Documents", home));
                self.placeholders.insert("desktop".to_string(), format!("{}/Desktop", home));
                self.placeholders.insert("music".to_string(), format!("{}/Music", home));
                self.placeholders.insert("pictures".to_string(), format!("{}/Pictures", home));
                self.placeholders.insert("videos".to_string(), format!("{}/Videos", home));
            }

            // System directories
            self.placeholders.insert("opt".to_string(), "/opt".to_string());
            self.placeholders.insert("usr".to_string(), "/usr".to_string());
            self.placeholders.insert("var".to_string(), "/var".to_string());
        }

        Ok(())
    }

    /// Add cross-platform common placeholders
    fn add_common_placeholders(&mut self) -> DatabaseResult<()> {
        // Current working directory (relative paths)
        if let Ok(cwd) = std::env::current_dir() {
            self.placeholders.insert("cwd".to_string(), cwd.to_string_lossy().to_string());
        }

        // Temporary directory
        if let Some(temp) = std::env::temp_dir().to_str() {
            self.placeholders.insert("temp".to_string(), temp.to_string());
        }

        Ok(())
    }

    /// Add environment variable placeholder
    fn add_env_placeholder(&mut self, name: &str, env_var: &str) -> DatabaseResult<()> {
        if let Ok(value) = std::env::var(env_var) {
            self.placeholders.insert(name.to_string(), value);
        }
        Ok(())
    }

    /// Add Windows special folder placeholder
    #[cfg(target_os = "windows")]
    fn add_special_folder(&mut self, name: &str, folder_name: &str) -> DatabaseResult<()> {
        use winapi::um::shlobj::SHGetFolderPathW;
        use winapi::um::shfolder::CSIDL_PERSONAL;
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        // This is a simplified version - in production would use proper Windows API
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let path = PathBuf::from(userprofile);
            match folder_name {
                "Personal" => {
                    let documents = path.join("Documents");
                    if documents.exists() {
                        self.placeholders.insert(name.to_string(), documents.to_string_lossy().to_string());
                    }
                }
                "Desktop" => {
                    let desktop = path.join("Desktop");
                    if desktop.exists() {
                        self.placeholders.insert(name.to_string(), desktop.to_string_lossy().to_string());
                    }
                }
                "Saved Games" => {
                    let saved_games = path.join("Saved Games");
                    if saved_games.exists() {
                        self.placeholders.insert(name.to_string(), saved_games.to_string_lossy().to_string());
                    }
                }
                _ => {} // Other folders would need similar handling
            }
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn add_special_folder(&mut self, _name: &str, _folder_name: &str) -> DatabaseResult<()> {
        // Non-Windows platforms don't have the same special folder concept
        Ok(())
    }

    /// Find Steam installation path
    fn find_steam_path() -> DatabaseResult<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            // Windows: Check registry first, then common paths
            if let Ok(steam_path) = windows::find_steam_registry() {
                return Ok(steam_path);
            }

            let common_paths = vec![
                "C:\\Program Files (x86)\\Steam",
                "C:\\Program Files\\Steam",
            ];

            for path_str in common_paths {
                let path = PathBuf::from(path_str);
                if path.exists() {
                    return Ok(path);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let common_paths = vec![
                "~/.steam/steam",
                "~/.local/share/Steam",
                "/usr/share/steam",
            ];

            for path_str in common_paths {
                let expanded = if path_str.starts_with('~') {
                    if let Ok(home) = std::env::var("HOME") {
                        path_str.replacen('~', &home, 1)
                    } else {
                        continue;
                    }
                } else {
                    path_str.to_string()
                };

                let path = PathBuf::from(expanded);
                if path.exists() {
                    return Ok(path);
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            let mac_paths = vec![
                "~/Library/Application Support/Steam",
                "/Applications/Steam.app/Contents/MacOS",
            ];

            for path_str in mac_paths {
                let expanded = if path_str.starts_with('~') {
                    if let Ok(home) = std::env::var("HOME") {
                        path_str.replacen('~', &home, 1)
                    } else {
                        continue;
                    }
                } else {
                    path_str.to_string()
                };

                let path = PathBuf::from(expanded);
                if path.exists() {
                    return Ok(path);
                }
            }
        }

        Err(crate::database::DatabaseError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Steam installation not found"
        )).into())
    }

    /// Get all available placeholders (for debugging)
    pub fn list_placeholders(&self) -> &HashMap<String, String> {
        &self.placeholders
    }
}

/// Windows-specific utilities
#[cfg(target_os = "windows")]
mod windows {
    use std::path::PathBuf;

    pub fn find_steam_registry() -> Result<PathBuf, Box<dyn std::error::Error>> {
        // Windows registry access would go here
        // For now, return error to fall back to path search
        Err("Registry access not implemented".into())
    }
}
