# Hash-Based Change Detection System for Save Steward

## Overview
This document outlines the implementation of a robust hash-based change detection system for monitoring game save files and triggering automatic backups when changes are detected.

## Why Hash-Based Detection?

### Advantages Over Timestamp/Size Detection
- **Reliability**: Detects changes even when timestamps are manipulated
- **Integrity**: Verifies file content hasn't been corrupted
- **Granularity**: Detects subtle changes in large files
- **Deduplication**: Identifies identical files across different locations
- **Security**: Provides tamper-evident logging

## Hash Algorithm Selection

### Primary Algorithm: SHA-256
```rust
use sha2::{Sha256, Digest};

pub struct SaveFileHash {
    pub algorithm: HashAlgorithm,
    pub hash_value: String,
    pub file_size: u64,
    pub last_modified: SystemTime,
}

pub enum HashAlgorithm {
    SHA256,
    Blake3,        // Future optimization
    XXH3,          // Ultra-fast for large files
}
```

### Performance Considerations
- **SHA-256**: ~200-300 MB/s on modern CPUs
- **Blake3**: ~1 GB/s, cryptographically secure
- **XXH3**: ~5-10 GB/s, non-cryptographic but ultra-fast

## Change Detection Architecture

### Core Components

```rust
pub struct ChangeDetectionEngine {
    hash_calculator: HashCalculator,
    file_watcher: FileSystemWatcher,
    change_detector: ChangeDetector,
    event_publisher: EventPublisher,
    cache_manager: HashCacheManager,
}

pub struct HashCalculator {
    algorithm: HashAlgorithm,
    chunk_size: usize,        // Default: 64KB
    parallel_threshold: u64,    // Files > 100MB use parallel processing
}
```

### Hash Calculation Strategies

#### 1. Full File Hashing (Default)
```rust
impl HashCalculator {
    pub fn calculate_file_hash(&self, file_path: &Path) -> Result<String, HashError> {
        let mut file = File::open(file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0; self.chunk_size];
        
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 { break; }
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }
}
```

#### 2. Chunked Hashing for Large Files
```rust
impl HashCalculator {
    pub fn calculate_chunked_hash(&self, file_path: &Path) -> Result<ChunkedHash, HashError> {
        let file_size = fs::metadata(file_path)?.len();
        let chunk_count = (file_size + self.chunk_size - 1) / self.chunk_size;
        
        let mut chunk_hashes = Vec::new();
        let mut file = File::open(file_path)?;
        let mut buffer = vec![0; self.chunk_size];
        
        for chunk_index in 0..chunk_count {
            let bytes_read = file.read(&mut buffer)?;
            let chunk_hash = self.hash_chunk(&buffer[..bytes_read]);
            chunk_hashes.push(chunk_hash);
        }
        
        // Calculate overall hash from chunk hashes
        let overall_hash = self.hash_combined(&chunk_hashes);
        
        Ok(ChunkedHash {
            overall_hash,
            chunk_hashes,
            chunk_size: self.chunk_size,
            file_size,
        })
    }
}
```

#### 3. Parallel Processing for Very Large Files
```rust
impl HashCalculator {
    pub fn calculate_parallel_hash(&self, file_path: &Path) -> Result<String, HashError> {
        use rayon::prelude::*;
        
        let file_size = fs::metadata(file_path)?.len();
        let num_threads = rayon::current_num_threads();
        let chunk_size = file_size / num_threads as u64;
        
        let hashes: Vec<String> = (0..num_threads)
            .into_par_iter()
            .map(|thread_index| {
                let start = thread_index as u64 * chunk_size;
                let end = if thread_index == num_threads - 1 {
                    file_size
                } else {
                    start + chunk_size
                };
                
                self.hash_file_section(file_path, start, end)
            })
            .collect::<Result<Vec<_>, _>>()?;
        
        // Combine thread hashes
        self.hash_combined(&hashes)
    }
}
```

## File System Monitoring

### Platform-Specific Implementation

#### Windows: ReadDirectoryChangesW
```rust
#[cfg(target_os = "windows")]
impl FileSystemWatcher {
    pub fn watch_directory(&self, path: &Path) -> Result<(), WatchError> {
        use winapi::um::fileapi::*;
        use winapi::um::winbase::*;
        
        let path_wide: Vec<u16> = path.to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        
        let handle = unsafe {
            CreateFileW(
                path_wide.as_ptr(),
                FILE_LIST_DIRECTORY,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                std::ptr::null_mut(),
            )
        };
        
        if handle == INVALID_HANDLE_VALUE {
            return Err(WatchError::WindowsError);
        }
        
        // Set up overlapped I/O for async monitoring
        self.start_monitoring(handle)
    }
}
```

#### Linux: inotify
```rust
#[cfg(target_os = "linux")]
impl FileSystemWatcher {
    pub fn watch_directory(&self, path: &Path) -> Result<(), WatchError> {
        use inotify::{Inotify, WatchMask};
        
        let mut inotify = Inotify::init()?;
        
        inotify.add_watch(
            path,
            WatchMask::CREATE | WatchMask::DELETE | WatchMask::MODIFY | WatchMask::MOVED
        )?;
        
        let mut buffer = [0; 1024];
        loop {
            let events = inotify.read_events(&mut buffer)?;
            
            for event in events {
                self.handle_file_event(&event)?;
            }
        }
    }
}
```

#### macOS: FSEvents
```rust
#[cfg(target_os = "macos")]
impl FileSystemWatcher {
    pub fn watch_directory(&self, path: &Path) -> Result<(), WatchError> {
        use fsevent::{Event, EventStream};
        
        let (tx, rx) = std::sync::mpsc::channel();
        
        let mut stream = EventStream::new(&[path], tx);
        stream.start()?;
        
        while let Ok(event) = rx.recv() {
            match event {
                Event::Created(path) => self.handle_created(&path)?,
                Event::Modified(path) => self.handle_modified(&path)?,
                Event::Removed(path) => self.handle_removed(&path)?,
                _ => {}
            }
        }
        
        Ok(())
    }
}
```

## Change Detection Logic

### Smart Change Detection Algorithm
```rust
pub struct ChangeDetector {
    min_change_interval: Duration,      // Debounce time: 5 seconds
    max_batch_size: usize,              // Maximum files to process together
    similarity_threshold: f64,          // For detecting similar saves
}

impl ChangeDetector {
    pub fn detect_changes(&self, save_path: &Path) -> Result<Vec<ChangeEvent>, DetectionError> {
        let current_hash = self.calculate_current_hash(save_path)?;
        let previous_hash = self.get_previous_hash(save_path)?;
        
        // Quick comparison first
        if current_hash == previous_hash {
            return Ok(vec![]); // No changes detected
        }
        
        // Detailed analysis for changed files
        let change_analysis = self.analyze_file_changes(save_path, &current_hash)?;
        
        // Filter out temporary/irrelevant changes
        let significant_changes = self.filter_significant_changes(change_analysis)?;
        
        // Batch related changes together
        let batched_changes = self.batch_related_changes(significant_changes)?;
        
        Ok(batched_changes)
    }
    
    fn analyze_file_changes(&self, save_path: &Path, current_hash: &str) -> Result<ChangeAnalysis, DetectionError> {
        let previous_state = self.get_previous_file_state(save_path)?;
        let current_state = self.get_current_file_state(save_path)?;
        
        Ok(ChangeAnalysis {
            files_added: self.find_added_files(&previous_state, &current_state),
            files_removed: self.find_removed_files(&previous_state, &current_state),
            files_modified: self.find_modified_files(&previous_state, &current_state),
            total_change_percentage: self.calculate_change_percentage(&previous_state, &current_state),
        })
    }
}
```

### Change Event Classification
```rust
#[derive(Debug, Clone)]
pub enum ChangeEvent {
    FileCreated {
        path: PathBuf,
        size: u64,
        hash: String,
        confidence: ChangeConfidence,
    },
    FileModified {
        path: PathBuf,
        old_hash: String,
        new_hash: String,
        change_percentage: f64,
        modification_type: ModificationType,
    },
    FileDeleted {
        path: PathBuf,
        old_hash: String,
        deletion_type: DeletionType,
    },
    DirectoryRestructure {
        old_structure: DirectoryStructure,
        new_structure: DirectoryStructure,
        change_summary: RestructureSummary,
    },
}

#[derive(Debug, Clone)]
pub enum ChangeConfidence {
    High,       // Hash change + size change + timestamp change
    Medium,     // Hash change + one other indicator
    Low,        // Only hash change (could be timestamp manipulation)
}
```

## Hash Cache Management

### Cache Strategy
```rust
pub struct HashCacheManager {
    cache: Arc<RwLock<HashMap<PathBuf, CachedHash>>>,
    cache_size_limit: usize,
    cache_ttl: Duration,
    persistence_manager: CachePersistenceManager,
}

#[derive(Debug, Clone)]
pub struct CachedHash {
    pub hash: String,
    pub file_size: u64,
    pub last_modified: SystemTime,
    pub cached_at: SystemTime,
    pub access_count: u64,
    pub file_metadata: FileMetadata,
}

impl HashCacheManager {
    pub fn get_or_calculate_hash(&self, path: &Path) -> Result<String, CacheError> {
        // Check cache first
        if let Some(cached) = self.get_from_cache(path) {
            if self.is_cache_valid(&cached, path) {
                return Ok(cached.hash);
            }
        }
        
        // Calculate hash if not in cache or invalid
        let hash = self.calculate_hash(path)?;
        
        // Store in cache
        self.store_in_cache(path, &hash)?;
        
        Ok(hash)
    }
    
    fn is_cache_valid(&self, cached: &CachedHash, path: &Path) -> bool {
        // Check if file metadata matches cached values
        if let Ok(current_metadata) = fs::metadata(path) {
            let file_size_matches = cached.file_size == current_metadata.len();
            let modified_time_matches = cached.last_modified == current_metadata.modified().unwrap();
            let cache_not_expired = cached.cached_at.elapsed().unwrap() < self.cache_ttl;
            
            file_size_matches && modified_time_matches && cache_not_expired
        } else {
            false
        }
    }
}
```

### Cache Persistence
```rust
pub struct CachePersistenceManager {
    cache_file_path: PathBuf,
    save_interval: Duration,
    last_save: Instant,
}

impl CachePersistenceManager {
    pub async fn save_cache(&self, cache: &HashMap<PathBuf, CachedHash>) -> Result<(), PersistenceError> {
        let cache_data = serde_json::to_string(cache)?;
        let encrypted_data = self.encrypt_cache_data(cache_data)?;
        
        fs::write(&self.cache_file_path, encrypted_data).await?;
        
        Ok(())
    }
    
    pub async fn load_cache(&self) -> Result<HashMap<PathBuf, CachedHash>, PersistenceError> {
        if !self.cache_file_path.exists() {
            return Ok(HashMap::new());
        }
        
        let encrypted_data = fs::read(&self.cache_file_path).await?;
        let cache_data = self.decrypt_cache_data(encrypted_data)?;
        let cache: HashMap<PathBuf, CachedHash> = serde_json::from_str(&cache_data)?;
        
        Ok(cache)
    }
}
```

## Performance Optimization

### Hash Calculation Optimization
```rust
pub struct HashPerformanceOptimizer {
    parallel_threshold: u64,        // Files larger than this use parallel processing
    memory_map_threshold: u64,      // Files larger than this use memory mapping
    chunk_size: usize,              // 64KB chunks for processing
}

impl HashPerformanceOptimizer {
    pub fn optimize_hash_calculation(&self, file_path: &Path) -> Result<HashCalculationStrategy, OptimizationError> {
        let file_size = fs::metadata(file_path)?.len();
        
        match file_size {
            size if size < 10 * 1024 * 1024 => {
                // Small files (< 10MB): Direct reading
                Ok(HashCalculationStrategy::DirectRead)
            }
            size if size < 100 * 1024 * 1024 => {
                // Medium files (10-100MB): Chunked processing
                Ok(HashCalculationStrategy::Chunked(self.chunk_size))
            }
            size if size < 1024 * 1024 * 1024 => {
                // Large files (100MB-1GB): Parallel processing
                Ok(HashCalculationStrategy::Parallel)
            }
            _ => {
                // Very large files (> 1GB): Memory mapping
                Ok(HashCalculationStrategy::MemoryMapped)
            }
        }
    }
}
```

### Background Processing
```rust
pub struct BackgroundHashProcessor {
    worker_pool: Arc<ThreadPool>,
    task_queue: Arc<SegQueue<HashTask>>,
    result_channel: Channel<HashResult>,
}

impl BackgroundHashProcessor {
    pub fn process_save_directory(&self, save_path: &Path) -> Result<(), ProcessingError> {
        let save_files = self.collect_save_files(save_path)?;
        
        // Queue hash tasks for all files
        for file_path in save_files {
            let task = HashTask {
                file_path: file_path.clone(),
                priority: self.calculate_priority(&file_path),
                deadline: Instant::now() + Duration::from_secs(30),
            };
            
            self.task_queue.push(task);
        }
        
        // Process tasks in background
        self.start_background_processing()?;
        
        Ok(())
    }
}
```

## Error Handling and Recovery

### Robust Error Handling
```rust
#[derive(Debug, thiserror::Error)]
pub enum HashDetectionError {
    #[error("File not accessible: {0}")]
    FileNotAccessible(PathBuf),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),
    
    #[error("File system error: {0}")]
    FileSystemError(#[from] std::io::Error),
    
    #[error("Hash calculation failed: {0}")]
    HashCalculationFailed(String),
    
    #[error("Cache corruption detected")]
    CacheCorruption,
    
    #[error("File locked by another process: {0}")]
    FileLocked(PathBuf),
}

impl ChangeDetectionEngine {
    pub fn detect_changes_with_recovery(&self, save_path: &Path) -> Result<Vec<ChangeEvent>, RecoveryError> {
        let mut retry_count = 0;
        let max_retries = 3;
        
        loop {
            match self.attempt_change_detection(save_path) {
                Ok(changes) => return Ok(changes),
                Err(error) => {
                    match self.classify_error(&error) {
                        ErrorType::Retryable => {
                            if retry_count < max_retries {
                                retry_count += 1;
                                std::thread::sleep(Duration::from_millis(100 * retry_count));
                                continue;
                            }
                        }
                        ErrorType::Permanent => {
                            return Err(RecoveryError::PermanentError(error));
                        }
                        ErrorType::RequiresUserAction => {
                            return Err(RecoveryError::UserActionRequired(error));
                        }
                    }
                }
            }
        }
    }
}
```

## Integration with Save Steward

### Event System Integration
```rust
pub struct SaveChangeEventHandler {
    backup_engine: Arc<BackupEngine>,
    notification_manager: Arc<NotificationManager>,
    user_preferences: Arc<UserPreferences>,
}

impl SaveChangeEventHandler {
    pub async fn handle_save_change(&self, change_event: ChangeEvent) -> Result<(), HandlerError> {
        // Validate the change event
        self.validate_change_event(&change_event)?;
        
        // Check user preferences for this game
        if !self.should_backup_change(&change_event).await? {
            return Ok(());
        }
        
        // Trigger backup
        let backup_result = self.backup_engine.create_backup(&change_event).await?;
        
        // Notify user if configured
        if self.user_preferences.should_notify(&change_event) {
            self.notification_manager.notify_backup_created(&backup_result).await?;
        }
        
        Ok(())
    }
}
```

### Configuration Options
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDetectionConfig {
    pub enabled: bool,
    pub scan_interval: Duration,           // How often to scan for changes
    pub debounce_delay: Duration,          // Delay before processing changes
    pub hash_algorithm: HashAlgorithm,     // SHA-256, Blake3, etc.
    pub cache_size_limit: usize,           // Maximum cache entries
    pub parallel_processing_threshold: u64, // File size threshold
    pub excluded_patterns: Vec<String>,      // File patterns to ignore
    pub backup_triggers: BackupTriggers,    // When to create backups
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupTriggers {
    pub on_file_create: bool,
    pub on_file_modify: bool,
    pub on_file_delete: bool,
    pub minimum_change_size: u64,          // Only backup if change > X bytes
    pub maximum_backup_frequency: Duration, // Rate limiting
}
```

## Testing and Validation

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hash_calculation_accuracy() {
        let test_file = create_test_file_with_content("test content");
        let calculator = HashCalculator::new(HashAlgorithm::SHA256);
        
        let hash1 = calculator.calculate_file_hash(&test_file).unwrap();
        let hash2 = calculator.calculate_file_hash(&test_file).unwrap();
        
        assert_eq!(hash1, hash2); // Same file should produce same hash
        assert_eq!(hash1, "ed7002b439e9ac845f22357d822bac1444730fbdb6016d3ec9432297b9ec9f73");
    }
    
    #[test]
    fn test_change_detection_accuracy() {
        let temp_dir = create_temp_save_directory();
        let detector = ChangeDetector::new();
        
        // Initial state
        let initial_changes = detector.detect_changes(&temp_dir).unwrap();
        assert_eq!(initial_changes.len(), 0);
        
        // Modify a file
        modify_test_file(&temp_dir.join("save.dat"));
        let changes = detector.detect_changes(&temp_dir).unwrap();
        
        assert_eq!(changes.len(), 1);
        assert!(matches!(changes[0], ChangeEvent::FileModified { .. }));
    }
}
```

### Performance Benchmarks
```rust
#[cfg(bench)]
mod benchmarks {
    use super::*;
    
    #[bench]
    fn bench_large_file_hashing(b: &mut Bencher) {
        let large_file = create_large_test_file(100 * 1024 * 1024); // 100MB
        let calculator = HashCalculator::new(HashAlgorithm::SHA256);
        
        b.iter(|| {
            calculator.calculate_file_hash(&large_file).unwrap()
        });
    }
}
```

This hash-based change detection system provides robust, efficient, and reliable monitoring of game save files while maintaining excellent performance and user experience.