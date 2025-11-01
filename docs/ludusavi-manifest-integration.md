# Ludusavi Manifest Integration for Save Steward

## Overview

The Ludusavi Manifest integration provides programmatic access to a comprehensive database of game save locations compiled from PCGamingWiki data. This integration serves as a primary data source for Save Steward's save location detection system, offering structured save path information for over 10,000 games across multiple platforms and distribution services.

## What is Ludusavi Manifest?

The Ludusavi Manifest is a YAML-based database that compiles save location information from PCGamingWiki and other sources. It provides:
- **10,000+ games** with documented save locations
- **Cross-platform support** (Windows, Linux, macOS)
- **Multi-store coverage** (Steam, Epic, GOG, Microsoft Store, etc.)
- **Standardized path formats** with placeholder support
- **Regular updates** from community-maintained sources

### Manifest Structure

```yaml
# Example manifest entry
"123456":  # Steam App ID
  name: "Example Game"
  files:
    "<winAppData>":
      - "GameFolder/saves/"
      - "GameFolder/settings.ini"
    "<winLocalAppData>":
      - "GameFolder/"
  registry:
    "HKEY_CURRENT_USER\\Software\\GameDeveloper\\ExampleGame":
      - "SavePath"
```

## Integration Architecture

### Primary Manifest URL
- **Main Manifest**: `https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest`
- **Update Frequency**: Daily to weekly (automatically cached locally)
- **Format**: YAML with UTF-8 encoding
- **Size**: ~15MB compressed, ~50MB uncompressed

### Fallback Sources
- **Backup Manifest**: `https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml`
- **Historical Versions**: Available via GitHub releases
- **Local Cache**: Persistent storage for offline access

## Path Placeholder System

The manifest uses standardized placeholders for common system paths:

### Windows Placeholders
- `<base>` - Game installation directory
- `<winAppData>` - `%APPDATA%` (Roaming)
- `<winLocalAppData>` - `%LOCALAPPDATA%` (Local)
- `<winPublic>` - `%PUBLIC%` (Public documents)
- `<winDocuments>` - `%USERPROFILE%\Documents`
- `<winProgramData>` - `%PROGRAMDATA%`
- `<winSaveGames>` - `%USERPROFILE%\Saved Games`
- `<winHome>` - `%USERPROFILE%`
- `<storeUserId>` - Platform-specific user ID
- `<storeRoot>` - Platform installation root

### Linux Placeholders
- `<base>` - Game installation directory
- `<home>` - `$HOME` (User home directory)
- `<xdgData>` - `$XDG_DATA_HOME` or `~/.local/share`
- `<xdgConfig>` - `$XDG_CONFIG_HOME` or `~/.config`
- `<storeUserId>` - Platform-specific user ID

### macOS Placeholders
- `<base>` - Game installation directory
- `<home>` - `$HOME` (User home directory)
- `<osxApplicationSupport>` - `~/Library/Application Support`
- `<osxPreferences>` - `~/Library/Preferences`
- `<osxSaveGames>` - `~/Documents/Save Games`
- `<storeUserId>` - Platform-specific user ID

## Integration Implementation

### Download and Cache Strategy

```python
# Example manifest download implementation
import requests
import yaml
import hashlib
from pathlib import Path
from datetime import datetime, timedelta

class LudusaviManifest:
    def __init__(self, cache_dir: Path):
        self.cache_dir = cache_dir
        self.manifest_path = cache_dir / "manifest.yaml"
        self.manifest_url = "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest"
        self.cache_duration = timedelta(days=7)
    
    def get_manifest(self) -> dict:
        """Get cached or fresh manifest data"""
        if self._is_cache_valid():
            return self._load_cached_manifest()
        else:
            return self._download_and_cache_manifest()
    
    def _is_cache_valid(self) -> bool:
        """Check if cached manifest is still valid"""
        if not self.manifest_path.exists():
            return False
        
        cache_age = datetime.now() - datetime.fromtimestamp(self.manifest_path.stat().st_mtime)
        return cache_age < self.cache_duration
    
    def _download_and_cache_manifest(self) -> dict:
        """Download fresh manifest and cache locally"""
        try:
            response = requests.get(self.manifest_url, timeout=30)
            response.raise_for_status()
            
            # Validate manifest format
            manifest_data = yaml.safe_load(response.text)
            
            # Cache the manifest
            self.cache_dir.mkdir(parents=True, exist_ok=True)
            with open(self.manifest_path, 'w', encoding='utf-8') as f:
                f.write(response.text)
            
            return manifest_data
        except Exception as e:
            # Fallback to cached version if available
            if self.manifest_path.exists():
                return self._load_cached_manifest()
            raise e
```

### Path Resolution System

```python
import os
from pathlib import Path
from typing import List, Dict, Optional

class PathResolver:
    def __init__(self):
        self.platform_placeholders = {
            'windows': {
                '<base>': lambda game_id: self._get_game_install_dir(game_id),
                '<winAppData>': lambda: os.environ.get('APPDATA', ''),
                '<winLocalAppData>': lambda: os.environ.get('LOCALAPPDATA', ''),
                '<winDocuments>': lambda: Path.home() / 'Documents',
                '<winSaveGames>': lambda: Path.home() / 'Saved Games',
                '<winHome>': lambda: Path.home(),
                '<storeUserId>': lambda: self._get_store_user_id(),
            },
            'linux': {
                '<base>': lambda game_id: self._get_game_install_dir(game_id),
                '<home>': lambda: Path.home(),
                '<xdgData>': lambda: Path(os.environ.get('XDG_DATA_HOME', Path.home() / '.local/share')),
                '<xdgConfig>': lambda: Path(os.environ.get('XDG_CONFIG_HOME', Path.home() / '.config')),
                '<storeUserId>': lambda: self._get_store_user_id(),
            },
            'macos': {
                '<base>': lambda game_id: self._get_game_install_dir(game_id),
                '<home>': lambda: Path.home(),
                '<osxApplicationSupport>': lambda: Path.home() / 'Library/Application Support',
                '<osxPreferences>': lambda: Path.home() / 'Library/Preferences',
                '<osxSaveGames>': lambda: Path.home() / 'Documents/Save Games',
                '<storeUserId>': lambda: self._get_store_user_id(),
            }
        }
    
    def resolve_paths(self, game_id: str, manifest_entry: dict) -> List[Path]:
        """Resolve all save paths for a game from manifest entry"""
        platform = self._detect_platform()
        resolved_paths = []
        
        # Handle files section
        if 'files' in manifest_entry:
            for placeholder, paths in manifest_entry['files'].items():
                base_path = self._resolve_placeholder(placeholder, platform, game_id)
                if base_path:
                    for path in paths:
                        resolved_path = Path(base_path) / path
                        resolved_paths.append(resolved_path)
        
        return resolved_paths
    
    def _resolve_placeholder(self, placeholder: str, platform: str, game_id: str) -> Optional[Path]:
        """Resolve a single placeholder to actual path"""
        platform_resolvers = self.platform_placeholders.get(platform, {})
        
        if placeholder in platform_resolvers:
            resolver = platform_resolvers[placeholder]
            if callable(resolver):
                # Handle resolvers that need game_id
                if resolver.__code__.co_argcount > 0:
                    result = resolver(game_id)
                else:
                    result = resolver()
                
                if result:
                    return Path(result)
        
        return None
```

## Integration with Save Steward

### Detection Pipeline Integration

```python
class ManifestDetectionEngine:
    def __init__(self, manifest: LudusaviManifest, resolver: PathResolver):
        self.manifest = manifest
        self.resolver = resolver
        self.confidence_scores = {
            'manifest_direct': 85,
            'manifest_resolved': 75,
            'manifest_partial': 60,
        }
    
    def detect_save_locations(self, game_id: str, game_name: str) -> List[Dict]:
        """Detect save locations using Ludusavi Manifest"""
        results = []
        
        # Get manifest data
        manifest_data = self.manifest.get_manifest()
        
        # Try direct Steam App ID match
        if game_id in manifest_data:
            entry = manifest_data[game_id]
            paths = self.resolver.resolve_paths(game_id, entry)
            
            for path in paths:
                if path.exists():
                    results.append({
                        'path': str(path),
                        'confidence': self.confidence_scores['manifest_direct'],
                        'source': 'ludusavi_manifest',
                        'game_id': game_id,
                        'verified': True
                    })
        
        # Fallback to name-based search
        if not results:
            name_matches = self._search_by_name(game_name, manifest_data)
            for match in name_matches:
                paths = self.resolver.resolve_paths(match['id'], match['entry'])
                for path in paths:
                    if path.exists():
                        results.append({
                            'path': str(path),
                            'confidence': self.confidence_scores['manifest_resolved'],
                            'source': 'ludusavi_manifest_name_match',
                            'game_id': match['id'],
                            'verified': True
                        })
        
        return results
    
    def _search_by_name(self, game_name: str, manifest_data: dict) -> List[Dict]:
        """Search manifest by game name (fuzzy matching)"""
        matches = []
        
        for game_id, entry in manifest_data.items():
            if 'name' in entry:
                # Simple fuzzy matching (can be enhanced with more sophisticated algorithms)
                name_similarity = self._calculate_name_similarity(game_name, entry['name'])
                if name_similarity > 0.8:  # 80% similarity threshold
                    matches.append({
                        'id': game_id,
                        'entry': entry,
                        'similarity': name_similarity
                    })
        
        return sorted(matches, key=lambda x: x['similarity'], reverse=True)
```

## Error Handling and Fallbacks

### Network Error Handling

```python
class ManifestErrorHandler:
    def __init__(self):
        self.max_retries = 3
        self.retry_delay = 5  # seconds
        self.fallback_sources = [
            "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest",
            "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml",
        ]
    
    def handle_download_failure(self, error: Exception, attempt: int) -> bool:
        """Handle download failures with retry logic"""
        if attempt < self.max_retries:
            # Exponential backoff
            delay = self.retry_delay * (2 ** attempt)
            time.sleep(delay)
            return True  # Retry
        
        return False  # Give up after max retries
    
    def get_fallback_manifest(self) -> Optional[dict]:
        """Try alternative sources for manifest data"""
        for source in self.fallback_sources:
            try:
                response = requests.get(source, timeout=30)
                if response.status_code == 200:
                    return yaml.safe_load(response.text)
            except Exception:
                continue
        
        return None
```

### Data Validation

```python
class ManifestValidator:
    def validate_manifest_entry(self, entry: dict) -> bool:
        """Validate manifest entry structure"""
        required_fields = ['name']
        valid_fields = ['name', 'files', 'registry', 'installDir']
        
        # Check required fields
        for field in required_fields:
            if field not in entry:
                return False
        
        # Validate field types
        if 'files' in entry and not isinstance(entry['files'], dict):
            return False
        
        if 'registry' in entry and not isinstance(entry['registry'], dict):
            return False
        
        return True
    
    def validate_path_patterns(self, paths: List[str]) -> bool:
        """Validate path patterns for security and correctness"""
        dangerous_patterns = [
            '..',  # Directory traversal
            '~',   # Home directory (should be resolved)
            '${',  # Environment variable injection
        ]
        
        for path in paths:
            for pattern in dangerous_patterns:
                if pattern in path:
                    return False
        
        return True
```

## Performance Optimization

### Caching Strategy

```python
class ManifestCache:
    def __init__(self, cache_dir: Path):
        self.cache_dir = cache_dir
        self.manifest_cache = cache_dir / "manifest_cache.json"
        self.path_cache = cache_dir / "path_cache.json"
        self.cache_ttl = 86400  # 24 hours in seconds
    
    def get_cached_paths(self, game_id: str) -> Optional[List[str]]:
        """Get cached paths for a game"""
        if not self.path_cache.exists():
            return None
        
        try:
            with open(self.path_cache, 'r') as f:
                cache_data = json.load(f)
            
            if game_id in cache_data:
                entry = cache_data[game_id]
                if time.time() - entry['timestamp'] < self.cache_ttl:
                    return entry['paths']
        except Exception:
            pass
        
        return None
    
    def cache_paths(self, game_id: str, paths: List[str]):
        """Cache resolved paths for a game"""
        try:
            cache_data = {}
            if self.path_cache.exists():
                with open(self.path_cache, 'r') as f:
                    cache_data = json.load(f)
            
            cache_data[game_id] = {
                'paths': paths,
                'timestamp': time.time()
            }
            
            with open(self.path_cache, 'w') as f:
                json.dump(cache_data, f)
        except Exception:
            pass  # Fail silently for cache operations
```

### Memory Management

```python
class ManifestMemoryManager:
    def __init__(self, max_memory_mb: int = 100):
        self.max_memory_bytes = max_memory_mb * 1024 * 1024
        self.current_manifest = None
        self.manifest_size = 0
    
    def load_manifest_chunked(self, manifest_path: Path) -> dict:
        """Load manifest in chunks to manage memory usage"""
        # For very large manifests, implement streaming YAML parser
        # This is a simplified version for moderate-sized manifests
        
        if self.current_manifest is None:
            with open(manifest_path, 'r', encoding='utf-8') as f:
                self.current_manifest = yaml.safe_load(f)
                self.manifest_size = len(str(self.current_manifest))
        
        return self.current_manifest
    
    def unload_manifest(self):
        """Unload manifest from memory to free resources"""
        self.current_manifest = None
        self.manifest_size = 0
        gc.collect()  # Force garbage collection
```

## Privacy and Security

### Data Sanitization

```python
class ManifestPrivacyFilter:
    def sanitize_paths_for_logging(self, paths: List[str]) -> List[str]:
        """Sanitize paths for logging to protect user privacy"""
        sanitized = []
        
        for path in paths:
            # Replace user-specific paths with placeholders
            sanitized_path = path
            
            # Replace home directory
            home_dir = str(Path.home())
            if home_dir in sanitized_path:
                sanitized_path = sanitized_path.replace(home_dir, '<HOME>')
            
            # Replace username in paths
            username = os.environ.get('USERNAME', os.environ.get('USER', ''))
            if username and username in sanitized_path:
                sanitized_path = sanitized_path.replace(username, '<USER>')
            
            sanitized.append(sanitized_path)
        
        return sanitized
    
    def validate_manifest_integrity(self, manifest_data: dict) -> bool:
        """Validate manifest hasn't been tampered with"""
        # Check for suspicious patterns that might indicate tampering
        suspicious_patterns = [
            'eval(',
            'exec(',
            '__import__',
            'subprocess',
            'os.system',
        ]
        
        manifest_str = str(manifest_data)
        for pattern in suspicious_patterns:
            if pattern in manifest_str:
                return False
        
        return True
```

### Network Security

```python
class ManifestSecurityManager:
    def __init__(self):
        self.verified_checksums = {
            'manifest': 'sha256:expected_checksum_here',  # Updated with actual checksums
        }
        self.allowed_origins = [
            'raw.githubusercontent.com',
            'github.com',
        ]
    
    def verify_manifest_checksum(self, manifest_data: bytes) -> bool:
        """Verify manifest integrity using checksums"""
        import hashlib
        
        calculated_checksum = hashlib.sha256(manifest_data).hexdigest()
        expected_checksum = self.verified_checksums.get('manifest', '').replace('sha256:', '')
        
        return calculated_checksum == expected_checksum
    
    def validate_url_origin(self, url: str) -> bool:
        """Validate URL origin is from trusted source"""
        from urllib.parse import urlparse
        
        parsed = urlparse(url)
        return parsed.netloc in self.allowed_origins
```

## Integration Benefits

### Advantages Over Manual Detection
- **Comprehensive Coverage**: 10,000+ games vs. manual discovery
- **Standardized Format**: Consistent path resolution across platforms
- **Community Maintained**: Regular updates from PCGamingWiki contributors
- **Cross-Platform**: Unified approach for Windows, Linux, macOS
- **Multi-Store Support**: Covers Steam, Epic, GOG, and other platforms

### Confidence Scoring Integration
- **Direct Matches**: 85-95 confidence for exact Steam App ID matches
- **Name-Based Matches**: 70-80 confidence for fuzzy name matching
- **Partial Matches**: 50-70 confidence for incomplete data
- **Verified Paths**: Additional +10 confidence when paths exist on disk

### Fallback Strategy
1. **Primary**: Ludusavi Manifest with exact Steam App ID
2. **Secondary**: Ludusavi Manifest with name-based matching
3. **Tertiary**: Platform API detection (Steam, Epic, GOG)
4. **Quaternary**: Registry and heuristic scanning
5. **Final**: User manual specification

## Future Enhancements

### Machine Learning Integration
- **Pattern Recognition**: Learn from successful detections to improve matching
- **Predictive Detection**: Anticipate save locations for new games
- **Confidence Optimization**: Adjust scoring based on user feedback
- **Anomaly Detection**: Identify unusual or suspicious save locations

### Community Features
- **Contribution Feedback**: Report successful/unsuccessful detections
- **Missing Game Detection**: Identify games not in manifest
- **Path Validation**: Community verification of save locations
- **Update Notifications**: Alert users to manifest updates

### Performance Improvements
- **Streaming Parser**: Handle large manifests without loading entirely into memory
- **Incremental Updates**: Only download changed portions of manifest
- **Parallel Processing**: Multi-threaded path resolution
- **Smart Caching**: Predictive caching based on user gaming patterns