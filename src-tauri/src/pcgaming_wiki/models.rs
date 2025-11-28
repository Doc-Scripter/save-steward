use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PcgwGameInfo {
    #[serde(rename = "Steam AppID")]
    pub steam_appid: Option<String>,
    #[serde(rename = "Publishers")]
    pub publishers: Option<String>,
    #[serde(rename = "Released")]
    pub released: Option<String>,
    #[serde(rename = "Genres")]
    pub genres: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PcgwSaveGameData {
    #[serde(rename = "_pageName")]
    pub page_name: String,
    #[serde(rename = "Windows")]
    pub windows: Option<String>,
    #[serde(rename = "Linux")]
    pub linux: Option<String>,
    #[serde(rename = "macOS")]
    pub macos: Option<String>,
    #[serde(rename = "Steam_Play")]
    pub steam_play: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CargoQueryResponse<T> {
    pub cargoquery: Vec<CargoQueryResult<T>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CargoQueryResult<T> {
    pub title: T,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameSearchResult {
    pub name: String,
    pub steam_id: Option<String>,
    pub publishers: Option<String>,
    pub cover_image_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveLocationResult {
    pub windows: Vec<String>,
    pub linux: Vec<String>,
    pub macos: Vec<String>,
    pub steam_play: Vec<String>,
}
