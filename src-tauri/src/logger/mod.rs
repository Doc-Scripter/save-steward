//! General purpose logging utility for Save-Steward application
//! 
//! This module provides simple, lightweight logging capabilities:
//! - File logging with automatic rotation
//! - Multiple log levels (INFO, WARN, ERROR, DEBUG)
//! - Simple API for easy integration

use std::fs::{File, OpenOptions};
use std::io::{Write, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use once_cell::sync::Lazy;

static LOGGER: Lazy<Arc<Mutex<Logger>>> = Lazy::new(|| {
    Arc::new(Mutex::new(Logger::new()))
});

#[derive(Debug, Clone)]
pub struct LogConfig {
    pub log_file_path: PathBuf,
    pub max_file_size_bytes: u64,
    pub max_log_files: usize,
    pub enable_console_output: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        // Use same app data directory as database files for consistency
        let app_data_dir = crate::database::connection::DatabasePaths::default_app_data_dir();

        Self {
            log_file_path: app_data_dir.join("save-steward.log"),
            max_file_size_bytes: 10 * 1024 * 1024, // 10MB
            max_log_files: 5,
            enable_console_output: true,
        }
    }
}

struct Logger {
    config: LogConfig,
    current_file: Option<BufWriter<File>>,
}

impl Logger {
    pub fn new() -> Self {
        let config = LogConfig::default();
        Self {
            config,
            current_file: None,
        }
    }
    
    pub fn with_config(config: LogConfig) -> Self {
        Self {
            config,
            current_file: None,
        }
    }
    
    fn get_timestamp() -> String {
        let now = SystemTime::now();
        let datetime: chrono::DateTime<chrono::Utc> = chrono::DateTime::from(now);
        datetime.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
    }
    
    fn format_log_entry(level: &str, component: &str, message: &str, details: Option<&str>) -> String {
        let timestamp = Self::get_timestamp();
        if let Some(detail_text) = details {
            format!("[{}] {} [{}] {} - {}", timestamp, level, component, message, detail_text)
        } else {
            format!("[{}] {} [{}] {}", timestamp, level, component, message)
        }
    }
    
    fn write_to_file(&mut self, log_entry: &str) -> std::io::Result<()> {
        // Check if we need to rotate the log file
        if let Ok(metadata) = std::fs::metadata(&self.config.log_file_path) {
            if metadata.len() > self.config.max_file_size_bytes {
                self.rotate_log_files()?;
            }
        }
        
        // Open or create the log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_file_path)?;
            
        let mut writer = BufWriter::new(file);
        writer.write_all(log_entry.as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        
        Ok(())
    }
    
    fn rotate_log_files(&mut self) -> std::io::Result<()> {
        // Remove oldest log file if we have too many
        for i in (1..self.config.max_log_files).rev() {
            let old_path = self.config.log_file_path.with_extension(format!("log.{}", i));
            let new_path = self.config.log_file_path.with_extension(format!("log.{}", i + 1));
            
            if old_path.exists() {
                if i + 1 == self.config.max_log_files {
                    std::fs::remove_file(&old_path)?;
                } else {
                    std::fs::rename(&old_path, &new_path)?;
                }
            }
        }
        
        // Rename current log file
        if self.config.log_file_path.exists() {
            let backup_path = self.config.log_file_path.with_extension("log.1");
            std::fs::rename(&self.config.log_file_path, backup_path)?;
        }
        
        Ok(())
    }
    
    pub fn log(&mut self, level: LogLevel, component: &str, message: &str, details: Option<&str>) {
        let log_entry = Self::format_log_entry(level.as_str(), component, message, details);
        
        // Write to console if enabled
        if self.config.enable_console_output {
            match level {
                LogLevel::Error => eprintln!("{}", log_entry),
                _ => println!("{}", log_entry),
            }
        }
        
        // Write to file
        if let Err(e) = self.write_to_file(&log_entry) {
            eprintln!("Failed to write to log file: {}", e);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Initialize the logging system with default configuration
pub fn initialize_logging() -> Result<(), Box<dyn std::error::Error>> {
    initialize_logging_with_config(LogConfig::default())
}

/// Initialize the logging system with custom configuration
pub fn initialize_logging_with_config(config: LogConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Create log directory if it doesn't exist
    if let Some(parent) = config.log_file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let mut logger = LOGGER.lock().unwrap();
    logger.config = config;
    
    // Log initialization
    logger.log(LogLevel::Info, "LOGGER", "Logging system initialized", None);
    
    Ok(())
}

/// Log a debug message
pub fn debug(component: &str, message: &str, details: Option<&str>) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Debug, component, message, details);
    }
}

/// Log an info message
pub fn info(component: &str, message: &str, details: Option<&str>) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Info, component, message, details);
    }
}

/// Log a warning message
pub fn warn(component: &str, message: &str, details: Option<&str>) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Warn, component, message, details);
    }
}

/// Log an error message
pub fn error(component: &str, message: &str, details: Option<&str>) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Error, component, message, details);
    }
}

/// Database-specific logging functions
pub mod database {
    use super::*;
    
    /// Log database creation operation
    pub fn creation_start(db_path: &Path) {
        info("DATABASE", &format!("Starting database creation at {}", db_path.display()), None);
    }
    
    /// Log database creation success
    pub fn creation_success(db_path: &Path, version: u32, tables_created: &[&str]) {
        info("DATABASE", 
             &format!("Database created successfully at {} (version {})", db_path.display(), version),
             Some(&format!("Tables created: {}", tables_created.join(", "))));
    }
    
    /// Log database creation error
    pub fn creation_error(db_path: &Path, error_msg: &str) {
        error("DATABASE", 
              &format!("Database creation failed at {}", db_path.display()),
              Some(error_msg));
    }
    
    /// Log database connection operation
    pub fn connection_attempt(db_path: &Path) {
        info("DATABASE", &format!("Attempting database connection to {}", db_path.display()), None);
    }
    
    /// Log database connection success
    pub fn connection_success(db_path: &Path) {
        info("DATABASE", &format!("Database connection established to {}", db_path.display()), None);
    }
    
    /// Log database connection error
    pub fn connection_error(db_path: &Path, error_msg: &str) {
        error("DATABASE", 
              &format!("Database connection failed to {}", db_path.display()),
              Some(error_msg));
    }
    
    /// Log schema creation operation
    pub fn schema_creation_start() {
        info("DATABASE", "Starting database schema creation", None);
    }
    
    /// Log schema creation success
    pub fn schema_creation_success() {
        info("DATABASE", "Database schema created successfully", None);
    }
    
    /// Log schema creation error
    pub fn schema_creation_error(error_msg: &str) {
        error("DATABASE", "Database schema creation failed", Some(error_msg));
    }
    
    /// Log table creation operation
    pub fn table_creation(table_name: &str, success: bool) {
        if success {
            info("DATABASE", &format!("Table '{}' created successfully", table_name), None);
        } else {
            error("DATABASE", &format!("Failed to create table '{}'", table_name), None);
        }
    }
    
    /// Log index creation operation
    pub fn index_creation(index_name: &str, success: bool) {
        if success {
            info("DATABASE", &format!("Index '{}' created successfully", index_name), None);
        } else {
            error("DATABASE", &format!("Failed to create index '{}'", index_name), None);
        }
    }
    
    /// Log database migration operation
    pub fn migration(from_version: u32, to_version: u32, success: bool) {
        if success {
            info("DATABASE", 
                 &format!("Database migration completed: v{} -> v{}", from_version, to_version),
                 None);
        } else {
            error("DATABASE", 
                  &format!("Database migration failed: v{} -> v{}", from_version, to_version),
                  None);
        }
    }
    
    /// Log database initialization check
    pub fn initialization_check(db_path: &Path, is_initialized: bool) {
        info("DATABASE", 
             &format!("Database initialization check for {}: {}", db_path.display(), if is_initialized { "initialized" } else { "not initialized" }),
             None);
    }
    
}

/// Get current log configuration
pub fn get_log_config() -> LogConfig {
    LOGGER.lock().unwrap().config.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_log_levels() {
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }
    
    #[test]
    fn test_format_log_entry() {
        let entry = Logger::format_log_entry("INFO", "TEST", "Test message", Some("Additional details"));
        assert!(entry.contains("INFO"));
        assert!(entry.contains("TEST"));
        assert!(entry.contains("Test message"));
        assert!(entry.contains("Additional details"));
    }
    
    #[test]
    fn test_database_logging() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let config = LogConfig {
            log_file_path: log_path.clone(),
            max_file_size_bytes: 1024 * 1024,
            max_log_files: 3,
            enable_console_output: false,
        };
        
        initialize_logging_with_config(config).unwrap();
        
        // Test database logging
        database::creation_start(Path::new("test.db"));
        database::creation_success(Path::new("test.db"), 1, &["games", "save_locations"]);
        database::connection_attempt(Path::new("test.db"));
        database::connection_success(Path::new("test.db"));
        
        // Verify log file was created and contains entries
        let log_content = std::fs::read_to_string(&log_path).unwrap();
        assert!(log_content.contains("Starting database creation"));
        assert!(log_content.contains("Database created successfully"));
        assert!(log_content.contains("Attempting database connection"));
        assert!(log_content.contains("Database connection established"));
    }
}
