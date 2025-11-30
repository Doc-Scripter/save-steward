use anyhow::Result as AnyhowResult;
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::schema::DATABASE_VERSION;

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

    pub async fn is_initialized(&self) -> DatabaseResult<bool> {
        let conn = self.get_connection().await;
        let count: i64 = conn.query_row(
            "SELECT count(name) FROM sqlite_master WHERE type='table' AND name='games'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        Ok(count > 0)
    }

    pub async fn initialize_schema(&self) -> DatabaseResult<()> {
        let conn = self.get_connection().await;
        crate::database::schema::DatabaseSchema::create_tables(&conn)?;
        Ok(())
    }

    /// Initialize the database with proper versioning and schema creation
    pub async fn initialize_database(&self) -> DatabaseResult<()> {
        let conn = self.get_connection().await;
        
        // Get current database version
        let current_version = crate::database::schema::DatabaseSchema::get_database_version(&conn)?;
        
        // If database is not initialized or has wrong version, recreate schema
        if current_version != DATABASE_VERSION {
            eprintln!("Database schema version mismatch (current: {}, required: {}). Recreating schema...", current_version, DATABASE_VERSION);
            
            // Drop all tables and recreate
            crate::database::schema::DatabaseSchema::drop_tables(&conn)?;
            crate::database::schema::DatabaseSchema::create_tables(&conn)?;
            
            // Set the version
            crate::database::schema::DatabaseSchema::set_database_version(&conn, DATABASE_VERSION)?;
            
            println!("Database schema created successfully with version {}", DATABASE_VERSION);
        } else {
            println!("Database schema is up to date (version {})", current_version);
        }
        
        Ok(())
    }

    pub async fn close(self) -> DatabaseResult<()> {
        // SQLite connections are automatically closed when dropped
        Ok(())
    }
}

// Flag file approach for robust database initialization
/// Ensure database is ready for use - creates tables if needed using flag file
pub async fn ensure_database_ready() -> Result<Arc<tokio::sync::Mutex<Database>>, String> {
    let db_path = DatabasePaths::database_file();
    let flag_path = db_path.with_extension("db_initialized");
    
    // Create database connection
    let db = Database::new(&db_path)
        .await
        .map_err(|e| format!("Database connection error: {}", e))?;
    
    // Check if database needs initialization
    if !flag_path.exists() {
        println!("Database not initialized, creating tables...");
        db.initialize_database().await
            .map_err(|e| format!("Database initialization error: {}", e))?;
        
        // Create flag file to indicate initialization is complete
        std::fs::write(&flag_path, "initialized").map_err(|e| {
            format!("Failed to create initialization flag: {}", e)
        })?;
        println!("Database initialization complete");
        
        // Also handle schema version changes - check version after initialization
        let temp_conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open database for version check: {}", e))?;
        let current_version = crate::database::schema::DatabaseSchema::get_database_version(&temp_conn)
            .map_err(|e| format!("Failed to get database version: {}", e))?;
        
        if current_version != DATABASE_VERSION {
            eprintln!("Database schema version mismatch detected during startup check");
            db.initialize_database().await
                .map_err(|e| format!("Database re-initialization error: {}", e))?;
            println!("Database schema updated successfully");
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
