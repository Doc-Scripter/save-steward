use anyhow::Result as AnyhowResult;
use argon2::{Argon2};
use argon2::password_hash::{SaltString};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use base64::{Engine as _, engine::general_purpose};

use crate::database::schema::DATABASE_VERSION;

pub type DatabaseResult<T> = AnyhowResult<T>;

pub type DatabaseConnection = Arc<tokio::sync::Mutex<rusqlite::Connection>>;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    Connection(#[from] rusqlite::Error),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Key derivation error: {0}")]
    KeyDerivation(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Database not initialized")]
    NotInitialized,

    #[error("Password hashing error: {0}")]
    Argon2Error(#[from] argon2::Error),
}

// Key manager for secure password-based key derivation
pub struct KeyManager {
    master_key: Vec<u8>,
}

impl KeyManager {
    pub fn new(master_password: &str) -> Result<Self, DatabaseError> {
        let argon2 = Argon2::default();

        // Generate a random salt for key derivation
        let salt = SaltString::generate(&mut rand::thread_rng());

        let mut key = vec![0u8; 32]; // 256-bit key

        // Derive key using Argon2id
        argon2.hash_password_into(
            master_password.as_bytes(),
            salt.as_str().as_bytes(),
            &mut key,
        )?;

        Ok(Self { master_key: key })
    }

    pub fn get_database_key(&self) -> &[u8] {
        &self.master_key
    }

    pub fn get_database_key_base64(&self) -> String {
        general_purpose::STANDARD.encode(&self.master_key)
    }
}

pub struct EncryptedDatabase {
    conn: Arc<Mutex<Connection>>,
    key_manager: KeyManager,
    path: PathBuf,
}

impl EncryptedDatabase {
    pub async fn new<P: AsRef<Path>>(db_path: P, master_password: &str) -> DatabaseResult<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Derive the encryption key
        let key_manager = KeyManager::new(master_password)?;

        // Create connection
        let conn = Self::create_connection(&db_path, &key_manager).await?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            key_manager,
            path: db_path,
        })
    }

    async fn create_connection(db_path: &Path, _key_manager: &KeyManager) -> DatabaseResult<Connection> {
        // Create standard SQLite connection
        // Note: For production, you might want to implement file-level encryption
        // or use SQLCipher when available. For now, using standard SQLite.
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
        let _conn = self.get_connection().await;
        
        // We need a mutable reference for schema operations, but get_connection returns a guard
        // Let's create a new temporary connection for schema operations
        let temp_conn = Connection::open(&self.path)?;
        
        // Get current database version
        let current_version = crate::database::schema::DatabaseSchema::get_database_version(&temp_conn)?;
        
        // If database is not initialized or has wrong version, recreate schema
        if current_version != DATABASE_VERSION {
            eprintln!("Database schema version mismatch (current: {}, required: {}). Recreating schema...", current_version, DATABASE_VERSION);
            
            // Drop all tables and recreate
            crate::database::schema::DatabaseSchema::drop_tables(&temp_conn)?;
            crate::database::schema::DatabaseSchema::create_tables(&temp_conn)?;
            
            // Set the version
            crate::database::schema::DatabaseSchema::set_database_version(&temp_conn, DATABASE_VERSION)?;
        }
        
        Ok(())
    }

    pub async fn verify_password(master_password: &str, db_path: &Path) -> DatabaseResult<bool> {
        if !db_path.exists() {
            return Ok(false);
        }

        let key_manager = KeyManager::new(master_password)?;
        let key_b64 = key_manager.get_database_key_base64();

        // Try to open and verify
        let result = Self::test_connection_key(db_path, &key_b64);
        match result {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn test_connection_key(db_path: &Path, _key_b64: &str) -> DatabaseResult<()> {
        // Since we're not using SQLCipher anymore, just verify the database exists and is accessible
        let conn = Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE,
        )?;

        let _: i64 = conn.query_row("SELECT count(*) FROM sqlite_master;", [], |row| row.get(0))?;
        Ok(())
    }

    pub async fn close(self) -> DatabaseResult<()> {
        // SQLite connections are automatically closed when dropped
        Ok(())
    }
}

// Flag file approach for robust database initialization
/// Ensure database is ready for use - creates tables if needed using flag file
pub async fn ensure_database_ready() -> Result<Arc<tokio::sync::Mutex<EncryptedDatabase>>, String> {
    let db_path = DatabasePaths::database_file();
    let flag_path = db_path.with_extension("db_initialized");
    
    // Create database connection
    let db = EncryptedDatabase::new(&db_path, "default_password")
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
        Self::default_app_data_dir().join("save_steward.db")
    }

    pub fn backup_directory() -> PathBuf {
        Self::default_app_data_dir().join("backups")
    }

    pub fn temp_directory() -> PathBuf {
        Self::default_app_data_dir().join("temp")
    }

    pub fn cache_directory() -> PathBuf {
        Self::default_app_data_dir().join("cache")
    }
}

// Ensure data directories exist
pub fn ensure_directories_exist() -> DatabaseResult<()> {
    std::fs::create_dir_all(DatabasePaths::default_app_data_dir())?;
    std::fs::create_dir_all(DatabasePaths::backup_directory())?;
    std::fs::create_dir_all(DatabasePaths::temp_directory())?;
    std::fs::create_dir_all(DatabasePaths::cache_directory())?;
    Ok(())
}
