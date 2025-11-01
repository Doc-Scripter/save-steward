# Save Location Detection System for Save Steward

## Overview

The save location detection system automatically identifies where games store their save files across different platforms and gaming services. This system eliminates manual configuration while providing reliable save file discovery.

## Detection Strategy

### Multi-Layered Approach
The system uses several detection methods working in parallel, with confidence scoring to determine the most reliable save locations.

**Primary Methods:**
1. **Platform API Integration** - Direct queries to Steam, Epic, GOG APIs
2. **Registry Analysis** - Windows registry keys for installed games
3. **Heuristic Scanning** - Pattern matching in common save directories
4. **Community Database** - PCGamingWiki and crowd-sourced locations
5. **Executable Analysis** - Reading game metadata and configuration files

### Confidence Scoring
Each detected location receives a confidence score (0-100) based on:
- **API-verified locations**: 95-100 points
- **Registry-discovered paths**: 80-90 points  
- **Heuristic pattern matches**: 60-80 points
- **Community database entries**: 50-70 points
- **User-specified locations**: 100 points (override)

## Platform-Specific Detection

### Windows Detection
**Registry Locations:**
- `HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`
- `HKEY_CURRENT_USER\Software\[GameDeveloper]\[GameName]`
- `HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\[GameDeveloper]`

**Common Save Directories:**
- `%APPDATA%` - Roaming application data
- `%LOCALAPPDATA%` - Local application data  
- `%USERPROFILE%\Documents\My Games` - Legacy game saves
- `%USERPROFILE%\Saved Games` - Windows 7+ standard
- Game-specific folders in Program Files

### Steam Integration
**Steam API Methods:**
- Parse `libraryfolders.vdf` for game installation paths
- Read `appmanifest_[appid].acf` files for game metadata
- Query Steam Web API for game information
- Analyze Steam Cloud configuration

**Steam Save Locations:**
- `Steam\userdata\[steamid]\[appid]\` - Cloud-enabled games
- Game installation folders for non-Steam Cloud games
- Workshop content in `Steam\workshop\content\[appid]`

### Epic Games Store
**Detection Methods:**
- Parse `EpicGamesLauncher\Data\Manifests` JSON files
- Read `catalog_items` cache for installed games
- Analyze Epic Online Services configuration

**Save Location Patterns:**
- `%LOCALAPPDATA%\EpicGamesLauncher\Saved\SaveGames`
- Game-specific folders in Epic Games directories
- Cloud save synchronization folders

### GOG Galaxy
**Integration Points:**
- `GOG Galaxy\storage\galaxy-2.0.db` SQLite database
- `GOG.com\Games\[game]\goggame-[id].info` files
- Galaxy API for cloud saves

## Change Detection

### File System Monitoring
The system continuously monitors detected save locations using platform-specific file system watchers:

**Windows:** ReadDirectoryChangesW API
**Linux:** inotify system calls  
**macOS:** FSEvents framework

### Smart Detection Logic
Rather than reacting to every file change, the system:
1. **Batches rapid changes** - Groups multiple file modifications within 5-second windows
2. **Filters temporary files** - Ignores `.tmp`, `.bak`, lock files
3. **Validates save integrity** - Confirms changes represent complete save states
4. **Prioritizes user activity** - Focuses on games currently running or recently played

### Hash-Based Verification
When changes are detected:
1. Calculate file hash (SHA-256) after a stabilization period
2. Compare with previous known good state
3. Only trigger backup if hash differs significantly
4. Validate file structure matches expected save format

## Performance Optimization

### Scanning Strategy
- **Incremental scans** - Only check directories that have changed
- **Cached metadata** - Store file timestamps and sizes to avoid re-scanning
- **Background processing** - Low-priority threads for initial detection
- **Selective monitoring** - Focus on games user actually plays

### Resource Management
- **Memory limits** - Cap directory cache at 50MB
- **CPU throttling** - Reduce scan frequency during active gaming
- **Network efficiency** - Batch API calls and cache responses
- **Storage optimization** - Compress historical detection logs

## Error Handling

### Detection Failures
- **Permission errors** - Retry with elevated permissions or skip gracefully
- **Corrupted registry** - Fall back to heuristic scanning
- **Network timeouts** - Use cached data with reduced confidence
- **Malformed files** - Validate against known save file signatures

### Recovery Mechanisms
- **Backup detection methods** - Multiple ways to find same saves
- **User override capability** - Manual location specification
- **Automatic retry logic** - Exponential backoff for failed operations
- **Comprehensive logging** - Detailed error reporting for debugging

## Privacy Considerations

### Data Collection
- **Local-only processing** - No save content leaves user's machine
- **Minimal metadata** - Only collect file paths and timestamps
- **User consent** - Explicit permission for cloud sync features
- **Encrypted storage** - All detection metadata encrypted at rest

### Security Measures
- **Sandbox detection** - Run file operations in restricted context
- **Malware scanning** - Validate executables before analysis
- **Access control** - Respect file system permissions
- **Audit logging** - Track all detection activities for security review

## Integration Points

### Save Steward Core
The detection system integrates with:
- **Database layer** - Stores discovered locations with confidence scores
- **Backup engine** - Triggers save archiving when changes detected
- **UI components** - Shows detection status and allows manual override
- **Sync service** - Shares locations across user devices

### External Services
- **PCGamingWiki API** - Community-maintained save location database
- **Steam Web API** - Official game metadata and cloud status
- **Epic Online Services** - Epic Games Store integration
- **GOG Galaxy API** - GOG game library information

## Future Enhancements

### Machine Learning Integration
- **Pattern recognition** - Learn user-specific save behaviors
- **Predictive detection** - Anticipate save locations for new games
- **Anomaly detection** - Identify unusual save file modifications
- **Confidence optimization** - Improve scoring based on user feedback

### Community Features
- **Crowd-sourced locations** - Share anonymized detection results
- **Game developer integration** - Official save location APIs
- **Cross-platform sync** - Unified save management across devices
- **Advanced analytics** - Gaming habit insights with privacy controls