use crate::detection::{ProcessInfo, DetectionError};
use std::collections::HashMap;
use regex::Regex;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct RuntimeSignature {
    pub window_title_patterns: Vec<String>,
    pub process_name_patterns: Vec<String>,
    pub required_modules: Vec<String>,        // DLLs/shared libraries loaded
    pub environment_variables: Vec<String>,  // Specific env vars
    pub command_line_patterns: Vec<String>,  // Command line arguments
    pub parent_process_patterns: Vec<String>, // Processes that launched this one
    pub memory_usage_min: Option<u64>,       // Minimum memory usage
    pub memory_usage_max: Option<u64>,       // Maximum memory usage
    pub cpu_usage_min: Option<f32>,          // Minimum CPU usage
    pub cpu_usage_max: Option<f32>,          // Maximum CPU usage
    pub network_ports: Vec<u16>,             // Ports the process might listen on
}

#[derive(Debug, Clone)]
pub struct RuntimeDetector {
    cached_signatures: Arc<RwLock<HashMap<String, Vec<RuntimeSignature>>>>,
    window_title_cache: Arc<RwLock<HashMap<u32, String>>>,
    compiled_patterns: Arc<RwLock<HashMap<String, Regex>>>,
}

impl RuntimeDetector {
    pub fn new() -> Self {
        Self {
            cached_signatures: Arc::new(RwLock::new(HashMap::new())),
            window_title_cache: Arc::new(RwLock::new(HashMap::new())),
            compiled_patterns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn detect_from_process(&self, process_info: &ProcessInfo) -> Result<RuntimeDetectionResult, DetectionError> {
        let window_title = self.get_window_title(process_info.pid).await;
        let current_time = chrono::Utc::now();

        Ok(RuntimeDetectionResult {
            current_window_title: window_title,
            process_name_match_score: self.calculate_process_name_score(&process_info.name).await,
            memory_pattern_match: self.check_memory_pattern(process_info),
            cpu_pattern_match: self.check_cpu_pattern(process_info),
            parent_process_match: self.check_parent_process(&process_info),
            detected_at: current_time,
            confidence_score: 0.0, // Will be calculated by caller
        })
    }

    pub async fn detect_runtime_signatures(&self, executable_path: &str) -> Result<RuntimeSignature, DetectionError> {
        // This would analyze an executable to determine what runtime signatures
        // it might exhibit. In practice, this would require running the executable
        // in a controlled environment or having a database of known signatures.

        // For now, return a basic signature based on the executable path
        let file_stem = std::path::Path::new(executable_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        Ok(RuntimeSignature {
            window_title_patterns: vec![
                format!("(?i){}", regex::escape(file_stem)), // Case insensitive match
                format!("(?i)^{}$", regex::escape(file_stem)), // Exact match at start
            ],
            process_name_patterns: vec![
                format!("(?i)^{}\\.exe$", regex::escape(file_stem)),
                format!("(?i)^{}$", regex::escape(file_stem)),
            ],
            required_modules: vec![], // Would need deeper analysis
            environment_variables: vec![],
            command_line_patterns: vec![],
            parent_process_patterns: vec![
                "(?i)(steam|explorer|epicgameslauncher|gog)".to_string(),
            ],
            memory_usage_min: Some(10_000_000), // 10MB minimum for games
            memory_usage_max: Some(16_000_000_000), // 16GB maximum
            cpu_usage_min: None,
            cpu_usage_max: None,
            network_ports: vec![], // Most games don't listen on ports
        })
    }

    async fn get_window_title(&self, pid: u32) -> Option<String> {
        // Check cache first
        {
            let cache = self.window_title_cache.read().await;
            if let Some(title) = cache.get(&pid) {
                return Some(title.clone());
            }
        }

        // Platform-specific window title detection
        let title = self.get_window_title_platform_specific(pid).await;

        // Cache the result if found
        if let Some(ref title_str) = title {
            let mut cache = self.window_title_cache.write().await;
            cache.insert(pid, title_str.clone());
        }

        title
    }

    async fn get_window_title_platform_specific(&self, _pid: u32) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            // Windows implementation using Windows API
            // This requires additional dependencies and is complex
            // For now, return None
            None
        }

        #[cfg(target_os = "macos")]
        {
            // macOS implementation using Objective-C runtime
            // Would need to access NSApplication or similar
            None
        }

        #[cfg(target_os = "linux")]
        {
            // Linux implementation using X11 or Wayland APIs
            // Complex and environment-dependent
            None
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    pub async fn detect_window_title_patterns(&self, pid: u32) -> Result<Vec<String>, DetectionError> {
        let title = self.get_window_title(pid).await;
        let mut patterns = Vec::new();

        if let Some(title) = title {
            // Generate various patterns from the window title
            patterns.push(title.clone());
            patterns.push(title.to_lowercase());
            patterns.push(Self::normalize_window_title(&title));
            patterns.push(Self::extract_game_name_from_title(&title));
        }

        Ok(patterns)
    }

    fn normalize_window_title(title: &str) -> String {
        // Remove common suffixes and prefixes
        let mut normalized = title
            .replace("(TM)", "")
            .replace("©", "")
            .replace("*", "")
            .replace("-", " ")
            .replace("_", " ");

        // Remove trailing version numbers, resolution info, etc.
        let version_patterns = [
            r"\s+\d+\.\d+(\.\d+)*$", // version numbers at end
            r"\s+DX\d+$", // DirectX versions
            r"\s+\d+x\d+$", // resolutions
            r"\s+Windowed$", // windowed mode
            r"\s+Fullscreen$",
        ];

        for pattern in &version_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                normalized = regex.replace_all(&normalized, "").trim().to_string();
            }
        }

        normalized
    }

    fn extract_game_name_from_title(title: &str) -> String {
        // Extract the most likely game name from window title
        // This is heuristic-based

        // Look for common game title patterns
        if let Some(first_dash) = title.find(" - ") {
            // Pattern: "Game Name - Launcher"
            title[..first_dash].trim().to_string()
        } else if let Some(last_bracket) = title.rfind('(') {
            // Pattern: "Game Name (DLC/Patch info)"
            title[..last_bracket].trim().to_string()
        } else {
            title.trim().to_string()
        }
    }

    async fn calculate_process_name_score(&self, process_name: &str) -> f32 {
        // Score how "game-like" a process name is

        let game_indicators = [
            "game", "unity", "unreal", "frostbite", "cryengine",
            "source", "goldsrc", "idtech", "exe", "bin", "app"
        ];

        let system_process_indicators = [
            "svchost", "explorer", "winlogon", "csrss", "smss",
            "services", "lsass", "wininit", "system", "kernel",
            "init", "systemd", "launchd", "dock", "windowserver"
        ];

        let name_lower = process_name.to_lowercase();
        let mut score: f32 = 0.0; // Base score

        // Game indicators boost score
        for indicator in &game_indicators {
            if name_lower.contains(indicator) {
                score += 20.0;
            }
        }

        // System indicators reduce score
        for indicator in &system_process_indicators {
            if name_lower.contains(indicator) {
                score -= 30.0;
            }
        }

        // File extensions are neutral
        if name_lower.ends_with(".exe") || name_lower.ends_with(".bin") {
            // File extensions don't affect score much
        }

        // Very long process names might indicate bundlers/launchers
        if process_name.len() > 50 {
            score += 10.0; // Bundlers often have long names
        }

        score.max(0.0_f32).min(100.0_f32)
    }

    fn check_memory_pattern(&self, process_info: &ProcessInfo) -> f32 {
        let memory_mb = process_info.memory_usage as f64 / 1024.0 / 1024.0;

        // Game memory usage patterns (heuristic)
        if memory_mb < 10.0 {
            // Too small for a game
            0.0_f32
        } else if memory_mb < 100.0 {
            // Indie/small game range
            60.0_f32
        } else if memory_mb < 1000.0 {
            // Medium/large game range
            80.0_f32
        } else if memory_mb < 4000.0 {
            // AAA game range
            90.0_f32
        } else {
            // Very large or problematic
            30.0_f32
        }
    }

    fn check_cpu_pattern(&self, process_info: &ProcessInfo) -> f32 {
        let cpu = process_info.cpu_usage;

        // CPU usage patterns for games
        if cpu < 1.0 {
            // Very low CPU usage is suspicious
            10.0
        } else if cpu < 10.0 {
            // Normal idle/idle game
            50.0
        } else if cpu < 50.0 {
            // Active game
            80.0
        } else if cpu < 100.0 {
            // Very active game
            90.0
        } else {
            // 100%+ CPU usage (might be loading or bug)
            60.0
        }
    }

    fn check_parent_process(&self, process_info: &ProcessInfo) -> f32 {
        if let Some(parent_pid) = process_info.parent_pid {
            // In most cases, we would need process monitor to get parent process name
            // For now, just check if parent PID looks reasonable

            if parent_pid == 1 || parent_pid == 0 {
                // Direct parent is init/system
                40.0 // Could be launched by any process
            } else {
                // Has a normal parent process
                60.0
            }
        } else {
            // No parent process info
            50.0
        }
    }

    pub async fn load_known_signatures(&self, game_name: &str) -> Result<Vec<RuntimeSignature>, DetectionError> {
        // Load known runtime signatures for a specific game from database
        // This would be populated from community data or manifest information

        let mut signatures = Vec::new();
        let name_lower = game_name.to_lowercase();

        // Common game patterns
        if name_lower.contains("steam") {
            signatures.push(RuntimeSignature {
                window_title_patterns: vec![
                    "(?i)steam".to_string(),
                ],
                process_name_patterns: vec![
                    "(?i)steam|steamwebhelper".to_string(),
                ],
                ..Default::default()
            });
        }

        Ok(signatures)
    }

    pub async fn clear_caches(&self) {
        let mut title_cache = self.window_title_cache.write().await;
        title_cache.clear();

        let mut pattern_cache = self.compiled_patterns.write().await;
        pattern_cache.clear();
    }
}

impl Default for RuntimeSignature {
    fn default() -> Self {
        Self {
            window_title_patterns: Vec::new(),
            process_name_patterns: Vec::new(),
            required_modules: Vec::new(),
            environment_variables: Vec::new(),
            command_line_patterns: Vec::new(),
            parent_process_patterns: Vec::new(),
            memory_usage_min: None,
            memory_usage_max: None,
            cpu_usage_min: None,
            cpu_usage_max: None,
            network_ports: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeDetectionResult {
    pub current_window_title: Option<String>,
    pub process_name_match_score: f32,
    pub memory_pattern_match: f32,
    pub cpu_pattern_match: f32,
    pub parent_process_match: f32,
    pub detected_at: chrono::DateTime<chrono::Utc>,
    pub confidence_score: f32,
}

impl RuntimeDetectionResult {
    pub fn overall_confidence(&self) -> f32 {
        // Weighted combination of different detection methods
        let weights = [
            (self.process_name_match_score, 0.4),
            (self.memory_pattern_match, 0.3),
            (self.cpu_pattern_match, 0.2),
            (self.parent_process_match, 0.1),
        ];

        let weighted_sum: f32 = weights.iter().map(|(score, weight)| score * weight).sum();
        let total_weight: f32 = weights.iter().map(|(_, weight)| weight).sum();

        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_title_normalization() {
        let test_cases = vec![
            ("Game Name 1.0.0", "Game Name"),
            ("Amazing Game - Launcher", "Amazing Game"),
            ("Awesome Game (DLC)", "Awesome Game"),
            ("Simple Game - DX11", "Simple Game"),
            ("Cool Game 1920x1080 Fullscreen", "Cool Game"),
        ];

        for (input, expected) in test_cases {
            let result = RuntimeDetector::normalize_window_title(input);
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_game_name_extraction() {
        let test_cases = vec![
            ("Game Name - Steam", "Game Name"),
            ("Another Game (Beta)", "Another Game"),
            ("Simple Game", "Simple Game"),
            ("Complex Name With Many Words - Launcher v2", "Complex Name With Many Words"),
        ];

        for (input, expected) in test_cases {
            let result = RuntimeDetector::extract_game_name_from_title(input);
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[tokio::test]
    async fn test_process_name_scoring() {
        let detector = RuntimeDetector::new();

        let test_cases = vec![
            ("game.exe", 70.0), // Should have good score
            ("svchost.exe", 20.0), // Should have low score
            ("steam.exe", 90.0), // Should have high score
            ("explorer.exe", 20.0), // Should have low score
        ];

        for (process_name, expected_range) in test_cases {
            let score = detector.calculate_process_name_score(process_name).await;
            // Just check that the ranges are roughly correct
            let expected_min = expected_range - 20.0;
            let expected_max = expected_range + 20.0;
            assert!(score >= expected_min && score <= expected_max,
                   "Score {} for {} not in expected range [{}, {}]",
                   score, process_name, expected_min, expected_max);
        }
    }
}
