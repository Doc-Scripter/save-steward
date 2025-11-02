# Save Steward Database Schema Design

## Overview
This document outlines the database schema for storing game save locations, game identification data, and metadata for the Save Steward application.

## Database Technology Choice: SQLite (Local) → PostgreSQL (Cloud)

### Local Database: SQLite
- **Lightweight**: Perfect for desktop application
- **Zero configuration**: No server setup required
- **Single file**: Easy backup and migration
- **Encryption ready**: Supports SQLCipher for encryption
- **Cross-platform**: Works on Windows, macOS, Linux

### Cloud Database: PostgreSQL via Supabase
- **Scalable**: Handles growth as user base expands
- **Rich features**: Advanced querying and indexing
- **Supabase integration**: Built-in auth and real-time features
- **Migration path**: Easy upgrade from SQLite

## Core Tables

### 1. games Table
```sql
CREATE TABLE games (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    developer TEXT,
    publisher TEXT,
    platform TEXT NOT NULL, -- 'steam', 'epic', 'gog', 'standalone'
    platform_app_id TEXT,   -- Steam AppID, Epic Game ID, etc.
    executable_path TEXT,
    installation_path TEXT,
    genre TEXT,
    release_date DATE,
    cover_image_url TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE
);
```

### 2. save_locations Table
```sql
CREATE TABLE save_locations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    path_pattern TEXT NOT NULL,           -- Save directory path with variables
    path_type TEXT NOT NULL,               -- 'directory', 'file', 'registry'
    platform TEXT,                         -- 'windows', 'macos', 'linux'
    save_type TEXT DEFAULT 'auto',         -- 'auto', 'manual', 'cloud'
    file_patterns TEXT,                    -- JSON array of file patterns to monitor
    exclude_patterns TEXT,                 -- JSON array of patterns to exclude
    is_relative_to_user BOOLEAN DEFAULT TRUE,
    environment_variable TEXT,            -- %APPDATA%, %LOCALAPPDATA%, etc.
    priority INTEGER DEFAULT 1,             -- Detection priority (1-10)
    detection_method TEXT,                -- 'heuristic', 'api', 'manual', 'community'
    community_confirmed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id)
);
```

### 3. detected_saves Table
```sql
CREATE TABLE detected_saves (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    save_location_id INTEGER NOT NULL,
    actual_path TEXT NOT NULL,
    current_hash TEXT,
    file_size INTEGER,
    last_modified TIMESTAMP,
    first_detected TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_checked TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    metadata_json TEXT,                    -- JSON blob for game-specific metadata
    FOREIGN KEY (game_id) REFERENCES games(id),
    FOREIGN KEY (save_location_id) REFERENCES save_locations(id)
);
```

### 4. save_versions Table
```sql
CREATE TABLE save_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    detected_save_id INTEGER NOT NULL,
    version_number INTEGER NOT NULL,
    backup_path TEXT NOT NULL,
    compressed_size INTEGER,
    original_hash TEXT NOT NULL,
    compressed_hash TEXT NOT NULL,
    compression_method TEXT DEFAULT 'zstd',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    backup_reason TEXT,                    -- 'auto', 'manual', 'pre_restore'
    metadata_json TEXT,
    FOREIGN KEY (detected_save_id) REFERENCES detected_saves(id)
);
```

### 5. game_identifiers Table
```sql
CREATE TABLE game_identifiers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    identifier_type TEXT NOT NULL,        -- 'executable_hash', 'window_title', 'process_name'
    identifier_value TEXT NOT NULL,
    confidence_score REAL DEFAULT 1.0,   -- 0.0 to 1.0
    detection_context TEXT,                -- 'runtime', 'installation', 'manual'
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id),
    UNIQUE(identifier_type, identifier_value)
);
```

### 6. user_games Table (for user-specific overrides)
```sql
CREATE TABLE user_games (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    custom_name TEXT,
    custom_install_path TEXT,
    custom_save_path TEXT,
    is_favorite BOOLEAN DEFAULT FALSE,
    backup_enabled BOOLEAN DEFAULT TRUE,
    auto_backup_interval INTEGER DEFAULT 3600, -- seconds
    max_versions INTEGER DEFAULT 10,
    compression_level INTEGER DEFAULT 3,      -- 1-22 for zstd
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id)
);
```

## Game Identification Strategy

### Primary Identification Methods

1. **Platform API Integration**
   - Steam: AppID from Steam API
   - Epic Games: Catalog item ID
   - GOG: Product ID

2. **Executable Analysis**
   - File hash (SHA-256)
   - Digital signature
   - Version information
   - File metadata

3. **Runtime Detection**
   - Process name matching
   - Window title patterns
   - Executable path patterns

4. **Xbox 360 Style Reference**
   - Title ID system (similar to Steam AppID)
   - Media ID for disc-based games
   - Executable hash verification

## Save Location Detection Patterns

### Common Windows Patterns
```
%USERPROFILE%\Documents\My Games\[GameName]\
%USERPROFILE%\Documents\[GameName]\
%APPDATA%\[Developer]\[GameName]\
%LOCALAPPDATA%\[Developer]\[GameName]\
%LOCALAPPDATA%Low\[Developer]\[GameName]\
%USERPROFILE%\Saved Games\[GameName]\
```

### Platform-Specific Patterns
```
# Steam
%STEAM%\userdata\[SteamID]\[AppID]\

# Epic Games
%LOCALAPPDATA%\EpicGamesLauncher\SavedSaves\[AccountID]\

# GOG
%GOG_GAMES%\[GameName]\
```

## Hash-Based Change Detection

### Implementation Strategy
1. **File System Monitoring**
   - Watch for file modifications in save directories
   - Use file system events (inotify on Linux, ReadDirectoryChangesW on Windows)

2. **Hash Calculation**
   - SHA-256 for file integrity
   - MD5 for quick comparison (performance optimization)
   - Per-file hashing for granular change detection

3. **Change Detection Logic**
   - Compare current hash with last known hash
   - Timestamp-based fallback for hash failures
   - File size and modification time as secondary indicators

## Database Encryption Strategy

### Local Encryption (SQLite)
- **SQLCipher**: Transparent 256-bit AES encryption
- **Key derivation**: PBKDF2 with user-provided password
- **Encryption scope**: Entire database file
- **Performance impact**: ~5-10% overhead

### Implementation Example
```rust
// Using rusqlite with sqlcipher
let conn = Connection::open_with_flags_and_vfs(
    "save_steward.db",
    OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    "sqlcipher"
)?;

conn.execute("PRAGMA key = 'user_provided_password';", [])?;
conn.execute("PRAGMA cipher_page_size = 4096;", [])?;
conn.execute("PRAGMA kdf_iter = 64000;", [])?;
```

## Migration Path to Supabase/PostgreSQL

### Schema Compatibility
- Maintain identical table structures
- Use PostgreSQL-compatible data types
- Implement data migration scripts

### Supabase Integration Benefits
- **Row Level Security**: Fine-grained access control
- **Real-time subscriptions**: Live save sync across devices
- **Edge functions**: Server-side save processing
- **Authentication**: Built-in user management

## Performance Optimization

### Indexing Strategy
```sql
-- For fast game lookups
CREATE INDEX idx_games_platform_app_id ON games(platform, platform_app_id);
CREATE INDEX idx_games_name ON games(name);

-- For save location queries
CREATE INDEX idx_save_locations_game_id ON save_locations(game_id);
CREATE INDEX idx_save_locations_platform ON save_locations(platform);

-- For version history
CREATE INDEX idx_save_versions_detected_save_id ON save_versions(detected_save_id);
CREATE INDEX idx_save_versions_created_at ON save_versions(created_at);
```

### Caching Strategy
- In-memory cache for frequently accessed games
- File system watcher cache for active save directories
- Hash cache with TTL for performance

## Security Considerations

1. **Data Encryption**: All sensitive data encrypted at rest
2. **Access Control**: User-based permissions for save access
3. **Audit Logging**: Track all save operations
4. **Secure Deletion**: Proper cleanup of deleted saves
5. **Backup Encryption**: Encrypted backups for cloud sync

## Next Steps

1. Implement SQLite schema with basic encryption
2. Create game identification service
3. Build save location detection engine
4. Implement hash-based change detection
5. Create Supabase migration scripts