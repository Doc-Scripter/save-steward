# Git Implementation for Save Steward

## Overview

Save Steward uses a **Master Repository Strategy** for Git-based save version control. This approach stores all game saves in a single Git repository, with each save represented as a separate branch.

## Architecture

### Repository Structure

```
~/.local/share/save-steward/game_saves/  (single Git repository)
├── .git/                              (one .git folder)
├── Skyrim-MainQuest-Campaign/   (branch: Skyrim-MainQuest-Campaign)  
├── Skyrim-MainQuest-Imperial/         (branch: Skyrim-MainQuest-Imperial)
├── Cyberpunk2077-Nomad-EndGame/       (branch: Cyberpunk2077-Nomad-EndGame)
└── Cyberpunk2077-StreetKid-BadEnd/    (branch: Cyberpunk2077-StreetKid-BadEnd)
```

### Key Design Decisions

1. **Single Repository**: All game saves in one Git repository for efficiency
2. **Branch-per-Save**: Each save equals a Git branch with naming convention: `game-name-save-name`
3. **Directory Isolation**: Uses OS-standard data directories to avoid conflicts with user's projects
4. **Database Integration**: SQLite database tracks branch metadata and active states

## Branch Naming Convention

### Format: `game-name-save-name`

**Examples:**
- `Skyrim-MainQuest-Dragonbane`
- `Skyrim-Dragonborn-DLC`  
- `Factorio-Megabase-EarlyGame`
- `StardewValley-Spring-Year1`
- `EldenRing-Strength-RuneLevel150`

### Rules:
- **Separator**: Use `-` (dash) instead of `+` for Git-friendly naming
- **Game Name**: Retrieved from database (`games` table)
- **Save Name**: User-provided or auto-generated
- **Length**: Limited to 255 characters (Git constraint)
- **Characters**: Alphanumeric, dashes, underscores only

## Implementation Details

### Core Components

#### 1. `git_manager::GitSaveManager`
- **Purpose**: Main interface for Git operations
- **Location**: `src-tauri/src/git_manager/mod.rs`
- **Responsibilities**: 
  - Repository initialization
  - Branch creation and management
  - Save checkpoint creation
  - History and restoration operations

#### 2. `git_manager::branching`
- **Purpose**: Branch-specific operations
- **Location**: `src-tauri/src/git_manager/branching.rs`
- **Functions**:
  - `create_save_checkpoint()`: Creates/switches to save branches
  - `switch_save_branch()`: Switches between save branches
  - `get_game_branches()`: Lists branches for specific games
  - `delete_save_branch()`: Removes save branches

#### 3. `git_manager::repository`
- **Purpose**: Repository management
- **Location**: `src-tauri/src/git_manager/repository.rs`
- **Functions**:
  - `initialize_master_repo()`: Sets up the main Git repository
  - Repository configuration and Git attributes

### Database Schema

#### `git_repositories` Table
```sql
CREATE TABLE git_repositories (
    id INTEGER PRIMARY KEY,
    local_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_sync_at TEXT
);
```

#### `git_branches` Table
```sql
CREATE TABLE git_branches (
    game_id INTEGER NOT NULL,
    branch_name TEXT NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT 0,
    created_at TEXT NOT NULL,
    PRIMARY KEY (game_id, branch_name)
);
```

**Important**: Branch names are stored in the database for:
- Tracking which branch is active for each game
- Maintaining branch metadata (descriptions, creation timestamps)
- Efficient querying of branches by game
- Ensuring data consistency between Git operations and local state

**Database Integration**: When a save checkpoint is created, the resulting branch name (game-name-save-name) is stored in the `git_branches` table alongside the game ID, allowing efficient queries and state management.

## Operations

### Creating a Save Checkpoint

1. **Get Game Name**: Query database for game name by ID
2. **User Input**: User provides only the save name via UI
3. **Generate Branch Name**: Format as `game-name-save-name`
   - **Game Name**: Retrieved from database (e.g., "Skyrim")
   - **Save Name**: User input from UI (e.g., "Main Quest - Dragonbane")
   - **Final Branch**: "Skyrim-Main Quest - Dragonbane"
4. **Check Existence**: If branch exists, switch to it instead of creating
5. **Create Branch**: Fork from current HEAD
6. **Checkout**: Set working directory to new branch
7. **Update Database**: Store branch name and mark as active for game

**Data Flow:**
```typescript
// Frontend: User only enters save name
await invoke('create_save_checkpoint', {
  gameId: 123,
  message: 'Main Quest - Dragonbane'  // User input only
});
```

```rust
// Backend: Game name comes from database
pub async fn create_save_checkpoint(
    db: &std::sync::Arc<tokio::sync::Mutex<Database>>,
    master_repo_path: &str,
    game_id: i64, 
    save_name: &str  // User input from UI
) -> Result<String, String> {
    // Query game name from database
    let game_name = { /* SELECT name FROM games WHERE id = ? */ };
    
    // Construct branch name: game-name-save-name
    let branch_name = format!("{}-{}", game_name, save_name);
    // Result: "Skyrim-Main Quest - Dragonbane"
    
    // Store branch name in database
    save_branch_info(db, game_id, &branch_name, None).await?;
}
```

### Switching Branches

1. **Find Branch**: Locate Git branch by name
2. **Checkout**: Update working directory
3. **Set HEAD**: Point repository HEAD to branch
4. **Update Database**: Update active branch state

### Restoring to Commit

1. **Find Commit**: Locate by hash or timestamp
2. **Checkout Tree**: Update working directory to commit state
3. **Verify**: Ensure game-specific data is restored

## Edge Cases & Handling

### Repository Conflicts
- **Problem**: User might be in another Git repository
- **Solution**: Uses isolated data directory (`~/.local/share/save-steward/`)
- **Protection**: Never operates in user's working directories

### Branch Naming Conflicts
- **Problem**: User tries to create duplicate branches
- **Solution**: Checks if branch exists, switches to it instead of failing
- **Validation**: Name format validation prevents problematic characters

### Concurrent Access
- **Problem**: Multiple operations on same repository
- **Solution**: Uses `tokio::sync::Mutex<Database>` for thread safety
- **Implementation**: Database operations are serialized

### Large File Handling
- **Problem**: Game saves can be very large
- **Solution**: Git LFS configuration in `.gitattributes`:
```gitattributes
*.sav filter=lfs diff=lfs merge=lfs -text
*.save filter=lfs diff=lfs merge=lfs -text
*.zst filter=lfs diff=lfs merge=lfs -text
```

### Repository Corruption
- **Problem**: Git repository becomes corrupted
- **Solution**: 
  - Validates repository existence before operations
  - Handles Git errors gracefully with descriptive messages
  - Can reinitialize from backup if needed

## Legacy Code Deprecation

### Old Implementation (`git.rs`)
- **File**: `src-tauri/src/git.rs` (marked deprecated)
- **Approach**: Individual repositories per game
- **Status**: Replaced by master repository approach
- **Migration**: All references point to `git_manager::GitSaveManager`

### Deprecation Strategy
- Old `GitRepositoryManager` replaced with `GitSaveManager`
- Branch naming changed from `gamename+save-name` to `gamename-save-name`
- All deprecated functions now panic with migration instructions

## Performance Considerations

### Scalability
- **Single Repository**: More efficient than multiple repositories
- **Branch Management**: Git handles many branches efficiently
- **File Organization**: Each game gets separate subdirectory

### Memory Usage
- **Repository Caching**: Maintains open repository handles
- **Database Caching**: Caches game names and branch metadata
- **Async Operations**: Non-blocking Git operations

### Storage Efficiency
- **Git Compression**: Built-in compression for save files
- **Incremental Saves**: Only commits changed files
- **Branch Sharing**: Shared history reduces storage overhead

## Cloud Sync (Future)

### Planned Features
- **Multiple Providers**: GitHub, GitLab, Gitea, self-hosted
- **Selective Sync**: Choose which branches to sync
- **Encryption**: Client-side encryption before upload
- **Conflict Resolution**: Handle sync conflicts intelligently

### Current Status
- Cloud sync module exists but disabled due to compilation errors
- Infrastructure in place for future implementation

## Migration Guide

### For Existing Users
1. **Backup Current Saves**: Export existing save files
2. **Enable Git**: Use `enable_git_for_game` command
3. **Create First Branch**: Save current game state
4. **Continue Playing**: Git tracks future save points

### For Developers
- **Use `git_manager`**: All new Git operations should use this module
- **Branch Naming**: Always use `-` separator
- **Error Handling**: Check for `Result` types and handle errors
- **Database Integration**: Track branch metadata in `git_branches` table

## API Reference

### Tauri Commands
- `enable_git_for_game(game_id)`: Initialize Git for a game
- `create_save_checkpoint(game_id, message)`: Create/switch to save branch
- `create_save_branch(game_id, branch_name, description?)`: Create named branch
- `switch_save_branch(game_id, branch_name)`: Switch to existing branch
- `restore_to_commit(game_id, commit_hash)`: Restore to specific commit
- `restore_to_timestamp(game_id, timestamp)`: Restore to nearest commit by time
- `get_git_history(game_id)`: Get commit history and branch info
- `sync_to_cloud(game_id)`: Sync branches to cloud provider

### Rust Functions
- `GitSaveManager::new(db)`: Create new manager instance
- `initialize_master_repo()`: Set up main Git repository
- `create_save_checkpoint()`: Create save branch
- `get_save_history()`: Retrieve commit history
- `list_all_branches()`: Get all repository branches
- `get_game_branches(game_name)`: Filter branches by game

## Troubleshooting

### Common Issues

1. **Repository Not Found**
   - Ensure Git is installed
   - Run `enable_git_for_game` first
   - Check permissions on data directory

2. **Branch Creation Fails**
   - Verify game exists in database
   - Check branch name format
   - Ensure repository is writable

3. **Checkout Errors**
   - Ensure working directory is clean
   - Check for file conflicts
   - Verify branch exists

4. **Performance Issues**
   - Large save files may slow operations
   - Consider enabling Git LFS
   - Monitor disk space usage

### Debug Logging
Enable detailed logging with:
```rust
RUST_LOG=debug cargo tauri dev
```

Look for logs with:
- `git_manager`: Git operation details
- `database`: Database query information
- `branching`: Branch creation/switching operations

## Future Enhancements

### Planned Features
1. **Merge Support**: Allow merging save branches
2. **Visual Diff**: Show differences between save states
3. **Automatic Cleanup**: Prune old branches based on retention policy
4. **Backup Integration**: Automatic backup before major game events
5. **Game-Specific Rules**: Per-game Git configuration

### Advanced Features
1. **Save Comparison**: Visual diff between save states
2. **Branch Templates**: Reusable save branch patterns
3. **Collaboration**: Share save branches between users
4. **Game Auto-Detection**: Automatic save point detection
5. **Performance Metrics**: Track save/restore performance

This implementation provides a robust, scalable foundation for Git-based game save version control while maintaining simplicity and ease of use.
