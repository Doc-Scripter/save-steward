# PCGamingWiki API Integration Architecture

## Overview

Integration with PCGamingWiki's Cargo API to automatically fetch and identify game save file locations based on the game's platform and metadata.

---

## API Specification

### Base Endpoint
```
https://www.pcgamingwiki.com/w/api.php
```

### Required Parameters
- `action=cargoquery` - Query the Cargo database
- `tables=<CargoTable1,CargoTable2,...>` - Tables to query
- `fields=<comma-separated list>` - Fields to retrieve
- `where=<URL-encoded WHERE clause>` - Filter criteria
- `format=json` - Response format (or `jsonfm` for pretty-printed)

### Relevant Cargo Tables

Based on PCGamingWiki structure:

1. **`Infobox_game`** - Main game information
   - `_pageName` - Game title
   - `Steam_AppID` - Steam App ID
   - `GOGcom_ID` - GOG.com ID
   - `Epic_Games_Store_ID` - Epic Games Store ID
   - `Developers` - Developer names
   - `Publishers` - Publisher names

2. **`Cloud`** - Cloud save support
   - `_pageName` - Game title
   - `Steam_Cloud` - Steam Cloud support (true/false)
   - `GOG_Galaxy` - GOG Galaxy support
   - `Epic_Games_Launcher` - Epic Games Launcher support

3. **`Availability`** - Platform availability
   - `_pageName` - Game title
   - `Steam` - Available on Steam
   - `GOGcom` - Available on GOG
   - `Epic_Games_Store` - Available on Epic

4. **`Save_game_data`** - **PRIMARY TABLE FOR SAVE LOCATIONS**
   - `_pageName` - Game title
   - `Windows` - Windows save location path
   - `Linux` - Linux save location path  
   - `macOS` - macOS save location path
   - `Steam_Play` - Steam Play (Proton) save location
   - `Save_game_cloud_syncing` - Cloud sync support

---

## Architecture Design

### 1. Module Structure

```
src-tauri/src/
├── pcgaming_wiki/
│   ├── mod.rs              # Module entry point
│   ├── client.rs           # HTTP client for API calls
│   ├── models.rs           # Data models for API responses
│   ├── query_builder.rs   # Cargo query builder
│   ├── save_location_parser.rs  # Parse save location paths
│   └── cache.rs            # Cache API responses
```

### 2. Data Models

```rust
// models.rs

/// PCGamingWiki game save data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcgwSaveGameData {
    pub page_name: String,
    pub windows_path: Option<String>,
    pub linux_path: Option<String>,
    pub macos_path: Option<String>,
    pub steam_play_path: Option<String>,
    pub cloud_sync_support: Option<String>,
}

/// PCGamingWiki game info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcgwGameInfo {
    pub page_name: String,
    pub steam_app_id: Option<String>,
    pub gog_id: Option<String>,
    pub epic_id: Option<String>,
    pub developers: Option<Vec<String>>,
    pub publishers: Option<Vec<String>>,
}

/// Combined game data from PCGamingWiki
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcgwGameData {
    pub game_info: PcgwGameInfo,
    pub save_data: Vec<PcgwSaveGameData>,
    pub cloud_support: CloudSupportInfo,
}

/// Cloud save support information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSupportInfo {
    pub steam_cloud: bool,
    pub gog_galaxy: bool,
    pub epic_games: bool,
}
```

### 3. Query Builder

```rust
// query_builder.rs

pub struct CargoQueryBuilder {
    tables: Vec<String>,
    fields: Vec<String>,
    where_clause: Option<String>,
    limit: Option<u32>,
}

impl CargoQueryBuilder {
    pub fn new() -> Self { /* ... */ }
    
    pub fn table(mut self, table: &str) -> Self { /* ... */ }
    
    pub fn field(mut self, field: &str) -> Self { /* ... */ }
    
    pub fn where_clause(mut self, clause: &str) -> Self { /* ... */ }
    
    pub fn limit(mut self, limit: u32) -> Self { /* ... */ }
    
    pub fn build(&self) -> String {
        // Build URL-encoded query string
        // Example: action=cargoquery&tables=Save_game_data&fields=_pageName,Windows,Linux&where=_pageName="Game Name"&format=json
    }
}
```

### 4. API Client

```rust
// client.rs

pub struct PcgwClient {
    base_url: String,
    http_client: reqwest::Client,
    cache: Arc<Mutex<PcgwCache>>,
}

impl PcgwClient {
    pub fn new() -> Self { /* ... */ }
    
    /// Query save game data by game name
    pub async fn get_save_locations_by_name(&self, game_name: &str) -> Result<Vec<PcgwSaveGameData>> {
        let query = CargoQueryBuilder::new()
            .table("Save_game_data")
            .field("_pageName")
            .field("Windows")
            .field("Linux")
            .field("macOS")
            .field("Steam_Play")
            .field("Save_game_cloud_syncing")
            .where_clause(&format!("_pageName=\"{}\"", game_name))
            .build();
        
        self.execute_query(&query).await
    }
    
    /// Query save game data by Steam App ID
    pub async fn get_save_locations_by_steam_id(&self, app_id: &str) -> Result<Vec<PcgwSaveGameData>> {
        // First, get game name from Steam App ID
        let game_info = self.get_game_info_by_steam_id(app_id).await?;
        
        // Then query save locations
        self.get_save_locations_by_name(&game_info.page_name).await
    }
    
    /// Get game info by Steam App ID
    pub async fn get_game_info_by_steam_id(&self, app_id: &str) -> Result<PcgwGameInfo> {
        let query = CargoQueryBuilder::new()
            .table("Infobox_game")
            .field("_pageName")
            .field("Steam_AppID")
            .field("Developers")
            .field("Publishers")
            .where_clause(&format!("Steam_AppID=\"{}\"", app_id))
            .build();
        
        self.execute_query(&query).await
    }
    
    /// Execute a Cargo query
    async fn execute_query<T: DeserializeOwned>(&self, query: &str) -> Result<T> {
        // Check cache first
        if let Some(cached) = self.cache.lock().await.get(query) {
            return Ok(cached);
        }
        
        // Make HTTP request
        let url = format!("{}?{}", self.base_url, query);
        let response = self.http_client.get(&url).send().await?;
        let data: T = response.json().await?;
        
        // Cache the result
        self.cache.lock().await.set(query, &data);
        
        Ok(data)
    }
}
```

### 5. Save Location Parser

```rust
// save_location_parser.rs

pub struct SaveLocationParser;

impl SaveLocationParser {
    /// Parse PCGamingWiki path patterns to actual paths
    /// Handles variables like:
    /// - {{p|steam}} -> Steam installation path
    /// - {{p|localappdata}} -> %LOCALAPPDATA%
    /// - {{p|userprofile}} -> %USERPROFILE%
    /// - {{p|appdata}} -> %APPDATA%
    pub fn parse_path_pattern(pattern: &str, platform: &str) -> Vec<String> {
        let mut paths = Vec::new();
        
        // Replace PCGamingWiki template variables
        let resolved = pattern
            .replace("{{p|steam}}", Self::get_steam_path())
            .replace("{{p|localappdata}}", "%LOCALAPPDATA%")
            .replace("{{p|userprofile}}", "%USERPROFILE%")
            .replace("{{p|appdata}}", "%APPDATA%")
            .replace("{{p|documents}}", "%USERPROFILE%/Documents");
        
        paths.push(resolved);
        paths
    }
    
    /// Extract file patterns from save location
    pub fn extract_file_patterns(location: &str) -> Vec<String> {
        // Extract file extensions and patterns
        // Example: "*.sav", "save*.dat"
    }
    
    fn get_steam_path() -> &'static str {
        // Platform-specific Steam path detection
        #[cfg(target_os = "windows")]
        return "C:/Program Files (x86)/Steam";
        
        #[cfg(target_os = "linux")]
        return "~/.steam/steam";
        
        #[cfg(target_os = "macos")]
        return "~/Library/Application Support/Steam";
    }
}
```

### 6. Cache Implementation

```rust
// cache.rs

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct PcgwCache {
    entries: HashMap<String, CacheEntry>,
    ttl: Duration,
}

struct CacheEntry {
    data: serde_json::Value,
    inserted_at: Instant,
}

impl PcgwCache {
    pub fn new(ttl_hours: u64) -> Self {
        Self {
            entries: HashMap::new(),
            ttl: Duration::from_secs(ttl_hours * 3600),
        }
    }
    
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        if let Some(entry) = self.entries.get(key) {
            if entry.inserted_at.elapsed() < self.ttl {
                return Some(entry.data.clone());
            }
        }
        None
    }
    
    pub fn set(&mut self, key: &str, value: &serde_json::Value) {
        self.entries.insert(key.to_string(), CacheEntry {
            data: value.clone(),
            inserted_at: Instant::now(),
        });
    }
    
    pub fn clear_expired(&mut self) {
        self.entries.retain(|_, entry| entry.inserted_at.elapsed() < self.ttl);
    }
}
```

---

## Database Schema Updates

Add tables to store PCGamingWiki data:

```sql
-- PCGamingWiki cache table
CREATE TABLE IF NOT EXISTS pcgw_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_name TEXT NOT NULL,
    steam_app_id TEXT,
    gog_id TEXT,
    epic_id TEXT,
    save_data_json TEXT NOT NULL,  -- JSON blob of save locations
    cloud_support_json TEXT,        -- JSON blob of cloud support
    fetched_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,
    UNIQUE(game_name)
);

-- PCGamingWiki to game mapping
CREATE TABLE IF NOT EXISTS game_pcgw_mapping (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    pcgw_page_name TEXT NOT NULL,
    confidence_score REAL DEFAULT 1.0,  -- Match confidence (0.0-1.0)
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id),
    UNIQUE(game_id)
);
```

---

## Integration Flow

### User Workflow: Assisted Game Addition

This workflow balances automation with user control, as requested:

1.  **Search & Select**
    *   User types game name (e.g., "Witcher 3").
    *   App queries PCGamingWiki `Infobox_game` table.
    *   User selects the correct game from the list.

2.  **Locate Installation**
    *   User selects the game's **Installation Directory** on their disk.
    *   *Why?* This solves the hardest problem of finding where the game is installed, which varies wildly by store (Steam, Epic, GOG, manual).

3.  **Auto-Configuration (The "Magic" Step)**
    *   **Executable Detection**: App scans the selected folder for executables (`.exe`, binaries).
        *   *Heuristic*: Match filename to game name (e.g., `witcher3.exe`).
        *  Or use wiki api to identif the filename of the game file
    *   **Save Location Detection**: App queries PCGamingWiki `Save_game_data` table.
        *   App resolves path templates (e.g., `{{p|userprofile}}` -> `C:\Users\Name`).
        *   App checks if the resolved path exists.

4.  **Confirmation**
    *   App presents the "Add Game" modal pre-filled with:
        *   Name (from API)
        *   Icon (from API/Exe)
        *   Executable Path (Found in folder)
        *   Save Path (Resolved from API)
    *   User clicks "Save".

### Example API Queries

**Query 1: Search games by name**
```
https://www.pcgamingwiki.com/w/api.php?action=cargoquery&tables=Infobox_game&fields=_pageName,Steam_AppID,Developers,Publishers&where=_pageName LIKE "%Witcher%"&limit=10&format=json
```

**Query 2: Get save locations (once game is selected)**
```
https://www.pcgamingwiki.com/w/api.php?action=cargoquery&tables=Save_game_data&fields=_pageName,Windows,Linux,macOS,Steam_Play&where=_pageName="The Witcher 3: Wild Hunt"&format=json
```

---

## Tauri Commands

```rust
// lib.rs

#[tauri::command]
async fn fetch_save_locations_from_pcgw(
    game_name: String,
    steam_app_id: Option<String>
) -> Result<serde_json::Value, String> {
    let client = PcgwClient::new();
    
    let save_data = if let Some(app_id) = steam_app_id {
        client.get_save_locations_by_steam_id(&app_id).await
    } else {
        client.get_save_locations_by_name(&game_name).await
    }?;
    
    Ok(serde_json::to_value(save_data)?)
}

#[tauri::command]
async fn auto_detect_save_locations(game_id: i64) -> Result<Vec<SaveLocation>, String> {
    // 1. Get game from database
    // 2. Query PCGamingWiki
    // 3. Parse and store save locations
    // 4. Return detected locations
}
```

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum PcgwError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("Game not found in PCGamingWiki: {0}")]
    GameNotFound(String),
    
    #[error("Invalid API response: {0}")]
    InvalidResponse(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Cache error: {0}")]
    CacheError(String),
}
```

---

## Configuration

```toml
# config.toml

[pcgaming_wiki]
base_url = "https://www.pcgamingwiki.com/w/api.php"
cache_ttl_hours = 168  # 1 week
max_retries = 3
timeout_seconds = 30
user_agent = "SaveSteward/1.0 (https://github.com/yourusername/save-steward)"
```

---

## Testing Strategy

1. **Unit Tests**
   - Query builder URL encoding
   - Path pattern parsing
   - Cache expiration logic

2. **Integration Tests**
   - API response parsing
   - Database storage
   - End-to-end save location detection

3. **Mock Data**
   - Sample PCGamingWiki responses
   - Edge cases (missing data, malformed paths)

---

## Future Enhancements

1. **Fuzzy Matching** - Use Levenshtein distance for game name matching
2. **Batch Queries** - Fetch multiple games in one request
3. **Community Contributions** - Allow users to submit corrections
4. **Offline Mode** - Fallback to cached data when API unavailable
5. **Auto-Update** - Periodic refresh of save location data

---

## Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
url = "2.5"
```

---

## Summary

This architecture provides:
- ✅ Robust API client for PCGamingWiki
- ✅ Intelligent caching to minimize API calls
- ✅ Path pattern parsing for cross-platform support
- ✅ Database integration for persistent storage
- ✅ Error handling and retry logic
- ✅ Extensible design for future enhancements

Next steps: Review architecture → Implement modules → Test with real games
