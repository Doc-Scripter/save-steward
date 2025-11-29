use crate::database::models::*;
use chrono::Utc;
use regex::Regex;
use std::collections::HashMap;
use std::process::Command;

pub struct PcgwIntegration;

impl PcgwIntegration {
    /// Extract PCGW page name from game name (simplified - could be enhanced)
    pub fn extract_pcgw_page_name(game_name: &str) -> Option<String> {
        // Convert to PCGW page name format (spaces to underscores, clean special chars)
        let clean_name = game_name.replace(|c: char| !c.is_alphanumeric() && c != ' ', "_");
        Some(clean_name)
    }

    /// Fetch executables from PCGW wikitext
    pub fn fetch_pcgw_executables(page_name: &str) -> Option<String> {
        // Fetch wikitext using curl
        let output = Command::new("curl")
            .arg("-s")
            .arg(format!("https://www.pcgamingwiki.com/w/api.php?action=parse&page={}&prop=wikitext&format=json", page_name))
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
        let wikitext = json["parse"]["wikitext"]["*"].as_str()?;

        // Parse executables from wikitext
        let mut executables = HashMap::new();

        // Extract {{file|...}} templates
        let file_regex = Regex::new(r"\{\{file\|([^}]+)\}\}").ok()?;
        let mut file_matches: Vec<String> = file_regex.captures_iter(wikitext)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_lowercase()))
            .collect();

        // Filter for likely executable files
        file_matches.retain(|file| {
            file.ends_with(".exe") ||
            file.ends_with(".sh") ||
            file.ends_with(".bin") ||
            file.ends_with(".run") ||
            file.ends_with(".x86_64") ||
            file.ends_with(".app") ||
            !file.contains(".") // Files without extension (common on Linux)
        });

        // Check platform indicators in wikitext
        let has_linux = wikitext.contains("Linux") || wikitext.contains("linux");
        // let has_windows = wikitext.contains("Windows") || wikitext.contains("windows"); // Unused
        // let has_macos = wikitext.contains("OS X") || wikitext.contains("macOS") || wikitext.contains("Mac"); // Unused

        // Assign executables to platforms (simplified logic)
        let mut unassigned_files = Vec::new();
        for file in &file_matches {
            if file.ends_with(".exe") {
                executables.entry("windows".to_string()).or_insert_with(Vec::new).push(file.clone());
            } else if file.ends_with(".app") {
                executables.entry("macos".to_string()).or_insert_with(Vec::new).push(file.clone());
            } else if file.ends_with(".sh") || file.contains("run") || has_linux {
                executables.entry("linux".to_string()).or_insert_with(Vec::new).push(file.clone());
            } else {
                unassigned_files.push(file.clone());
            }
        }

        // Fallback: if we found likely executables but couldn't assign platforms,
        // assign to current platform
        if executables.is_empty() && !file_matches.is_empty() {
            #[cfg(target_os = "linux")]
            executables.insert("linux".to_string(), file_matches);
            #[cfg(target_os = "windows")]
            executables.insert("windows".to_string(), file_matches);
            #[cfg(target_os = "macos")]
            executables.insert("macos".to_string(), file_matches);
        } else if !unassigned_files.is_empty() {
            // Assign unassigned generic executables to current platform
            #[cfg(target_os = "linux")]
            executables.entry("linux".to_string()).or_insert_with(Vec::new).extend(unassigned_files);
            #[cfg(target_os = "windows")]
            executables.entry("windows".to_string()).or_insert_with(Vec::new).extend(unassigned_files);
            #[cfg(target_os = "macos")]
            executables.entry("macos".to_string()).or_insert_with(Vec::new).extend(unassigned_files);
        }

        // Convert to JSON string
        serde_json::to_string(&executables).ok()
    }

    pub fn convert_pcgw_locations(result: &crate::pcgaming_wiki::models::SaveLocationResult) -> Vec<SaveLocation> {
        let mut locations = Vec::new();

        // Windows paths
        for path in &result.windows {
            locations.push(SaveLocation {
                id: 0,
                game_id: 0,
                path_pattern: path.clone(),
                path_type: "directory".to_string(),
                platform: Some("windows".to_string()),
                save_type: "auto".to_string(),
                file_patterns: Some(r#"["*"]"#.to_string()), // Default to all files
                exclude_patterns: None,
                is_relative_to_user: false, // Paths are already resolved
                environment_variable: None,
                priority: 8,
                detection_method: Some("pcgamingwiki".to_string()),
                community_confirmed: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
        }

        // Linux paths
        for path in &result.linux {
            locations.push(SaveLocation {
                id: 0,
                game_id: 0,
                path_pattern: path.clone(),
                path_type: "directory".to_string(),
                platform: Some("linux".to_string()),
                save_type: "auto".to_string(),
                file_patterns: Some(r#"["*"]"#.to_string()),
                exclude_patterns: None,
                is_relative_to_user: false,
                environment_variable: None,
                priority: 8,
                detection_method: Some("pcgamingwiki".to_string()),
                community_confirmed: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
        }

        locations
    }
}
