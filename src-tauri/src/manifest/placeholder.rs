use std::collections::HashMap;
use regex::Regex;
use crate::database::DatabaseResult;

/// Placeholder resolver for cross-platform path templates
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

    /// Resolve all placeholders in a path template
    pub fn resolve(&self, template: &str) -> DatabaseResult<String> {
        let mut result = template.to_string();

        for capture in self.regex.captures_iter(template) {
            let placeholder = capture[1].trim();
            if let Some(value) = self.placeholders.get(placeholder) {
                let pattern = format!("{{{{{}}}}}", placeholder);
                result = result.replace(&pattern, value);
            }
        }

        Ok(Self::normalize_path_separators(&result))
    }

    /// Check if template can be fully resolved
    pub fn can_resolve(&self, template: &str) -> bool {
        self.find_unresolved(template).is_empty()
    }

    /// Get unresolved placeholders
    pub fn find_unresolved(&self, template: &str) -> Vec<String> {
        let mut unresolved = Vec::new();

        for capture in self.regex.captures_iter(template) {
            let placeholder = capture[1].trim();
            if !self.placeholders.contains_key(placeholder) {
                unresolved.push(placeholder.to_string());
            }
        }

        unresolved
    }

    /// Initialize platform-specific placeholders
    fn init_placeholders(&mut self) -> DatabaseResult<()> {
        self.add_environment_placeholders()?;
        self.add_special_folder_placeholders()?;
        self.add_system_placeholders()?;
        Ok(())
    }

    /// Environment variable placeholders
    fn add_environment_placeholders(&mut self) -> DatabaseResult<()> {
        let env_vars = [
            ("appdata", "APPDATA"),
            ("localappdata", "LOCALAPPDATA"),
            ("userprofile", "USERPROFILE"),
            ("home", "HOME"),
            ("xdg-data", "XDG_DATA_HOME"),
            ("xdg-config", "XDG_CONFIG_HOME"),
        ];

        for (key, env_var) in env_vars {
            if let Ok(value) = std::env::var(env_var) {
                self.placeholders.insert(key.to_string(), value);
            }
        }

        Ok(())
    }

    /// Special folder placeholders
    fn add_special_folder_placeholders(&mut self) -> DatabaseResult<()> {
        if let Ok(home) = std::env::var("HOME") {
            if cfg!(windows) {
                // Windows special folders
                if let Ok(appdata) = std::env::var("APPDATA") {
                    self.placeholders.insert("roaming".to_string(), appdata);
                }
                if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
                    self.placeholders.insert("local".to_string(), local_appdata);
                }
            } else {
                // Unix special folders
                self.placeholders.insert("config".to_string(), format!("{}/.config", home));
                self.placeholders.insert("data".to_string(), format!("{}/.local/share", home));
            }

            // Cross-platform folders
            self.placeholders.insert("documents".to_string(), format!("{}/Documents", home));
            self.placeholders.insert("pictures".to_string(), format!("{}/Pictures", home));
            self.placeholders.insert("music".to_string(), format!("{}/Music", home));
            self.placeholders.insert("videos".to_string(), format!("{}/Videos", home));
        }

        Ok(())
    }

    /// System placeholders
    fn add_system_placeholders(&mut self) -> DatabaseResult<()> {
        // Working directory
        if let Ok(cwd) = std::env::current_dir() {
            if let Some(cwd_str) = cwd.to_str() {
                self.placeholders.insert("cwd".to_string(), cwd_str.to_string());
            }
        }

        // Temp directory
        if let Some(temp) = std::env::temp_dir().to_str() {
            self.placeholders.insert("temp".to_string(), temp.to_string());
        }

        // Platform-specific prefixes
        if cfg!(windows) {
            self.placeholders.insert("programfiles".to_string(), "C:\\Program Files".to_string());
            self.placeholders.insert("windir".to_string(), "C:\\Windows".to_string());
        } else {
            self.placeholders.insert("opt".to_string(), "/opt".to_string());
            self.placeholders.insert("usr".to_string(), "/usr".to_string());
        }

        Ok(())
    }

    /// Normalize path separators for current platform
    fn normalize_path_separators(path: &str) -> String {
        if cfg!(windows) {
            path.replace('/', "\\")
        } else {
            path.replace('\\', "/")
        }
    }

    /// Add custom placeholder
    pub fn add_placeholder(&mut self, key: String, value: String) {
        self.placeholders.insert(key, value);
    }

    /// Get available placeholders (for debugging)
    pub fn list_available(&self) -> Vec<String> {
        self.placeholders.keys().cloned().collect()
    }
}

/// Steam-specific placeholders
pub struct SteamPlaceholderResolver;

impl SteamPlaceholderResolver {
    pub fn add_steam_placeholders(resolver: &mut PlaceholderResolver) -> DatabaseResult<()> {
        if let Ok(steam_path) = Self::find_steam_path() {
            if let Some(steam_str) = steam_path.to_str() {
                resolver.add_placeholder("steam".to_string(), steam_str.to_string());

                let steamapps = steam_path.join("steamapps");
                if let Some(steamapps_str) = steamapps.to_str() {
                    resolver.add_placeholder("steamapps".to_string(), steamapps_str.to_string());
                }

                let userdata = steam_path.join("userdata");
                if let Some(userdata_str) = userdata.to_str() {
                    resolver.add_placeholder("steam-userdata".to_string(), userdata_str.to_string());
                }
            }
        }

        Ok(())
    }

    fn find_steam_path() -> DatabaseResult<std::path::PathBuf> {
        let common_paths = if cfg!(windows) {
            vec![
                r"C:\Program Files (x86)\Steam",
                r"C:\Program Files\Steam",
            ]
        } else if cfg!(target_os = "linux") {
            vec![
                "~/.steam/steam",
                "~/.local/share/Steam",
            ]
        } else {
            vec!["~/Library/Application Support/Steam"]
        };

        for path_str in common_paths {
            let path = if path_str.starts_with('~') {
                if let Ok(home) = std::env::var("HOME") {
                    std::path::PathBuf::from(path_str.replacen('~', &home, 1))
                } else {
                    continue;
                }
            } else {
                std::path::PathBuf::from(path_str)
            };

            if path.exists() {
                return Ok(path);
            }
        }

        Err(crate::database::DatabaseError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Steam installation not found"
        )).into())
    }
}

/// Utility functions for common placeholder operations
pub struct PlaceholderUtils;

impl PlaceholderUtils {
    /// Validate a placeholder template
    pub fn validate_template(resolver: &PlaceholderResolver, template: &str) -> Result<(), Vec<String>> {
        let unresolved = resolver.find_unresolved(template);
        if unresolved.is_empty() {
            Ok(())
        } else {
            Err(unresolved)
        }
    }

    /// Extract all placeholders from a template
    pub fn extract_placeholders(template: &str) -> Vec<String> {
        let regex = Regex::new(r"\{\{\s*([^}]+)\s*\}\}").unwrap();

        regex.captures_iter(template)
            .map(|cap| cap[1].trim().to_string())
            .collect()
    }

    /// Check if template contains placeholders
    pub fn has_placeholders(template: &str) -> bool {
        Regex::new(r"\{\{[^}]+\}\}").unwrap().is_match(template)
    }
}
