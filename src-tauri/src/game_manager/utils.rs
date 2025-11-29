use crate::database::models::Game;

pub struct Utils;

impl Utils {
    /// Get current system platform string
    pub fn get_current_platform() -> &'static str {
        #[cfg(target_os = "linux")]
        { "linux" }
        #[cfg(target_os = "windows")]
        { "windows" }
        #[cfg(target_os = "macos")]
        { "macos" }
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        { "unknown" }
    }

    /// Get executable path for current platform from stored data
    pub fn get_platform_executable(game: &Game) -> Option<String> {
        let platform = Self::get_current_platform();

        if let Some(executables_json) = &game.platform_executables {
            if let Ok(executables) = serde_json::from_str::<std::collections::HashMap<String, Vec<String>>>(executables_json) {
                if let Some(platform_files) = executables.get(platform) {
                    // Return first executable for this platform
                    platform_files.first().cloned()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}
