# Encrypted Local Database Strategy for Save Steward

## Overview

This document outlines the implementation strategy for encrypted local database storage of save metadata in Save Steward, addressing privacy concerns and data protection requirements.

## Why Encryption is Needed

### Privacy Concerns
- Save files contain personal gaming progress and potentially sensitive information
- Save metadata reveals gaming habits, play times, and game preferences
- Local database files may be accessible to other applications or users
- Compliance with data protection regulations (GDPR, CCPA)

### Threat Model
- **Physical Access**: Stolen/lost devices with unencrypted databases
- **Multi-user Systems**: Other users on shared computers accessing save data
- **Malware**: Applications scanning for gaming data
- **Cloud Sync**: Accidental exposure during file synchronization

## Implementation Options

### Option 1: SQLite with SQLCipher (Recommended)

**SQLCipher** is an open-source extension to SQLite that provides transparent 256-bit AES encryption of database files.

#### Advantages
- Drop-in replacement for SQLite
- Transparent encryption/decryption
- Industry-standard AES-256 encryption
- Key derivation using PBKDF2
- Performance overhead <5%
- Cross-platform support (Windows, macOS, Linux)
- Active development and community support

#### Implementation Details
```rust
// Example using sqlcipher-rs crate
use sqlcipher::{Connection, OpenFlags};

fn create_encrypted_db(db_path: &str, password: &str) -> Result<Connection, Error> {
    let mut conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_CREATE)?;
    
    // Set encryption key
    conn.execute(&format!("PRAGMA key = '{}';", password), [])?;
    
    // Verify encryption is working
    conn.execute("SELECT count(*) FROM sqlite_master;", [])?;
    
    Ok(conn)
}
```

#### Key Management Strategy
```rust
// Key derivation from master password
use argon2::{Argon2, PasswordHasher, PasswordHash, PasswordVerifier};

pub struct KeyManager {
    master_key: Vec<u8>,
}

impl KeyManager {
    pub fn new(master_password: &str) -> Result<Self, Error> {
        // Use Argon2id for key derivation
        let argon2 = Argon2::default();
        let salt = generate_secure_salt();
        let mut key = vec![0u8; 32]; // 256-bit key
        
        argon2.hash_password_into(
            master_password.as_bytes(),
            &salt,
            &mut key
        )?;
        
        Ok(Self { master_key: key })
    }
    
    pub fn get_database_key(&self) -> &[u8] {
        &self.master_key
    }
}
```

### Option 2: Application-Level Encryption

Encrypt specific columns or the entire database content at the application level.

#### Advantages
- Fine-grained control over what gets encrypted
- Can use different encryption keys for different data types
- No dependency on SQLCipher

#### Disadvantages
- More complex implementation
- Requires careful key management
- Potential performance impact on queries
- Need to handle encryption/decryption manually

#### Implementation Example
```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};

pub struct EncryptedField<T> {
    encrypted_data: Vec<u8>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> EncryptedField<T> {
    pub fn new(value: &T, cipher: &Aes256Gcm) -> Result<Self, Error> {
        let serialized = serde_json::to_vec(value)?;
        let nonce = generate_nonce();
        let encrypted = cipher.encrypt(&nonce, serialized.as_ref())?;
        
        Ok(Self {
            encrypted_data: encrypted,
            _phantom: std::marker::PhantomData,
        })
    }
    
    pub fn decrypt(&self, cipher: &Aes256Gcm) -> Result<T, Error> {
        let decrypted = cipher.decrypt(&self.nonce, self.encrypted_data.as_ref())?;
        let value = serde_json::from_slice(&decrypted)?;
        Ok(value)
    }
}
```

## Recommended Architecture

### Local Database Encryption Flow
```
User Password → Argon2id → Database Key → SQLCipher → Encrypted SQLite
     ↓
Secure Key Storage (OS-specific)
```

### Key Management Components

1. **Master Password Input**
   - Secure password input with masking
   - Password strength validation
   - Optional biometric authentication (platform-specific)

2. **Key Derivation**
   - Argon2id with appropriate parameters
   - Unique salt per database
   - Configurable memory/CPU cost

3. **Secure Storage**
   - Windows: Credential Manager or DPAPI
   - macOS: Keychain
   - Linux: Secret Service API or keyring

4. **Session Management**
   - In-memory key storage with secure cleanup
   - Session timeout and re-authentication
   - Background task handling during encryption

## Implementation Strategy

### Phase 1: Basic Encryption
```rust
// Database connection with encryption
pub struct EncryptedDatabase {
    conn: sqlcipher::Connection,
    key_manager: KeyManager,
}

impl EncryptedDatabase {
    pub fn new(db_path: &str, password: &str) -> Result<Self, Error> {
        let key_manager = KeyManager::new(password)?;
        let conn = create_encrypted_db(db_path, &base64_encode(key_manager.get_database_key()))?;
        
        Ok(Self { conn, key_manager })
    }
    
    pub fn execute(&mut self, query: &str, params: &[&dyn ToSql]) -> Result<usize, Error> {
        self.conn.execute(query, params)
    }
}
```

### Phase 2: Advanced Security Features
- Hardware security module integration
- Multi-factor authentication
- Secure backup and recovery
- Audit logging for security events

### Phase 3: Cloud Integration Preparation
- Key escrow for cloud synchronization
- End-to-end encryption for cloud storage
- Secure key exchange protocols

## Performance Considerations

### Benchmarks
- **SQLCipher overhead**: ~3-5% for typical operations
- **Memory usage**: Minimal increase (<1MB)
- **Startup time**: Additional 50-100ms for key derivation
- **Query performance**: No significant impact on indexed queries

### Optimization Strategies
1. **Connection Pooling**: Reuse encrypted connections
2. **Lazy Decryption**: Only decrypt accessed data
3. **Index Optimization**: Maintain performance on encrypted data
4. **Background Operations**: Encrypt/decrypt in background threads

## Security Best Practices

### Password Requirements
- Minimum 12 characters
- Mixed case, numbers, and symbols
- No common dictionary words
- Regular password rotation reminders

### Key Management
- Never store passwords in plain text
- Use secure random number generation
- Implement proper key rotation
- Secure memory cleanup after use

### Database Security
- Regular integrity checks
- Backup encryption verification
- Secure deletion of old database files
- Monitor for suspicious access patterns

## Migration Strategy

### Existing Database Migration
```rust
pub fn migrate_to_encrypted(old_path: &str, new_path: &str, password: &str) -> Result<(), Error> {
    // 1. Open existing unencrypted database
    let old_conn = Connection::open(old_path)?;
    
    // 2. Create new encrypted database
    let mut new_conn = create_encrypted_db(new_path, password)?;
    
    // 3. Export and import data
    let backup = backup::Backup::new(&old_conn, &new_conn)?;
    backup.run_to_completion(100, std::time::Duration::from_millis(250), None)?;
    
    // 4. Securely delete old database
    secure_delete(old_path)?;
    
    Ok(())
}
```

### Rollback Plan
- Maintain backup of unencrypted database
- Test migration on sample data first
- Provide decryption utilities for recovery

## Testing and Validation

### Security Testing
- Penetration testing of encryption implementation
- Key derivation validation
- Memory dump analysis
- Timing attack resistance

### Performance Testing
- Benchmark encrypted vs unencrypted operations
- Memory usage profiling
- Concurrent access testing
- Large database performance

## Future Considerations

### Supabase Integration
When integrating with Supabase:
- Use different encryption keys for local vs cloud
- Implement secure key exchange for cloud sync
- Consider client-side encryption before cloud upload
- Maintain encryption during transit and storage

### Advanced Features
- Hardware security module (HSM) support
- Biometric authentication integration
- Multi-user encryption with access control
- Zero-knowledge architecture for cloud components

## Conclusion

SQLCipher provides the optimal balance of security, performance, and ease of implementation for Save Steward's encrypted local database requirements. The transparent encryption approach minimizes development overhead while providing robust protection for user save metadata.