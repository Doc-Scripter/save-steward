use anyhow::Result as AnyhowResult;
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

pub type DatabaseResult<T> = AnyhowResult<T>;

pub type DatabaseConnection = Arc<tokio::sync::Mutex<rusqlite::Connection>>;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    Connection(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database not initialized")]
    NotInitialized,
}

pub struct Database {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl Database {
    pub async fn new<P: AsRef<Path>>(db_path: P) -> DatabaseResult<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Create connection
        let conn = Self::create_connection(&db_path).await?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path: db_path,
        })
    }

    async fn create_connection(db_path: &Path) -> DatabaseResult<Connection> {
        // Create standard SQLite connection
        let conn = Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        // Basic SQLite optimizations
        let _: rusqlite::Result<String> = conn.query_row("PRAGMA journal_mode=WAL;", [], |_| Ok(String::new()));
        let _: rusqlite::Result<String> = conn.query_row("PRAGMA synchronous=NORMAL;", [], |_| Ok(String::new()));
        let _: rusqlite::Result<String> = conn.query_row("PRAGMA temp_store=memory;", [], |_| Ok(String::new()));
        let _: rusqlite::Result<i64> = conn.query_row("PRAGMA mmap_size=268435456;", [], |_| Ok(0));
        let _: rusqlite::Result<i64> = conn.query_row("PRAGMA cache_size=-64000;", [], |_| Ok(0));

        // Verify connection is working
        let _: i64 = conn.query_row("SELECT count(*) FROM sqlite_master;", [], |row| row.get(0))?;

        Ok(conn)
    }

    pub async fn get_connection(&self) -> tokio::sync::MutexGuard<'_, Connection> {
        self.conn.lock().await
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    /// Initialize the database schema - creates all tables
    pub async fn initialize_database(&self) -> DatabaseResult<()> {
        let conn = self.get_connection().await;
        crate::database::schema::DatabaseSchema::create_tables(&conn)?;
        Ok(())
    }

    pub async fn close(self) -> DatabaseResult<()> {
        // SQLite connections are automatically closed when dropped
        Ok(())
    }
}

// Simple database initialization - no flags, no versions
pub async fn ensure_database_ready() -> Result<Arc<tokio::sync::Mutex<Database>>, String> {
    let db_path = DatabasePaths::database_file();

    crate::logger::info("DATABASE", &format!("Connecting to database: {}", db_path.display()), None);

    // Create database connection
    let db = Database::new(&db_path)
        .await
        .map_err(|e| format!("Database connection error: {}", e))?;

    crate::logger::info("DATABASE", "Database connection established", None);

    // Check if tables exist
    {
        let conn = db.get_connection().await;
        let tables_exist = crate::database::schema::DatabaseSchema::check_tables_exist(&conn)
            .map_err(|e| format!("Database table check error: {}", e))?;
        
        if !tables_exist {
            crate::logger::info("DATABASE", "Required tables missing, initializing database schema", None);
            
            // Initialize database schema
            db.initialize_database().await
                .map_err(|e| format!("Database initialization error: {}", e))?;
                
            crate::logger::info("DATABASE", "Database schema initialized successfully", None);
        } else {
            crate::logger::info("DATABASE", "Database tables verified - schema already exists", None);
        }
    }

    Ok(Arc::new(tokio::sync::Mutex::new(db)))
}

// Database path management
pub struct DatabasePaths;

impl DatabasePaths {
    pub fn default_app_data_dir() -> PathBuf {
        if cfg!(target_os = "windows") {
            // %APPDATA%/SaveSteward
            std::env::var("APPDATA")
                .map(|app_data| PathBuf::from(app_data).join("SaveSteward"))
                .unwrap_or_else(|_| PathBuf::from("./data"))
        } else if cfg!(target_os = "macos") {
            // ~/Library/Application Support/SaveSteward
            std::env::var("HOME")
                .map(|home| PathBuf::from(home)
                     .join("Library")
                     .join("Application Support")
                     .join("SaveSteward"))
                .unwrap_or_else(|_| PathBuf::from("./data"))
        } else {
            // Linux: ~/.local/share/save-steward
            std::env::var("HOME")
                .map(|home| PathBuf::from(home)
                     .join(".local")
                     .join("share")
                     .join("save-steward"))
                .unwrap_or_else(|_| PathBuf::from("./data"))
        }
    }

    pub fn database_file() -> PathBuf {
        PathBuf::from(".").join("save_steward.db")
    }

    pub fn backup_directory() -> PathBuf {
        PathBuf::from(".").join("data").join("backups")
    }

    pub fn temp_directory() -> PathBuf {
        PathBuf::from(".").join("data").join("temp")
    }

    pub fn cache_directory() -> PathBuf {
        PathBuf::from(".").join("data").join("cache")
    }
}

// Ensure data directories exist
pub fn ensure_directories_exist() -> DatabaseResult<()> {
    std::fs::create_dir_all(DatabasePaths::database_file().parent().unwrap())?;
    std::fs::create_dir_all(DatabasePaths::backup_directory())?;
    std::fs::create_dir_all(DatabasePaths::temp_directory())?;
    std::fs::create_dir_all(DatabasePaths::cache_directory())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_ensure_database_ready_initializes_correctly() {
        // Setup: Clean up existing files
        let db_path = PathBuf::from(".").join("save_steward.db");
        let log_path = PathBuf::from(".").join("test_init.log");
        
        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_file(&log_path);

        // Setup: Configure logging to local file
        let log_config = crate::logger::LogConfig {
            log_file_path: log_path.clone(),
            max_file_size_bytes: 1024 * 1024,
            max_log_files: 1,
            enable_console_output: true,
        };
        // We ignore the error here as it might be already initialized
        let _ = crate::logger::initialize_logging_with_config(log_config);

        // Act: Run initialization
        let result = ensure_database_ready().await;
        
        // Assert: Check result
        assert!(result.is_ok(), "Initialization should succeed");
        
        // Assert: Check files exist
        assert!(db_path.exists(), "Database file should exist");
        assert!(log_path.exists(), "Log file should exist");

        // Assert: Check log content
        let log_content = fs::read_to_string(&log_path).unwrap_or_default();
        assert!(log_content.contains("Required tables missing"), "Log should contain initialization trigger");
        assert!(log_content.contains("Database schema initialized successfully"), "Log should contain init complete");

        // Assert: Check tables exist
        let db_mutex = result.unwrap();
        let db = db_mutex.lock().await;
        let conn = db.get_connection().await;
        let count: i64 = conn.query_row(
            "SELECT count(name) FROM sqlite_master WHERE type='table' AND name='games'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        assert_eq!(count, 1, "Games table should exist");
        
        // Cleanup
        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_file(&log_path);
    }
}
