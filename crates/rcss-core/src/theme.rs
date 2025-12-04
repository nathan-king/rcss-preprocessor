use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Theme {
    pub spacing: HashMap<String, String>,
    pub colors: HashMap<String, HashMap<String, String>>,
    pub font_size: HashMap<String, FontSizeEntry>,
    pub opacity: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct FontSizeEntry {
    pub size: String,

    #[serde(rename = "lineHeight")]
    pub line_height: String,
}

impl Theme {
    /// Load a fully merged theme from multiple JSON files inside a directory.
    ///
    /// Expected structure:
    /// theme/
    ///   spacing.json
    ///   colors.json
    ///   font.json
    ///   opacity.json
    pub fn load_from_dir(dir: &str) -> Result<Self, String> {
        let spacing: HashMap<String, String> = load_json(&format!("{}/spacing.json", dir))?;

        let colors: HashMap<String, HashMap<String, String>> =
            load_json(&format!("{}/colors.json", dir))?;

        let font_size: HashMap<String, FontSizeEntry> = load_json(&format!("{}/font.json", dir))?;

        let opacity: HashMap<String, String> = load_json(&format!("{}/opacity.json", dir))?;

        Ok(Self {
            spacing,
            colors,
            font_size,
            opacity,
        })
    }

    /// Original single-file loader still supported (optional)
    pub fn load(path: &str) -> Result<Self, String> {
        let data =
            fs::read_to_string(path).map_err(|e| format!("Could not read {}: {}", path, e))?;

        serde_json::from_str(&data).map_err(|e| format!("Invalid JSON in {}: {}", path, e))
    }
}

/// Generic JSON loader with type inference
fn load_json<T: for<'de> serde::Deserialize<'de>>(path: &str) -> Result<T, String> {
    let data = fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;

    serde_json::from_str::<T>(&data).map_err(|e| format!("Failed to parse JSON {}: {}", path, e))
}
