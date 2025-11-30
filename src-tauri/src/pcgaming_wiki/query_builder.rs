use url::Url;

const BASE_URL: &str = "https://www.pcgamingwiki.com/w/api.php";

pub struct QueryBuilder;

impl QueryBuilder {
    pub fn build_search_query(name: &str, limit: usize) -> Result<String, url::ParseError> {
        let mut url = Url::parse(BASE_URL)?;
        
        // Sanitize input for LIKE query
        let safe_name = name.replace("\"", "\\\"");
        let where_clause = format!("_pageName LIKE \"%{}%\"", safe_name);

        url.query_pairs_mut()
            .append_pair("action", "cargoquery")
            .append_pair("tables", "Infobox_game")
            // Removed _pageName from fields - API doesn't allow it
            // Removed Developers and Modes - not needed
            .append_pair("fields", "Steam_AppID,Publishers,Released,Genres")
            .append_pair("where", &where_clause)
            .append_pair("limit", &limit.to_string())
            .append_pair("format", "json");

        Ok(url.to_string())
    }

    pub fn build_save_location_query(game_name: &str) -> Result<String, url::ParseError> {
        let mut url = Url::parse(BASE_URL)?;
        
        let where_clause = format!("_pageName=\"{}\"", game_name);

        url.query_pairs_mut()
            .append_pair("action", "cargoquery")
            .append_pair("tables", "Save_game_data")
            .append_pair("fields", "_pageName,Windows,Linux,macOS,Steam_Play")
            .append_pair("where", &where_clause)
            .append_pair("format", "json");

        Ok(url.to_string())
    }

    pub fn build_wikitext_query(page_name: &str) -> Result<String, url::ParseError> {
        let mut url = Url::parse(BASE_URL)?;
        
        url.query_pairs_mut()
            .append_pair("action", "parse")
            .append_pair("page", page_name)
            .append_pair("prop", "wikitext")
            .append_pair("format", "json");

        Ok(url.to_string())
    }
}
