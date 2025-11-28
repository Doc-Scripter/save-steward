# PCGamingWiki API Usage Guide

## Overview
This document provides comprehensive guidance on using the PCGamingWiki (PGWK) API to retrieve game metadata and save file locations.

## API Endpoints

### Base URL
```
https://www.pcgamingwiki.com/w/api.php
```

## Game Metadata (Cargo API)

### Available Fields
The `Infobox_game` table provides the following fields:
- `Steam_AppID` - Steam application IDs (comma-separated for DLCs)
- `Developers` - Game developer(s)
- `Publishers` - Game publisher(s)
- `Released` - Release date (YYYY-MM-DD format)
- `Genres` - Game genres (comma-separated)
- `Modes` - Game modes (Singleplayer, Multiplayer, etc.)

### Search by Game Name

**Example: The Witcher 3**
```bash
curl -s "https://www.pcgamingwiki.com/w/api.php?\
action=cargoquery&\
tables=Infobox_game&\
fields=Steam_AppID,Developers,Publishers,Released,Genres,Modes&\
where=_pageName%20LIKE%20%22%25Witcher%203%25%22&\
limit=10&\
format=json"
```

**Response**:
```json
{
  "cargoquery": [
    {
      "title": {
        "Steam AppID": "292030,355880,370000,...",
        "Developers": "Company:CD Projekt Red",
        "Publishers": "Company:CD Projekt",
        "Released": "2015-05-19",
        "Genres": "Action,RPG,Open world,",
        "Modes": "Singleplayer"
      }
    }
  ]
}
```

### Search by Exact Page Name

**Example: Cyberpunk 2077**
```bash
curl -s "https://www.pcgamingwiki.com/w/api.php?\
action=cargoquery&\
tables=Infobox_game&\
fields=Steam_AppID,Developers,Publishers&\
where=_pageName=%22Cyberpunk%202077%22&\
format=json"
```

### Important Limitations

❌ **Cannot use `_pageName` in fields list**
```bash
# This will return an error:
fields=_pageName,Steam_AppID
# Error: "Field alias starts with underscore"
```

✅ **Can use `_pageName` in WHERE clause**
```bash
# This works:
where=_pageName LIKE "%Game Name%"
where=_pageName="Exact Game Name"
```

## Save Locations (Wikitext API)

Save file locations are stored in wikitext templates, not in Cargo tables. You must use the `parse` action to retrieve them.

### Fetch Wikitext

**Example: The Witcher 3**
```bash
curl -s "https://www.pcgamingwiki.com/w/api.php?\
action=parse&\
page=The_Witcher_3:_Wild_Hunt&\
prop=wikitext&\
format=json"
```

### Extract Save Locations

The wikitext contains templates like:
```wikitext
{{Game data/saves|Windows|{{p|userprofile\Documents}}\The Witcher 3\gamesaves\*.sav}}
{{Game data/config|Windows|{{p|userprofile\Documents}}\The Witcher 3\*.settings}}
```

**Parse with grep**:
```bash
curl -s "https://www.pcgamingwiki.com/w/api.php?\
action=parse&\
page=The_Witcher_3:_Wild_Hunt&\
prop=wikitext&\
format=json" | \
jq -r '.parse.wikitext."*"' | \
grep -E "{{Game data/(saves|config)"
```

**Output**:
```
{{Game data/config|Windows|{{p|userprofile\Documents}}\The Witcher 3\*.settings}}
{{Game data/saves|Windows|{{p|userprofile\Documents}}\The Witcher 3\gamesaves\*.png|{{p|userprofile\Documents}}\The Witcher 3\gamesaves\*.sav}}
```

### Path Placeholders

Templates use placeholders that need to be resolved:

| Placeholder | Windows | Linux/macOS |
|------------|---------|-------------|
| `{{p|userprofile}}` | `%USERPROFILE%` | `$HOME` |
| `{{p|localappdata}}` | `%LOCALAPPDATA%` | `~/.local/share` |
| `{{p|appdata}}` | `%APPDATA%` | `~/.config` |
| `{{p|osxhome}}` | N/A | `$HOME` |
| `{{p|game}}` | Game install directory | Game install directory |

### Example: Cyberpunk 2077 Save Locations

```bash
curl -s "https://www.pcgamingwiki.com/w/api.php?\
action=parse&\
page=Cyberpunk_2077&\
prop=wikitext&\
format=json" | \
jq -r '.parse.wikitext."*"' | \
grep -E "{{Game data/(saves|config)"
```

**Output**:
```
{{Game data/config|Windows|{{P|localappdata}}\CD Projekt Red\Cyberpunk 2077}}
{{Game data/config|OS X|{{P|osxhome}}/Library/Application Support/CD Projekt Red/Cyberpunk 2077}}
{{Game data/saves|Windows|{{p|userprofile}}\Saved Games\CD Projekt Red\Cyberpunk 2077}}
{{Game data/saves|OS X|{{P|osxhome}}/Library/Application Support/CD Projekt Red/Cyberpunk 2077/saves}}
```

## Platform IDs (From Wikitext)

GOG and Epic IDs are only available in the wikitext, not via Cargo API.

**Extract Platform IDs**:
```bash
curl -s "https://www.pcgamingwiki.com/w/api.php?\
action=parse&\
page=The_Witcher_3:_Wild_Hunt&\
prop=wikitext&\
format=json" | \
jq -r '.parse.wikitext."*"' | \
grep -i "steam appid\|gogcom id\|epic"
```

**Output**:
```
|steam appid  = 292030
|steam appid side = 355880, 370000, 370001, ...
|gogcom id    = 1495134320
|gogcom id side = 1207664663,1207664643,...
```

## Cover Images

Game cover art is available via the wikitext and MediaWiki imageinfo API.

### Extract Cover Filename

**From Wikitext**:
```bash
curl -s "https://www.pcgamingwiki.com/w/api.php?\
action=parse&\
page=The_Witcher_3:_Wild_Hunt&\
prop=wikitext&\
format=json" | \
jq -r '.parse.wikitext."*"' | \
grep -i "^|cover"
```

**Output**:
```
|cover        = The Witcher 3 Wild Hunt - cover.jpg
```

### Get Cover Image URL

**Query imageinfo API**:
```bash
curl -s "https://www.pcgamingwiki.com/w/api.php?\
action=query&\
titles=File:The_Witcher_3_Wild_Hunt_-_cover.jpg&\
prop=imageinfo&\
iiprop=url&\
format=json"
```

**Response**:
```json
{
  "query": {
    "pages": {
      "17681": {
        "title": "File:The Witcher 3 Wild Hunt - cover.jpg",
        "imageinfo": [{
          "url": "https://images.pcgamingwiki.com/a/a4/The_Witcher_3_Wild_Hunt_-_cover.jpg"
        }]
      }
    }
  }
}
```

**Direct URL**: `https://images.pcgamingwiki.com/a/a4/The_Witcher_3_Wild_Hunt_-_cover.jpg`

### Database Storage

Store cover images as binary data:
```sql
CREATE TABLE game_cover_images (
    game_id INTEGER PRIMARY KEY,
    image_url TEXT NOT NULL,
    image_data BLOB,
    last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
);
```

Download and store:
```bash
# Download cover image
curl -s "https://images.pcgamingwiki.com/a/a4/The_Witcher_3_Wild_Hunt_-_cover.jpg" \
  -o cover.jpg

# Store in database as BLOB
```

## Complete Workflow

### 1. Search for Game
```bash
curl -s "https://www.pcgamingwiki.com/w/api.php?\
action=cargoquery&\
tables=Infobox_game&\
fields=Steam_AppID,Publishers,Released,Genres&\
where=_pageName%20LIKE%20%22%25${GAME_NAME}%25%22&\
limit=10&\
format=json"
```

### 2. Get Exact Page Name
From search results, identify the exact page name (e.g., "The_Witcher_3:_Wild_Hunt")

### 3. Fetch Wikitext
```bash
curl -s "https://www.pcgamingwiki.com/w/api.php?\
action=parse&\
page=${PAGE_NAME}&\
prop=wikitext&\
format=json" | \
jq -r '.parse.wikitext."*"'
```

### 4. Parse Templates
Extract `{{Game data/saves}}`, `{{Game data/config}}`, and `|cover` field

### 5. Get Cover Image URL
```bash
curl -s "https://www.pcgamingwiki.com/w/api.php?\
action=query&\
titles=File:${COVER_FILENAME}&\
prop=imageinfo&\
iiprop=url&\
format=json"
```

### 6. Resolve Placeholders
Replace `{{p|...}}` placeholders with actual system paths

### 7. Download and Store Cover Image
Download image from URL and store as BLOB in database

## Database Storage

Store retrieved data in these tables:

**pcgw_game_metadata**:
- `page_name` - Exact PGWK page name
- `steam_appids` - Comma-separated Steam App IDs
- `gog_ids` - GOG IDs (from wikitext)
- `developers`, `publishers`, `release_date`, `genres`, `modes`

**pcgw_save_locations**:
- `platform` - Windows, Linux, OS X
- `location_type` - saves or config
- `path_template` - Path with placeholders
- `resolved_path` - Resolved for current OS
- `file_pattern` - e.g., *.sav, *.png

## Error Handling

### Common Errors

**1. Invalid field name**
```json
{
  "error": {
    "code": "internal_api_error_MWException",
    "info": "No field named \"FIELD_NAME\" found"
  }
}
```
**Solution**: Check available fields list above

**2. Table not found**
```json
{
  "error": {
    "code": "internal_api_error_MWException",
    "info": "Table TABLE_NAME not found"
  }
}
```
**Solution**: Use `Infobox_game` for metadata, `parse` action for save locations

**3. Page not found**
```json
{
  "parse": {
    "error": {
      "code": "missingtitle"
    }
  }
}
```
**Solution**: Verify exact page name, try search first

## Best Practices

1. **Cache responses** - PGWK data changes infrequently
2. **Use exact page names** - More reliable than LIKE queries
3. **Parse wikitext carefully** - Templates can have multiple formats
4. **Handle missing data** - Not all games have all fields populated
5. **Respect rate limits** - Add delays between requests
6. **Store page name** - Needed for future wikitext queries

## Testing

Test your integration with these well-documented games:
- The Witcher 3: Wild Hunt
- Cyberpunk 2077
- Elden Ring
- Baldur's Gate 3
