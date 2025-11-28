use rusqlite::Connection;
use std::path::Path;
use crate::database::connection::DatabaseResult;

pub const DATABASE_VERSION: u32 = 1;

pub struct DatabaseSchema;

impl DatabaseSchema {
    pub fn create_tables(conn: &Connection) -> DatabaseResult<()> {
        // Create games table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS games (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                developer TEXT,
                publisher TEXT,
                platform TEXT NOT NULL, -- 'steam', 'epic', 'gog', 'standalone', 'other'
                platform_app_id TEXT,   -- Steam AppID, Epic Game ID, etc.
                executable_path TEXT,
                installation_path TEXT,
                genre TEXT,
                release_date DATE,
                cover_image_url TEXT,
                icon_base64 TEXT,       -- Base64 encoded game icon
                icon_path TEXT,         -- Path to exe for icon extraction/update
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                is_active BOOLEAN DEFAULT TRUE
            )
            "#,
            [],
        )?;

        // Create save_locations table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS save_locations (
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
            )
            "#,
            [],
        )?;

        // Create detected_saves table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS detected_saves (
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
            )
            "#,
            [],
        )?;

        // Create save_versions table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS save_versions (
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
            )
            "#,
            [],
        )?;

        // Create game_identifiers table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS game_identifiers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL,
                identifier_type TEXT NOT NULL,        -- 'executable_hash', 'window_title', 'process_name'
                identifier_value TEXT NOT NULL,
                confidence_score REAL DEFAULT 1.0,   -- 0.0 to 1.0
                detection_context TEXT,                -- 'runtime', 'installation', 'manual'
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (game_id) REFERENCES games(id),
                UNIQUE(identifier_type, identifier_value)
            )
            "#,
            [],
        )?;

        // Create user_games table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS user_games (
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
            )
            "#,
            [],
        )?;

        // Create indexes for performance
        Self::create_indexes(conn)?;

        Ok(())
    }

    fn create_indexes(conn: &Connection) -> DatabaseResult<()> {
        // Game lookup indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_games_platform_app_id ON games(platform, platform_app_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_games_name ON games(name)",
            [],
        )?;

        // Save location indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_save_locations_game_id ON save_locations(game_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_save_locations_platform ON save_locations(platform)",
            [],
        )?;

        // Version history indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_save_versions_detected_save_id ON save_versions(detected_save_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_save_versions_created_at ON save_versions(created_at)",
            [],
        )?;

        // Identifier indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_game_identifiers_type_value ON game_identifiers(identifier_type, identifier_value)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_game_identifiers_game_id ON game_identifiers(game_id)",
            [],
        )?;

        Ok(())
    }

    pub fn migrate_database(conn: &Connection, from_version: u32, to_version: u32) -> DatabaseResult<()> {
        // For now, just recreate tables if version mismatch
        // In production, would implement proper migration logic
        if from_version != to_version {
            Self::drop_tables(conn)?;
            Self::create_tables(conn)?;
        }
        Ok(())
    }

    fn drop_tables(conn: &Connection) -> DatabaseResult<()> {
        let tables = [
            "user_games",
            "game_identifiers",
            "save_versions",
            "detected_saves",
            "save_locations",
            "games",
        ];

        for table in &tables {
            conn.execute(&format!("DROP TABLE IF EXISTS {}", table), [])?;
        }

        Ok(())
    }

    pub fn get_database_version(conn: &Connection) -> DatabaseResult<u32> {
        // Check if version table exists, if not create it
        conn.execute(
            "CREATE TABLE IF NOT EXISTS db_version (version INTEGER PRIMARY KEY)",
            [],
        )?;

        let version: u32 = conn.query_row(
            "SELECT version FROM db_version LIMIT 1",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        // If no version exists, set it to current version
        if version == 0 {
            conn.execute(
                "INSERT OR REPLACE INTO db_version (version) VALUES (?)",
                [DATABASE_VERSION],
            )?;
            Ok(DATABASE_VERSION)
        } else {
            Ok(version)
        }
    }

    pub fn set_database_version(conn: &Connection, version: u32) -> DatabaseResult<()> {
        conn.execute(
            "INSERT OR REPLACE INTO db_version (version) VALUES (?)",
            [version],
        )?;
        Ok(())
    }
}
