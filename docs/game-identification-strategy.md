# Game Identification Strategy for Save Steward

## Xbox 360 Game Identification Reference

Xbox 360 used a sophisticated multi-layered identification system that we can reference for our PC game identification strategy:

### Xbox 360 Identification Methods

1. **Title ID System**
   - 8-digit hexadecimal identifier (e.g., "4D5307D3")
   - Unique per game across all regions
   - Format: `[Publisher Code][Game Number][Version/Region]`
   - Example: "MS00402A" = Microsoft, Game 004, Version 02, Region A (America)

2. **Media ID**
   - Physical disc identification
   - Used for anti-piracy and region locking
   - Mastering code etched into inner ring of DVDs

3. **Executable Hash Verification**
   - XBE (Xbox Executable) file validation
   - Digital signature verification
   - CRC32 checksums for integrity

### Lessons for PC Game Identification

Based on Xbox 360's approach, our PC identification system should use:

1. **Multi-factor identification** (not relying on single method)
2. **Publisher + Game number system** (similar to Steam AppID)
3. **Hash-based verification** for executables
4. **Region/platform awareness** for different versions

## PC Game Identification Implementation

### Primary Identification Stack

#### 1. Platform-Specific IDs (Highest Confidence)
```json
{
  "steam": {
    "app_id": "570840",           // Steam AppID
    "depot_id": "570841",         // Steam Depot ID
    "manifest_id": "1234567890"   // Latest manifest
  },
  "epic": {
    "catalog_item_id": "catnip",   // Epic Games catalog ID
    "namespace": "fnitem",          // Epic namespace
    "item_id": "5c86a7d64d430e"   // Epic item ID
  },
  "gog": {
    "product_id": "1207658924",     // GOG product ID
    "build_id": "557955955118"     // GOG build ID
  }
}
```

#### 2. Executable Analysis (High Confidence)
```rust
pub struct ExecutableSignature {
    pub file_hash: String,           // SHA-256 of executable
    pub file_size: u64,
    pub digital_signature: Option<String>,
    pub version_info: VersionInfo,
    pub publisher: String,
    pub product_name: String,
    pub original_filename: String,
}

pub struct VersionInfo {
    pub file_version: String,
    pub product_version: String,
    pub company_name: String,
    pub file_description: String,
}
```

#### 3. Runtime Detection (Medium Confidence)
```rust
pub struct RuntimeSignature {
    pub process_name: String,
    pub window_title_patterns: Vec<String>,
    pub executable_path_pattern: String,
    pub loaded_modules: Vec<String>,
    pub registry_keys: Vec<String>,
}
```

#### 4. Installation Metadata (Low-Medium Confidence)
```rust
pub struct InstallationSignature {
    pub install_path: String,
    pub uninstall_string: String,
    pub install_date: String,
    pub publisher_registry: String,
    pub display_name: String,
    pub estimated_size: u64,
}
```

### Game Identification Confidence Scoring

```rust
pub enum IdentificationConfidence {
    Definitive = 95,     // Platform API match + executable hash
    High = 80,          // Multiple strong indicators
    Medium = 60,        // Single strong + multiple weak
    Low = 30,           // Weak indicators only
    Uncertain = 10,     // Manual user confirmation needed
}

pub struct GameIdentification {
    pub game_id: u64,
    pub confidence_score: IdentificationConfidence,
    pub identification_methods: Vec<String>,
    pub conflicting_games: Vec<u64>,
    pub requires_manual_confirmation: bool,
}
```

### Implementation Strategy

#### Phase 1: Basic Identification (MVP)
```rust
impl GameIdentifier {
    pub fn identify_game(process_path: &str) -> Result<GameIdentification, IdentificationError> {
        let mut evidence = IdentificationEvidence::new();
        
        // 1. Check executable hash against database
        evidence.add_executable_analysis(&process_path)?;
        
        // 2. Check for platform-specific IDs
        evidence.add_platform_ids(&process_path)?;
        
        // 3. Check window title patterns
        evidence.add_runtime_detection()?;
        
        // 4. Calculate confidence score
        evidence.calculate_confidence()
    }
}
```

#### Phase 2: Advanced Detection
```rust
// Machine learning-based identification
pub struct MLGameIdentifier {
    model: GameIdentificationModel,
    feature_extractor: FeatureExtractor,
}

impl MLGameIdentifier {
    pub fn identify_with_ml(&self, game_features: GameFeatures) -> GameIdentification {
        // Use trained model for complex identification cases
        let prediction = self.model.predict(game_features);
        self.post_process_prediction(prediction)
    }
}
```

### Database Schema for Game Identification

```sql
-- Game identification evidence table
CREATE TABLE game_identification_evidence (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    evidence_type TEXT NOT NULL, -- 'executable_hash', 'steam_appid', 'window_title'
    evidence_value TEXT NOT NULL,
    confidence_weight REAL NOT NULL,
    detection_context TEXT, -- 'installation', 'runtime', 'manual'
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id)
);

-- Executable signatures table
CREATE TABLE executable_signatures (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    file_hash_sha256 TEXT NOT NULL UNIQUE,
    file_size INTEGER,
    digital_signature TEXT,
    version_string TEXT,
    company_name TEXT,
    product_name TEXT,
    original_filename TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id)
);

-- Platform-specific game IDs
CREATE TABLE platform_game_ids (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    platform TEXT NOT NULL, -- 'steam', 'epic', 'gog', 'origin'
    platform_id TEXT NOT NULL,
    platform_metadata TEXT, -- JSON blob for platform-specific data
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id),
    UNIQUE(platform, platform_id)
);
```

### Conflict Resolution

When multiple games match identification criteria:

```rust
pub struct IdentificationConflict {
    pub primary_candidate: GameIdentification,
    pub alternatives: Vec<GameIdentification>,
    pub conflicting_evidence: Vec<EvidenceConflict>,
    pub resolution_strategy: ConflictResolutionStrategy,
}

pub enum ConflictResolutionStrategy {
    HighestConfidence,
    UserChoice,
    PlatformPriority, // Steam > Epic > GOG > Others
    MostRecent,
}
```

### User Override System

```rust
pub struct UserGameOverride {
    pub user_id: u64,
    pub game_id: u64,
    pub override_type: OverrideType,
    pub override_value: String,
    pub confidence_boost: i32,
}

pub enum OverrideType {
    CustomExecutableHash,
    CustomInstallPath,
    CustomWindowTitle,
    ForceIdentification,
}
```

### Performance Optimization

1. **Caching Strategy**
   ```rust
   pub struct GameIdentificationCache {
       executable_cache: HashMap<String, GameIdentification>,
       window_title_cache: HashMap<String, Vec<GameIdentification>>,
       platform_id_cache: HashMap<(String, String), GameIdentification>,
       cache_ttl: Duration,
   }
   ```

2. **Batch Processing**
   - Identify multiple running games simultaneously
   - Background identification for installed games
   - Incremental updates to identification database

3. **Database Indexing**
   ```sql
   CREATE INDEX idx_executable_hash ON executable_signatures(file_hash_sha256);
   CREATE INDEX idx_platform_game_id ON platform_game_ids(platform, platform_id);
   CREATE INDEX idx_game_evidence ON game_identification_evidence(evidence_type, evidence_value);
   ```

### Testing and Validation

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_steam_game_identification() {
        let steam_game = test_data::get_steam_game();
        let identification = GameIdentifier::identify_game(&steam_game.executable_path);
        
        assert_eq!(identification.confidence_score, IdentificationConfidence::Definitive);
        assert!(identification.identification_methods.contains(&"steam_appid".to_string()));
    }
    
    #[test]
    fn test_conflict_resolution() {
        let conflict = create_identification_conflict();
        let resolved = conflict.resolve(ConflictResolutionStrategy::PlatformPriority);
        
        assert!(resolved.confidence_score > 80);
    }
}
```

### Future Enhancements

1. **Machine Learning Integration**
   - Train models on user identification patterns
   - Community-driven identification improvement
   - Anomaly detection for misidentified games

2. **Community Database**
   - User-contributed identification patterns
   - Crowdsourced save location data
   - Community validation of identifications

3. **Advanced Heuristics**
   - Game engine detection (Unity, Unreal, etc.)
   - Save file format analysis
   - Multi-language game title matching

This strategy provides robust game identification while learning from Xbox 360's proven multi-factor approach, adapted for the complexity of PC gaming platforms.