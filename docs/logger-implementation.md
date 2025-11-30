# Logging Implementation Documentation

## Overview

The Save-Steward application now includes a comprehensive logging system designed to provide detailed database creation logs and debug logs throughout the application lifecycle. This documentation covers the general purpose logging utility and its integration into the database creation pipeline.

## Architecture

### Core Components

1. **General Purpose Logger** (`src-tauri/src/logger/mod.rs`)
   - Lightweight, thread-safe logging utility
   - File-based logging with automatic rotation
   - Multiple log levels (DEBUG, INFO, WARN, ERROR)
   - Simple API for easy integration

2. **Database-Specific Logging**
   - Comprehensive database creation logging
   - Schema operation tracking
   - Error logging for database operations
   - Performance and migration logging

## Logger Features

### File Logging
- **Automatic Rotation**: Logs rotate when they exceed 10MB (configurable)
- **Multiple Files**: Maintains up to 5 rotated log files
- **Console Output**: Optional console output for development
- **Timestamp Format**: ISO 8601 format with milliseconds: `2025-11-30 07:25:12.345`

### Log Levels
- **DEBUG**: Detailed diagnostic information
- **INFO**: General application events
- **WARN**: Warning conditions
- **ERROR**: Error conditions

### Log Format
```
[2025-11-30 07:25:12.345] LEVEL [COMPONENT] Message - Details
```

Example:
```
[2025-11-30 07:25:12.345] INFO [DATABASE] Starting database table creation - Created 13 tables: games, save_locations, detected_saves, save_versions, game_identifiers, user_games, git_repositories, git_save_commits, git_branches, cloud_sync_log, git_save_snapshots, pcgw_cache, game_pcgw_mapping
```

## Usage

### Basic Logging Functions

```rust
use crate::logger;

// Info logging
logger::info("COMPONENT", "Operation completed successfully", None);

// Error logging with details
logger::error("DATABASE", "Failed to create table", Some("Table name: users"));

// Debug logging
logger::debug("API", "Processing request", Some("Request ID: 12345"));

// Warning logging
logger::warn("CONFIG", "Deprecated setting used", Some("Use new_setting instead"));
```

### Database-Specific Logging

```rust
use crate::logger::database;

// Database creation logging
database::creation_start(db_path);
database::creation_success(db_path, version, &["games", "users"]);
database::creation_error(db_path, "Permission denied");

// Table operation logging
database::table_creation("users", true);  // Success
database::table_creation("users", false); // Failure

// Index operation logging
database::index_creation("idx_users_email", true);
database::index_creation("idx_users_email", false);

// Migration logging
database::migration(1, 2, true);  // Success
database::migration(1, 2, false); // Failure

// Connection logging
database::connection_attempt(db_path);
database::connection_success(db_path);
database::connection_error(db_path, "Connection timeout");
```

## Database Creation Pipeline Logging

The database creation process now includes comprehensive logging at every step:

### 1. Application Startup
```rust
// lib.rs
crate::logger::info("APP", "Starting Save-Steward application", None);
crate::logger::database::connection_attempt(&db_path);
```

### 2. Database Connection
```rust
// database/connection.rs
logger::database::connection_attempt(db_path);
logger::database::connection_success(db_path);
logger::database::connection_error(db_path, error_details);
```

### 3. Schema Creation
```rust
// database/schema.rs
logger::info("DATABASE", "Starting database table creation", None);

// Individual table creation
database::table_creation("games", true);
logger::info("DATABASE", "Successfully created table: games", None);

// Index creation
database::index_creation("idx_games_name", true);
logger::info("DATABASE", "Successfully created database indexes", None);

// Error handling
database::table_creation("games", false);
logger::error("DATABASE", "Failed to create table: games", Some(&error.to_string()));
```

### 4. Version Management
```rust
// database/schema.rs
logger::debug("DATABASE", "Retrieving database version", None);
logger::info("DATABASE", "Initialized database version", Some("Set version to 1"));
```

### 5. Migration Logging
```rust
// database/schema.rs
logger::info("DATABASE", "Starting database migration from v1 to v2", None);
database::migration(1, 2, true);
logger::info("DATABASE", "Successfully recreated tables during migration", None);
```

## Configuration

### Default Configuration
```rust
LogConfig {
    log_file_path: PathBuf::from("save-steward.log"),
    max_file_size_bytes: 10 * 1024 * 1024, // 10MB
    max_log_files: 5,
    enable_console_output: true,
}
```

### Custom Configuration
```rust
use crate::logger::{initialize_logging_with_config, LogConfig};

let config = LogConfig {
    log_file_path: PathBuf::from("/var/log/save-steward/app.log"),
    max_file_size_bytes: 20 * 1024 * 1024, // 20MB
    max_log_files: 10,
    enable_console_output: false,
};

initialize_logging_with_config(config)?;
```

## Log Files

### Main Log File
- **File**: `save-steward.log`
- **Content**: All log levels (DEBUG, INFO, WARN, ERROR)
- **Rotation**: When file exceeds 10MB

### Log File Rotation
- **Pattern**: `save-steward.log.1`, `save-steward.log.2`, etc.
- **Maximum Files**: 5 rotated files
- **Oldest File**: Automatically deleted when limit reached

## Benefits

### 1. **Debugging Database Issues**
- Detailed logging of database creation process
- Clear error messages with context
- Step-by-step tracking of operations

### 2. **Monitoring**
- Application startup tracking
- Database connection status
- Schema migration tracking
- Performance monitoring

### 3. **Error Diagnosis**
- Comprehensive error logging
- Context information for failures
- Stack trace correlation

### 4. **Audit Trail**
- Complete history of database operations
- Version tracking
- User action logging

## Example Log Output

```
[2025-11-30 07:25:12.345] INFO [APP] Starting Save-Steward application - None
[2025-11-30 07:25:12.456] INFO [APP] Initializing database - None
[2025-11-30 07:25:12.567] INFO [DATABASE] Attempting database connection to ./save_steward.db - None
[2025-11-30 07:25:12.678] INFO [DATABASE] Database connection established to ./save_steward.db - None
[2025-11-30 07:25:12.789] INFO [DATABASE] Starting database schema creation - None
[2025-11-30 07:25:12.890] INFO [DATABASE] Successfully created table: games - None
[2025-11-30 07:25:12.901] INFO [DATABASE] Successfully created table: save_locations - None
[2025-11-30 07:25:13.012] INFO [DATABASE] Successfully created database indexes - None
[2025-11-30 07:25:13.123] INFO [DATABASE] Database schema created successfully - Version 1
[2025-11-30 07:25:13.234] INFO [APP] Database initialization complete - None
```

## Best Practices

### 1. **Log Levels**
- Use `DEBUG` for detailed diagnostic information
- Use `INFO` for normal operational events
- Use `WARN` for unusual but not error conditions
- Use `ERROR` for error conditions

### 2. **Component Naming**
- Use consistent component names: `DATABASE`, `APP`, `API`, `GIT`
- Keep component names short but descriptive
- Use uppercase for consistency

### 3. **Error Context**
- Always include relevant details in error logs
- Provide enough context to reproduce issues
- Include error codes or specific error messages

### 4. **Performance**
- Logging is asynchronous to minimize performance impact
- Consider log volume in production environments
- Use appropriate log levels to control verbosity

## Troubleshooting

### Common Issues

1. **Log File Not Created**
   - Check file permissions in the log directory
   - Verify the log file path is writable
   - Ensure the parent directory exists

2. **High Log Volume**
   - Reduce debug logging in production
   - Adjust log rotation settings
   - Monitor disk space usage

3. **Missing Database Logs**
   - Check if database creation actually runs
   - Verify database path is correct
   - Check for database initialization errors

### Debug Steps

1. Check log file exists and is writable:
   ```bash
   ls -la save-steward.log
   ```

2. Monitor logs in real-time:
   ```bash
   tail -f save-steward.log
   ```

3. Check for specific error patterns:
   ```bash
   grep "ERROR" save-steward.log
   ```

## Future Enhancements

### Planned Features
1. **Structured Logging**: JSON format for better parsing
2. **Remote Logging**: Send logs to remote servers
3. **Log Filtering**: Filter logs by component or level
4. **Performance Metrics**: Built-in performance monitoring
5. **Alert System**: Alert on critical errors

### Integration Opportunities
1. **System Monitoring**: Integrate with system monitoring tools
2. **Error Tracking**: Connect to error tracking services
3. **Analytics**: Log analysis for usage patterns
4. **Debugging Tools**: Integration with debugging frameworks

## Conclusion

The logging system provides comprehensive database creation logging and debug capabilities for the Save-Steward application. The general purpose logger is lightweight and easy to use, while the database-specific logging ensures detailed tracking of all database operations. This implementation significantly improves troubleshooting capabilities and provides a solid foundation for monitoring and debugging the application.
