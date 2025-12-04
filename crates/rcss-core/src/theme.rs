use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Theme {
    pub spacing: HashMap<String, String>,
    pub colors: HashMap<String, HashMap<String, String>>,
    pub font_size: HashMap<String, FontSizeEntry>,
}

#[derive(Debug, Deserialize)]
pub struct FontSizeEntry {
    pub size: String,

    #[serde(rename = "lineHeight")]
    pub line_height: String,
}

impl Theme {
    pub fn load(path: &str) -> Result<Self, String> {
        let data =
            std::fs::read_to_string(path).map_err(|e| format!("Could not read {}: {}", path, e))?;

        serde_json::from_str(&data).map_err(|e| format!("Invalid JSON in {}: {}", path, e))
    }
}
