use rusqlite::Connection;
use crate::database::connection::DatabaseResult;
use crate::logger;

pub struct DatabaseSchema;

impl DatabaseSchema {
    pub fn create_tables(conn: &Connection) -> DatabaseResult<()> {
        logger::info("DATABASE", "Starting database table creation", None);
        
        let tables: [(&str, fn(&Connection) -> DatabaseResult<()>); 13] = [
            ("games", Self::create_games_table),
            ("save_locations", Self::create_save_locations_table),
            ("detected_saves", Self::create_detected_saves_table),
            ("save_versions", Self::create_save_versions_table),
            ("game_identifiers", Self::create_game_identifiers_table),
            ("user_games", Self::create_user_games_table),
            ("git_repositories", Self::create_git_repositories_table),
            ("git_save_commits", Self::create_git_save_commits_table),
            ("git_branches", Self::create_git_branches_table),
            ("cloud_sync_log", Self::create_cloud_sync_log_table),
            ("git_save_snapshots", Self::create_git_save_snapshots_table),
            ("pcgw_cache", Self::create_pcgw_cache_table),
            ("game_pcgw_mapping", Self::create_game_pcgw_mapping_table),
        ];
        
        let mut created_tables = Vec::new();
        let mut failed_tables = Vec::new();
        
        logger::info("DATABASE", &format!("Attempting to create {} tables", tables.len()), None);
        
        for (table_name, create_fn) in &tables {
            match create_fn(conn) {
                Ok(_) => {
                    created_tables.push(*table_name);
                    crate::logger::database::table_creation(*table_name, true);
                    logger::info("DATABASE", &format!("✓ Successfully created table: {}", table_name), None);
                }
                Err(e) => {
                    failed_tables.push((*table_name, e.to_string()));
                    crate::logger::database::table_creation(*table_name, false);
                    logger::error("DATABASE", &format!("✗ Failed to create table '{}': {}", table_name, e), None);
                    
                    // Continue with next table instead of returning immediately
                    // This helps us identify all failing tables
                }
            }
        }
        
        // Report results
        logger::info("DATABASE", &format!("Table creation summary: {} created, {} failed", created_tables.len(), failed_tables.len()), None);
        
        if !failed_tables.is_empty() {
            let failed_names: Vec<&str> = failed_tables.iter().map(|(name, _)| *name).collect();
            logger::error("DATABASE", 
                &format!("Failed to create {} tables: {}", failed_tables.len(), failed_names.join(", ")), 
                Some("Check the table definitions above for specific SQL errors"));
            
            // Only proceed with indexes if we have at least some tables created
            if created_tables.is_empty() {
                return Err(anyhow::anyhow!("No tables were created successfully. Cannot proceed with index creation."));
            }
        }
        
        // Create indexes (only if we have some tables)
        match Self::create_indexes(conn) {
            Ok(_) => {
                logger::info("DATABASE", "Successfully created database indexes", None);
                crate::logger::database::index_creation("all_indexes", true);
            }
            Err(e) => {
                crate::logger::database::index_creation("all_indexes", false);
                logger::error("DATABASE", "Failed to create database indexes", Some(&e.to_string()));
                return Err(e);
            }
        }
        
        if failed_tables.is_empty() {
            logger::info("DATABASE", "Database table creation completed successfully", Some(&format!("Created {} tables: {}", created_tables.len(), created_tables.join(", "))));
        } else {
            logger::warn("DATABASE", "Database table creation completed with failures", Some(&format!("Created {} tables, {} failed. Created: {}", created_tables.len(), failed_tables.len(), created_tables.join(", "))));
        }
        
        Ok(())
    }

    fn create_games_table(conn: &Connection) -> DatabaseResult<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS games (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                developer TEXT,
                publisher TEXT,
                platform TEXT NOT NULL, -- 'steam', 'epic', 'gog', 'standalone', 'other'
                platform_app_id TEXT,   -- Steam AppID, Epic Game ID, etc.
                executable_path TEXT,   -- Legacy single executable path
                installation_path TEXT,
                platform_executables TEXT, -- JSON: {"linux": "run.sh", "windows": "Game.exe", "macos": "Game.app"}
                genre TEXT,
                release_date DATE,
                cover_image_url TEXT,
                icon_base64 TEXT,       -- Base64 encoded game icon
                icon_path TEXT,         -- Path to exe for icon extraction/update
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                is_active INTEGER DEFAULT 1
            )
            "#,
            [],
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating games table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_save_locations_table(conn: &Connection) -> DatabaseResult<()> {
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
                is_relative_to_user INTEGER DEFAULT 1,
                environment_variable TEXT,            -- %APPDATA%, %LOCALAPPDATA%, etc.
                priority INTEGER DEFAULT 1,             -- Detection priority (1-10)
                detection_method TEXT,                -- 'heuristic', 'api', 'manual', 'community'
                community_confirmed INTEGER DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (game_id) REFERENCES games(id)
            )
            "#,
            [],
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating save_locations table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_detected_saves_table(conn: &Connection) -> DatabaseResult<()> {
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
                is_active INTEGER DEFAULT 1,
                metadata_json TEXT,                    -- JSON blob for game-specific metadata
                FOREIGN KEY (game_id) REFERENCES games(id),
                FOREIGN KEY (save_location_id) REFERENCES save_locations(id)
            )
            "#,
            [],
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating detected_saves table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_save_versions_table(conn: &Connection) -> DatabaseResult<()> {
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
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating save_versions table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_game_identifiers_table(conn: &Connection) -> DatabaseResult<()> {
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
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating game_identifiers table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_user_games_table(conn: &Connection) -> DatabaseResult<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS user_games (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL,
                custom_name TEXT,
                custom_install_path TEXT,
                custom_save_path TEXT,
                is_favorite INTEGER DEFAULT 0,
                backup_enabled INTEGER DEFAULT 1,
                auto_backup_interval INTEGER DEFAULT 3600, -- seconds
                max_versions INTEGER DEFAULT 10,
                compression_level INTEGER DEFAULT 3,      -- 1-22 for zstd
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (game_id) REFERENCES games(id)
            )
            "#,
            [],
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating user_games table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_git_repositories_table(conn: &Connection) -> DatabaseResult<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS git_repositories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL UNIQUE,
                local_path TEXT NOT NULL,
                remote_url TEXT,
                cloud_provider TEXT,                    -- 'github', 'gitlab', 'gitea', 'selfhosted'
                default_branch TEXT DEFAULT 'main',
                auto_commit INTEGER DEFAULT 1,
                auto_branch INTEGER DEFAULT 1,
                git_lfs_enabled INTEGER DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                last_sync_at TIMESTAMP,
                FOREIGN KEY (game_id) REFERENCES games(id)
            )
            "#,
            [],
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating git_repositories table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_git_save_commits_table(conn: &Connection) -> DatabaseResult<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS git_save_commits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL,
                git_commit_hash TEXT NOT NULL,
                branch_name TEXT NOT NULL,
                message TEXT NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                cloud_synced INTEGER DEFAULT 0,
                cloud_sync_url TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                is_current INTEGER DEFAULT 0,
                FOREIGN KEY (game_id) REFERENCES games(id)
            )
            "#,
            [],
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating git_save_commits table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_git_branches_table(conn: &Connection) -> DatabaseResult<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS git_branches (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL,
                branch_name TEXT NOT NULL,
                description TEXT,
                is_active INTEGER DEFAULT 0,
                last_commit_hash TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (game_id) REFERENCES games(id),
                UNIQUE(game_id, branch_name)
            )
            "#,
            [],
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating git_branches table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_cloud_sync_log_table(conn: &Connection) -> DatabaseResult<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS cloud_sync_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL,
                sync_type TEXT NOT NULL,                -- 'push', 'pull', 'merge'
                cloud_provider TEXT NOT NULL,
                sync_status TEXT NOT NULL,              -- 'success', 'failed', 'pending'
                error_message TEXT,
                sync_url TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (game_id) REFERENCES games(id)
            )
            "#,
            [],
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating cloud_sync_log table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_git_save_snapshots_table(conn: &Connection) -> DatabaseResult<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS git_save_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL,
                commit_id INTEGER,
                branch_name TEXT NOT NULL,
                version_name TEXT NOT NULL,
                compressed_path TEXT NOT NULL,
                file_size_bytes INTEGER NOT NULL,
                hash_sha256 TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                is_current INTEGER DEFAULT 0,
                FOREIGN KEY (commit_id) REFERENCES git_save_commits(id),
                FOREIGN KEY (game_id) REFERENCES games(id)
            )
            "#,
            [],
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating git_save_snapshots table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_pcgw_cache_table(conn: &Connection) -> DatabaseResult<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS pcgw_cache (
                query_key TEXT PRIMARY KEY,
                response_json TEXT NOT NULL,
                fetched_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            )
            "#,
            [],
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating pcgw_cache table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_game_pcgw_mapping_table(conn: &Connection) -> DatabaseResult<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS game_pcgw_mapping (
                game_id INTEGER PRIMARY KEY,
                pcgw_page_name TEXT NOT NULL,
                last_synced_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (game_id) REFERENCES games(id)
            )
            "#,
            [],
        ).map_err(|e| {
            logger::error("DATABASE", "Error creating game_pcgw_mapping table", Some(&e.to_string()));
            e.into()
        }).map(|_| ())
    }

    fn create_indexes(conn: &Connection) -> DatabaseResult<()> {
        logger::debug("DATABASE", "Creating database indexes", None);
        
        let indexes = [
            ("idx_games_platform_app_id", "CREATE INDEX IF NOT EXISTS idx_games_platform_app_id ON games(platform, platform_app_id)"),
            ("idx_games_name", "CREATE INDEX IF NOT EXISTS idx_games_name ON games(name)"),
            ("idx_save_locations_game_id", "CREATE INDEX IF NOT EXISTS idx_save_locations_game_id ON save_locations(game_id)"),
            ("idx_save_locations_platform", "CREATE INDEX IF NOT EXISTS idx_save_locations_platform ON save_locations(platform)"),
            ("idx_save_versions_detected_save_id", "CREATE INDEX IF NOT EXISTS idx_save_versions_detected_save_id ON save_versions(detected_save_id)"),
            ("idx_save_versions_created_at", "CREATE INDEX IF NOT EXISTS idx_save_versions_created_at ON save_versions(created_at)"),
            ("idx_game_identifiers_type_value", "CREATE INDEX IF NOT EXISTS idx_game_identifiers_type_value ON game_identifiers(identifier_type, identifier_value)"),
            ("idx_game_identifiers_game_id", "CREATE INDEX IF NOT EXISTS idx_game_identifiers_game_id ON game_identifiers(game_id)"),
        ];
        
        for (index_name, sql) in &indexes {
            match conn.execute(sql, []) {
                Ok(_) => {
                    logger::debug("DATABASE", &format!("Successfully created index: {}", index_name), None);
                }
                Err(e) => {
                    logger::error("DATABASE", &format!("Failed to create index: {}", index_name), Some(&e.to_string()));
                    crate::logger::database::index_creation(index_name, false);
                    return Err(e.into());
                }
            }
        }
        
        crate::logger::database::index_creation("all_indexes", true);
        Ok(())
    }

    pub fn drop_tables(conn: &Connection) -> DatabaseResult<()> {
        logger::info("DATABASE", "Starting database table deletion", None);
        
        let tables = [
            // Git-related tables (in reverse dependency order)
            "git_save_snapshots",
            "cloud_sync_log", 
            "git_save_commits",
            "git_branches",
            "git_repositories",
            // Save management tables
            "save_versions",
            "detected_saves",
            "save_locations",
            "user_games",
            "game_identifiers",
            // PCGW tables
            "game_pcgw_mapping",
            "pcgw_cache",
            // Core table last
            "games",
        ];

        let mut dropped_tables = Vec::new();
        
        for table in &tables {
            match conn.execute(&format!("DROP TABLE IF EXISTS {}", table), []) {
                Ok(_) => {
                    dropped_tables.push(*table);
                    logger::debug("DATABASE", &format!("Successfully dropped table: {}", table), None);
                }
                Err(e) => {
                    logger::error("DATABASE", &format!("Failed to drop table: {}", table), Some(&e.to_string()));
                    return Err(e.into());
                }
            }
        }

        logger::info("DATABASE", "Database table deletion completed", Some(&format!("Dropped {} tables: {}", dropped_tables.len(), dropped_tables.join(", "))));
        Ok(())
    }

    pub fn check_tables_exist(conn: &Connection) -> DatabaseResult<bool> {
        logger::debug("DATABASE", "Verifying all required tables exist", None);
        
        let required_tables = [
            "games",
            "save_locations",
            "detected_saves",
            "save_versions",
            "game_identifiers",
            "user_games",
            "git_repositories",
            "git_save_commits",
            "git_branches",
            "cloud_sync_log",
            "git_save_snapshots",
            "pcgw_cache",
            "game_pcgw_mapping",
        ];

        for table in &required_tables {
            let count: i64 = conn.query_row(
                "SELECT count(name) FROM sqlite_master WHERE type='table' AND name=?",
                [table],
                |row| row.get(0),
            ).unwrap_or(0);

            if count == 0 {
                logger::warn("DATABASE", &format!("Missing required table: {}", table), None);
                return Ok(false);
            }
        }

        logger::debug("DATABASE", "All required tables verified", None);
        Ok(true)
    }
}
