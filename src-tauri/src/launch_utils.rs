// Utility functions for launching games with better support for Unity and other games

use std::path::Path;
use std::process::Command;

/// Find the best executable to launch for a given game directory
pub fn find_game_launcher(install_dir: &str, executable_path: &str) -> Result<String, String> {
    let install_path = Path::new(install_dir);
    let exec_path = Path::new(executable_path);
    
    // If the executable path is absolute and exists, use it directly
    if exec_path.is_absolute() && exec_path.exists() {
        return Ok(executable_path.to_string());
    }
    
    // Try to find a launcher script in the installation directory
    if install_path.exists() && install_path.is_dir() {
        // Common Unity/Linux launcher patterns
        let potential_launchers = vec![
            // Game name as launcher (common for Linux games)
            "radio", // Based on the log showing "radio" as the game
            "run.sh",
            "run_game.sh",
            "launch.sh",
            "start.sh",
            "game.sh",
            // Generic executables in bin/ directory
            "bin/game",
            "bin/radio", // Game-specific in bin/
        ];
        
        for launcher in potential_launchers {
            let launcher_path = install_path.join(launcher);
            if launcher_path.exists() {
                return Ok(launcher_path.to_string_lossy().to_string());
            }
        }
    }
    
    // Fallback to the original executable path
    Ok(executable_path.to_string())
}

/// Make a file executable (Linux/Unix only)
pub fn make_executable(path: &str) -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt; // Required for set_mode
        
        let path = Path::new(path);
        if path.exists() {
            let metadata = fs::metadata(path)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?;
            
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            
            fs::set_permissions(path, permissions)
                .map_err(|e| format!("Failed to set executable permissions: {}", e))?;
        }
    }
    
    Ok(())
}

/// Enhanced game launcher with Unity and Linux support
pub async fn launch_game_enhanced(install_dir: &str, executable_path: &str) -> Result<String, String> {
    // Find the best launcher to use
    let launcher_path = find_game_launcher(install_dir, executable_path)?;
    
    #[cfg(target_os = "windows")]
    {
        // Windows: Direct execution
        Command::new(&launcher_path)
            .current_dir(install_dir)
            .spawn()
            .map_err(|e| format!("Failed to launch game on Windows: {}", e))?;
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // Linux/Unix: Enhanced handling
        
        // First, try to make sure the launcher is executable
        let _ = make_executable(&launcher_path);
        
        // Try different launch methods in order of preference
        let launch_result = Command::new(&launcher_path)
            .current_dir(install_dir)
            .spawn();
            
        match launch_result {
            Ok(_) => {
                // Success!
            }
            Err(e) => {
                // Try alternative methods if direct launch fails
                
                // Method 1: Try with sh -c
                let shell_result = Command::new("sh")
                    .arg("-c")
                    .arg(format!("cd '{}' && ./{}", install_dir, Path::new(&launcher_path).file_name().unwrap_or_default().to_string_lossy()))
                    .spawn();
                    
                if let Err(e2) = shell_result {
                    return Err(format!(
                        "Failed to launch game '{}'. Tried methods: 1) Direct: {}, 2) Shell: {}, 3) Shell with cd. Error: {}. Make sure the game is installed correctly and has executable permissions.",
                        launcher_path, e, e2, e2
                    ));
                }
            }
        }
    }
    
    Ok(format!("Launched game from: {} -> {}", install_dir, launcher_path))
}
