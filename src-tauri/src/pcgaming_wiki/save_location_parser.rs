use std::path::PathBuf;
use platform_dirs::{AppDirs, UserDirs};

pub struct SaveLocationParser;

impl SaveLocationParser {
    pub fn resolve(path_template: &str) -> Option<PathBuf> {
        let mut path_str = path_template.to_string();

        // 1. Resolve common PCGamingWiki templates
        // Windows
        if let Some(user_dirs) = UserDirs::new() {
            let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
            path_str = path_str.replace("{{p|userprofile}}", &home);
            
            let documents = user_dirs.document_dir.to_string_lossy();
            path_str = path_str.replace("{{p|docs}}", &documents);
        }

        if let Some(app_dirs) = AppDirs::new(None, false) {
             // AppData/Roaming
             // Note: platform-dirs config_dir is usually Roaming on Windows
             // But we need to be careful. Let's use std::env for Windows specifics if needed.
        }
        
        // Better approach using std::env for Windows specific vars
        if cfg!(target_os = "windows") {
             if let Ok(val) = std::env::var("APPDATA") {
                 path_str = path_str.replace("{{p|appdata}}", &val);
             }
             if let Ok(val) = std::env::var("LOCALAPPDATA") {
                 path_str = path_str.replace("{{p|localappdata}}", &val);
             }
             if let Ok(val) = std::env::var("ProgramData") {
                 path_str = path_str.replace("{{p|programdata}}", &val);
             }
             if let Ok(val) = std::env::var("USERPROFILE") {
                 path_str = path_str.replace("{{p|userprofile}}", &val);
             }
             // {{p|uid}} is a wildcard for user ID (e.g. Steam ID)
             path_str = path_str.replace("{{p|uid}}", "*");
        }

        // Linux/Unix
        if cfg!(unix) {
             if let Ok(val) = std::env::var("HOME") {
                 path_str = path_str.replace("{{p|home}}", &val);
                 path_str = path_str.replace("{{p|userprofile}}", &val); // Fallback
                 
                 // XDG paths
                 let xdg_data = std::env::var("XDG_DATA_HOME").unwrap_or(format!("{}/.local/share", val));
                 path_str = path_str.replace("{{p|xdgdata}}", &xdg_data);
                 
                 let xdg_config = std::env::var("XDG_CONFIG_HOME").unwrap_or(format!("{}/.config", val));
                 path_str = path_str.replace("{{p|xdgconfig}}", &xdg_config);
             }
        }

        // Clean up path separators
        let path = PathBuf::from(path_str);
        Some(path)
    }
}
